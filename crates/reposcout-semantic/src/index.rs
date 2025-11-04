use crate::error::{Result, SemanticError};
use crate::models::{EmbeddingEntry, IndexStats};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info};
use usearch::ffi::{IndexOptions, MetricKind, ScalarKind};
use usearch::Index as USearchIndex;

/// Vector index for semantic search using usearch
pub struct VectorIndex {
    /// usearch index for fast similarity search
    index: USearchIndex,

    /// Mapping from usearch internal ID to repository ID
    id_to_repo: HashMap<u64, String>,

    /// Mapping from repository ID to usearch internal ID
    repo_to_id: HashMap<String, u64>,

    /// Metadata for each repository (source text, timestamps)
    metadata: HashMap<String, EmbeddingEntry>,

    /// Next available ID
    next_id: u64,

    /// Vector dimension
    dimension: usize,

    /// Index statistics
    stats: IndexStats,

    /// Path where index is stored
    index_path: PathBuf,
}

impl VectorIndex {
    /// Create a new empty vector index
    pub fn new(dimension: usize, model_name: String, index_path: PathBuf) -> Result<Self> {
        // Create usearch index with cosine similarity
        let options = IndexOptions {
            dimensions: dimension,
            metric: MetricKind::Cos, // Cosine similarity
            quantization: ScalarKind::F32,
            connectivity: 16, // HNSW connectivity parameter
            expansion_add: 128,
            expansion_search: 64,
        };

        let index = USearchIndex::new(&options).map_err(|e| {
            SemanticError::IndexError(format!("Failed to create usearch index: {}", e))
        })?;

        Ok(Self {
            index,
            id_to_repo: HashMap::new(),
            repo_to_id: HashMap::new(),
            metadata: HashMap::new(),
            next_id: 0,
            dimension,
            stats: IndexStats::new(model_name, dimension),
            index_path,
        })
    }

    /// Get the vector dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Get index statistics
    pub fn stats(&self) -> &IndexStats {
        &self.stats
    }

    /// Add a repository embedding to the index
    pub fn add(&mut self, entry: EmbeddingEntry) -> Result<()> {
        if entry.vector.len() != self.dimension {
            return Err(SemanticError::IndexError(format!(
                "Vector dimension mismatch: expected {}, got {}",
                self.dimension,
                entry.vector.len()
            )));
        }

        let repo_id = entry.repo_id.clone();

        // Check if repository already exists
        if let Some(&existing_id) = self.repo_to_id.get(&repo_id) {
            // Update existing entry
            debug!("Updating existing entry for {}", repo_id);
            self.index
                .update(existing_id, &entry.vector)
                .map_err(|e| SemanticError::IndexError(e.to_string()))?;
        } else {
            // Add new entry
            let id = self.next_id;
            self.index
                .add(id, &entry.vector)
                .map_err(|e| SemanticError::IndexError(e.to_string()))?;

            self.id_to_repo.insert(id, repo_id.clone());
            self.repo_to_id.insert(repo_id.clone(), id);
            self.next_id += 1;
        }

        self.metadata.insert(repo_id, entry);

        Ok(())
    }

    /// Add multiple repository embeddings in batch
    pub fn add_batch(&mut self, entries: Vec<EmbeddingEntry>) -> Result<()> {
        for entry in entries {
            self.add(entry)?;
        }
        Ok(())
    }

    /// Remove a repository from the index
    pub fn remove(&mut self, repo_id: &str) -> Result<()> {
        if let Some(&id) = self.repo_to_id.get(repo_id) {
            self.index
                .remove(id)
                .map_err(|e| SemanticError::IndexError(e.to_string()))?;

            self.id_to_repo.remove(&id);
            self.repo_to_id.remove(repo_id);
            self.metadata.remove(repo_id);

            Ok(())
        } else {
            Err(SemanticError::RepositoryNotFound {
                repo_id: repo_id.to_string(),
            })
        }
    }

