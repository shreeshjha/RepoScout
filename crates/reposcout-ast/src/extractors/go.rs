use super::AstExtractor;
use crate::error::{AstError, Result};
use crate::parser::ParserCache;
use reposcout_core::models::{
    AstMetadata, FunctionSignature, ImportStatement, Parameter, TypeDefinition, TypeKind,
    Visibility,
};
use tree_sitter::Node;

pub struct GoExtractor;

impl AstExtractor for GoExtractor {
    fn language(&self) -> &'static str {
        "go"
    }

    fn extract_functions(&self, node: Node, source: &str) -> Vec<FunctionSignature> {
        let mut functions = Vec::new();
        self.extract_functions_recursive(node, source, &mut functions);
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
            summary_parts.push(format!("types: {}", type_names.join(", ")));
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

impl GoExtractor {
    fn extract_functions_recursive(
        &self,
        node: Node,
        source: &str,
        functions: &mut Vec<FunctionSignature>,
    ) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_declaration" | "method_declaration" => {
                    if let Some(func) = self.parse_function(child, source) {
                        functions.push(func);
                    }
                }
                _ => {
                    self.extract_functions_recursive(child, source, functions);
                }
            }
        }
    }

    fn parse_function(&self, node: Node, source: &str) -> Option<FunctionSignature> {
        let mut name = String::new();
        let mut parameters = Vec::new();
        let mut return_type = None;
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
                "parameter_list" => {
                    parameters = self.parse_parameters(child, source);
                }
                "type_identifier" | "pointer_type" | "slice_type" => {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        return_type = Some(text.to_string());
                    }
                }
                _ => {}
            }
        }

        if !name.is_empty() {
            // In Go, visibility is based on capitalization
            let visibility = if name.chars().next().unwrap_or('a').is_uppercase() {
                Visibility::Public
            } else {
                Visibility::Private
            };

            Some(FunctionSignature {
                name,
                parameters,
                return_type,
                visibility,
                is_async: false, // Go doesn't have async/await
                is_generic: false,
                doc_comment: None,
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
            if child.kind() == "parameter_declaration" {
                if let Some(param) = self.parse_parameter(child, source) {
                    parameters.push(param);
                }
            }
        }

        parameters
    }

    fn parse_parameter(&self, node: Node, source: &str) -> Option<Parameter> {
        let mut name = String::new();
        let mut param_type = None;

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
                "type_identifier" | "pointer_type" | "slice_type" | "array_type" => {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        param_type = Some(text.to_string());
                    }
                }
                _ => {}
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

    fn extract_types_recursive(
        &self,
        node: Node,
        source: &str,
        types: &mut Vec<TypeDefinition>,
    ) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "type_declaration" {
                if let Some(type_def) = self.parse_type_declaration(child, source) {
                    types.push(type_def);
                }
            } else {
                self.extract_types_recursive(child, source, types);
            }
        }
    }

    fn parse_type_declaration(&self, node: Node, source: &str) -> Option<TypeDefinition> {
        let mut name = String::new();
        let mut kind = TypeKind::Struct;
        let line_number = node.start_position().row + 1;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_spec" {
                let mut spec_cursor = child.walk();
                for spec_child in child.children(&mut spec_cursor) {
                    match spec_child.kind() {
                        "type_identifier" => {
                            if let Ok(text) = spec_child.utf8_text(source.as_bytes()) {
                                name = text.to_string();
                            }
                        }
                        "struct_type" => {
                            kind = TypeKind::Struct;
                        }
                        "interface_type" => {
                            kind = TypeKind::Interface;
                        }
                        _ => {}
                    }
                }
            }
        }

        if !name.is_empty() {
            Some(TypeDefinition {
                name,
                kind,
                fields: Vec::new(),
                methods: Vec::new(),
                implements: Vec::new(),
                doc_comment: None,
                line_number,
            })
        } else {
            None
        }
    }

    fn extract_imports_recursive(
        &self,
        node: Node,
        source: &str,
        imports: &mut Vec<ImportStatement>,
    ) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "import_declaration" {
                self.parse_import_declaration(child, source, imports);
            } else {
                self.extract_imports_recursive(child, source, imports);
            }
        }
    }

    fn parse_import_declaration(
        &self,
        node: Node,
        source: &str,
        imports: &mut Vec<ImportStatement>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "import_spec" | "import_spec_list" => {
                    if child.kind() == "import_spec" {
                        if let Some(import) = self.parse_import_spec(child, source) {
                            imports.push(import);
                        }
                    } else {
                        let mut list_cursor = child.walk();
                        for list_child in child.children(&mut list_cursor) {
                            if list_child.kind() == "import_spec" {
                                if let Some(import) = self.parse_import_spec(list_child, source) {
                                    imports.push(import);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn parse_import_spec(&self, node: Node, source: &str) -> Option<ImportStatement> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "interpreted_string_literal" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    let module = text.trim_matches('"').to_string();
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
    fn test_extract_go_function() {
        let code = r#"
package main

func Add(a int, b int) int {
    return a + b
}
"#;

        let extractor = GoExtractor;
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.functions.len(), 1);
        assert_eq!(ast.functions[0].name, "Add");
        assert_eq!(ast.functions[0].parameters.len(), 2);
        assert_eq!(ast.functions[0].visibility, Visibility::Public); // Capitalized
    }

    #[test]
    fn test_extract_struct() {
        let code = r#"
package main

type User struct {
    Name string
    Age  int
}
"#;

        let extractor = GoExtractor;
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.types.len(), 1);
        assert_eq!(ast.types[0].name, "User");
        assert_eq!(ast.types[0].kind, TypeKind::Struct);
    }

    #[test]
    fn test_extract_interface() {
        let code = r#"
package main

type Reader interface {
    Read(p []byte) (n int, err error)
}
"#;

        let extractor = GoExtractor;
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.types.len(), 1);
        assert_eq!(ast.types[0].name, "Reader");
        assert_eq!(ast.types[0].kind, TypeKind::Interface);
    }

    #[test]
    fn test_extract_imports() {
        let code = r#"
package main

import (
    "fmt"
    "net/http"
)
"#;

        let extractor = GoExtractor;
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.imports.len(), 2);
        assert!(ast.imports.iter().any(|i| i.module == "fmt"));
        assert!(ast.imports.iter().any(|i| i.module == "net/http"));
    }

    #[test]
    fn test_visibility_from_case() {
        let code = r#"
package main

func PublicFunc() {}
func privateFunc() {}
"#;

        let extractor = GoExtractor;
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.functions.len(), 2);
        assert_eq!(ast.functions[0].visibility, Visibility::Public);
        assert_eq!(ast.functions[1].visibility, Visibility::Private);
    }
}
