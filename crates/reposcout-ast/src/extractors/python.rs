use super::AstExtractor;
use crate::error::{AstError, Result};
use crate::parser::ParserCache;
use reposcout_core::models::{
    AstMetadata, FunctionSignature, ImportStatement, Parameter, TypeDefinition, TypeKind,
    Visibility,
};
use tree_sitter::Node;

pub struct PythonExtractor;

impl AstExtractor for PythonExtractor {
    fn language(&self) -> &'static str {
        "python"
    }

    fn extract_functions(&self, node: Node, source: &str) -> Vec<FunctionSignature> {
        let mut functions = Vec::new();
        self.extract_functions_recursive(node, source, &mut functions, false);
        functions
    }

    fn extract_types(&self, node: Node, source: &str) -> Vec<TypeDefinition> {
        let mut types = Vec::new();
        self.extract_types_recursive(node, source, &mut types);
        types
    }

    fn extract_imports(&self, node: Node, source: &str) -> Vec<ImportStatement> {
        let mut imports = Vec::new();
        self.extract_imports_recursive(node, source, &mut imports);
        imports
    }

    fn extract_all(&self, code: &str) -> Result<AstMetadata> {
        let cache = ParserCache::get();
        let tree = cache
            .parse(code, self.language())
            .map_err(|e| AstError::ParseError(e.to_string()))?;

        let root_node = tree.root_node();

        let functions = self.extract_functions(root_node, code);
        let types = self.extract_types(root_node, code);
        let imports = self.extract_imports(root_node, code);

        // Generate structure summary
        let mut summary_parts = Vec::new();
        if !functions.is_empty() {
            let fn_names: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();
            summary_parts.push(format!("functions: {}", fn_names.join(", ")));
        }
        if !types.is_empty() {
            let type_names: Vec<&str> = types.iter().map(|t| t.name.as_str()).collect();
            summary_parts.push(format!("classes: {}", type_names.join(", ")));
        }

        Ok(AstMetadata {
            language: self.language().to_string(),
            functions,
            types,
            imports,
            structure_summary: summary_parts.join("; "),
            parse_success: true,
            parse_error: None,
        })
    }
}

