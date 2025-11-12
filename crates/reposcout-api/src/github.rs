use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::retry::{is_retryable_status, with_retry, RetryConfig};

const GITHUB_API_BASE: &str = "https://api.github.com";

#[derive(Error, Debug)]
pub enum GitHubError {
    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("Rate limit exceeded. Resets at {reset_at}")]
    RateLimitExceeded { reset_at: DateTime<Utc> },

    #[error("Repository not found: {0}")]
    NotFound(String),

    #[error("Authentication required")]
    AuthRequired,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("JSON parsing failed: {0}")]
    ParseError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, GitHubError>;

pub struct GitHubClient {
    client: reqwest::Client,
    token: Option<String>,
    base_url: String,
    retry_config: RetryConfig,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Self {
        Self::with_base_url(token, GITHUB_API_BASE.to_string())
    }

    /// For GitHub Enterprise or testing with custom API URL
    pub fn with_base_url(token: Option<String>, base_url: String) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("RepoScout/0.1.0"),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/vnd.github.v3+json"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build HTTP client"); // This should never fail

        Self {
            client,
            token,
            base_url,
            retry_config: RetryConfig::default(),
        }
    }

    /// Create client with custom retry configuration
    pub fn with_retry_config(token: Option<String>, retry_config: RetryConfig) -> Self {
        let mut client = Self::new(token);
        client.retry_config = retry_config;
        client
    }

    /// Search repositories on GitHub
    pub async fn search_repositories(&self, query: &str, per_page: u32) -> Result<Vec<GitHubRepo>> {
        let url = format!("{}/search/repositories", self.base_url);
        let token = self.token.clone();

        // Wrap in retry logic
        with_retry(&self.retry_config, || async {
            let mut request = self.client.get(&url).query(&[
                ("q", query),
                ("per_page", &per_page.to_string()),
                ("sort", "stars"),
            ]);

            if let Some(ref token) = token {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;

            // Check rate limit before processing response
            self.check_rate_limit(&response)?;

            if response.status() == 404 {
                return Err(GitHubError::NotFound(query.to_string()));
            }

            let status = response.status();

            // Don't retry client errors (except rate limit which is checked above)
            if status.is_client_error() && !is_retryable_status(status) {
                let body = response.text().await.unwrap_or_default();
                return Err(GitHubError::RequestFailed(format!(
                    "Status {}: {}",
                    status, body
                )));
            }

            // Retry server errors
            if !response.status().is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(GitHubError::RequestFailed(format!(
                    "Status {}: {}",
                    status, body
                )));
            }

            let search_result: SearchResponse = response.json().await?;
            Ok(search_result.items)
        })
        .await
    }

    /// Get repository README content
    pub async fn get_readme(&self, owner: &str, repo: &str) -> Result<String> {
        let url = format!("{}/repos/{}/{}/readme", self.base_url, owner, repo);
        let token = self.token.clone();

        with_retry(&self.retry_config, || async {
            let mut request = self.client.get(&url).header(
                reqwest::header::ACCEPT,
                // Request raw markdown content
                reqwest::header::HeaderValue::from_static("application/vnd.github.raw+json"),
            );

            if let Some(ref token) = token {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;

            self.check_rate_limit(&response)?;

            if response.status() == 404 {
                return Err(GitHubError::NotFound(format!("{}/{}", owner, repo)));
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(GitHubError::RequestFailed(format!(
                    "Status {}: {}",
                    status, body
                )));
            }

            let readme_content = response.text().await?;
            Ok(readme_content)
        })
        .await
    }

    /// Get file content from repository
    pub async fn get_file_content(&self, owner: &str, repo: &str, path: &str) -> Result<String> {
        let url = format!("{}/repos/{}/{}/contents/{}", self.base_url, owner, repo, path);
        let token = self.token.clone();

        with_retry(&self.retry_config, || async {
            let mut request = self.client.get(&url).header(
                reqwest::header::ACCEPT,
                // Request raw content
                reqwest::header::HeaderValue::from_static("application/vnd.github.raw+json"),
            );

            if let Some(ref token) = token {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;

            self.check_rate_limit(&response)?;

            if response.status() == 404 {
                return Err(GitHubError::NotFound(format!("{}/{}/{}", owner, repo, path)));
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(GitHubError::RequestFailed(format!(
                    "Status {}: {}",
                    status, body
                )));
            }

            let content = response.text().await?;
            Ok(content)
        })
        .await
    }

    /// Get Cargo.toml for Rust projects
    pub async fn get_cargo_toml(&self, owner: &str, repo: &str) -> Result<String> {
        self.get_file_content(owner, repo, "Cargo.toml").await
    }

    /// Get package.json for Node.js projects
    pub async fn get_package_json(&self, owner: &str, repo: &str) -> Result<String> {
        self.get_file_content(owner, repo, "package.json").await
    }

    /// Get requirements.txt for Python projects
    pub async fn get_requirements_txt(&self, owner: &str, repo: &str) -> Result<String> {
        self.get_file_content(owner, repo, "requirements.txt").await
    }

    /// Search for code across GitHub repositories
    ///
    /// GitHub Code Search API has special rate limits:
    /// - 20 requests per minute for authenticated users
    /// - 5 requests per minute for unauthenticated
    pub async fn search_code(&self, query: &str, per_page: u32) -> Result<Vec<CodeSearchItem>> {
        let url = format!("{}/search/code", self.base_url);
        let token = self.token.clone();

        with_retry(&self.retry_config, || async {
            let mut request = self.client
                .get(&url)
                .query(&[
                    ("q", query),
                    ("per_page", &per_page.to_string()),
                ])
                // Request text matches to get code snippets
                .header(
                    reqwest::header::ACCEPT,
                    reqwest::header::HeaderValue::from_static("application/vnd.github.text-match+json"),
                );

            if let Some(ref token) = token {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;

            // Check rate limit before processing response
            self.check_rate_limit(&response)?;

            if response.status() == 404 {
                return Err(GitHubError::NotFound(query.to_string()));
            }

            let status = response.status();

            // Handle authentication requirement (401 or 403)
            if status == 401 || (status == 403 && token.is_none()) {
                return Err(GitHubError::AuthRequired);
            }

            // Don't retry client errors
            if status.is_client_error() && !is_retryable_status(status) {
                let body = response.text().await.unwrap_or_default();
                return Err(GitHubError::RequestFailed(format!(
                    "Status {}: {}",
                    status, body
                )));
            }

            // Retry server errors
            if !response.status().is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(GitHubError::RequestFailed(format!(
                    "Status {}: {}",
                    status, body
                )));
            }

            // Get response text for debugging
            let response_text = response.text().await?;
            tracing::debug!("GitHub code search response: {}", &response_text[..response_text.len().min(500)]);

            let search_result: CodeSearchResponse = serde_json::from_str(&response_text)
                .map_err(|e| {
                    tracing::error!("Failed to parse GitHub response: {}", e);
                    tracing::error!("Response snippet: {}", &response_text[..response_text.len().min(1000)]);
                    GitHubError::ParseError(e)
                })?;
            Ok(search_result.items)
        })
        .await
    }

    /// Get detailed info about a specific repository
    pub async fn get_repository(&self, owner: &str, repo: &str) -> Result<GitHubRepo> {
        let url = format!("{}/repos/{}/{}", self.base_url, owner, repo);
        let token = self.token.clone();
        let full_name = format!("{}/{}", owner, repo);

        // Wrap in retry logic
        with_retry(&self.retry_config, || async {
            let mut request = self.client.get(&url);

            if let Some(ref token) = token {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;
            self.check_rate_limit(&response)?;

            if response.status() == 404 {
                return Err(GitHubError::NotFound(full_name.clone()));
            }

            let status = response.status();

            // Don't retry client errors
            if status.is_client_error() && !is_retryable_status(status) {
                return Err(GitHubError::RequestFailed(format!(
                    "Failed to fetch repo: {}",
                    status
                )));
            }

            // Retry server errors
            if !response.status().is_success() {
                return Err(GitHubError::RequestFailed(format!(
                    "Failed to fetch repo: {}",
                    status
                )));
            }

            let repo: GitHubRepo = response.json().await?;
            Ok(repo)
        })
        .await
    }

    /// Get notifications for the authenticated user
    pub async fn get_notifications(
        &self,
        all: bool,           // false = only unread, true = all
        participating: bool, // true = only notifications user is participating in
        per_page: u32,
    ) -> Result<Vec<crate::notifications::Notification>> {
        let url = format!("{}/notifications", self.base_url);
        let token = self.token.clone();

        with_retry(&self.retry_config, || async {
            let mut request = self.client
                .get(&url)
                .query(&[
                    ("all", if all { "true" } else { "false" }),
                    ("participating", if participating { "true" } else { "false" }),
                    ("per_page", &per_page.to_string()),
                ]);

            if let Some(ref token) = token {
                request = request.bearer_auth(token);
            } else {
                return Err(GitHubError::AuthRequired);
            }

            let response = request.send().await?;
            self.check_rate_limit(&response)?;

            let status = response.status();

            if status == 401 {
                return Err(GitHubError::AuthRequired);
            }

            if !response.status().is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(GitHubError::RequestFailed(format!(
                    "Failed to fetch notifications: {}",
                    status
                )));
            }

            let notifications: Vec<crate::notifications::Notification> = response.json().await?;
            Ok(notifications)
        })
        .await
    }

    /// Mark a notification thread as read
    pub async fn mark_notification_read(&self, thread_id: &str) -> Result<()> {
        let url = format!("{}/notifications/threads/{}", self.base_url, thread_id);
        let token = self.token.clone();

        with_retry(&self.retry_config, || async {
            let mut request = self.client.patch(&url);

            if let Some(ref token) = token {
                request = request.bearer_auth(token);
            } else {
                return Err(GitHubError::AuthRequired);
            }

            let response = request.send().await?;
            self.check_rate_limit(&response)?;

            let status = response.status();

            if status == 401 {
                return Err(GitHubError::AuthRequired);
            }

            if !response.status().is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(GitHubError::RequestFailed(format!(
                    "Failed to mark notification as read: {}",
                    status
                )));
            }

            Ok(())
        })
        .await
    }

    /// Mark all notifications as read
    pub async fn mark_all_notifications_read(&self) -> Result<()> {
        let url = format!("{}/notifications", self.base_url);
        let token = self.token.clone();

        with_retry(&self.retry_config, || async {
            let mut request = self.client
                .put(&url)
                .json(&serde_json::json!({"read": true}));

            if let Some(ref token) = token {
                request = request.bearer_auth(token);
            } else {
                return Err(GitHubError::AuthRequired);
            }

            let response = request.send().await?;
            self.check_rate_limit(&response)?;

            let status = response.status();

            if status == 401 {
                return Err(GitHubError::AuthRequired);
            }

            // GitHub returns 205 or 202 for this endpoint
            if status != reqwest::StatusCode::RESET_CONTENT && status != reqwest::StatusCode::ACCEPTED {
                let body = response.text().await.unwrap_or_default();
                return Err(GitHubError::RequestFailed(format!(
                    "Failed to mark all notifications as read: {}",
                    status
                )));
            }

            Ok(())
        })
        .await
    }

    /// Check if we're hitting rate limits and return helpful error
    fn check_rate_limit(&self, response: &reqwest::Response) -> Result<()> {
        if response.status() == 403 {
            // Rate limit hit - GitHub returns 403
            if let Some(reset) = response.headers().get("x-ratelimit-reset") {
                if let Ok(reset_str) = reset.to_str() {
                    if let Ok(reset_timestamp) = reset_str.parse::<i64>() {
                        let reset_at = DateTime::from_timestamp(reset_timestamp, 0)
                            .unwrap_or_else(|| Utc::now());
                        return Err(GitHubError::RateLimitExceeded { reset_at });
                    }
                }
            }
        }
        Ok(())
    }
}

