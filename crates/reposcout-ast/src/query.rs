use serde::{Deserialize, Serialize};

/// Parsed query with extracted filters
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedQuery {
    /// The cleaned search query
    pub query: String,
    /// Extracted filters
    pub filters: AstQueryFilters,
}

/// AST-based query filters
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AstQueryFilters {
    /// Function name filter (from "fn:parse" or "--function-name")
    pub function_name: Option<String>,
    /// Class/struct name filter (from "class:User" or "--class-name")
    pub class_name: Option<String>,
    /// Async function filter (from "async fn" in query)
    pub is_async: Option<bool>,
    /// Public visibility filter
    pub is_public: Option<bool>,
    /// Generic type filter
    pub is_generic: Option<bool>,
}

impl AstQueryFilters {
    /// Check if any filters are set
    pub fn has_filters(&self) -> bool {
        self.function_name.is_some()
            || self.class_name.is_some()
            || self.is_async.is_some()
            || self.is_public.is_some()
            || self.is_generic.is_some()
    }

    /// Merge with another filter set (other takes precedence)
    pub fn merge(&mut self, other: &AstQueryFilters) {
        if other.function_name.is_some() {
            self.function_name = other.function_name.clone();
        }
        if other.class_name.is_some() {
            self.class_name = other.class_name.clone();
        }
        if other.is_async.is_some() {
            self.is_async = other.is_async;
        }
        if other.is_public.is_some() {
            self.is_public = other.is_public;
        }
        if other.is_generic.is_some() {
            self.is_generic = other.is_generic;
        }
    }
}

/// Parse a search query to extract AST filters
pub fn parse_query(query: &str) -> ParsedQuery {
    let mut filters = AstQueryFilters::default();
    let mut cleaned_parts = Vec::new();

    // Split query into words
    let words: Vec<&str> = query.split_whitespace().collect();

    for word in words {
        // Check for prefix syntax: "fn:name", "class:Name"
        if let Some((prefix, value)) = word.split_once(':') {
            match prefix.to_lowercase().as_str() {
                "fn" | "function" | "func" | "method" => {
                    filters.function_name = Some(value.to_string());
                    continue;
                }
                "class" | "struct" | "type" | "interface" | "trait" => {
                    filters.class_name = Some(value.to_string());
                    continue;
                }
                _ => {}
            }
        }

        // Check for standalone keywords
        match word.to_lowercase().as_str() {
            "async" => {
                filters.is_async = Some(true);
                continue;
            }
            "pub" | "public" => {
                filters.is_public = Some(true);
                continue;
            }
            "generic" => {
                filters.is_generic = Some(true);
                continue;
            }
            _ => {}
        }

        // Keep the word in the query
        cleaned_parts.push(word);
    }

    // Detect natural language intent
    detect_intent(&cleaned_parts, &mut filters);

    ParsedQuery {
        query: cleaned_parts.join(" "),
        filters,
    }
}

/// Detect intent from natural language queries
fn detect_intent(words: &[&str], filters: &mut AstQueryFilters) {
    let query_lower = words.join(" ").to_lowercase();

    // Detect function-related queries
    if query_lower.contains("function") || query_lower.contains("method") {
        // Look for "function that" or "function to" patterns
        if query_lower.contains("function that") || query_lower.contains("function to") {
            // This is a function search
            if filters.function_name.is_none() {
                // Try to extract the function purpose
                // e.g., "function that parses json" -> might want to search for "parse"
                for (i, word) in words.iter().enumerate() {
                    if word.to_lowercase() == "function" && i + 2 < words.len() {
                        // Skip "that" or "to"
                        if ["that", "to", "which"].contains(&words[i + 1].to_lowercase().as_str())
                        {
                            if let Some(&verb) = words.get(i + 2) {
                                // Use the verb as a hint
                                filters.function_name = Some(verb.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Detect async pattern
    if query_lower.contains("async function")
        || query_lower.contains("asynchronous")
        || query_lower.contains("async fn")
    {
        filters.is_async = Some(true);
    }

    // Detect class/struct pattern
    if query_lower.contains("class") || query_lower.contains("struct") {
        // This might be a type search
        // e.g., "class User" or "struct Config"
        for (i, word) in words.iter().enumerate() {
            if ["class", "struct", "type"]
                .contains(&word.to_lowercase().as_str())
                && i + 1 < words.len()
            {
                if let Some(&name) = words.get(i + 1) {
                    if !["that", "which", "with", "for"]
                        .contains(&name.to_lowercase().as_str())
                    {
                        filters.class_name = Some(name.to_string());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_function_prefix() {
        let parsed = parse_query("fn:parse json data");
        assert_eq!(parsed.filters.function_name, Some("parse".to_string()));
        assert_eq!(parsed.query, "json data");
    }

    #[test]
    fn test_parse_class_prefix() {
        let parsed = parse_query("class:User authentication");
        assert_eq!(parsed.filters.class_name, Some("User".to_string()));
        assert_eq!(parsed.query, "authentication");
    }

    #[test]
    fn test_parse_async_keyword() {
        let parsed = parse_query("async fetch data");
        assert_eq!(parsed.filters.is_async, Some(true));
        assert_eq!(parsed.query, "fetch data");
    }

    #[test]
    fn test_parse_combined_filters() {
        let parsed = parse_query("async fn:fetch_data public");
        assert_eq!(parsed.filters.function_name, Some("fetch_data".to_string()));
        assert_eq!(parsed.filters.is_async, Some(true));
        assert_eq!(parsed.filters.is_public, Some(true));
        assert_eq!(parsed.query, "");
    }

    #[test]
    fn test_parse_natural_language_function() {
        let parsed = parse_query("function that parses command line arguments");
        // Should detect this is a function query
        assert_eq!(parsed.filters.function_name, Some("parses".to_string()));
        assert!(parsed.query.contains("command"));
    }

    #[test]
    fn test_parse_natural_language_async() {
        let parsed = parse_query("async function to fetch user data");
        assert_eq!(parsed.filters.is_async, Some(true));
    }

    #[test]
    fn test_parse_class_name() {
        let parsed = parse_query("class User with email field");
        assert_eq!(parsed.filters.class_name, Some("User".to_string()));
        assert!(parsed.query.contains("email"));
    }

    #[test]
    fn test_no_filters() {
        let parsed = parse_query("search for authentication code");
        assert!(!parsed.filters.has_filters());
        assert_eq!(parsed.query, "search for authentication code");
    }

    #[test]
    fn test_merge_filters() {
        let mut filters1 = AstQueryFilters {
            function_name: Some("test".to_string()),
            is_async: Some(true),
            ..Default::default()
        };

        let filters2 = AstQueryFilters {
            function_name: Some("override".to_string()),
            class_name: Some("User".to_string()),
            ..Default::default()
        };

        filters1.merge(&filters2);
        assert_eq!(filters1.function_name, Some("override".to_string()));
        assert_eq!(filters1.is_async, Some(true));
        assert_eq!(filters1.class_name, Some("User".to_string()));
    }
}