    /// Search for similar repositories
    pub fn search(&self, query_vector: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        if query_vector.len() != self.dimension {
            return Err(SemanticError::SearchError(format!(
                "Query vector dimension mismatch: expected {}, got {}",
                self.dimension,
                query_vector.len()
            )));
        }

        // Perform search
        let results = self
            .index
            .search(query_vector, k)
            .map_err(|e| SemanticError::SearchError(e.to_string()))?;

        // Convert results to (repo_id, similarity_score) pairs
        let mut output = Vec::new();
        for result in results.keys.iter().zip(results.distances.iter()) {
            let (id, distance) = result;
            if let Some(repo_id) = self.id_to_repo.get(id) {
                // Convert distance to similarity score
                // For cosine distance: similarity = 1 - distance
                let similarity = 1.0 - distance;
                output.push((repo_id.clone(), similarity));
            }
        }

        Ok(output)
    }

    /// Get metadata for a repository
    pub fn get_metadata(&self, repo_id: &str) -> Option<&EmbeddingEntry> {
        self.metadata.get(repo_id)
    }

    /// Get the number of repositories in the index
    pub fn len(&self) -> usize {
        self.metadata.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.metadata.is_empty()
    }

    /// Check if a repository is in the index
    pub fn contains(&self, repo_id: &str) -> bool {
        self.repo_to_id.contains_key(repo_id)
    }

    /// Get all repository IDs in the index
    pub fn repo_ids(&self) -> Vec<String> {
        self.repo_to_id.keys().cloned().collect()
    }

    /// Save the index to disk
    pub fn save(&mut self) -> Result<()> {
        info!("Saving semantic index to {:?}", self.index_path);

        // Create directory if it doesn't exist
        if let Some(parent) = self.index_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Save usearch index
        let index_file = self.index_path.join("index.usearch");
        self.index
            .save(&index_file.to_string_lossy())
            .map_err(|e| SemanticError::IndexError(format!("Failed to save index: {}", e)))?;

        // Save metadata using MessagePack
        let metadata_file = self.index_path.join("metadata.msgpack");
        let metadata_data = rmp_serde::to_vec(&self.metadata).map_err(|e| {
            SemanticError::SerializationError(format!("Failed to serialize metadata: {}", e))
        })?;
        std::fs::write(&metadata_file, metadata_data)?;

        // Save ID mappings
        let mappings_file = self.index_path.join("mappings.json");
        let mappings = serde_json::json!({
            "id_to_repo": self.id_to_repo,
            "repo_to_id": self.repo_to_id,
            "next_id": self.next_id,
        });
        std::fs::write(&mappings_file, serde_json::to_string_pretty(&mappings)?)?;

        // Update and save stats
        let index_size = Self::calculate_index_size(&self.index_path)?;
        self.stats.update(self.len(), index_size);

        let stats_file = self.index_path.join("stats.json");
        std::fs::write(&stats_file, serde_json::to_string_pretty(&self.stats)?)?;

        info!("Semantic index saved successfully");
        Ok(())
    }

