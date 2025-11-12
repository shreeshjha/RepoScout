use thiserror::Error;

/// Result type for semantic search operations
pub type Result<T> = std::result::Result<T, SemanticError>;

/// Errors that can occur during semantic search operations
#[derive(Error, Debug)]
pub enum SemanticError {
    #[error("Failed to load embedding model: {0}")]
    ModelLoadError(String),

    #[error("Failed to generate embeddings: {0}")]
    EmbeddingError(String),

    #[error("Vector index error: {0}")]
    IndexError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Index not found at {path}")]
    IndexNotFound { path: String },

    #[error("Index is corrupted or invalid")]
    CorruptedIndex,

    #[error("Repository not found in index: {repo_id}")]
    RepositoryNotFound { repo_id: String },

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Text preprocessing failed: {0}")]
    PreprocessingError(String),

    #[error("Search operation failed: {0}")]
    SearchError(String),

    #[error("Model not initialized. Call initialize() first.")]
    ModelNotInitialized,
}
