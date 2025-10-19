use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
        }
    }

    /// Search repositories on GitHub
    pub async fn search_repositories(&self, query: &str, per_page: u32) -> Result<Vec<GitHubRepo>> {
        let url = format!("{}/search/repositories", self.base_url);

        let mut request = self.client.get(&url).query(&[
            ("q", query),
            ("per_page", &per_page.to_string()),
            ("sort", "stars"),
        ]);

        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await?;

        // Check rate limit before processing response
        self.check_rate_limit(&response)?;

        if response.status() == 404 {
            return Err(GitHubError::NotFound(query.to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(GitHubError::RequestFailed(format!(
                "Status {}: {}",
                status, body
            )));
        }

        let search_result: SearchResponse = response.json().await?;
        Ok(search_result.items)
    }

    /// Get detailed info about a specific repository
    pub async fn get_repository(&self, owner: &str, repo: &str) -> Result<GitHubRepo> {
        let url = format!("{}/repos/{}/{}", self.base_url, owner, repo);

        let mut request = self.client.get(&url);

        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await?;
        self.check_rate_limit(&response)?;

        if response.status() == 404 {
            return Err(GitHubError::NotFound(format!("{}/{}", owner, repo)));
        }

        if !response.status().is_success() {
            let status = response.status();
            return Err(GitHubError::RequestFailed(format!(
                "Failed to fetch repo: {}",
                status
            )));
        }

        let repo: GitHubRepo = response.json().await?;
        Ok(repo)
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

/// GitHub API search response
#[derive(Debug, Deserialize)]
struct SearchResponse {
    items: Vec<GitHubRepo>,
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
