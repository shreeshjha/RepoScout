use crate::query::AstQueryFilters;
use reposcout_core::models::AstMetadata;

#[cfg(test)]
use reposcout_core::models::{FunctionSignature, TypeDefinition};

/// Score AST metadata against query filters
/// Returns a score from 0.0 (no match) to 1.0 (perfect match)
pub fn score_ast_match(ast: &AstMetadata, filters: &AstQueryFilters, query: &str) -> f32 {
    if !filters.has_filters() {
        // No AST filters, just do text matching on structure
        return score_text_match(ast, query);
    }

    let mut score = 0.0;
    let mut weight_sum = 0.0;

    // Function name match (weight: 1.0)
    if let Some(ref fn_name) = filters.function_name {
        weight_sum += 1.0;
        for func in &ast.functions {
            let match_score = string_similarity(&func.name, fn_name);
            if match_score > score {
                score += match_score;
            }
        }
    }

    // Class/struct name match (weight: 1.0)
    if let Some(ref class_name) = filters.class_name {
        weight_sum += 1.0;
        for type_def in &ast.types {
            let match_score = string_similarity(&type_def.name, class_name);
            if match_score > score {
                score += match_score;
            }
        }
    }

    // Async modifier match (weight: 0.5)
    if let Some(is_async) = filters.is_async {
        weight_sum += 0.5;
        let has_async = ast.functions.iter().any(|f| f.is_async);
        if has_async == is_async {
            score += 0.5;
        }
    }

    // Public visibility match (weight: 0.3)
    if let Some(is_public) = filters.is_public {
        weight_sum += 0.3;
        let has_public = ast.functions.iter().any(|f| f.visibility == reposcout_core::models::Visibility::Public)
            || ast.types.iter().any(|t| {
                t.fields.iter().any(|field| field.visibility == reposcout_core::models::Visibility::Public)
            });
        if has_public == is_public {
            score += 0.3;
        }
    }

    // Generic type match (weight: 0.3)
    if let Some(is_generic) = filters.is_generic {
        weight_sum += 0.3;
        let has_generic = ast.functions.iter().any(|f| f.is_generic);
        if has_generic == is_generic {
            score += 0.3;
        }
    }

    // Normalize score by total weight
    if weight_sum > 0.0 {
        score / weight_sum
    } else {
        0.0
    }
}

/// Score based on text similarity to query
fn score_text_match(ast: &AstMetadata, query: &str) -> f32 {
    if query.is_empty() {
        return 0.5; // Neutral score if no query text
    }

    let query_lower = query.to_lowercase();
    let mut best_score = 0.0;

    // Check function names
    for func in &ast.functions {
        let func_score = fuzzy_contains(&func.name.to_lowercase(), &query_lower);
        if func_score > best_score {
            best_score = func_score;
        }
    }

    // Check type names
    for type_def in &ast.types {
        let type_score = fuzzy_contains(&type_def.name.to_lowercase(), &query_lower);
        if type_score > best_score {
            best_score = type_score;
        }
    }

    // Check structure summary
    if !ast.structure_summary.is_empty() {
        let summary_score =
            fuzzy_contains(&ast.structure_summary.to_lowercase(), &query_lower) * 0.5;
        if summary_score > best_score {
            best_score = summary_score;
        }
    }

    best_score
}

/// Calculate string similarity (simple edit distance based)
pub fn string_similarity(s1: &str, s2: &str) -> f32 {
    let s1_lower = s1.to_lowercase();
    let s2_lower = s2.to_lowercase();

    // Exact match
    if s1_lower == s2_lower {
        return 1.0;
    }

    // Contains match
    if s1_lower.contains(&s2_lower) || s2_lower.contains(&s1_lower) {
        return 0.8;
    }

    // Prefix match
    if s1_lower.starts_with(&s2_lower) || s2_lower.starts_with(&s1_lower) {
        return 0.7;
    }

    // Calculate Levenshtein-like similarity
    let max_len = s1_lower.len().max(s2_lower.len());
    if max_len == 0 {
        return 1.0;
    }

    let common_chars = s1_lower
        .chars()
        .filter(|c| s2_lower.contains(*c))
        .count();

    common_chars as f32 / max_len as f32
}

/// Check if text fuzzy contains query
fn fuzzy_contains(text: &str, query: &str) -> f32 {
    if text.contains(query) {
        return 1.0;
    }

    // Check if all query words are in text
    let query_words: Vec<&str> = query.split_whitespace().collect();
    let matched_words = query_words.iter().filter(|w| text.contains(*w)).count();

    if query_words.is_empty() {
        0.0
    } else {
        matched_words as f32 / query_words.len() as f32
    }
}

