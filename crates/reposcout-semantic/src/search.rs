use crate::embeddings::EmbeddingGenerator;
use crate::error::Result;
use crate::index::VectorIndex;
use crate::models::{IndexStats, SemanticConfig, SemanticSearchResult};
use reposcout_core::models::Repository;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Semantic search engine
pub struct SemanticSearchEngine {
    /// Embedding generator
    embedder: Arc<EmbeddingGenerator>,

    /// Vector index
    index: Arc<RwLock<VectorIndex>>,

    /// Configuration
    config: SemanticConfig,

    /// Repository cache for quick lookup
    repo_cache: Arc<RwLock<HashMap<String, Repository>>>,
}

impl SemanticSearchEngine {
    /// Create a new semantic search engine
    pub fn new(config: SemanticConfig) -> Result<Self> {
        let embedder = Arc::new(EmbeddingGenerator::new(config.model.clone()));

        let index_path = PathBuf::from(&config.cache_path);

        // Try to load existing index, or create new one
        let index = match VectorIndex::load(index_path.clone(), embedder.dimension()) {
            Ok(idx) => {
                info!("Loaded existing semantic index");
                idx
            }
            Err(e) => {
                warn!("Could not load existing index: {}. Creating new one.", e);
                VectorIndex::new(embedder.dimension(), config.model.clone(), index_path)?
            }
        };

        Ok(Self {
            embedder,
            index: Arc::new(RwLock::new(index)),
            config,
            repo_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Initialize the embedding model
    pub async fn initialize(&self) -> Result<()> {
        self.embedder.initialize().await
    }

    /// Index a single repository
    pub async fn index_repository(&self, repo: &Repository, readme: Option<&str>) -> Result<()> {
        debug!("Indexing repository: {}", repo.full_name);

        // Generate embedding
        let entry = self.embedder.embed_repository(repo, readme).await?;

        // Add to index
        let mut index = self.index.write().await;
        index.add(entry)?;

        // Cache repository
        let repo_id = format!("{}:{}", repo.platform, repo.full_name);
        self.repo_cache.write().await.insert(repo_id, repo.clone());

        Ok(())
    }

    /// Index multiple repositories in batch
    pub async fn index_repositories(
        &self,
        repos: Vec<(Repository, Option<String>)>,
    ) -> Result<usize> {
        if repos.is_empty() {
            return Ok(0);
        }

        info!("Indexing {} repositories...", repos.len());

        // Prepare for batch embedding
        debug!("Preparing repository references for embedding");
        let repo_refs: Vec<(&Repository, Option<&str>)> = repos
            .iter()
            .map(|(repo, readme)| (repo, readme.as_deref()))
            .collect();
        debug!("Prepared {} repository references", repo_refs.len());

        // Generate embeddings in batch
        info!("Generating embeddings for {} repositories", repo_refs.len());
        let entries = self.embedder.embed_repositories(repo_refs).await?;
        info!("Generated {} embeddings", entries.len());

        // Add to index
        info!("Adding {} entries to vector index", entries.len());
        let mut index = self.index.write().await;
        index.add_batch(entries)?;
        info!("Added entries to index successfully");

        // Cache repositories
        debug!("Caching repositories");
        let mut cache = self.repo_cache.write().await;
        for (repo, _) in &repos {
            let repo_id = format!("{}:{}", repo.platform, repo.full_name);
            cache.insert(repo_id, repo.clone());
        }

        info!("Successfully indexed {} repositories", repos.len());
        Ok(repos.len())
    }

    /// Perform semantic search
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SemanticSearchResult>> {
        debug!("Semantic search query: {}", query);

        // Generate query embedding
        let query_vector = self.embedder.embed_query(query).await?;

        // Search in vector index
        let index = self.index.read().await;
        let raw_results = index.search(&query_vector, limit)?;

        // Filter by minimum similarity threshold
        let filtered_results: Vec<_> = raw_results
            .into_iter()
            .filter(|(_, score)| *score >= self.config.min_similarity)
            .collect();

        debug!("Found {} results above threshold", filtered_results.len());

        // Convert to search results
        let cache = self.repo_cache.read().await;
        let mut results = Vec::new();

        for (repo_id, similarity) in filtered_results {
            if let Some(repo) = cache.get(&repo_id) {
                let distance = 1.0 - similarity;
                results.push(SemanticSearchResult::semantic_only(
                    repo.clone(),
                    similarity,
                    distance,
                ));
            } else {
                warn!("Repository {} not found in cache", repo_id);
            }
        }

        // Sort by similarity score (descending)
        results.sort_by(|a, b| {
            b.semantic_score
                .partial_cmp(&a.semantic_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        results.truncate(self.config.max_results.min(limit));

        Ok(results)
    }

    /// Perform hybrid search (combining semantic and keyword scores)
    pub async fn hybrid_search(
        &self,
        query: &str,
        keyword_results: Vec<(Repository, f32)>,
        limit: usize,
    ) -> Result<Vec<SemanticSearchResult>> {
        debug!("Hybrid search query: {}", query);

        // First, index the keyword results if they aren't already indexed
        let repos_to_index: Vec<(Repository, Option<String>)> = keyword_results
            .iter()
            .map(|(repo, _)| (repo.clone(), None))
            .collect();

        if !repos_to_index.is_empty() {
            info!(
                "Indexing {} keyword results for semantic search",
                repos_to_index.len()
            );
            self.index_repositories(repos_to_index).await?;
        }

        // Perform semantic search
        let semantic_results = self.search(query, limit * 2).await?;

        // Create a map of repo_id to semantic score
        let mut semantic_map: HashMap<String, f32> = HashMap::new();
        for result in &semantic_results {
            let repo_id = format!(
                "{}:{}",
                result.repository.platform, result.repository.full_name
            );
            semantic_map.insert(repo_id, result.semantic_score);
        }

        // Create a map of repo_id to keyword score (normalized)
        let max_keyword_score = keyword_results
            .iter()
            .map(|(_, score)| *score)
            .fold(0.0f32, f32::max);

        let mut keyword_map: HashMap<String, f32> = HashMap::new();
        for (repo, score) in &keyword_results {
            let repo_id = format!("{}:{}", repo.platform, repo.full_name);
            let normalized_score = if max_keyword_score > 0.0 {
                score / max_keyword_score
            } else {
                *score
            };
            keyword_map.insert(repo_id, normalized_score);
        }

        // Combine results
        let mut all_repo_ids: std::collections::HashSet<String> =
            semantic_map.keys().cloned().collect();
        all_repo_ids.extend(keyword_map.keys().cloned());

        let mut hybrid_results = Vec::new();
        let cache = self.repo_cache.read().await;

        for repo_id in all_repo_ids {
            if let Some(repo) = cache.get(&repo_id) {
                let semantic_score = semantic_map.get(&repo_id).copied().unwrap_or(0.0);
                let keyword_score = keyword_map.get(&repo_id).copied().unwrap_or(0.0);

                // Calculate distance (for semantic-only results)
                let distance = 1.0 - semantic_score;

                let result = SemanticSearchResult::hybrid(
                    repo.clone(),
                    semantic_score,
                    keyword_score,
                    self.config.semantic_weight,
                    distance,
                );

                hybrid_results.push(result);
            }
        }

        // Sort by hybrid score (descending)
        hybrid_results.sort_by(|a, b| {
            b.hybrid_score
                .partial_cmp(&a.hybrid_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        hybrid_results.truncate(limit);

        debug!("Hybrid search returned {} results", hybrid_results.len());

        Ok(hybrid_results)
    }

    /// Check if a repository is indexed
    pub async fn is_indexed(&self, repo_id: &str) -> bool {
        let index = self.index.read().await;
        index.contains(repo_id)
    }

    /// Remove a repository from the index
    pub async fn remove_repository(&self, repo_id: &str) -> Result<()> {
        let mut index = self.index.write().await;
        index.remove(repo_id)?;

        self.repo_cache.write().await.remove(repo_id);

        Ok(())
    }

    /// Get index statistics
    pub async fn stats(&self) -> IndexStats {
        let index = self.index.read().await;
        index.stats().clone()
    }

    /// Get the number of indexed repositories
    pub async fn indexed_count(&self) -> usize {
        let index = self.index.read().await;
        index.len()
    }

    /// Save the index to disk
    pub async fn save(&self) -> Result<()> {
        let mut index = self.index.write().await;
        index.save()
    }

    /// Clear the entire index
    pub async fn clear(&self) -> Result<()> {
        let mut index = self.index.write().await;
        index.clear()?;

        self.repo_cache.write().await.clear();

        Ok(())
    }

    /// Rebuild the index from scratch
    pub async fn rebuild(&self, repos: Vec<(Repository, Option<String>)>) -> Result<usize> {
        info!("Rebuilding semantic index...");

        // Clear existing index
        self.clear().await?;

        // Index all repositories
        let count = self.index_repositories(repos).await?;

        // Save to disk
        self.save().await?;

        info!("Index rebuild complete: {} repositories", count);

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reposcout_core::models::Platform;
    use tempfile::TempDir;

    fn create_test_repo(name: &str, description: &str) -> Repository {
        Repository {
            platform: Platform::GitHub,
            full_name: name.to_string(),
            description: Some(description.to_string()),
            url: format!("https://github.com/{}", name),
            homepage_url: None,
            stars: 100,
            forks: 10,
            watchers: 50,
            open_issues: 5,
            language: Some("Rust".to_string()),
            topics: vec!["rust".to_string(), "cli".to_string()],
            license: Some("MIT".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            pushed_at: chrono::Utc::now(),
            size: 1024,
            default_branch: "main".to_string(),
            is_archived: false,
            is_private: false,
            health: None,
        }
    }

    #[tokio::test]
    async fn test_semantic_search_basic() {
        let temp_dir = TempDir::new().unwrap();

        let config = SemanticConfig {
            enabled: true,
            cache_path: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };

        let engine = SemanticSearchEngine::new(config).unwrap();
        engine.initialize().await.unwrap();

        // Index some test repositories
        let repo1 = create_test_repo("user/logging-lib", "A logging library for Rust");
        let repo2 = create_test_repo("user/web-framework", "A web framework for building APIs");

        engine.index_repository(&repo1, None).await.unwrap();
        engine.index_repository(&repo2, None).await.unwrap();

        // Search
        let results = engine.search("logging library", 10).await.unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].repository.full_name, "user/logging-lib");
    }
}
