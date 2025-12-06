use crate::embeddings::{cosine_similarity, EmbeddingGenerator};
use crate::error::Result;
use reposcout_core::models::CodeSearchResult;
use tracing::{debug, info};

/// Re-rank code search results using semantic similarity
pub struct CodeReranker {
    embedder: EmbeddingGenerator,
    ast_analyzer: Option<crate::ast_analyzer::AstAnalyzer>,
}

impl CodeReranker {
    /// Create a new code re-ranker
    pub fn new(model_name: String) -> Self {
        Self {
            embedder: EmbeddingGenerator::new(model_name),
            ast_analyzer: None,
        }
    }

    /// Create a new code re-ranker with AST analysis enabled
    pub fn with_ast(model_name: String) -> Self {
        Self {
            embedder: EmbeddingGenerator::new(model_name),
            ast_analyzer: Some(crate::ast_analyzer::AstAnalyzer::new(true)),
        }
    }

    /// Initialize the embedding model
    pub async fn initialize(&self) -> Result<()> {
        self.embedder.initialize().await
    }

    /// Re-rank code search results based on semantic similarity to query
    pub async fn rerank(
        &self,
        query: &str,
        results: Vec<CodeSearchResult>,
        limit: usize,
    ) -> Result<Vec<(CodeSearchResult, f32)>> {
        if results.is_empty() {
            return Ok(Vec::new());
        }

        info!(
            "Re-ranking {} code search results for query: {}",
            results.len(),
            query
        );

        // Generate query embedding
        let query_embedding = self.embedder.embed_query(query).await?;

        // Generate embeddings for all code snippets in batch
        let snippets: Vec<(&str, Option<&str>, &str, Option<&reposcout_core::models::AstMetadata>)> = results
            .iter()
            .map(|result| {
                let code = if let Some(first_match) = result.matches.first() {
                    first_match.content.as_str()
                } else {
                    ""
                };
                (
                    code,
                    result.language.as_deref(),
                    result.file_path.as_str(),
                    result.ast_metadata.as_ref(),
                )
            })
            .collect();

        debug!("Generating embeddings for {} code snippets", snippets.len());
        let code_embeddings = self.embedder.embed_code_snippets(snippets).await?;

        // Calculate similarity scores
        let mut scored_results: Vec<(CodeSearchResult, f32)> = results
            .into_iter()
            .zip(code_embeddings.iter())
            .map(|(result, code_embedding)| {
                let similarity = cosine_similarity(&query_embedding, code_embedding);
                (result, similarity)
            })
            .collect();

        // Sort by similarity (descending)
        scored_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Limit results
        scored_results.truncate(limit);

        info!(
            "Re-ranking complete. Top result has similarity: {:.3}",
            scored_results
                .first()
                .map(|(_, score)| *score)
                .unwrap_or(0.0)
        );

        Ok(scored_results)
    }

