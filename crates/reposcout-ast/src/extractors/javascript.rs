use super::AstExtractor;
use crate::error::{AstError, Result};
use crate::parser::ParserCache;
use reposcout_core::models::{
    AstMetadata, FunctionSignature, ImportStatement, Parameter, TypeDefinition, TypeKind,
    Visibility,
};
use tree_sitter::Node;

pub struct JavaScriptExtractor {
    language: &'static str,
}

impl JavaScriptExtractor {
    pub fn new(language: &'static str) -> Self {
        Self { language }
    }
}

impl AstExtractor for JavaScriptExtractor {
    fn language(&self) -> &'static str {
        self.language
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

impl JavaScriptExtractor {
    fn extract_functions_recursive(
        &self,
        node: Node,
        source: &str,
        functions: &mut Vec<FunctionSignature>,
    ) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_declaration" | "function" | "method_definition" | "function_signature" => {
                    if let Some(func) = self.parse_function(child, source) {
                        functions.push(func);
                    }
                }
                "lexical_declaration" | "variable_declaration" => {
                    // Check for arrow functions and function expressions
                    if let Some(func) = self.parse_variable_function(child, source) {
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
        let mut is_async = false;
        let line_number = node.start_position().row + 1;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "async" => {
                    is_async = true;
                }
                "identifier" | "property_identifier" => {
                    if name.is_empty() {
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            name = text.to_string();
                        }
                    }
                }
                "formal_parameters" => {
                    parameters = self.parse_parameters(child, source);
                }
                "type_annotation" => {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        return_type = Some(text.trim_start_matches(':').trim().to_string());
                    }
                }
                _ => {}
            }
        }

        // For methods without explicit names, try to get from parent
        if name.is_empty() {
            if let Some(parent) = node.parent() {
                if parent.kind() == "pair" {
                    // Object method
                    if let Some(key_node) = parent.child_by_field_name("key") {
                        if let Ok(text) = key_node.utf8_text(source.as_bytes()) {
                            name = text.to_string();
                        }
                    }
                }
            }
        }

        if !name.is_empty() {
            Some(FunctionSignature {
                name,
                parameters,
                return_type,
                visibility: Visibility::Public, // JS doesn't have explicit visibility
                is_async,
                is_generic: false,
                doc_comment: None,
                line_number,
            })
        } else {
            None
        }
    }

    fn parse_variable_function(&self, node: Node, source: &str) -> Option<FunctionSignature> {
        // Look for: const foo = async (params) => {}
        // Or: const foo = function(params) {}
        let mut name = String::new();
        let mut _is_arrow_func = false;
        let mut _is_func_expr = false;
        let mut is_async = false;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                let mut decl_cursor = child.walk();
                for decl_child in child.children(&mut decl_cursor) {
                    match decl_child.kind() {
                        "identifier" => {
                            if let Ok(text) = decl_child.utf8_text(source.as_bytes()) {
                                name = text.to_string();
                            }
                        }
                        "arrow_function" => {
                            _is_arrow_func = true;
                            // Check for async
                            let mut arrow_cursor = decl_child.walk();
                            for arrow_child in decl_child.children(&mut arrow_cursor) {
                                if arrow_child.kind() == "async" {
                                    is_async = true;
                                    break;
                                }
                            }
                            if !name.is_empty() {
                                return self.parse_arrow_function(decl_child, source, &name, is_async);
                            }
                        }
                        "function" | "function_expression" => {
                            _is_func_expr = true;
                            if !name.is_empty() {
                                return self.parse_function(decl_child, source).map(|mut f| {
                                    f.name = name.clone();
                                    f
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        None
    }

    fn parse_arrow_function(
        &self,
        node: Node,
        source: &str,
        name: &str,
        is_async: bool,
    ) -> Option<FunctionSignature> {
        let mut parameters = Vec::new();
        let mut return_type = None;
        let line_number = node.start_position().row + 1;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "formal_parameters" | "identifier" => {
                    if child.kind() == "identifier" {
                        // Single parameter without parentheses
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            parameters.push(Parameter {
                                name: text.to_string(),
                                param_type: None,
                                is_optional: false,
                            });
                        }
                    } else {
                        parameters = self.parse_parameters(child, source);
                    }
                }
                "type_annotation" => {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        return_type = Some(text.trim_start_matches(':').trim().to_string());
                    }
                }
                _ => {}
            }
        }

        Some(FunctionSignature {
            name: name.to_string(),
            parameters,
            return_type,
            visibility: Visibility::Public,
            is_async,
            is_generic: false,
            doc_comment: None,
            line_number,
        })
    }

    fn parse_parameters(&self, node: Node, source: &str) -> Vec<Parameter> {
        let mut parameters = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "required_parameter" | "optional_parameter" | "identifier" => {
                    if let Some(param) = self.parse_parameter(child, source) {
                        parameters.push(param);
                    }
                }
                _ => {}
            }
        }

        parameters
    }

    fn parse_parameter(&self, node: Node, source: &str) -> Option<Parameter> {
        let mut name = String::new();
        let mut param_type = None;
        let is_optional = node.kind() == "optional_parameter";

        if node.kind() == "identifier" {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                name = text.to_string();
            }
        } else {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "identifier" => {
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            name = text.to_string();
                        }
                    }
                    "type_annotation" => {
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
                is_optional,
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
            match child.kind() {
                "class_declaration" => {
                    if let Some(type_def) = self.parse_class(child, source) {
                        types.push(type_def);
                    }
                }
                "interface_declaration" => {
                    if let Some(type_def) = self.parse_interface(child, source) {
                        types.push(type_def);
                    }
                }
                "type_alias_declaration" => {
                    if let Some(type_def) = self.parse_type_alias(child, source) {
                        types.push(type_def);
                    }
                }
                _ => {
                    self.extract_types_recursive(child, source, types);
                }
            }
        }
    }

    fn parse_class(&self, node: Node, source: &str) -> Option<TypeDefinition> {
        let mut name = String::new();
        let mut methods = Vec::new();
        let mut implements = Vec::new();
        let line_number = node.start_position().row + 1;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" | "type_identifier" => {
                    if name.is_empty() {
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            name = text.to_string();
                        }
                    }
                }
                "class_heritage" => {
                    // Parse extends/implements
                    let mut heritage_cursor = child.walk();
                    for heritage_child in child.children(&mut heritage_cursor) {
                        if heritage_child.kind() == "identifier" || heritage_child.kind() == "type_identifier" {
                            if let Ok(text) = heritage_child.utf8_text(source.as_bytes()) {
                                implements.push(text.to_string());
                            }
                        }
                    }
                }
                "class_body" => {
                    // Extract method names
                    let mut body_cursor = child.walk();
                    for body_child in child.children(&mut body_cursor) {
                        if body_child.kind() == "method_definition" {
                            if let Some(method_name) = self.get_method_name(body_child, source) {
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
                fields: Vec::new(),
                methods,
                implements,
                doc_comment: None,
                line_number,
            })
        } else {
            None
        }
    }

    fn parse_interface(&self, node: Node, source: &str) -> Option<TypeDefinition> {
        let mut name = String::new();
        let line_number = node.start_position().row + 1;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_identifier" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    name = text.to_string();
                    break;
                }
            }
        }

        if !name.is_empty() {
            Some(TypeDefinition {
                name,
                kind: TypeKind::Interface,
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

    fn parse_type_alias(&self, node: Node, source: &str) -> Option<TypeDefinition> {
        let mut name = String::new();
        let line_number = node.start_position().row + 1;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_identifier" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    name = text.to_string();
                    break;
                }
            }
        }

        if !name.is_empty() {
            Some(TypeDefinition {
                name,
                kind: TypeKind::Type,
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

    fn get_method_name(&self, node: Node, source: &str) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "property_identifier" {
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
            if child.kind() == "import_statement" {
                if let Some(import) = self.parse_import(child, source) {
                    imports.push(import);
                }
            } else {
                self.extract_imports_recursive(child, source, imports);
            }
        }
    }

    fn parse_import(&self, node: Node, source: &str) -> Option<ImportStatement> {
        let mut module = String::new();
        let mut items = Vec::new();
        let mut is_wildcard = false;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "string" => {
                    // Module path
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        module = text.trim_matches('"').trim_matches('\'').to_string();
                    }
                }
                "import_clause" => {
                    // Parse named imports
                    let mut clause_cursor = child.walk();
                    for clause_child in child.children(&mut clause_cursor) {
                        match clause_child.kind() {
                            "identifier" => {
                                if let Ok(text) = clause_child.utf8_text(source.as_bytes()) {
                                    items.push(text.to_string());
                                }
                            }
                            "named_imports" => {
                                let mut named_cursor = clause_child.walk();
                                for named_child in clause_child.children(&mut named_cursor) {
                                    if named_child.kind() == "import_specifier" {
                                        if let Some(name) = self.get_import_specifier_name(named_child, source) {
                                            items.push(name);
                                        }
                                    }
                                }
                            }
                            "namespace_import" => {
                                is_wildcard = true;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        if !module.is_empty() {
            Some(ImportStatement {
                module,
                items,
                is_wildcard,
            })
        } else {
            None
        }
    }

    fn get_import_specifier_name(&self, node: Node, source: &str) -> Option<String> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_function_declaration() {
        let code = r#"
function greet(name) {
    return `Hello, ${name}!`;
}
"#;

        let extractor = JavaScriptExtractor::new("javascript");
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.functions.len(), 1);
        assert_eq!(ast.functions[0].name, "greet");
        assert_eq!(ast.functions[0].parameters.len(), 1);
        assert_eq!(ast.functions[0].parameters[0].name, "name");
    }

    #[test]
    fn test_extract_arrow_function() {
        let code = r#"
const add = (a, b) => a + b;
"#;

        let extractor = JavaScriptExtractor::new("javascript");
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.functions.len(), 1);
        assert_eq!(ast.functions[0].name, "add");
        assert_eq!(ast.functions[0].parameters.len(), 2);
    }

    #[test]
    fn test_extract_async_function() {
        let code = r#"
async function fetchData(url) {
    const response = await fetch(url);
    return response.json();
}
"#;

        let extractor = JavaScriptExtractor::new("javascript");
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.functions.len(), 1);
        assert_eq!(ast.functions[0].name, "fetchData");
        assert!(ast.functions[0].is_async);
    }

    #[test]
    fn test_extract_class() {
        let code = r#"
class User {
    constructor(name) {
        this.name = name;
    }

    greet() {
        return `Hello, ${this.name}`;
    }
}
"#;

        let extractor = JavaScriptExtractor::new("javascript");
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.types.len(), 1);
        assert_eq!(ast.types[0].name, "User");
        assert_eq!(ast.types[0].kind, TypeKind::Class);
        assert_eq!(ast.types[0].methods.len(), 2);
        assert!(ast.types[0].methods.contains(&"constructor".to_string()));
        assert!(ast.types[0].methods.contains(&"greet".to_string()));
    }

    #[test]
    fn test_extract_imports() {
        let code = r#"
import React from 'react';
import { useState, useEffect } from 'react';
import * as utils from './utils';
"#;

        let extractor = JavaScriptExtractor::new("javascript");
        let ast = extractor.extract_all(code).unwrap();

        assert_eq!(ast.imports.len(), 3);
        assert!(ast.imports.iter().any(|i| i.module == "react" && i.items.contains(&"React".to_string())));
        assert!(ast.imports.iter().any(|i| i.module == "react" && i.items.contains(&"useState".to_string())));
        assert!(ast.imports.iter().any(|i| i.module == "./utils" && i.is_wildcard));
    }
}
