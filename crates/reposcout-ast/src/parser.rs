use crate::error::{AstError, Result};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use tree_sitter::{Language, Parser};

/// Global parser cache for reuse across requests
pub struct ParserCache {
    parsers: Mutex<HashMap<String, Parser>>,
    languages: HashMap<String, Language>,
}

static PARSER_CACHE: Lazy<ParserCache> = Lazy::new(|| {
    let mut languages = HashMap::new();

    // Load all tree-sitter grammars
    languages.insert("rust".to_string(), tree_sitter_rust::language());
    languages.insert("python".to_string(), tree_sitter_python::language());
    languages.insert("javascript".to_string(), tree_sitter_javascript::language());
    languages.insert(
        "typescript".to_string(),
        tree_sitter_typescript::language_typescript(),
    );
    languages.insert("tsx".to_string(), tree_sitter_typescript::language_tsx());
    languages.insert("go".to_string(), tree_sitter_go::language());
    languages.insert("c".to_string(), tree_sitter_c::language());
    languages.insert("cpp".to_string(), tree_sitter_cpp::language());
    languages.insert("c++".to_string(), tree_sitter_cpp::language());

    ParserCache {
        parsers: Mutex::new(HashMap::new()),
        languages,
    }
});

impl ParserCache {
    /// Get the global parser cache instance
    pub fn get() -> &'static ParserCache {
        &PARSER_CACHE
    }

    /// Parse code with language-specific parser
    pub fn parse(&self, code: &str, language: &str) -> Result<tree_sitter::Tree> {
        let lang_lower = language.to_lowercase();

        let language = self
            .languages
            .get(&lang_lower)
            .ok_or_else(|| AstError::UnsupportedLanguage(language.to_string()))?;

        let mut parsers = self.parsers.lock().unwrap();
        let parser = parsers.entry(lang_lower.clone()).or_insert_with(|| {
            let mut p = Parser::new();
            p.set_language(language)
                .expect("Failed to set language for parser");
            p
        });

        parser
            .parse(code, None)
            .ok_or_else(|| AstError::ParseError("Failed to parse code".to_string()))
    }

    /// Detect language from file extension
    pub fn detect_language(file_path: &str) -> Option<String> {
        let ext = std::path::Path::new(file_path)
            .extension()?
            .to_str()?
            .to_lowercase();

        match ext.as_str() {
            "rs" => Some("rust".to_string()),
            "py" | "pyw" => Some("python".to_string()),
            "js" | "mjs" | "cjs" => Some("javascript".to_string()),
            "ts" => Some("typescript".to_string()),
            "tsx" => Some("tsx".to_string()),
            "go" => Some("go".to_string()),
            "c" | "h" => Some("c".to_string()),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "h++" => Some("cpp".to_string()),
            _ => None,
        }
    }

    /// Check if a language is supported
    pub fn is_language_supported(language: &str) -> bool {
        let lang_lower = language.to_lowercase();
        PARSER_CACHE.languages.contains_key(&lang_lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(
            ParserCache::detect_language("test.rs"),
            Some("rust".to_string())
        );
        assert_eq!(
            ParserCache::detect_language("test.py"),
            Some("python".to_string())
        );
        assert_eq!(
            ParserCache::detect_language("test.js"),
            Some("javascript".to_string())
        );
        assert_eq!(
            ParserCache::detect_language("test.ts"),
            Some("typescript".to_string())
        );
        assert_eq!(
            ParserCache::detect_language("test.go"),
            Some("go".to_string())
        );
        assert_eq!(ParserCache::detect_language("test.c"), Some("c".to_string()));
        assert_eq!(
            ParserCache::detect_language("test.cpp"),
            Some("cpp".to_string())
        );
        assert_eq!(ParserCache::detect_language("test.txt"), None);
    }

    #[test]
    fn test_is_language_supported() {
        assert!(ParserCache::is_language_supported("rust"));
        assert!(ParserCache::is_language_supported("Rust"));
        assert!(ParserCache::is_language_supported("python"));
        assert!(!ParserCache::is_language_supported("unknown"));
    }

    #[test]
    fn test_parse_rust() {
        let code = "fn main() {}";
        let cache = ParserCache::get();
        let tree = cache.parse(code, "rust").unwrap();
        assert!(tree.root_node().kind() == "source_file");
    }

    #[test]
    fn test_parse_unsupported_language() {
        let code = "some code";
        let cache = ParserCache::get();
        let result = cache.parse(code, "unknown");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AstError::UnsupportedLanguage(_)));
    }
}
