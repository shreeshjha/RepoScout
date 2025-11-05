use regex::Regex;
use reposcout_core::models::Repository;

/// Maximum tokens to use for embedding (BERT limit)
const MAX_TOKENS: usize = 512;

/// Preprocess repository data into text suitable for embedding
pub fn preprocess_repository(repo: &Repository, readme: Option<&str>) -> String {
    let mut parts = Vec::new();

    // 1. Repository name (important for matching)
    parts.push(repo.full_name.clone());

    // 2. Language (if available)
    if let Some(lang) = &repo.language {
        parts.push(lang.clone());
    }

    // 3. Description (high priority)
    if let Some(desc) = &repo.description {
        if !desc.is_empty() {
            parts.push(clean_text(desc));
        }
    }

    // 4. Topics (good semantic signal)
    if !repo.topics.is_empty() {
        parts.push(repo.topics.join(" "));
    }

    // 5. README excerpt (first 500 words for context)
    if let Some(readme_text) = readme {
        if !readme_text.is_empty() {
            let excerpt = extract_readme_excerpt(readme_text, 500);
            if !excerpt.is_empty() {
                parts.push(clean_text(&excerpt));
            }
        }
    }

    // Combine all parts
    let combined = parts.join(" ");

    // Truncate to token limit
    truncate_to_tokens(&combined, MAX_TOKENS)
}

/// Preprocess a search query
pub fn preprocess_query(query: &str) -> String {
    let cleaned = clean_text(query);
    truncate_to_tokens(&cleaned, MAX_TOKENS)
}

/// Clean text by removing special characters and normalizing whitespace
fn clean_text(text: &str) -> String {
    // Remove URLs
    let url_pattern = Regex::new(r"https?://[^\s]+").unwrap();
    let text = url_pattern.replace_all(text, "");

    // Remove markdown syntax
    let markdown_pattern = Regex::new(r"[#*`\[\]()_~]").unwrap();
    let text = markdown_pattern.replace_all(&text, " ");

    // Remove special characters but keep letters, numbers, spaces
    let special_chars = Regex::new(r"[^a-zA-Z0-9\s\-]").unwrap();
    let text = special_chars.replace_all(&text, " ");

    // Normalize whitespace
    let whitespace = Regex::new(r"\s+").unwrap();
    let text = whitespace.replace_all(&text, " ");

    // Lowercase for consistency
    text.trim().to_lowercase()
}

/// Extract meaningful excerpt from README
fn extract_readme_excerpt(readme: &str, max_words: usize) -> String {
    // Try to skip the title and badges, focus on description
    let lines: Vec<&str> = readme.lines().collect();

    let mut content_start = 0;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        // Skip title lines (starting with #)
        // Skip badge lines (containing shields.io, badge, etc.)
        if !trimmed.starts_with('#')
            && !trimmed.contains("shields.io")
            && !trimmed.contains("badge")
            && !trimmed.contains("![")
            && trimmed.len() > 20
        {
            content_start = i;
            break;
        }
    }

    // Get text from content start
    let content = lines[content_start..].join(" ");

    // Split into words and take first N words
    let words: Vec<&str> = content.split_whitespace().take(max_words).collect();

    words.join(" ")
}

/// Truncate text to approximately N tokens
/// This is a simple word-based approximation (1 token ~= 1 word for English)
fn truncate_to_tokens(text: &str, max_tokens: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.len() <= max_tokens {
        return text.to_string();
    }

    words[..max_tokens].join(" ")
}

/// Calculate simple text similarity (for testing preprocessing quality)
pub fn calculate_text_similarity(text1: &str, text2: &str) -> f32 {
    let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
    let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();

    if words1.is_empty() && words2.is_empty() {
        return 1.0;
    }

    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f32 / union as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text() {
        let input = "Hello! This is a **test** with [links](http://example.com) and `code`.";
        let output = clean_text(input);
        assert!(!output.contains('!'));
        assert!(!output.contains('*'));
        assert!(!output.contains('['));
        assert!(!output.contains("http"));
        assert!(output.contains("hello"));
        assert!(output.contains("test"));
    }

    #[test]
    fn test_truncate_to_tokens() {
        let text = (0..1000).map(|i| format!("word{}", i)).collect::<Vec<_>>().join(" ");
        let truncated = truncate_to_tokens(&text, 100);
        let word_count = truncated.split_whitespace().count();
        assert_eq!(word_count, 100);
    }

    #[test]
    fn test_extract_readme_excerpt() {
        let readme = r#"
# Project Title

[![Build Status](https://shields.io/badge/build-passing-green)]

This is the actual description of the project.
It provides useful context about what the project does.
More information here.
        "#;

        let excerpt = extract_readme_excerpt(readme, 20);
        assert!(excerpt.contains("description"));
        assert!(!excerpt.contains("shields.io"));
        assert!(!excerpt.contains('#'));
    }

    #[test]
    fn test_calculate_text_similarity() {
        let text1 = "rust web framework";
        let text2 = "rust web server framework";
        let similarity = calculate_text_similarity(text1, text2);
        assert!(similarity > 0.5);

        let text3 = "completely different words";
        let similarity2 = calculate_text_similarity(text1, text3);
        assert!(similarity2 < 0.3);
    }
}
