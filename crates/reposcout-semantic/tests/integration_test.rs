use reposcout_core::models::{Platform, Repository};
use reposcout_semantic::{
    EmbeddingGenerator, SemanticConfig, SemanticSearchEngine, VectorIndex,
};
use tempfile::TempDir;

fn create_test_repo(name: &str, description: &str, language: &str) -> Repository {
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
        language: Some(language.to_string()),
        topics: vec!["test".to_string()],
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
async fn test_embedding_generator_initialization() {
    let generator = EmbeddingGenerator::new("sentence-transformers/all-MiniLM-L6-v2".to_string());
    assert_eq!(generator.dimension(), 384);

    // Initialize the model
    let result = generator.initialize().await;
    assert!(result.is_ok(), "Model initialization failed: {:?}", result);
}

#[tokio::test]
async fn test_embed_text() {
    let generator = EmbeddingGenerator::new("sentence-transformers/all-MiniLM-L6-v2".to_string());
    generator.initialize().await.unwrap();

    let text = "This is a test sentence for embedding generation";
    let embedding = generator.embed_text(text).await.unwrap();

    assert_eq!(embedding.len(), 384);
    // Check that embedding is not all zeros
    assert!(embedding.iter().any(|&x| x != 0.0));
}

#[tokio::test]
async fn test_embed_repository() {
    let generator = EmbeddingGenerator::new("sentence-transformers/all-MiniLM-L6-v2".to_string());
    generator.initialize().await.unwrap();

    let repo = create_test_repo(
        "user/test-repo",
        "A test repository for unit testing",
        "Rust",
    );

    let entry = generator.embed_repository(&repo, None).await.unwrap();

    assert_eq!(entry.repo_id, "GitHub:user/test-repo");
    assert_eq!(entry.vector.len(), 384);
    assert!(entry.source_text.contains("test-repo"));
    assert!(entry.source_text.contains("test repository"));
}

#[tokio::test]
async fn test_embed_batch() {
    let generator = EmbeddingGenerator::new("sentence-transformers/all-MiniLM-L6-v2".to_string());
    generator.initialize().await.unwrap();

    let texts = vec![
        "First test sentence".to_string(),
        "Second test sentence".to_string(),
        "Third test sentence".to_string(),
    ];

    let embeddings = generator.embed_batch(texts).await.unwrap();

    assert_eq!(embeddings.len(), 3);
    for embedding in &embeddings {
        assert_eq!(embedding.len(), 384);
    }
}

#[tokio::test]
async fn test_vector_index_basic() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().to_path_buf();

    let mut index =
        VectorIndex::new(384, "test-model".to_string(), index_path.clone()).unwrap();

    let generator = EmbeddingGenerator::new("sentence-transformers/all-MiniLM-L6-v2".to_string());
    generator.initialize().await.unwrap();

    // Create and add some test repositories
    let repo1 = create_test_repo("user/repo1", "A logging library for Rust", "Rust");
    let entry1 = generator.embed_repository(&repo1, None).await.unwrap();

    let repo2 = create_test_repo("user/repo2", "A web framework for building APIs", "Rust");
    let entry2 = generator.embed_repository(&repo2, None).await.unwrap();

    index.add(entry1).unwrap();
    index.add(entry2).unwrap();

    assert_eq!(index.len(), 2);
    assert!(index.contains("GitHub:user/repo1"));
    assert!(index.contains("GitHub:user/repo2"));
}

#[tokio::test]
async fn test_vector_search() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().to_path_buf();

    let mut index =
        VectorIndex::new(384, "test-model".to_string(), index_path.clone()).unwrap();

    let generator = EmbeddingGenerator::new("sentence-transformers/all-MiniLM-L6-v2".to_string());
    generator.initialize().await.unwrap();

    // Create test repositories
    let repos = vec![
        create_test_repo("user/logger", "A logging library for Rust applications", "Rust"),
        create_test_repo("user/webfw", "A modern web framework for Rust", "Rust"),
        create_test_repo("user/parser", "A JSON parser written in Rust", "Rust"),
    ];

    // Add to index
    for repo in &repos {
        let entry = generator.embed_repository(repo, None).await.unwrap();
        index.add(entry).unwrap();
    }

    // Search for logging-related repos
    let query = "logging library";
    let query_vector = generator.embed_query(query).await.unwrap();
    let results = index.search(&query_vector, 3).unwrap();

    assert_eq!(results.len(), 3);
    // The logging library should be most similar
    assert!(results[0].0.contains("logger"));
    assert!(results[0].1 > 0.5, "Similarity score too low");
}

#[tokio::test]
async fn test_index_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().to_path_buf();

    let generator = EmbeddingGenerator::new("sentence-transformers/all-MiniLM-L6-v2".to_string());
    generator.initialize().await.unwrap();

    // Create and save index
    {
        let mut index =
            VectorIndex::new(384, "test-model".to_string(), index_path.clone()).unwrap();

        let repo = create_test_repo("user/test", "A test repository", "Rust");
        let entry = generator.embed_repository(&repo, None).await.unwrap();

        index.add(entry).unwrap();
        index.save().unwrap();

        assert_eq!(index.len(), 1);
    }

    // Load index from disk
    {
        let index = VectorIndex::load(index_path, 384).unwrap();
        assert_eq!(index.len(), 1);
        assert!(index.contains("GitHub:user/test"));

        let stats = index.stats();
        assert_eq!(stats.total_repositories, 1);
        assert_eq!(stats.dimension, 384);
    }
}

