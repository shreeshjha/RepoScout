//! BM25 scoring for keyword-based retrieval
//!
//! Implements the Okapi BM25 ranking function for better keyword matching.

use reposcout_core::models::Repository;
use std::collections::HashMap;

/// BM25 scoring parameters
const K1: f32 = 1.2; // Term frequency saturation
const B: f32 = 0.75; // Document length normalization

/// BM25 scorer for repositories
pub struct BM25Scorer {
    /// Document frequencies for each term
    doc_frequencies: HashMap<String, usize>,
    /// Total number of documents
    total_docs: usize,
    /// Average document length
    avg_doc_len: f32,
}

impl BM25Scorer {
    /// Create a new BM25 scorer from a collection of repositories
    pub fn new(repos: &[Repository]) -> Self {
        let mut doc_frequencies: HashMap<String, usize> = HashMap::new();
        let mut total_length = 0usize;

        for repo in repos {
            let text = repository_to_text(repo);
            let tokens = tokenize(&text);
            total_length += tokens.len();

            // Count unique terms in this document
            let unique_terms: std::collections::HashSet<_> = tokens.into_iter().collect();
            for term in unique_terms {
                *doc_frequencies.entry(term).or_insert(0) += 1;
            }
        }

        let total_docs = repos.len();
        let avg_doc_len = if total_docs > 0 {
            total_length as f32 / total_docs as f32
        } else {
            1.0
        };

        Self {
            doc_frequencies,
            total_docs,
            avg_doc_len,
        }
    }

    /// Score a single repository against a query
    pub fn score(&self, repo: &Repository, query: &str) -> f32 {
        let doc_text = repository_to_text(repo);
        let doc_tokens = tokenize(&doc_text);
        let query_tokens = tokenize(query);

        if doc_tokens.is_empty() || query_tokens.is_empty() {
            return 0.0;
        }

        // Count term frequencies in document
        let mut term_freqs: HashMap<String, usize> = HashMap::new();
        for token in &doc_tokens {
            *term_freqs.entry(token.clone()).or_insert(0) += 1;
        }

        let doc_len = doc_tokens.len() as f32;
        let mut score = 0.0;

        for term in query_tokens {
            let freq = *term_freqs.get(&term).unwrap_or(&0) as f32;
            if freq == 0.0 {
                continue;
            }

            // Calculate IDF
            let n = *self.doc_frequencies.get(&term).unwrap_or(&0) as f32;
            let idf = ((self.total_docs as f32 - n + 0.5) / (n + 0.5) + 1.0).ln();

            // Calculate BM25 term score
            let numerator = freq * (K1 + 1.0);
            let denominator = freq + K1 * (1.0 - B + B * doc_len / self.avg_doc_len);

            score += idf * (numerator / denominator);
        }

        score
    }

    /// Score multiple repositories and return sorted results
    pub fn score_all(&self, repos: &[Repository], query: &str) -> Vec<(Repository, f32)> {
        let mut scored: Vec<(Repository, f32)> = repos
            .iter()
            .map(|repo| {
                let score = self.score(repo, query);
                (repo.clone(), score)
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        scored
    }
}

/// Convert a repository to searchable text
fn repository_to_text(repo: &Repository) -> String {
    let mut parts = Vec::new();

    // Name (weighted by repetition for importance)
    let name = repo.full_name.split('/').last().unwrap_or(&repo.full_name);
    parts.push(name.to_string());
    parts.push(name.to_string()); // Double weight for name

    // Description
    if let Some(desc) = &repo.description {
        parts.push(desc.clone());
    }

    // Language
    if let Some(lang) = &repo.language {
        parts.push(lang.clone());
    }

    // Topics (important for search)
    for topic in &repo.topics {
        parts.push(topic.clone());
    }

    parts.join(" ")
}

/// Tokenize text into lowercase terms
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty() && s.len() > 1) // Skip single chars
        .map(|s| s.to_string())
        .collect()
}

/// Score keyword results using BM25
///
/// Takes pre-fetched keyword results and computes proper BM25 scores
pub fn score_keyword_results(repos: Vec<Repository>, query: &str) -> Vec<(Repository, f32)> {
    if repos.is_empty() {
        return Vec::new();
    }

    let scorer = BM25Scorer::new(&repos);
    scorer.score_all(&repos, query)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reposcout_core::models::Platform;

    fn create_test_repo(name: &str, description: &str, topics: Vec<&str>) -> Repository {
        Repository {
            platform: Platform::GitHub,
            full_name: format!("user/{}", name),
            description: Some(description.to_string()),
            url: format!("https://github.com/user/{}", name),
            homepage_url: None,
            stars: 100,
            forks: 10,
            watchers: 50,
            open_issues: 5,
            language: Some("Rust".to_string()),
            topics: topics.into_iter().map(|s| s.to_string()).collect(),
            license: Some("MIT".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            pushed_at: chrono::Utc::now(),
            size: 1024,
            default_branch: "main".to_string(),
            is_archived: false,
            is_private: false,
            health: None,
        }
    }

    #[test]
    fn test_bm25_basic_scoring() {
        let repos = vec![
            create_test_repo("logging-lib", "A logging library for applications", vec!["logging", "library"]),
            create_test_repo("web-server", "A web server framework", vec!["web", "server"]),
            create_test_repo("log-parser", "Parse log files efficiently", vec!["log", "parser"]),
        ];

        let scorer = BM25Scorer::new(&repos);

        // Query for "logging" should rank logging-lib highest
        let score1 = scorer.score(&repos[0], "logging");
        let score2 = scorer.score(&repos[1], "logging");
        let score3 = scorer.score(&repos[2], "logging");

        assert!(score1 > score2, "logging-lib should score higher than web-server for 'logging'");
        assert!(score1 > score3, "logging-lib should score higher than log-parser for 'logging'");
    }

    #[test]
    fn test_bm25_multi_term_query() {
        let repos = vec![
            create_test_repo("web-framework", "A web framework for building APIs", vec!["web", "api"]),
            create_test_repo("api-client", "Client library for APIs", vec!["api", "client"]),
        ];

        let scorer = BM25Scorer::new(&repos);

        // Query for "web api" should consider both terms
        let results = scorer.score_all(&repos, "web api");

        // web-framework should rank first (has both web and api)
        assert_eq!(results[0].0.full_name, "user/web-framework");
    }

    #[test]
    fn test_score_keyword_results() {
        let repos = vec![
            create_test_repo("rust-cli", "Command line tool in Rust", vec!["rust", "cli"]),
            create_test_repo("python-cli", "Command line tool in Python", vec!["python", "cli"]),
        ];

        let results = score_keyword_results(repos, "rust cli");

        assert!(!results.is_empty());
        assert_eq!(results[0].0.full_name, "user/rust-cli");
        assert!(results[0].1 > results[1].1);
    }

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello, World! This is a test-string");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        assert!(tokens.contains(&"string".to_string()));
        // Single char 'a' should be filtered out
        assert!(!tokens.contains(&"a".to_string()));
    }
}