    /// Re-rank with hybrid scoring (combining semantic similarity and keyword match)
    pub async fn rerank_hybrid(
        &self,
        query: &str,
        results: Vec<CodeSearchResult>,
        limit: usize,
        semantic_weight: f32,
    ) -> Result<Vec<(CodeSearchResult, f32, f32)>> {
        if results.is_empty() {
            return Ok(Vec::new());
        }

        info!(
            "Hybrid re-ranking {} code search results (semantic_weight: {:.2})",
            results.len(),
            semantic_weight
        );

        // Get semantic scores
        let semantic_results = self.rerank(query, results, usize::MAX).await?;

        // Calculate keyword scores (simple word overlap)
        let query_words: std::collections::HashSet<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let mut hybrid_results: Vec<(CodeSearchResult, f32, f32)> = semantic_results
            .into_iter()
            .map(|(result, semantic_score)| {
                // Calculate keyword score based on word overlap
                let code_text = result
                    .matches
                    .iter()
                    .map(|m| m.content.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");

                let code_words: std::collections::HashSet<String> = code_text
                    .to_lowercase()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                let overlap = query_words.intersection(&code_words).count();
                let keyword_score = if query_words.is_empty() {
                    0.0
                } else {
                    overlap as f32 / query_words.len() as f32
                };

                // Combined hybrid score
                let hybrid_score =
                    (semantic_score * semantic_weight) + (keyword_score * (1.0 - semantic_weight));

                (result, semantic_score, hybrid_score)
            })
            .collect();

        // Sort by hybrid score (descending)
        hybrid_results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        // Limit results
        hybrid_results.truncate(limit);

        info!(
            "Hybrid re-ranking complete. Top result has hybrid score: {:.3}",
            hybrid_results
                .first()
                .map(|(_, _, score)| *score)
                .unwrap_or(0.0)
        );

        Ok(hybrid_results)
    }

    /// Re-rank with AST enrichment (if enabled)
    /// Enriches results with AST metadata before performing hybrid reranking
    pub async fn rerank_with_ast(
        &self,
        query: &str,
        mut results: Vec<CodeSearchResult>,
        limit: usize,
        semantic_weight: f32,
    ) -> Result<Vec<(CodeSearchResult, f32, f32)>> {
        // Enrich results with AST if analyzer is available
        if let Some(ref analyzer) = self.ast_analyzer {
            info!("Enriching {} results with AST metadata", results.len());
            analyzer.enrich_results(&mut results)?;

            let enriched_count = results.iter().filter(|r| r.ast_metadata.is_some()).count();
            info!("{} results successfully enriched with AST", enriched_count);
        }

        // Perform hybrid reranking (now with AST-enhanced embeddings)
        self.rerank_hybrid(query, results, limit, semantic_weight).await
    }

    /// Re-rank with AST filtering and scoring
    /// Applies AST filters and incorporates AST similarity scores into ranking
    pub async fn rerank_with_ast_filters(
        &self,
        query: &str,
        mut results: Vec<CodeSearchResult>,
        filters: &reposcout_ast::AstQueryFilters,
        limit: usize,
        semantic_weight: f32,
        ast_weight: f32,
    ) -> Result<Vec<(CodeSearchResult, f32, f32, f32)>> {
        if results.is_empty() {
            return Ok(Vec::new());
        }

        // Enrich results with AST if analyzer is available
        if let Some(ref analyzer) = self.ast_analyzer {
            info!("Enriching {} results with AST metadata", results.len());
            analyzer.enrich_results(&mut results)?;

            let enriched_count = results.iter().filter(|r| r.ast_metadata.is_some()).count();
            let successful_count = results.iter().filter(|r| {
                r.ast_metadata.as_ref().map(|ast| ast.parse_success).unwrap_or(false)
            }).count();
            info!("{} results enriched with AST ({} successful, {} failed)",
                enriched_count, successful_count, enriched_count - successful_count);
        } else {
            info!("No AST analyzer available - AST scoring will use neutral scores");
        }

        // Apply hard filters if any are set
        if filters.has_filters() {
            info!("Applying AST filters to {} results", results.len());
            results.retain(|result| {
                if let Some(ref ast) = result.ast_metadata {
                    // Apply hard filters using scorer module
                    if let Some(ref fn_name) = filters.function_name {
                        if !ast.functions.iter().any(|f| {
                            reposcout_ast::scorer::string_similarity(&f.name, fn_name) > 0.5
                        }) {
                            return false;
                        }
                    }

                    if let Some(ref class_name) = filters.class_name {
                        if !ast.types.iter().any(|t| {
                            reposcout_ast::scorer::string_similarity(&t.name, class_name) > 0.5
                        }) {
                            return false;
                        }
                    }

                    if let Some(is_async) = filters.is_async {
                        let has_async = ast.functions.iter().any(|f| f.is_async);
                        if has_async != is_async {
                            return false;
                        }
                    }

                    true
                } else {
                    // Keep results without AST metadata - they'll get neutral AST scores
                    // Don't filter them out just because AST parsing failed
                    true
                }
            });
            info!("After filtering: {} results remain", results.len());
        }

        if results.is_empty() {
            return Ok(Vec::new());
        }

        info!(
            "AST-filtered hybrid re-ranking {} results (semantic: {:.2}, ast: {:.2})",
            results.len(),
            semantic_weight,
            ast_weight
        );

        // Get semantic scores
        let semantic_results = self.rerank(query, results, usize::MAX).await?;

        // Calculate keyword scores and AST scores
        let query_words: std::collections::HashSet<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let keyword_weight = 1.0 - semantic_weight - ast_weight;

        let mut scored_results: Vec<(CodeSearchResult, f32, f32, f32)> = semantic_results
            .into_iter()
            .map(|(result, semantic_score)| {
                // Calculate keyword score
                let code_text = result
                    .matches
                    .iter()
                    .map(|m| m.content.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");

                let code_words: std::collections::HashSet<String> = code_text
                    .to_lowercase()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                let overlap = query_words.intersection(&code_words).count();
                let keyword_score = if query_words.is_empty() {
                    0.0
                } else {
                    overlap as f32 / query_words.len() as f32
                };

                // Calculate AST score
                // If AST metadata is missing or failed to parse, use neutral score (0.5)
                // This prevents results from being unfairly penalized when AST extraction fails
                let ast_score = if let Some(ref ast) = result.ast_metadata {
                    if ast.parse_success {
                        reposcout_ast::scorer::score_ast_match(ast, filters, query)
                    } else {
                        // AST parsing failed - use neutral score
                        0.5
                    }
                } else {
                    // No AST metadata - use neutral score
                    0.5
                };

                // Combined hybrid score with AST
                let hybrid_score = (semantic_score * semantic_weight)
                    + (keyword_score * keyword_weight)
                    + (ast_score * ast_weight);

                (result, semantic_score, ast_score, hybrid_score)
            })
            .collect();

        // Sort by hybrid score (descending)
        scored_results.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

        // Limit results
        scored_results.truncate(limit);

        info!(
            "AST-filtered hybrid re-ranking complete. Top result has hybrid score: {:.3}",
            scored_results
                .first()
                .map(|(_, _, _, score)| *score)
                .unwrap_or(0.0)
        );

        Ok(scored_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reposcout_core::models::{CodeMatch, Platform};

    fn create_test_result(code: &str, language: &str) -> CodeSearchResult {
        CodeSearchResult {
            platform: Platform::GitHub,
            repository: "test/repo".to_string(),
            file_path: "src/main.rs".to_string(),
            language: Some(language.to_string()),
            file_url: "https://github.com/test/repo".to_string(),
            repository_url: "https://github.com/test/repo".to_string(),
            matches: vec![CodeMatch {
                content: code.to_string(),
                line_number: 1,
                context_before: vec![],
                context_after: vec![],
                matched_functions: None,
                matched_types: None,
            }],
            repository_stars: 100,
            ast_metadata: None,
        }
    }

    #[tokio::test]
    async fn test_reranker_initialization() {
        let reranker = CodeReranker::new("BAAI/bge-small-en-v1.5".to_string());
        assert!(reranker.initialize().await.is_ok());
    }

    #[tokio::test]
    async fn test_empty_results() {
        let reranker = CodeReranker::new("BAAI/bge-small-en-v1.5".to_string());
        reranker.initialize().await.unwrap();

        let results = reranker
            .rerank("test query", vec![], 10)
            .await
            .unwrap();

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_rerank_code_snippets() {
        let reranker = CodeReranker::new("BAAI/bge-small-en-v1.5".to_string());
        reranker.initialize().await.unwrap();

        let results = vec![
            create_test_result("fn add(a: i32, b: i32) -> i32 { a + b }", "Rust"),
            create_test_result("fn parse_args() -> Vec<String> { vec![] }", "Rust"),
            create_test_result("fn multiply(x: i32, y: i32) -> i32 { x * y }", "Rust"),
        ];

        let reranked = reranker
            .rerank("function that parses command line arguments", results, 10)
            .await
            .unwrap();

        // The parse_args function should rank highest
        assert!(!reranked.is_empty());
        assert!(reranked[0].0.matches[0].content.contains("parse_args"));
    }
}
