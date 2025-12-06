use crate::error::Result;
use reposcout_core::models::{AstMetadata, FunctionSignature, ImportStatement, TypeDefinition};
use tree_sitter::Node;

pub mod rust;
pub mod python;
pub mod javascript;
pub mod go;

/// Trait for language-specific AST extractors
pub trait AstExtractor {
    /// Get the language name
    fn language(&self) -> &'static str;

    /// Extract functions from AST
    fn extract_functions(&self, node: Node, source: &str) -> Vec<FunctionSignature>;

    /// Extract types (classes, structs, etc.) from AST
    fn extract_types(&self, node: Node, source: &str) -> Vec<TypeDefinition>;

    /// Extract imports from AST
    fn extract_imports(&self, node: Node, source: &str) -> Vec<ImportStatement>;

    /// Extract all metadata from code
    fn extract_all(&self, code: &str) -> Result<AstMetadata> {
        let cache = crate::parser::ParserCache::get();
        let tree = cache.parse(code, self.language())?;
        let root = tree.root_node();

        Ok(AstMetadata {
            language: self.language().to_string(),
            functions: self.extract_functions(root, code),
            types: self.extract_types(root, code),
            imports: self.extract_imports(root, code),
            structure_summary: self.generate_summary(root, code),
            parse_success: true,
            parse_error: None,
        })
    }

    /// Generate a text summary of the code structure for embeddings
    fn generate_summary(&self, node: Node, source: &str) -> String {
        let mut parts = Vec::new();

        let functions = self.extract_functions(node, source);
        if !functions.is_empty() {
            let names: Vec<_> = functions.iter().map(|f| f.name.as_str()).collect();
            parts.push(format!("functions: {}", names.join(", ")));
        }

        let types = self.extract_types(node, source);
        if !types.is_empty() {
            let names: Vec<_> = types.iter().map(|t| t.name.as_str()).collect();
            parts.push(format!("types: {}", names.join(", ")));
        }

        parts.join(" | ")
    }
}

/// Get extractor for a specific language
pub fn get_extractor(language: &str) -> Option<Box<dyn AstExtractor>> {
    match language.to_lowercase().as_str() {
        "rust" => Some(Box::new(rust::RustExtractor)),
        "python" | "py" => Some(Box::new(python::PythonExtractor)),
        "javascript" | "js" => Some(Box::new(javascript::JavaScriptExtractor::new("javascript"))),
        "typescript" | "ts" => Some(Box::new(javascript::JavaScriptExtractor::new("typescript"))),
        "tsx" => Some(Box::new(javascript::JavaScriptExtractor::new("tsx"))),
        "go" => Some(Box::new(go::GoExtractor)),
        // TODO: Add C and C++ as they're implemented
        // "c" => Some(Box::new(c::CExtractor)),
        // "cpp" | "c++" => Some(Box::new(cpp::CppExtractor)),
        _ => None,
    }
}
