// Semantic search for RepoScout
//
// This crate provides semantic search capabilities using embedding models
// and vector similarity search. It enables natural language queries and
// finding repositories by use case rather than just keywords.

pub mod bm25;
pub mod embeddings;
pub mod error;
pub mod index;
pub mod models;
pub mod preprocessing;
pub mod search;

// Re-export main types
pub use bm25::{score_keyword_results, BM25Scorer};
pub use embeddings::{cosine_similarity, EmbeddingGenerator};
pub use error::{Result, SemanticError};
pub use index::VectorIndex;
pub use models::{EmbeddingEntry, IndexStats, SemanticConfig, SemanticSearchResult};
pub use preprocessing::{preprocess_query, preprocess_repository};
pub use search::SemanticSearchEngine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_imports() {
        // Just verify that all modules compile and export correctly
        let _ = SemanticConfig::default();
    }
}
