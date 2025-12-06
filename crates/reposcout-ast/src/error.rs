use thiserror::Error;

#[derive(Error, Debug)]
pub enum AstError {
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Tree-sitter error: {0}")]
    TreeSitterError(String),

    #[error("Extraction failed: {0}")]
    ExtractionError(String),

    #[error("Query parsing error: {0}")]
    QueryParseError(String),

    #[error("Timeout while parsing (exceeded {timeout_ms}ms)")]
    ParseTimeout { timeout_ms: u64 },

    #[error("File too large: {size} bytes (max: {max})")]
    FileTooLarge { size: usize, max: usize },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AstError>;
