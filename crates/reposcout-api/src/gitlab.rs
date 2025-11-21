use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::retry::{is_retryable_status, with_retry, RetryConfig};

const GITLAB_API_BASE: &str = "https://gitlab.com/api/v4";

#[derive(Error, Debug)]
pub enum GitLabError {
    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Project not found: {0}")]
    NotFound(String),

    #[error("Authentication required")]
    AuthRequired,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("JSON parsing failed: {0}")]
    ParseError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, GitLabError>;

pub struct GitLabClient {
    client: reqwest::Client,
    token: Option<String>,
    base_url: String,
    retry_config: RetryConfig,
}

impl GitLabClient {
    pub fn new(token: Option<String>) -> Self {
        Self::with_base_url(token, GITLAB_API_BASE.to_string())
    }

    /// For self-hosted GitLab instances
    pub fn with_base_url(token: Option<String>, base_url: String) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("RepoScout/0.1.0"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build HTTP client");

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

    /// Search projects on GitLab
    pub async fn search_projects(&self, query: &str, per_page: u32) -> Result<Vec<GitLabProject>> {
        let url = format!("{}/projects", self.base_url);
        let token = self.token.clone();

        // Wrap in retry logic
        with_retry(&self.retry_config, || async {
            let mut request = self.client.get(&url).query(&[
                ("search", query),
                ("per_page", &per_page.to_string()),
                ("order_by", "star_count"),
                ("sort", "desc"),
            ]);

            if let Some(ref token) = token {
                request = request.header("PRIVATE-TOKEN", token);
            }

            let response = request.send().await?;

            if response.status() == 404 {
                return Err(GitLabError::NotFound(query.to_string()));
            }

            if response.status() == 401 {
                return Err(GitLabError::AuthRequired);
            }

            if response.status() == 429 {
                return Err(GitLabError::RateLimitExceeded);
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();

                if is_retryable_status(status) {
                    return Err(GitLabError::RequestFailed(format!(
                        "Status {}: {}",
                        status, body
                    )));
                }

                return Err(GitLabError::RequestFailed(format!(
                    "Status {}: {}",
                    status, body
                )));
            }

            let projects: Vec<GitLabProject> = response.json().await?;
            Ok(projects)
        })
        .await
    }

    /// Get project README content
    pub async fn get_readme(&self, path: &str) -> Result<String> {
        // GitLab uses URL-encoded paths
        let encoded_path = urlencoding::encode(path);
        let url = format!(
            "{}/projects/{}/repository/files/README.md/raw",
            self.base_url, encoded_path
        );
        let token = self.token.clone();

        with_retry(&self.retry_config, || async {
            let mut request = self.client.get(&url).query(&[("ref", "HEAD")]);

            if let Some(ref token) = token {
                request = request.header("PRIVATE-TOKEN", token);
            }

            let response = request.send().await?;

            if response.status() == 404 {
                // Try other common README names
                return Err(GitLabError::NotFound(format!(
                    "README not found for {}",
                    path
                )));
            }

            if response.status() == 401 {
                return Err(GitLabError::AuthRequired);
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(GitLabError::RequestFailed(format!(
                    "Status {}: {}",
                    status, body
                )));
            }

            let readme_content = response.text().await?;
            Ok(readme_content)
        })
        .await
    }

    /// Get file content from project repository
    pub async fn get_file_content(&self, path: &str, file_path: &str) -> Result<String> {
        // GitLab uses URL-encoded paths for both project and file
        let encoded_path = urlencoding::encode(path);
        let encoded_file = urlencoding::encode(file_path);
        let url = format!(
            "{}/projects/{}/repository/files/{}/raw",
            self.base_url, encoded_path, encoded_file
        );
        let token = self.token.clone();

        with_retry(&self.retry_config, || async {
            let mut request = self.client.get(&url).query(&[("ref", "HEAD")]);

            if let Some(ref token) = token {
                request = request.header("PRIVATE-TOKEN", token);
            }

            let response = request.send().await?;

            if response.status() == 404 {
                return Err(GitLabError::NotFound(format!(
                    "{} not found in {}",
                    file_path, path
                )));
            }

            if response.status() == 401 {
                return Err(GitLabError::AuthRequired);
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(GitLabError::RequestFailed(format!(
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
    pub async fn get_cargo_toml(&self, path: &str) -> Result<String> {
        self.get_file_content(path, "Cargo.toml").await
    }

    /// Get package.json for Node.js projects
    pub async fn get_package_json(&self, path: &str) -> Result<String> {
        self.get_file_content(path, "package.json").await
    }

    /// Get requirements.txt for Python projects
    pub async fn get_requirements_txt(&self, path: &str) -> Result<String> {
        self.get_file_content(path, "requirements.txt").await
    }

    /// Search for code across GitLab projects
    ///
    /// Uses the GitLab Search API with scope=blobs
    /// Requires authentication for most searches
    pub async fn search_code(
        &self,
        query: &str,
        per_page: u32,
    ) -> Result<Vec<GitLabCodeSearchItem>> {
        let url = format!("{}/search", self.base_url);
        let token = self.token.clone();

        with_retry(&self.retry_config, || async {
            let mut request = self.client.get(&url).query(&[
                ("scope", "blobs"),
                ("search", query),
                ("per_page", &per_page.to_string()),
            ]);

            if let Some(ref token) = token {
                request = request.header("PRIVATE-TOKEN", token);
            }

            let response = request.send().await?;

            if response.status() == 404 {
                return Err(GitLabError::NotFound(query.to_string()));
            }

            if response.status() == 401 {
                return Err(GitLabError::AuthRequired);
            }

            if response.status() == 429 {
                return Err(GitLabError::RateLimitExceeded);
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();

                if is_retryable_status(status) {
                    return Err(GitLabError::RequestFailed(format!(
                        "Status {}: {}",
                        status, body
                    )));
                }

                return Err(GitLabError::RequestFailed(format!(
                    "Status {}: {}",
                    status, body
                )));
            }

            let results: Vec<GitLabCodeSearchItem> = response.json().await?;
            Ok(results)
        })
        .await
    }

    /// Get a specific project by path (e.g., "gitlab-org/gitlab")
    pub async fn get_project(&self, path: &str) -> Result<GitLabProject> {
        // GitLab uses URL-encoded paths
        let encoded_path = urlencoding::encode(path);
        let url = format!("{}/projects/{}", self.base_url, encoded_path);
        let token = self.token.clone();

        with_retry(&self.retry_config, || async {
            let mut request = self.client.get(&url);

            if let Some(ref token) = token {
                request = request.header("PRIVATE-TOKEN", token);
            }

            let response = request.send().await?;

            if response.status() == 404 {
                return Err(GitLabError::NotFound(path.to_string()));
            }

            if response.status() == 401 {
                return Err(GitLabError::AuthRequired);
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(GitLabError::RequestFailed(format!(
                    "Status {}: {}",
                    status, body
                )));
            }

            let project: GitLabProject = response.json().await?;
            Ok(project)
        })
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabProject {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub path_with_namespace: String,
    pub description: Option<String>,
    #[serde(default)]
    pub star_count: u32,
    #[serde(default)]
    pub forks_count: u32,
    #[serde(default, alias = "open_issues_count")]
    pub open_issues: u32,
    pub web_url: String,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub tag_list: Vec<String>,
    #[serde(default)]
    pub visibility: String,
    pub default_branch: Option<String>,
    pub namespace: GitLabNamespace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabNamespace {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub kind: String,
    pub full_path: String,
}

/// GitLab code search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabCodeSearchItem {
    pub basename: String,
    pub data: String,
    pub path: String,
    pub filename: String,
    pub id: Option<u64>,
    pub ref_: Option<String>,
    pub startline: usize,
    pub project_id: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_projects() {
        let client = GitLabClient::new(None);
        let results = client.search_projects("rust", 5).await;

        // Should work without token for public projects
        if let Err(e) = &results {
            eprintln!("GitLab search error: {:?}", e);
        }
        assert!(results.is_ok(), "GitLab search failed: {:?}", results.err());
        let projects = results.unwrap();
        assert!(!projects.is_empty());
    }

    #[tokio::test]
    async fn test_get_project() {
        let client = GitLabClient::new(None);
        let result = client.get_project("gitlab-org/gitlab").await;

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.path_with_namespace, "gitlab-org/gitlab");
    }
}
