use reposcout_ast::extractors::get_extractor;
use reposcout_core::models::{TypeKind, Visibility};

#[test]
fn test_extract_simple_function() {
    let code = r#"
        pub fn add(a: i32, b: i32) -> i32 {
            a + b
        }
    "#;

    let extractor = get_extractor("rust").expect("Rust extractor should be available");
    let ast = extractor.extract_all(code).unwrap();

    assert!(ast.parse_success);
    assert_eq!(ast.functions.len(), 1);

    let func = &ast.functions[0];
    assert_eq!(func.name, "add");
    assert_eq!(func.parameters.len(), 2);
    assert_eq!(func.parameters[0].name, "a");
    assert_eq!(func.parameters[0].param_type, Some("i32".to_string()));
    assert_eq!(func.parameters[1].name, "b");
    assert_eq!(func.parameters[1].param_type, Some("i32".to_string()));
    assert_eq!(func.return_type, Some("i32".to_string()));
    assert_eq!(func.visibility, Visibility::Public);
    assert!(!func.is_async);
}

#[test]
fn test_extract_async_function() {
    let code = r#"
        async fn fetch_data() -> Result<String> {
            Ok("data".to_string())
        }
    "#;

    let extractor = get_extractor("rust").unwrap();
    let ast = extractor.extract_all(code).unwrap();

    assert_eq!(ast.functions.len(), 1);
    let func = &ast.functions[0];
    assert_eq!(func.name, "fetch_data");
    assert!(func.is_async);
    assert_eq!(func.return_type, Some("Result<String>".to_string()));
}

#[test]
fn test_extract_function_with_self() {
    let code = r#"
        pub fn method(&self, value: u32) -> bool {
            true
        }
    "#;

    let extractor = get_extractor("rust").unwrap();
    let ast = extractor.extract_all(code).unwrap();

    assert_eq!(ast.functions.len(), 1);
    let func = &ast.functions[0];
    assert_eq!(func.name, "method");
    assert_eq!(func.parameters.len(), 2);
    assert!(func.parameters[0].name.contains("self"));
    assert_eq!(func.parameters[1].name, "value");
}

#[test]
fn test_extract_struct() {
    let code = r#"
        pub struct User {
            pub id: u64,
            name: String,
        }
    "#;

    let extractor = get_extractor("rust").unwrap();
    let ast = extractor.extract_all(code).unwrap();

    assert_eq!(ast.types.len(), 1);
    let type_def = &ast.types[0];
    assert_eq!(type_def.name, "User");
    assert_eq!(type_def.kind, TypeKind::Struct);
    assert_eq!(type_def.fields.len(), 2);

    assert_eq!(type_def.fields[0].name, "id");
    assert_eq!(type_def.fields[0].field_type, Some("u64".to_string()));
    assert_eq!(type_def.fields[0].visibility, Visibility::Public);

    assert_eq!(type_def.fields[1].name, "name");
    assert_eq!(type_def.fields[1].field_type, Some("String".to_string()));
    assert_eq!(type_def.fields[1].visibility, Visibility::Private);
}

#[test]
fn test_extract_enum() {
    let code = r#"
        pub enum Status {
            Active,
            Inactive,
        }
    "#;

    let extractor = get_extractor("rust").unwrap();
    let ast = extractor.extract_all(code).unwrap();

    assert_eq!(ast.types.len(), 1);
    let type_def = &ast.types[0];
    assert_eq!(type_def.name, "Status");
    assert_eq!(type_def.kind, TypeKind::Enum);
}

#[test]
fn test_extract_trait() {
    let code = r#"
        pub trait Display {
            fn display(&self) -> String;
        }
    "#;

    let extractor = get_extractor("rust").unwrap();
    let ast = extractor.extract_all(code).unwrap();

    assert_eq!(ast.types.len(), 1);
    let type_def = &ast.types[0];
    assert_eq!(type_def.name, "Display");
    assert_eq!(type_def.kind, TypeKind::Trait);
}

#[test]
fn test_extract_use_statements() {
    let code = r#"
        use std::collections::HashMap;
        use serde::{Serialize, Deserialize};
        use crate::models::*;
    "#;

    let extractor = get_extractor("rust").unwrap();
    let ast = extractor.extract_all(code).unwrap();

    assert_eq!(ast.imports.len(), 3);
    assert!(ast.imports[0].module.contains("HashMap"));
    assert!(ast.imports[1].module.contains("Serialize"));
    assert!(ast.imports[2].is_wildcard);
}

#[test]
fn test_extract_multiple_functions() {
    let code = r#"
        fn helper() {}

        pub async fn process(data: &str) -> Result<()> {
            Ok(())
        }

        pub fn validate(input: String) -> bool {
            true
        }
    "#;

    let extractor = get_extractor("rust").unwrap();
    let ast = extractor.extract_all(code).unwrap();

    assert_eq!(ast.functions.len(), 3);
    assert_eq!(ast.functions[0].name, "helper");
    assert_eq!(ast.functions[1].name, "process");
    assert_eq!(ast.functions[2].name, "validate");
}

#[test]
fn test_extract_complex_code() {
    let code = r#"
        use std::fmt;

        pub struct Config {
            pub port: u16,
            host: String,
        }

        impl Config {
            pub fn new(port: u16) -> Self {
                Self {
                    port,
                    host: "localhost".to_string(),
                }
            }

            pub async fn connect(&self) -> Result<Connection> {
                todo!()
            }
        }

        pub enum Status {
            Connected,
            Disconnected,
        }
    "#;

    let extractor = get_extractor("rust").unwrap();
    let ast = extractor.extract_all(code).unwrap();

    // Should extract functions from impl block
    assert!(ast.functions.len() >= 2);

    // Should extract struct and enum
    assert!(ast.types.len() >= 2);

    // Should have imports
    assert!(ast.imports.len() >= 1);

    // Should generate summary
    assert!(!ast.structure_summary.is_empty());
}

#[test]
fn test_unsupported_language() {
    let result = get_extractor("cobol");
    assert!(result.is_none());
}

#[test]
fn test_line_numbers() {
    let code = r#"
fn first() {}

fn second() {}

fn third() {}
"#;

    let extractor = get_extractor("rust").unwrap();
    let ast = extractor.extract_all(code).unwrap();

    assert_eq!(ast.functions.len(), 3);
    assert_eq!(ast.functions[0].line_number, 2);
    assert_eq!(ast.functions[1].line_number, 4);
    assert_eq!(ast.functions[2].line_number, 6);
}
