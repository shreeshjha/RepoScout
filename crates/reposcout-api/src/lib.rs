// API client implementations for various platforms
pub mod bitbucket;
pub mod github;
pub mod gitlab;
pub mod notifications;
pub mod retry;

// Re-export common types
pub use bitbucket::{BitbucketClient, BitbucketRepository};
pub use github::{GitHubClient, GitHubRepo};
pub use gitlab::{GitLabClient, GitLabProject};
pub use notifications::{Notification, NotificationFilters, NotificationReason};
pub use retry::RetryConfig;
