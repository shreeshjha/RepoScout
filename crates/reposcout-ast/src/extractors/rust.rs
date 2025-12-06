use super::AstExtractor;
use reposcout_core::models::{
    Field, FunctionSignature, ImportStatement, Parameter, TypeDefinition, TypeKind, Visibility,
};
use tree_sitter::Node;

pub struct RustExtractor;

impl AstExtractor for RustExtractor {
    fn language(&self) -> &'static str {
        "rust"
    }

    fn extract_functions(&self, node: Node, source: &str) -> Vec<FunctionSignature> {
        let mut functions = Vec::new();
        visit_functions(node, source, &mut functions);
        functions
    }

    fn extract_types(&self, node: Node, source: &str) -> Vec<TypeDefinition> {
        let mut types = Vec::new();
        visit_types(node, source, &mut types);
        types
    }

    fn extract_imports(&self, node: Node, source: &str) -> Vec<ImportStatement> {
        let mut imports = Vec::new();
        visit_imports(node, source, &mut imports);
        imports
    }
}

/// Recursively visit nodes to find function declarations
fn visit_functions(node: Node, source: &str, functions: &mut Vec<FunctionSignature>) {
    if node.kind() == "function_item" {
        if let Some(func) = parse_function(node, source) {
            functions.push(func);
        }
    }

    // Recurse to children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_functions(child, source, functions);
    }
}

/// Parse a function node into FunctionSignature
fn parse_function(node: Node, source: &str) -> Option<FunctionSignature> {
    let mut name = None;
    let mut parameters = Vec::new();
    let mut return_type = None;
    let mut visibility = Visibility::Private;
    let mut is_async = false;
    let mut found_arrow = false;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "visibility_modifier" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    if text.contains("pub") {
                        visibility = Visibility::Public;
                    }
                }
            }
            "identifier" => {
                if name.is_none() && !found_arrow {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        name = Some(text.to_string());
                    }
                }
            }
            "parameters" => {
                parameters = parse_parameters(child, source);
            }
            "->" => {
                found_arrow = true;
            }
            _ if found_arrow
                && return_type.is_none()
                && (child.kind().ends_with("_type") || child.kind() == "type_identifier") =>
            {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    return_type = Some(text.to_string());
                }
            }
            _ => {}
        }
    }

    // Check for async modifier by examining the node text
    if let Ok(text) = node.utf8_text(source.as_bytes()) {
        is_async = text.contains("async fn");
    }

    Some(FunctionSignature {
        name: name?,
        parameters,
        return_type,
        visibility,
        is_async,
        is_generic: false, // TODO: detect generics
        doc_comment: None, // TODO: extract doc comments
        line_number: node.start_position().row + 1,
    })
}

/// Parse function parameters
fn parse_parameters(node: Node, source: &str) -> Vec<Parameter> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "parameter" || child.kind() == "self_parameter" {
            if let Some(param) = parse_parameter(child, source) {
                params.push(param);
            }
        }
    }

    params
}

/// Parse a single parameter
fn parse_parameter(node: Node, source: &str) -> Option<Parameter> {
    let mut name = None;
    let mut param_type = None;

    // Handle self parameter specially
    if node.kind() == "self_parameter" {
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            return Some(Parameter {
                name: text.to_string(),
                param_type: None,
                is_optional: false,
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "pattern" => {
                if name.is_none() {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        // For patterns, extract just the identifier
                        let clean_name = text.split(':').next().unwrap_or(text).trim();
                        name = Some(clean_name.to_string());
                    }
                }
            }
            _ if child.kind().ends_with("_type") || child.kind() == "type_identifier" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    param_type = Some(text.to_string());
                }
            }
            _ => {}
        }
    }

    Some(Parameter {
        name: name?,
        param_type,
        is_optional: false,
    })
}

/// Recursively visit nodes to find type declarations
fn visit_types(node: Node, source: &str, types: &mut Vec<TypeDefinition>) {
    match node.kind() {
        "struct_item" => {
            if let Some(type_def) = parse_struct(node, source) {
                types.push(type_def);
            }
        }
        "enum_item" => {
            if let Some(type_def) = parse_enum(node, source) {
                types.push(type_def);
            }
        }
        "trait_item" => {
            if let Some(type_def) = parse_trait(node, source) {
                types.push(type_def);
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_types(child, source, types);
    }
}

/// Parse a struct definition
fn parse_struct(node: Node, source: &str) -> Option<TypeDefinition> {
    let mut name = None;
    let mut fields = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "type_identifier" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    name = Some(text.to_string());
                }
            }
            "field_declaration_list" => {
                fields = parse_fields(child, source);
            }
            _ => {}
        }
    }

    Some(TypeDefinition {
        name: name?,
        kind: TypeKind::Struct,
        fields,
        methods: Vec::new(), // TODO: associate impl blocks
        implements: Vec::new(),
        doc_comment: None,
        line_number: node.start_position().row + 1,
    })
}

/// Parse struct fields
fn parse_fields(node: Node, source: &str) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "field_declaration" {
            if let Some(field) = parse_field(child, source) {
                fields.push(field);
            }
        }
    }

    fields
}

/// Parse a single field
fn parse_field(node: Node, source: &str) -> Option<Field> {
    let mut name = None;
    let mut field_type = None;
    let mut visibility = Visibility::Private;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "visibility_modifier" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    if text.contains("pub") {
                        visibility = Visibility::Public;
                    }
                }
            }
            "field_identifier" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    name = Some(text.to_string());
                }
            }
            _ if child.kind().ends_with("_type") || child.kind() == "type_identifier" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    field_type = Some(text.to_string());
                }
            }
            _ => {}
        }
    }

    Some(Field {
        name: name?,
        field_type,
        visibility,
    })
}

/// Parse an enum definition
fn parse_enum(node: Node, source: &str) -> Option<TypeDefinition> {
    let mut name = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_identifier" {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                name = Some(text.to_string());
            }
            break;
        }
    }

    Some(TypeDefinition {
        name: name?,
        kind: TypeKind::Enum,
        fields: Vec::new(),
        methods: Vec::new(),
        implements: Vec::new(),
        doc_comment: None,
        line_number: node.start_position().row + 1,
    })
}

/// Parse a trait definition
fn parse_trait(node: Node, source: &str) -> Option<TypeDefinition> {
    let mut name = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_identifier" {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                name = Some(text.to_string());
            }
            break;
        }
    }

    Some(TypeDefinition {
        name: name?,
        kind: TypeKind::Trait,
        fields: Vec::new(),
        methods: Vec::new(),
        implements: Vec::new(),
        doc_comment: None,
        line_number: node.start_position().row + 1,
    })
}

/// Recursively visit nodes to find use declarations
fn visit_imports(node: Node, source: &str, imports: &mut Vec<ImportStatement>) {
    if node.kind() == "use_declaration" {
        if let Some(import) = parse_use(node, source) {
            imports.push(import);
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_imports(child, source, imports);
    }
}

/// Parse a use declaration
fn parse_use(node: Node, source: &str) -> Option<ImportStatement> {
    // Simple implementation: just get the whole use statement as module name
    let text = node.utf8_text(source.as_bytes()).ok()?;
    let cleaned = text
        .trim_start_matches("use ")
        .trim_end_matches(';')
        .trim();

    Some(ImportStatement {
        module: cleaned.to_string(),
        items: Vec::new(),
        is_wildcard: cleaned.contains('*'),
    })
}