/// Filter results based on AST query filters
pub fn filter_by_ast(results: Vec<(AstMetadata, f32)>, filters: &AstQueryFilters, min_score: f32) -> Vec<(AstMetadata, f32)> {
    results
        .into_iter()
        .filter(|(ast, score)| {
            if *score < min_score {
                return false;
            }

            // Apply hard filters
            if let Some(ref fn_name) = filters.function_name {
                if !ast.functions.iter().any(|f| {
                    let similarity = string_similarity(&f.name, fn_name);
                    similarity > 0.5 // At least 50% similar
                }) {
                    return false;
                }
            }

            if let Some(ref class_name) = filters.class_name {
                if !ast.types.iter().any(|t| {
                    let similarity = string_similarity(&t.name, class_name);
                    similarity > 0.5
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
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use reposcout_core::models::{ImportStatement, Parameter, Visibility};

    fn create_test_function(name: &str, is_async: bool, is_public: bool) -> FunctionSignature {
        FunctionSignature {
            name: name.to_string(),
            parameters: vec![],
            return_type: None,
            visibility: if is_public {
                Visibility::Public
            } else {
                Visibility::Private
            },
            is_async,
            is_generic: false,
            doc_comment: None,
            line_number: 1,
        }
    }

    fn create_test_type(name: &str) -> TypeDefinition {
        TypeDefinition {
            name: name.to_string(),
            kind: reposcout_core::models::TypeKind::Struct,
            fields: vec![],
            methods: vec![],
            implements: vec![],
            doc_comment: None,
            line_number: 1,
        }
    }

    #[test]
    fn test_function_name_exact_match() {
        let ast = AstMetadata {
            language: "rust".to_string(),
            functions: vec![create_test_function("parse_args", false, true)],
            types: vec![],
            imports: vec![],
            structure_summary: String::new(),
            parse_success: true,
            parse_error: None,
        };

        let filters = AstQueryFilters {
            function_name: Some("parse_args".to_string()),
            ..Default::default()
        };

        let score = score_ast_match(&ast, &filters, "");
        assert_eq!(score, 1.0, "Exact function name match should score 1.0");
    }

    #[test]
    fn test_function_name_partial_match() {
        let ast = AstMetadata {
            language: "rust".to_string(),
            functions: vec![create_test_function("parse_arguments", false, true)],
            types: vec![],
            imports: vec![],
            structure_summary: String::new(),
            parse_success: true,
            parse_error: None,
        };

        let filters = AstQueryFilters {
            function_name: Some("parse".to_string()),
            ..Default::default()
        };

        let score = score_ast_match(&ast, &filters, "");
        assert!(score > 0.7, "Partial match should score > 0.7");
    }

    #[test]
    fn test_async_filter() {
        let ast = AstMetadata {
            language: "rust".to_string(),
            functions: vec![create_test_function("fetch", true, true)],
            types: vec![],
            imports: vec![],
            structure_summary: String::new(),
            parse_success: true,
            parse_error: None,
        };

        let filters = AstQueryFilters {
            is_async: Some(true),
            ..Default::default()
        };

        let score = score_ast_match(&ast, &filters, "");
        assert_eq!(score, 1.0, "Async match should score 1.0");
    }

    #[test]
    fn test_class_name_match() {
        let ast = AstMetadata {
            language: "rust".to_string(),
            functions: vec![],
            types: vec![create_test_type("User")],
            imports: vec![],
            structure_summary: String::new(),
            parse_success: true,
            parse_error: None,
        };

        let filters = AstQueryFilters {
            class_name: Some("User".to_string()),
            ..Default::default()
        };

        let score = score_ast_match(&ast, &filters, "");
        assert_eq!(score, 1.0, "Exact class name match should score 1.0");
    }

    #[test]
    fn test_combined_filters() {
        let ast = AstMetadata {
            language: "rust".to_string(),
            functions: vec![create_test_function("fetch_data", true, true)],
            types: vec![],
            imports: vec![],
            structure_summary: String::new(),
            parse_success: true,
            parse_error: None,
        };

        let filters = AstQueryFilters {
            function_name: Some("fetch_data".to_string()),
            is_async: Some(true),
            is_public: Some(true),
            ..Default::default()
        };

        let score = score_ast_match(&ast, &filters, "");
        assert_eq!(score, 1.0, "All matching filters should score 1.0");
    }

    #[test]
    fn test_text_match_no_filters() {
        let ast = AstMetadata {
            language: "rust".to_string(),
            functions: vec![create_test_function("parse_json", false, true)],
            types: vec![],
            imports: vec![],
            structure_summary: "functions: parse_json".to_string(),
            parse_success: true,
            parse_error: None,
        };

        let filters = AstQueryFilters::default();
        let score = score_ast_match(&ast, &filters, "parse json");
        assert!(score > 0.5, "Text match should score > 0.5");
    }

    #[test]
    fn test_string_similarity() {
        assert_eq!(string_similarity("parse", "parse"), 1.0);
        assert!(string_similarity("parse_args", "parse") > 0.7);
        assert!(string_similarity("Parser", "parser") > 0.9);
    }
}
