use crate::error::Result;
use reposcout_ast::{extractors, parser::ParserCache};
use reposcout_core::models::CodeSearchResult;
use tracing::{debug, warn};

/// AST analyzer for enriching code search results
pub struct AstAnalyzer {
    enabled: bool,
}

impl AstAnalyzer {
    /// Create a new AST analyzer
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Enrich a single code search result with AST metadata
    pub fn enrich_result(&self, result: &mut CodeSearchResult) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let language = match &result.language {
            Some(lang) => lang.clone(),
            None => {
                // Try to detect from file path
                if let Some(detected) = ParserCache::detect_language(&result.file_path) {
                    detected
                } else {
                    debug!("Cannot detect language for {}", result.file_path);
                    return Ok(());
                }
            }
        };

        // Check if language is supported
        if !ParserCache::is_language_supported(&language) {
            debug!("Language not supported for AST: {}", language);
            return Ok(());
        }

        // Get extractor for this language
        let extractor = match extractors::get_extractor(&language) {
            Some(ext) => ext,
            None => {
                warn!("No AST extractor for language: {}", language);
                return Ok(());
            }
        };

        // Extract from first match's content
        if let Some(first_match) = result.matches.first() {
            match extractor.extract_all(&first_match.content) {
                Ok(ast_metadata) => {
                    debug!(
                        "Extracted AST for {}: {} functions, {} types",
                        result.file_path,
                        ast_metadata.functions.len(),
                        ast_metadata.types.len()
                    );
                    result.ast_metadata = Some(ast_metadata);
                }
                Err(e) => {
                    warn!("AST extraction failed for {}: {}", result.file_path, e);
                    // Set failed metadata
                    result.ast_metadata = Some(reposcout_core::models::AstMetadata {
                        language: language.clone(),
                        functions: Vec::new(),
                        types: Vec::new(),
                        imports: Vec::new(),
                        structure_summary: String::new(),
                        parse_success: false,
                        parse_error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(())
    }

    /// Enrich multiple results in batch
    pub fn enrich_results(&self, results: &mut [CodeSearchResult]) -> Result<()> {
        for result in results.iter_mut() {
            // Continue even if individual enrichment fails
            let _ = self.enrich_result(result);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reposcout_core::models::{CodeMatch, Platform};

    #[test]
    fn test_enrich_rust_code() {
        let analyzer = AstAnalyzer::new(true);

        let mut result = CodeSearchResult {
            platform: Platform::GitHub,
            repository: "test/repo".to_string(),
            file_path: "test.rs".to_string(),
            language: Some("rust".to_string()),
            file_url: "https://example.com".to_string(),
            repository_url: "https://example.com".to_string(),
            repository_stars: 100,
            ast_metadata: None,
            matches: vec![CodeMatch {
                content: "pub fn hello() -> String { String::from(\"hello\") }".to_string(),
                line_number: 1,
                context_before: Vec::new(),
                context_after: Vec::new(),
                matched_functions: None,
                matched_types: None,
            }],
        };

        analyzer.enrich_result(&mut result).unwrap();

        assert!(result.ast_metadata.is_some());
        let ast = result.ast_metadata.unwrap();
        assert!(ast.parse_success);
        assert_eq!(ast.functions.len(), 1);
        assert_eq!(ast.functions[0].name, "hello");
    }

    #[test]
    fn test_disabled_analyzer() {
        let analyzer = AstAnalyzer::new(false);

        let mut result = CodeSearchResult {
            platform: Platform::GitHub,
            repository: "test/repo".to_string(),
            file_path: "test.rs".to_string(),
            language: Some("rust".to_string()),
            file_url: "https://example.com".to_string(),
            repository_url: "https://example.com".to_string(),
            repository_stars: 100,
            ast_metadata: None,
            matches: vec![CodeMatch {
                content: "pub fn hello() {}".to_string(),
                line_number: 1,
                context_before: Vec::new(),
                context_after: Vec::new(),
                matched_functions: None,
                matched_types: None,
            }],
        };

        analyzer.enrich_result(&mut result).unwrap();

        assert!(result.ast_metadata.is_none());
    }
}
