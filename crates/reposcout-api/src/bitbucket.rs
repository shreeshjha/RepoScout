use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::retry::{is_retryable_status, with_retry, RetryConfig};

const BITBUCKET_API_BASE: &str = "https://api.bitbucket.org/2.0";

#[derive(Error, Debug)]
pub enum BitbucketError {
    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Repository not found: {0}")]
    NotFound(String),

    #[error("Authentication required")]
    AuthRequired,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("JSON parsing failed: {0}")]
    ParseError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, BitbucketError>;

pub struct BitbucketClient {
    client: reqwest::Client,
    username: Option<String>,
    app_password: Option<String>,
    base_url: String,
    retry_config: RetryConfig,
}

impl BitbucketClient {
    pub fn new(username: Option<String>, app_password: Option<String>) -> Self {
        Self::with_base_url(username, app_password, BITBUCKET_API_BASE.to_string())
    }

    /// For Bitbucket Server/Data Center or testing with custom API URL
    pub fn with_base_url(
        username: Option<String>,
        app_password: Option<String>,
        base_url: String,
    ) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("RepoScout/0.1.0"),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            username,
            app_password,
            base_url,
            retry_config: RetryConfig::default(),
        }
    }

    /// Create client with custom retry configuration
    pub fn with_retry_config(
        username: Option<String>,
        app_password: Option<String>,
        retry_config: RetryConfig,
    ) -> Self {
        let mut client = Self::new(username, app_password);
        client.retry_config = retry_config;
        client
    }

    /// Create Basic Auth header value
    fn basic_auth_header(&self) -> Option<String> {
        match (&self.username, &self.app_password) {
            (Some(username), Some(password)) => {
                let credentials = format!("{}:{}", username, password);
                let encoded = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    credentials.as_bytes(),
                );
                Some(format!("Basic {}", encoded))
            }
            _ => None,
        }
    }

    /// Search repositories on Bitbucket
    ///
    /// Note: Bitbucket API has limitations - it doesn't support global public repository search
    /// like GitHub. This method will return an empty list for now. To search Bitbucket repositories,
    /// you need workspace-specific access or use the workspace search endpoint.
    pub async fn search_repositories(
        &self,
        _query: &str,
        _per_page: u32,
    ) -> Result<Vec<BitbucketRepository>> {
        // Bitbucket doesn't support global public repository search without workspace access
        // Return empty results to avoid errors while keeping the integration functional
        Ok(Vec::new())
    }

    /// Get detailed info about a specific repository
    pub async fn get_repository(&self, workspace: &str, repo_slug: &str) -> Result<BitbucketRepository> {
        let url = format!("{}/repositories/{}/{}", self.base_url, workspace, repo_slug);
        let auth_header = self.basic_auth_header();
        let full_name = format!("{}/{}", workspace, repo_slug);

        with_retry(&self.retry_config, || async {
            let mut request = self.client.get(&url);

            if let Some(ref auth) = auth_header {
                request = request.header(reqwest::header::AUTHORIZATION, auth);
            }

            let response = request.send().await?;

            if response.status() == 404 {
                return Err(BitbucketError::NotFound(full_name.clone()));
            }

            if response.status() == 401 {
                return Err(BitbucketError::AuthRequired);
            }

            let status = response.status();

            if status.is_client_error() && !is_retryable_status(status) {
                return Err(BitbucketError::RequestFailed(format!(
                    "Failed to fetch repo: {}",
                    status
                )));
            }

            if !response.status().is_success() {
                return Err(BitbucketError::RequestFailed(format!(
                    "Failed to fetch repo: {}",
                    status
                )));
            }

            let repo: BitbucketRepository = response.json().await?;
            Ok(repo)
        })
        .await
    }

    /// Get repository README content
    pub async fn get_readme(&self, workspace: &str, repo_slug: &str) -> Result<String> {
        // Try common README file names
        for readme_name in &["README.md", "README.MD", "readme.md", "README", "README.rst"] {
            let url = format!(
                "{}/repositories/{}/{}/src/HEAD/{}",
                self.base_url, workspace, repo_slug, readme_name
            );
            let auth_header = self.basic_auth_header();

            let result = with_retry(&self.retry_config, || async {
                let mut request = self.client.get(&url);

                if let Some(ref auth) = auth_header {
                    request = request.header(reqwest::header::AUTHORIZATION, auth);
                }

                let response = request.send().await?;

                if response.status() == 404 {
                    return Err(BitbucketError::NotFound(format!("{}/{}", workspace, repo_slug)));
                }

                if response.status() == 401 {
                    return Err(BitbucketError::AuthRequired);
                }

                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    return Err(BitbucketError::RequestFailed(format!(
                        "Status {}: {}",
                        status, body
                    )));
                }

                let readme_content = response.text().await?;
                Ok(readme_content)
            })
            .await;

            // If we found the README, return it
            if result.is_ok() {
                return result;
            }
        }

        // None of the common names worked
        Err(BitbucketError::NotFound(format!(
            "README not found for {}/{}",
            workspace, repo_slug
        )))
    }

    /// Get file content from repository
    pub async fn get_file_content(
        &self,
        workspace: &str,
        repo_slug: &str,
        path: &str,
    ) -> Result<String> {
        let url = format!(
            "{}/repositories/{}/{}/src/HEAD/{}",
            self.base_url, workspace, repo_slug, path
        );
        let auth_header = self.basic_auth_header();

        with_retry(&self.retry_config, || async {
            let mut request = self.client.get(&url);

            if let Some(ref auth) = auth_header {
                request = request.header(reqwest::header::AUTHORIZATION, auth);
            }

            let response = request.send().await?;

            if response.status() == 404 {
                return Err(BitbucketError::NotFound(format!(
                    "{} not found in {}/{}",
                    path, workspace, repo_slug
                )));
            }

            if response.status() == 401 {
                return Err(BitbucketError::AuthRequired);
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(BitbucketError::RequestFailed(format!(
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
    pub async fn get_cargo_toml(&self, workspace: &str, repo_slug: &str) -> Result<String> {
        self.get_file_content(workspace, repo_slug, "Cargo.toml")
            .await
    }

    /// Get package.json for Node.js projects
    pub async fn get_package_json(&self, workspace: &str, repo_slug: &str) -> Result<String> {
        self.get_file_content(workspace, repo_slug, "package.json")
            .await
    }

    /// Get requirements.txt for Python projects
    pub async fn get_requirements_txt(&self, workspace: &str, repo_slug: &str) -> Result<String> {
        self.get_file_content(workspace, repo_slug, "requirements.txt")
            .await
    }

    /// Search for code across Bitbucket repositories
    /// Note: Bitbucket's code search API is limited compared to GitHub
    pub async fn search_code(
        &self,
        workspace: &str,
        repo_slug: &str,
        query: &str,
    ) -> Result<Vec<CodeSearchItem>> {
        let url = format!(
            "{}/repositories/{}/{}/search/code",
            self.base_url, workspace, repo_slug
        );
        let auth_header = self.basic_auth_header();

        with_retry(&self.retry_config, || async {
            let mut request = self.client.get(&url).query(&[("search_query", query)]);

            if let Some(ref auth) = auth_header {
                request = request.header(reqwest::header::AUTHORIZATION, auth);
            }

            let response = request.send().await?;

            if response.status() == 404 {
                return Err(BitbucketError::NotFound(query.to_string()));
            }

            if response.status() == 401 {
                return Err(BitbucketError::AuthRequired);
            }

            if response.status() == 429 {
                return Err(BitbucketError::RateLimitExceeded);
            }

            let status = response.status();

            if status.is_client_error() && !is_retryable_status(status) {
                let body = response.text().await.unwrap_or_default();
                return Err(BitbucketError::RequestFailed(format!(
                    "Status {}: {}",
                    status, body
                )));
            }

            if !response.status().is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(BitbucketError::RequestFailed(format!(
                    "Status {}: {}",
                    status, body
                )));
            }

            let search_result: CodeSearchResponse = response.json().await?;
            Ok(search_result.values)
        })
        .await
    }
}

/// Bitbucket API repository search response
#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[allow(dead_code)]
    values: Vec<BitbucketRepository>,
    #[serde(default)]
    #[allow(dead_code)]
    next: Option<String>,
}

/// Bitbucket API code search response
#[derive(Debug, Deserialize)]
struct CodeSearchResponse {
    values: Vec<CodeSearchItem>,
    #[serde(default)]
    #[allow(dead_code)]
    next: Option<String>,
}

/// Bitbucket code search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchItem {
    #[serde(default)]
    pub path_matches: Vec<PathMatch>,
    #[serde(default)]
    pub content_matches: Vec<ContentMatch>,
    pub file: FileInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathMatch {
    pub text: String,
    #[serde(default)]
    pub match_: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMatch {
    pub lines: Vec<LineMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineMatch {
    pub line: u32,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub text: String,
    #[serde(default, rename = "match")]
    pub match_: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    #[serde(rename = "type")]
    pub file_type: String,
}

/// Bitbucket repository representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitbucketRepository {
    pub uuid: String,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub is_private: bool,
    pub links: Links,
    pub created_on: DateTime<Utc>,
    pub updated_on: DateTime<Utc>,
    pub size: Option<u64>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub has_issues: bool,
    pub mainbranch: Option<MainBranch>,
    pub workspace: Workspace,
    pub owner: Owner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Links {
    pub html: Link,
    #[serde(default)]
    pub avatar: Option<Link>,
    #[serde(default)]
    pub clone: Option<Vec<CloneLink>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub href: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneLink {
    pub href: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainBranch {
    pub name: String,
    #[serde(rename = "type")]
    pub branch_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub slug: String,
    pub name: String,
    pub uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub display_name: String,
    pub uuid: String,
    pub username: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = BitbucketClient::new(None, None);
        assert!(client.username.is_none());
        assert!(client.app_password.is_none());
        assert_eq!(client.base_url, BITBUCKET_API_BASE);
    }

    #[test]
    fn test_client_with_credentials() {
        let username = "test_user".to_string();
        let password = "test_password".to_string();
        let client = BitbucketClient::new(Some(username.clone()), Some(password.clone()));
        assert_eq!(client.username, Some(username));
        assert_eq!(client.app_password, Some(password));
    }

    #[test]
    fn test_basic_auth_header() {
        let client = BitbucketClient::new(
            Some("testuser".to_string()),
            Some("testpass".to_string()),
        );
        let auth_header = client.basic_auth_header();
        assert!(auth_header.is_some());
        assert!(auth_header.unwrap().starts_with("Basic "));
    }
}