    /// Load the index from disk
    pub fn load(index_path: PathBuf, dimension: usize) -> Result<Self> {
        info!("Loading semantic index from {:?}", index_path);

        if !index_path.exists() {
            return Err(SemanticError::IndexNotFound {
                path: index_path.to_string_lossy().to_string(),
            });
        }

        // Load usearch index
        let index_file = index_path.join("index.usearch");
        if !index_file.exists() {
            return Err(SemanticError::CorruptedIndex);
        }

        let options = IndexOptions {
            dimensions: dimension,
            metric: MetricKind::Cos,
            quantization: ScalarKind::F32,
            connectivity: 16,
            expansion_add: 128,
            expansion_search: 64,
        };

        let index = USearchIndex::new(&options)
            .and_then(|mut idx| {
                idx.load(&index_file.to_string_lossy())?;
                Ok(idx)
            })
            .map_err(|e| SemanticError::IndexError(format!("Failed to load index: {}", e)))?;

        // Load metadata
        let metadata_file = index_path.join("metadata.msgpack");
        if !metadata_file.exists() {
            return Err(SemanticError::CorruptedIndex);
        }
        let metadata_data = std::fs::read(&metadata_file)?;
        let metadata: HashMap<String, EmbeddingEntry> =
            rmp_serde::from_slice(&metadata_data).map_err(|e| {
                SemanticError::SerializationError(format!("Failed to deserialize metadata: {}", e))
            })?;

        // Load ID mappings
        let mappings_file = index_path.join("mappings.json");
        if !mappings_file.exists() {
            return Err(SemanticError::CorruptedIndex);
        }
        let mappings_data = std::fs::read_to_string(&mappings_file)?;
        let mappings: serde_json::Value = serde_json::from_str(&mappings_data)?;

        let id_to_repo: HashMap<u64, String> =
            serde_json::from_value(mappings["id_to_repo"].clone()).map_err(|e| {
                SemanticError::SerializationError(format!(
                    "Failed to deserialize id_to_repo: {}",
                    e
                ))
            })?;

        let repo_to_id: HashMap<String, u64> =
            serde_json::from_value(mappings["repo_to_id"].clone()).map_err(|e| {
                SemanticError::SerializationError(format!(
                    "Failed to deserialize repo_to_id: {}",
                    e
                ))
            })?;

        let next_id: u64 = mappings["next_id"].as_u64().unwrap_or(0);

        // Load stats
        let stats_file = index_path.join("stats.json");
        let stats = if stats_file.exists() {
            let stats_data = std::fs::read_to_string(&stats_file)?;
            serde_json::from_str(&stats_data)?
        } else {
            IndexStats::new("unknown".to_string(), dimension)
        };

        info!("Semantic index loaded successfully: {} repositories", metadata.len());

        Ok(Self {
            index,
            id_to_repo,
            repo_to_id,
            metadata,
            next_id,
            dimension,
            stats,
            index_path,
        })
    }

    /// Calculate total index size on disk
    fn calculate_index_size(path: &Path) -> Result<u64> {
        let mut total_size = 0u64;

        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                if metadata.is_file() {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }

    /// Clear the entire index
    pub fn clear(&mut self) -> Result<()> {
        self.index = {
            let options = IndexOptions {
                dimensions: self.dimension,
                metric: MetricKind::Cos,
                quantization: ScalarKind::F32,
                connectivity: 16,
                expansion_add: 128,
                expansion_search: 64,
            };
            USearchIndex::new(&options)
                .map_err(|e| SemanticError::IndexError(format!("Failed to recreate index: {}", e)))?
        };

        self.id_to_repo.clear();
        self.repo_to_id.clear();
        self.metadata.clear();
        self.next_id = 0;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_vector_index_basic() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().to_path_buf();

        let mut index = VectorIndex::new(3, "test-model".to_string(), index_path).unwrap();

        // Add some test vectors
        let entry1 = EmbeddingEntry::new(
            "github:owner/repo1".to_string(),
            vec![1.0, 0.0, 0.0],
            "test repo 1".to_string(),
        );

        let entry2 = EmbeddingEntry::new(
            "github:owner/repo2".to_string(),
            vec![0.0, 1.0, 0.0],
            "test repo 2".to_string(),
        );

        index.add(entry1).unwrap();
        index.add(entry2).unwrap();

        assert_eq!(index.len(), 2);
        assert!(index.contains("github:owner/repo1"));
        assert!(index.contains("github:owner/repo2"));
    }

    #[test]
    fn test_vector_search() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().to_path_buf();

        let mut index = VectorIndex::new(3, "test-model".to_string(), index_path).unwrap();

        // Add test vectors
        let entry1 = EmbeddingEntry::new(
            "repo1".to_string(),
            vec![1.0, 0.0, 0.0],
            "test 1".to_string(),
        );
        let entry2 = EmbeddingEntry::new(
            "repo2".to_string(),
            vec![0.9, 0.1, 0.0],
            "test 2".to_string(),
        );

        index.add(entry1).unwrap();
        index.add(entry2).unwrap();

        // Search with a similar vector to repo1
        let query = vec![1.0, 0.0, 0.0];
        let results = index.search(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "repo1"); // Most similar should be repo1
        assert!(results[0].1 > results[1].1); // repo1 should have higher similarity
    }
}
