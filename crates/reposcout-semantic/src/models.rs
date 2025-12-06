use chrono::{DateTime, Utc};
use reposcout_core::models::Repository;
use serde::{Deserialize, Serialize};

/// Embedding entry for a repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingEntry {
    /// Repository identifier (platform:owner/name)
    pub repo_id: String,

    /// Embedding vector (typically 384 dimensions for all-MiniLM-L6-v2)
    #[serde(skip)]
    pub vector: Vec<f32>,

    /// When this embedding was generated
    pub generated_at: DateTime<Utc>,

    /// Source text that was embedded
    pub source_text: String,

    /// Text hash to detect changes
    pub text_hash: u64,
}

impl EmbeddingEntry {
    /// Create a new embedding entry
    pub fn new(repo_id: String, vector: Vec<f32>, source_text: String) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        source_text.hash(&mut hasher);
        let text_hash = hasher.finish();

        Self {
            repo_id,
            vector,
            generated_at: Utc::now(),
            source_text,
            text_hash,
        }
    }

    /// Check if the source text has changed
    pub fn text_changed(&self, new_text: &str) -> bool {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        new_text.hash(&mut hasher);
        let new_hash = hasher.finish();

        new_hash != self.text_hash
    }
}

/// Semantic search result with scores
#[derive(Debug, Clone)]
pub struct SemanticSearchResult {
    /// Repository information
    pub repository: Repository,

    /// Semantic similarity score (0.0-1.0, cosine similarity)
    pub semantic_score: f32,

    /// Traditional keyword score (0.0-1.0, normalized)
    pub keyword_score: Option<f32>,

    /// Combined hybrid score (0.0-1.0)
    pub hybrid_score: f32,

    /// Distance in vector space (lower is better)
    pub distance: f32,
}

impl SemanticSearchResult {
    /// Create a semantic-only result
    pub fn semantic_only(repository: Repository, semantic_score: f32, distance: f32) -> Self {
        Self {
            repository,
            semantic_score,
            keyword_score: None,
            hybrid_score: semantic_score,
            distance,
        }
    }

    /// Create a hybrid result combining semantic and keyword scores
    pub fn hybrid(
        repository: Repository,
        semantic_score: f32,
        keyword_score: f32,
        semantic_weight: f32,
        distance: f32,
    ) -> Self {
        let hybrid_score =
            (semantic_score * semantic_weight) + (keyword_score * (1.0 - semantic_weight));

        Self {
            repository,
            semantic_score,
            keyword_score: Some(keyword_score),
            hybrid_score,
            distance,
        }
    }
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Total number of repositories indexed
    pub total_repositories: usize,

    /// Index size in bytes
    pub index_size_bytes: u64,

    /// Last time the index was updated
    pub last_updated: DateTime<Utc>,

    /// Embedding model name
    pub model_name: String,

    /// Vector dimension
    pub dimension: usize,

    /// Index creation time
    pub created_at: DateTime<Utc>,
}

impl IndexStats {
    /// Create new index stats
    pub fn new(model_name: String, dimension: usize) -> Self {
        Self {
            total_repositories: 0,
            index_size_bytes: 0,
            last_updated: Utc::now(),
            model_name,
            dimension,
            created_at: Utc::now(),
        }
    }

    /// Update stats after indexing
    pub fn update(&mut self, repo_count: usize, size_bytes: u64) {
        self.total_repositories = repo_count;
        self.index_size_bytes = size_bytes;
        self.last_updated = Utc::now();
    }
}

/// Configuration for semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticConfig {
    /// Enable semantic search
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Embedding model to use
    #[serde(default = "default_model")]
    pub model: String,

    /// Auto-build index on startup
    #[serde(default = "default_auto_build")]
    pub index_auto_build: bool,

    /// Weight for semantic score in hybrid search (0.0-1.0)
    #[serde(default = "default_semantic_weight")]
    pub semantic_weight: f32,

    /// Minimum similarity threshold
    #[serde(default = "default_min_similarity")]
    pub min_similarity: f32,

    /// Maximum results to return
    #[serde(default = "default_max_results")]
    pub max_results: usize,

    /// Cache path
    #[serde(default = "default_cache_path")]
    pub cache_path: String,

    /// Maximum cache size in MB
    #[serde(default = "default_max_cache_size")]
    pub max_cache_size_mb: usize,
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            model: default_model(),
            index_auto_build: default_auto_build(),
            semantic_weight: default_semantic_weight(),
            min_similarity: default_min_similarity(),
            max_results: default_max_results(),
            cache_path: default_cache_path(),
            max_cache_size_mb: default_max_cache_size(),
        }
    }
}

// Default value functions
fn default_enabled() -> bool {
    true
}

fn default_model() -> String {
    "BAAI/bge-small-en-v1.5".to_string()
}

fn default_auto_build() -> bool {
    true
}

fn default_semantic_weight() -> f32 {
    0.6
}

fn default_min_similarity() -> f32 {
    0.5
}

fn default_max_results() -> usize {
    50
}

fn default_cache_path() -> String {
    dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(".cache"))
        .join("reposcout")
        .join("semantic")
        .to_string_lossy()
        .to_string()
}

fn default_max_cache_size() -> usize {
    500
}
