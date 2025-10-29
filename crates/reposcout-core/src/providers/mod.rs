// Provider implementations for different platforms
pub mod bitbucket;
pub mod github;
pub mod gitlab;

pub use bitbucket::BitbucketProvider;
pub use github::GitHubProvider;
pub use gitlab::GitLabProvider;
