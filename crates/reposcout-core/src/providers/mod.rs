// Provider implementations for different platforms
pub mod github;
pub mod gitlab;

pub use github::GitHubProvider;
pub use gitlab::GitLabProvider;
