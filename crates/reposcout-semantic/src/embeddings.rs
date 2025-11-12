use crate::error::{Result, SemanticError};
use crate::models::EmbeddingEntry;
use crate::preprocessing::{preprocess_query, preprocess_repository};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use reposcout_core::models::Repository;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Embedding generator using fastembed
pub struct EmbeddingGenerator {
    /// The underlying embedding model
    model: Arc<RwLock<Option<TextEmbedding>>>,

    /// Model name
    model_name: String,

    /// Vector dimension
    dimension: usize,
}

impl EmbeddingGenerator {
    /// Create a new embedding generator (lazy initialization)
    pub fn new(model_name: String) -> Self {
        // Determine dimension based on model
        let dimension = match model_name.as_str() {
            "sentence-transformers/all-MiniLM-L6-v2" => 384,
            "BAAI/bge-small-en-v1.5" => 384,
            "BAAI/bge-base-en-v1.5" => 768,
            _ => 384, // default
        };

        Self {
            model: Arc::new(RwLock::new(None)),
            model_name,
            dimension,
        }
    }

    /// Initialize the model (downloads if needed)
    pub async fn initialize(&self) -> Result<()> {
        let mut model_guard = self.model.write().await;

        if model_guard.is_some() {
            debug!("Embedding model already initialized");
            return Ok(());
        }

        info!("Initializing embedding model: {}", self.model_name);

        // Determine the model enum variant
        let model_type = match self.model_name.as_str() {
            "sentence-transformers/all-MiniLM-L6-v2" => EmbeddingModel::AllMiniLML6V2,
            "BAAI/bge-small-en-v1.5" => EmbeddingModel::BGESmallENV15,
            "BAAI/bge-base-en-v1.5" => EmbeddingModel::BGEBaseENV15,
            _ => {
                warn!(
                    "Unknown model {}, defaulting to all-MiniLM-L6-v2",
                    self.model_name
                );
                EmbeddingModel::AllMiniLML6V2
            }
        };

        // Initialize with options
        let init_options = InitOptions::new(model_type).with_show_download_progress(true);

        let embedding_model = TextEmbedding::try_new(init_options)
            .map_err(|e| SemanticError::ModelLoadError(e.to_string()))?;

        *model_guard = Some(embedding_model);

        info!("Embedding model initialized successfully");
        Ok(())
    }

    /// Get the vector dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Generate embedding for a single text
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        // Ensure model is initialized
        if self.model.read().await.is_none() {
            self.initialize().await?;
        }

        let model_guard = self.model.read().await;
        let model = model_guard
            .as_ref()
            .ok_or(SemanticError::ModelNotInitialized)?;

        // Generate embedding
        let embeddings = model
            .embed(vec![text.to_string()], None)
            .map_err(|e| SemanticError::EmbeddingError(e.to_string()))?;

        if embeddings.is_empty() {
            return Err(SemanticError::EmbeddingError(
                "No embeddings generated".to_string(),
            ));
        }

        Ok(embeddings[0].clone())
    }

    /// Generate embeddings for multiple texts in batch
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        use tracing::{debug, info};

        debug!("embed_batch called with {} texts", texts.len());

        // Ensure model is initialized
        if self.model.read().await.is_none() {
            self.initialize().await?;
        }

        let model_guard = self.model.read().await;
        let model = model_guard
            .as_ref()
            .ok_or(SemanticError::ModelNotInitialized)?;

        // Generate embeddings
        info!("Calling model.embed() for {} texts", texts.len());
        let embeddings = model
            .embed(texts, None)
            .map_err(|e| SemanticError::EmbeddingError(e.to_string()))?;
        info!("model.embed() returned {} embeddings", embeddings.len());

        Ok(embeddings)
    }

    /// Generate embedding for a repository
    pub async fn embed_repository(
        &self,
        repo: &Repository,
        readme: Option<&str>,
    ) -> Result<EmbeddingEntry> {
        // Preprocess repository data
        let source_text = preprocess_repository(repo, readme);

        if source_text.is_empty() {
            return Err(SemanticError::PreprocessingError(
                "No text content to embed".to_string(),
            ));
        }

        // Generate embedding
        let vector = self.embed_text(&source_text).await?;

        // Create repo ID
        let repo_id = format!("{}:{}", repo.platform, repo.full_name);

        Ok(EmbeddingEntry::new(repo_id, vector, source_text))
    }

    /// Generate embeddings for multiple repositories in batch
    pub async fn embed_repositories(
        &self,
        repos: Vec<(&Repository, Option<&str>)>,
    ) -> Result<Vec<EmbeddingEntry>> {
        if repos.is_empty() {
            return Ok(Vec::new());
        }

        // Preprocess all repositories
        let mut source_texts = Vec::new();
        let mut repo_ids = Vec::new();

        for (repo, readme) in &repos {
            let source_text = preprocess_repository(repo, *readme);
            if !source_text.is_empty() {
                source_texts.push(source_text);
                repo_ids.push(format!("{}:{}", repo.platform, repo.full_name));
            }
        }

        if source_texts.is_empty() {
            return Ok(Vec::new());
        }

        // Generate embeddings in batch
        let vectors = self.embed_batch(source_texts.clone()).await?;

        // Create embedding entries
        let mut entries = Vec::new();
        for ((vector, source_text), repo_id) in vectors
            .into_iter()
            .zip(source_texts.into_iter())
            .zip(repo_ids.into_iter())
        {
            entries.push(EmbeddingEntry::new(repo_id, vector, source_text));
        }

        Ok(entries)
    }

    /// Generate embedding for a search query
    pub async fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        // Preprocess query
        let processed_query = preprocess_query(query);

        if processed_query.is_empty() {
            return Err(SemanticError::PreprocessingError(
                "Empty query after preprocessing".to_string(),
            ));
        }

        // Generate embedding
        self.embed_text(&processed_query).await
    }
}

/// Calculate cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();

    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

/// Convert cosine similarity to distance (for consistency with usearch)
pub fn similarity_to_distance(similarity: f32) -> f32 {
    1.0 - similarity
}

/// Convert distance to similarity score
pub fn distance_to_similarity(distance: f32) -> f32 {
    1.0 - distance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.001);

        let d = vec![1.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &d);
        assert!(sim > 0.7 && sim < 0.8); // Should be ~0.707
    }

    #[test]
    fn test_similarity_distance_conversion() {
        let similarity = 0.8;
        let distance = similarity_to_distance(similarity);
        let back_to_similarity = distance_to_similarity(distance);
        assert!((similarity - back_to_similarity).abs() < 0.001);
    }
}