impl PythonExtractor {
    fn extract_functions_recursive(
        &self,
        node: Node,
        source: &str,
        functions: &mut Vec<FunctionSignature>,
        inside_class: bool,
    ) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_definition" => {
                    if let Some(func) = self.parse_function(child, source, inside_class) {
                        functions.push(func);
                    }
                }
                "class_definition" => {
                    // Extract methods from class
                    self.extract_functions_recursive(child, source, functions, true);
                }
                _ => {
                    self.extract_functions_recursive(child, source, functions, inside_class);
                }
            }
        }
    }

    fn parse_function(
        &self,
        node: Node,
        source: &str,
        _inside_class: bool,
    ) -> Option<FunctionSignature> {
        let mut name = String::new();
        let mut parameters = Vec::new();
        let mut return_type = None;
        let mut is_async = false;
        let mut doc_comment = None;
        let line_number = node.start_position().row + 1;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "async" => {
                    is_async = true;
                }
                "identifier" => {
                    if name.is_empty() {
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            name = text.to_string();
                        }
                    }
                }
                "parameters" => {
                    parameters = self.parse_parameters(child, source);
                }
                "type" => {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        return_type = Some(text.trim_start_matches("->").trim().to_string());
                    }
                }
                "block" => {
                    // Try to extract docstring
                    if let Some(first_stmt) = child.child(1) {
                        // Skip newline/indent
                        if first_stmt.kind() == "expression_statement" {
                            if let Some(string_node) = first_stmt.child(0) {
                                if string_node.kind() == "string" {
                                    if let Ok(text) = string_node.utf8_text(source.as_bytes()) {
                                        doc_comment = Some(
                                            text.trim_matches('"')
                                                .trim_matches('\'')
                                                .trim()
                                                .to_string(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if !name.is_empty() {
            // Determine visibility based on naming convention
            let visibility = if name.starts_with("__") && !name.ends_with("__") {
                Visibility::Private
            } else if name.starts_with('_') {
                Visibility::Protected
            } else {
                Visibility::Public
            };

            // Skip 'self' and 'cls' parameters for cleaner display
            parameters.retain(|p| p.name != "self" && p.name != "cls");

            Some(FunctionSignature {
                name,
                parameters,
                return_type,
                visibility,
                is_async,
                is_generic: false, // Python doesn't have explicit generics like Rust
                doc_comment,
                line_number,
            })
        } else {
            None
        }
    }

    fn parse_parameters(&self, node: Node, source: &str) -> Vec<Parameter> {
        let mut parameters = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "typed_parameter" {
                if let Some(param) = self.parse_parameter(child, source) {
                    parameters.push(param);
                }
            } else if child.kind() == "default_parameter" || child.kind() == "typed_default_parameter"
            {
                if let Some(param) = self.parse_default_parameter(child, source) {
                    parameters.push(param);
                }
            }
        }

        parameters
    }

    fn parse_parameter(&self, node: Node, source: &str) -> Option<Parameter> {
        let mut name = String::new();
        let mut param_type = None;

        if node.kind() == "identifier" {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                name = text.to_string();
            }
        } else if node.kind() == "typed_parameter" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "identifier" => {
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            name = text.to_string();
                        }
                    }
                    "type" => {
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            param_type = Some(text.trim_start_matches(':').trim().to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        if !name.is_empty() {
            Some(Parameter {
                name,
                param_type,
                is_optional: false,
            })
        } else {
            None
        }
    }

    fn parse_default_parameter(&self, node: Node, source: &str) -> Option<Parameter> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "typed_parameter" {
                return self.parse_parameter(child, source);
            }
        }
        None
    }

    fn extract_types_recursive(
        &self,
        node: Node,
        source: &str,
        types: &mut Vec<TypeDefinition>,
    ) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "class_definition" {
                if let Some(type_def) = self.parse_class(child, source) {
                    types.push(type_def);
                }
            }
            self.extract_types_recursive(child, source, types);
        }
    }

    fn parse_class(&self, node: Node, source: &str) -> Option<TypeDefinition> {
        let mut name = String::new();
        let mut methods = Vec::new();
        let mut implements = Vec::new();
        let mut doc_comment = None;
        let line_number = node.start_position().row + 1;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    if name.is_empty() {
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            name = text.to_string();
                        }
                    }
                }
                "argument_list" => {
                    // Parse base classes
                    let mut arg_cursor = child.walk();
                    for arg in child.children(&mut arg_cursor) {
                        if arg.kind() == "identifier" {
                            if let Ok(text) = arg.utf8_text(source.as_bytes()) {
                                implements.push(text.to_string());
                            }
                        }
                    }
                }
                "block" => {
                    // Extract docstring
                    if let Some(first_stmt) = child.child(1) {
                        if first_stmt.kind() == "expression_statement" {
                            if let Some(string_node) = first_stmt.child(0) {
                                if string_node.kind() == "string" {
                                    if let Ok(text) = string_node.utf8_text(source.as_bytes()) {
                                        doc_comment = Some(
                                            text.trim_matches('"')
                                                .trim_matches('\'')
                                                .trim()
                                                .to_string(),
                                        );
                                    }
                                }
                            }
                        }
                    }

                    // Extract method names
                    let mut block_cursor = child.walk();
                    for block_child in child.children(&mut block_cursor) {
                        if block_child.kind() == "function_definition" {
                            if let Some(method_name) = self.get_function_name(block_child, source) {
                                methods.push(method_name);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if !name.is_empty() {
            Some(TypeDefinition {
                name,
                kind: TypeKind::Class,
                fields: Vec::new(), // Python doesn't have explicit field declarations
                methods,
                implements,
                doc_comment,
                line_number,
            })
        } else {
            None
        }
    }

    fn get_function_name(&self, node: Node, source: &str) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    return Some(text.to_string());
                }
            }
        }
        None
    }

    fn extract_imports_recursive(
        &self,
        node: Node,
        source: &str,
        imports: &mut Vec<ImportStatement>,
    ) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "import_statement" | "import_from_statement" => {
                    if let Some(import) = self.parse_import(child, source) {
                        imports.push(import);
                    }
                }
                _ => {
                    self.extract_imports_recursive(child, source, imports);
                }
            }
        }
    }

    fn parse_import(&self, node: Node, source: &str) -> Option<ImportStatement> {
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            let text = text.trim();

            // Parse "import module" or "from module import items"
            if text.starts_with("from ") {
                // from module import item1, item2
                let parts: Vec<&str> = text.split("import").collect();
                if parts.len() == 2 {
                    let module = parts[0]
                        .trim_start_matches("from ")
                        .trim()
                        .to_string();
                    let items_str = parts[1].trim();
                    let is_wildcard = items_str == "*";
                    let items = if is_wildcard {
                        vec![]
                    } else {
                        items_str
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect()
                    };

                    return Some(ImportStatement {
                        module,
                        items,
                        is_wildcard,
                    });
                }
            } else if text.starts_with("import ") {
                // import module
                let module = text.trim_start_matches("import ").trim().to_string();
                if !module.is_empty() {
                    return Some(ImportStatement {
                        module,
                        items: vec![],
                        is_wildcard: false,
                    });
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_python_function() {
        let code = r#"
def greet(name: str) -> str:
    """Greet a person by name."""
    return f"Hello, {name}!"
"#;

        let extractor = PythonExtractor;
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.functions.len(), 1);
        assert_eq!(ast.functions[0].name, "greet");
        assert_eq!(ast.functions[0].parameters.len(), 1);
        assert_eq!(ast.functions[0].parameters[0].name, "name");
        assert_eq!(
            ast.functions[0].parameters[0].param_type,
            Some("str".to_string())
        );
        assert_eq!(ast.functions[0].return_type, Some("str".to_string()));
    }

    #[test]
    fn test_extract_async_function() {
        let code = r#"
async def fetch_data(url: str):
    """Fetch data from URL."""
    pass
"#;

        let extractor = PythonExtractor;
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.functions.len(), 1);
        assert_eq!(ast.functions[0].name, "fetch_data");
        assert!(ast.functions[0].is_async);
    }

    #[test]
    fn test_extract_class() {
        let code = r#"
class User:
    """A user class."""

    def __init__(self, name: str):
        self.name = name

    def greet(self):
        return f"Hello, {self.name}"
"#;

        let extractor = PythonExtractor;
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.types.len(), 1);
        assert_eq!(ast.types[0].name, "User");
        assert_eq!(ast.types[0].kind, TypeKind::Class);
        assert_eq!(ast.types[0].methods.len(), 2);
        assert!(ast.types[0].methods.contains(&"__init__".to_string()));
        assert!(ast.types[0].methods.contains(&"greet".to_string()));
    }

    #[test]
    fn test_visibility_from_naming() {
        let code = r#"
def public_func():
    pass

def _protected_func():
    pass

def __private_func():
    pass
"#;

        let extractor = PythonExtractor;
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.functions.len(), 3);
        assert_eq!(ast.functions[0].visibility, Visibility::Public);
        assert_eq!(ast.functions[1].visibility, Visibility::Protected);
        assert_eq!(ast.functions[2].visibility, Visibility::Private);
    }

    #[test]
    fn test_extract_imports() {
        let code = r#"
import os
import sys
from typing import List, Dict
from pathlib import Path
"#;

        let extractor = PythonExtractor;
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.imports.len(), 4);
        assert!(ast.imports.iter().any(|i| i.module == "os"));
        assert!(ast.imports.iter().any(|i| i.module == "sys"));
        assert!(ast.imports.iter().any(|i| i.module == "typing"));
        assert!(ast.imports.iter().any(|i| i.module == "pathlib"));
    }
}
