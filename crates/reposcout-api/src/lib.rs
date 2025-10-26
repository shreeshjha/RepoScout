// API client implementations for various platforms
pub mod bitbucket;
pub mod github;
pub mod gitlab;
pub mod retry;

// Re-export common types
pub use bitbucket::{BitbucketClient, BitbucketRepository};
pub use github::{GitHubClient, GitHubRepo};
pub use gitlab::{GitLabClient, GitLabProject};
pub use retry::RetryConfig;
