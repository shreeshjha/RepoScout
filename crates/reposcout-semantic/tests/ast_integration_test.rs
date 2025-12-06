use reposcout_core::models::{CodeMatch, CodeSearchResult, Platform};
use reposcout_semantic::{AstAnalyzer, CodeReranker};

#[tokio::test]
async fn test_ast_enrichment_flow() {
    // Create a test result with Rust code
    let mut result = CodeSearchResult {
        platform: Platform::GitHub,
        repository: "test/repo".to_string(),
        file_path: "src/main.rs".to_string(),
        language: Some("rust".to_string()),
        file_url: "https://example.com".to_string(),
        repository_url: "https://example.com".to_string(),
        repository_stars: 100,
        ast_metadata: None,
        matches: vec![CodeMatch {
            content: r#"
                pub async fn parse_args(args: Vec<String>) -> Result<Config> {
                    let mut config = Config::default();
                    for arg in args {
                        config.add(arg);
                    }
                    Ok(config)
                }

                pub struct Config {
                    pub values: Vec<String>,
                }
            "#
            .to_string(),
            line_number: 1,
            context_before: Vec::new(),
            context_after: Vec::new(),
            matched_functions: None,
            matched_types: None,
        }],
    };

    // Test AST enrichment
    let analyzer = AstAnalyzer::new(true);
    analyzer.enrich_result(&mut result).unwrap();

    // Verify AST metadata was extracted
    assert!(result.ast_metadata.is_some());
    let ast = result.ast_metadata.as_ref().unwrap();

    assert!(ast.parse_success, "AST parsing should succeed");
    assert_eq!(ast.language, "rust");

    // Check functions were extracted
    assert_eq!(ast.functions.len(), 1, "Should extract 1 function");
    assert_eq!(ast.functions[0].name, "parse_args");
    assert!(ast.functions[0].is_async, "Should detect async");
    assert_eq!(ast.functions[0].parameters.len(), 1);

    // Check types were extracted
    assert_eq!(ast.types.len(), 1, "Should extract 1 type");
    assert_eq!(ast.types[0].name, "Config");

    println!("✅ AST Enrichment Test Passed!");
    println!("   - Extracted {} functions", ast.functions.len());
    println!("   - Extracted {} types", ast.types.len());
    println!("   - Summary: {}", ast.structure_summary);
}

#[tokio::test]
async fn test_ast_enhanced_reranking() {
    // Create test results with different code snippets
    let results = vec![
        create_test_result(
            "fn multiply(x: i32, y: i32) -> i32 { x * y }",
            "multiply.rs",
        ),
        create_test_result(
            "async fn parse_json(data: &str) -> Result<Value> { serde_json::from_str(data) }",
            "parser.rs",
        ),
        create_test_result("fn add(a: i32, b: i32) -> i32 { a + b }", "math.rs"),
    ];

    // Initialize AST-enhanced reranker
    let reranker = CodeReranker::with_ast("BAAI/bge-small-en-v1.5".to_string());
    reranker.initialize().await.unwrap();

    // Re-rank with query about parsing
    let query = "parse json data";
    let reranked = reranker
        .rerank_with_ast(query, results, 10, 0.7)
        .await
        .unwrap();

    // Verify results
    assert!(!reranked.is_empty());

    // The parse_json function should rank highest for this query
    let top_result = &reranked[0].0;
    assert!(
        top_result.file_path.contains("parser"),
        "Parser file should rank highest for parse query"
    );

    // Verify AST metadata was added
    assert!(
        top_result.ast_metadata.is_some(),
        "AST metadata should be present"
    );

    let ast = top_result.ast_metadata.as_ref().unwrap();
    if ast.parse_success {
        println!("✅ AST-Enhanced Reranking Test Passed!");
        println!("   - Top result: {}", top_result.file_path);
        println!("   - Functions found: {}", ast.functions.len());
        if !ast.functions.is_empty() {
            println!("   - Top function: {}", ast.functions[0].name);
        }
    }
}

fn create_test_result(code: &str, file_path: &str) -> CodeSearchResult {
    CodeSearchResult {
        platform: Platform::GitHub,
        repository: "test/repo".to_string(),
        file_path: file_path.to_string(),
        language: Some("rust".to_string()),
        file_url: "https://example.com".to_string(),
        repository_url: "https://example.com".to_string(),
        repository_stars: 100,
        ast_metadata: None,
        matches: vec![CodeMatch {
            content: code.to_string(),
            line_number: 1,
            context_before: Vec::new(),
            context_after: Vec::new(),
            matched_functions: None,
            matched_types: None,
        }],
    }
}