#[tokio::test]
async fn test_semantic_search_engine() {
    let temp_dir = TempDir::new().unwrap();

    let config = SemanticConfig {
        enabled: true,
        cache_path: temp_dir.path().to_string_lossy().to_string(),
        min_similarity: 0.3,
        max_results: 10,
        ..Default::default()
    };

    let engine = SemanticSearchEngine::new(config).unwrap();
    engine.initialize().await.unwrap();

    // Index some test repositories
    let repos = vec![
        (
            create_test_repo(
                "user/serde",
                "A serialization framework for Rust",
                "Rust",
            ),
            None,
        ),
        (
            create_test_repo("user/tokio", "An async runtime for Rust", "Rust"),
            None,
        ),
        (
            create_test_repo(
                "user/actix",
                "A powerful web framework for Rust",
                "Rust",
            ),
            None,
        ),
    ];

    engine.index_repositories(repos).await.unwrap();

    // Perform semantic search
    let results = engine.search("web framework", 5).await.unwrap();

    assert!(!results.is_empty());
    assert!(results[0].semantic_score > 0.0);

    // actix should be most relevant for "web framework"
    assert!(results[0].repository.full_name.contains("actix"));
}

#[tokio::test]
async fn test_semantic_search_with_readme() {
    let temp_dir = TempDir::new().unwrap();

    let config = SemanticConfig {
        enabled: true,
        cache_path: temp_dir.path().to_string_lossy().to_string(),
        ..Default::default()
    };

    let engine = SemanticSearchEngine::new(config).unwrap();
    engine.initialize().await.unwrap();

    let readme = r#"
# FastLogger

FastLogger is a high-performance logging library for Rust applications.

## Features
- Zero-cost abstractions
- Async logging support
- Multiple output formats (JSON, plain text)
- Configurable log levels
"#;

    let repo = create_test_repo(
        "user/fastlogger",
        "High-performance logging for Rust",
        "Rust",
    );

    engine
        .index_repository(&repo, Some(readme))
        .await
        .unwrap();

    // Search should find the repo based on README content
    let results = engine
        .search("zero cost async logging", 5)
        .await
        .unwrap();

    assert!(!results.is_empty());
    assert!(results[0].repository.full_name.contains("fastlogger"));
}

#[tokio::test]
async fn test_index_update() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().to_path_buf();

    let generator = EmbeddingGenerator::new("sentence-transformers/all-MiniLM-L6-v2".to_string());
    generator.initialize().await.unwrap();

    let mut index =
        VectorIndex::new(384, "test-model".to_string(), index_path.clone()).unwrap();

    // Add initial version
    let repo = create_test_repo("user/test", "Initial description", "Rust");
    let entry1 = generator.embed_repository(&repo, None).await.unwrap();
    index.add(entry1.clone()).unwrap();

    assert_eq!(index.len(), 1);

    // Update with new description
    let updated_repo = create_test_repo("user/test", "Updated description with more details", "Rust");
    let entry2 = generator.embed_repository(&updated_repo, None).await.unwrap();
    index.add(entry2).unwrap();

    // Should still have 1 entry (updated, not added)
    assert_eq!(index.len(), 1);

    // Verify metadata is updated
    let metadata = index.get_metadata("GitHub:user/test").unwrap();
    assert!(metadata.source_text.contains("Updated description"));
}

#[tokio::test]
async fn test_index_removal() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().to_path_buf();

    let generator = EmbeddingGenerator::new("sentence-transformers/all-MiniLM-L6-v2".to_string());
    generator.initialize().await.unwrap();

    let mut index =
        VectorIndex::new(384, "test-model".to_string(), index_path.clone()).unwrap();

    // Add a repository
    let repo = create_test_repo("user/test", "Test repository", "Rust");
    let entry = generator.embed_repository(&repo, None).await.unwrap();
    index.add(entry).unwrap();

    assert_eq!(index.len(), 1);
    assert!(index.contains("GitHub:user/test"));

    // Remove it
    index.remove("GitHub:user/test").unwrap();

    assert_eq!(index.len(), 0);
    assert!(!index.contains("GitHub:user/test"));
}

#[tokio::test]
async fn test_cosine_similarity() {
    use reposcout_semantic::cosine_similarity;

    let vec1 = vec![1.0, 0.0, 0.0, 0.0];
    let vec2 = vec![1.0, 0.0, 0.0, 0.0];
    assert!((cosine_similarity(&vec1, &vec2) - 1.0).abs() < 0.001);

    let vec3 = vec![0.0, 1.0, 0.0, 0.0];
    assert!(cosine_similarity(&vec1, &vec3).abs() < 0.001);

    let vec4 = vec![0.707, 0.707, 0.0, 0.0];
    let sim = cosine_similarity(&vec1, &vec4);
    assert!(sim > 0.7 && sim < 0.8);
}
