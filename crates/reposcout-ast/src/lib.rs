//! AST-based code analysis for RepoScout
//!
//! This crate provides Abstract Syntax Tree (AST) parsing and analysis for multiple
//! programming languages using tree-sitter. It enables structural code search by
//! extracting functions, types, and other code elements.
//!
//! # Examples
//!
//! ```
//! use reposcout_ast::extractors::{get_extractor, AstExtractor};
//!
//! let code = r#"
//!     pub fn hello(name: &str) -> String {
//!         format!("Hello, {}!", name)
//!     }
//! "#;
//!
//! let extractor = get_extractor("rust").unwrap();
//! let ast = extractor.extract_all(code).unwrap();
//!
//! assert_eq!(ast.functions.len(), 1);
//! assert_eq!(ast.functions[0].name, "hello");
//! ```

pub mod error;
pub mod extractors;
pub mod parser;
pub mod query;
pub mod scorer;

// Re-export commonly used types
pub use error::{AstError, Result};
pub use extractors::{get_extractor, AstExtractor};
pub use parser::ParserCache;
pub use query::{parse_query, AstQueryFilters, ParsedQuery};
pub use scorer::{filter_by_ast, score_ast_match, string_similarity};
