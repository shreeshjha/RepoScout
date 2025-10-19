use crate::{models::Repository, Result};

/// Trait for search providers - makes testing easier and keeps things flexible
///
/// Each platform (GitHub, GitLab, etc.) will implement this trait.
/// This way we can swap providers without breaking everything.
#[async_trait::async_trait]
pub trait SearchProvider: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<Repository>>;
    async fn get_repository(&self, owner: &str, name: &str) -> Result<Repository>;
}

/// The main search engine that coordinates searches across platforms
pub struct SearchEngine {
    providers: Vec<Box<dyn SearchProvider>>,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn add_provider(&mut self, provider: Box<dyn SearchProvider>) {
        self.providers.push(provider);
    }

    /// Search across all registered providers
    ///
    /// Runs searches in parallel because waiting is for serial programmers
    pub async fn search_all(&self, query: &str) -> Result<Vec<Repository>> {
        use futures::future::join_all;

        let searches: Vec<_> = self
            .providers
            .iter()
            .map(|provider| provider.search(query))
            .collect();

        let results = join_all(searches).await;

        // Flatten all results, ignoring errors for now
        // TODO: Better error handling - maybe collect errors separately?
        let mut repos = Vec::new();
        for result in results {
            if let Ok(mut r) = result {
                repos.append(&mut r);
            }
        }

        Ok(repos)
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}