/// GitHub API repository search response
#[derive(Debug, Deserialize)]
struct SearchResponse {
    items: Vec<GitHubRepo>,
}

/// GitHub API code search response
#[derive(Debug, Deserialize)]
struct CodeSearchResponse {
    items: Vec<CodeSearchItem>,
}

/// GitHub code search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchItem {
    pub name: String,
    pub path: String,
    pub sha: String,
    pub url: String,
    pub git_url: String,
    pub html_url: String,
    pub repository: CodeSearchRepository,
    #[serde(default)]
    pub text_matches: Vec<TextMatch>,
}

/// Minimal repository object returned in code search results
/// This is different from the full GitHubRepo - code search returns fewer fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchRepository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub html_url: String,
    pub owner: Owner,
    #[serde(default)]
    pub private: bool,
}

/// Text match containing the actual code snippet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMatch {
    #[serde(default)]
    pub object_url: Option<String>,
    #[serde(default)]
    pub object_type: Option<String>,
    #[serde(default)]
    pub property: Option<String>,
    pub fragment: String,
    #[serde(default)]
    pub matches: Vec<Match>,
}

/// Individual match within a text fragment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub text: String,
    pub indices: Vec<usize>,
}

/// GitHub repository representation
/// Matches the structure GitHub API returns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepo {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub owner: Owner,
    pub description: Option<String>,
    pub html_url: String,
    pub homepage: Option<String>,
    pub stargazers_count: u32,
    pub forks_count: u32,
    pub watchers_count: u32,
    pub open_issues_count: u32,
    pub language: Option<String>,
    pub topics: Vec<String>,
    pub license: Option<License>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pushed_at: DateTime<Utc>,
    pub size: u64,
    pub default_branch: String,
    pub archived: bool,
    pub private: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub login: String,
    pub id: u64,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub key: String,
    pub name: String,
    pub spdx_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = GitHubClient::new(None);
        assert!(client.token.is_none());
        assert_eq!(client.base_url, GITHUB_API_BASE);
    }

    #[test]
    fn test_client_with_token() {
        let token = "ghp_test_token".to_string();
        let client = GitHubClient::new(Some(token.clone()));
        assert_eq!(client.token, Some(token));
    }

    // Integration tests would go here
    // Skipping for now since they require real API access
}
