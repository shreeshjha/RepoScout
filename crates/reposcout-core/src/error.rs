use thiserror::Error;

/// All the ways things can go wrong in RepoScout
///
/// We use thiserror here because it generates the boilerplate for us.
/// Life's too short to manually implement Display and Error traits.
#[derive(Error, Debug)]
pub enum Error {
    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Cache operation failed: {0}")]
    CacheError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Repository not found: {0}")]
    NotFound(String),

    #[error("Rate limit exceeded. Try again in {retry_after} seconds")]
    RateLimitExceeded { retry_after: u64 },

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unknown error occurred: {0}")]
    Unknown(String),
}
