// API client implementations for various platforms
pub mod github;

// Re-export common types
pub use github::{GitHubClient, GitHubRepo};
