// Search engine with caching support
use crate::{models::Repository, search::SearchProvider, Result};
use reposcout_cache::CacheManager;
use std::sync::Arc;
use tracing::{debug, info};

/// Search engine that checks cache before hitting APIs
pub struct CachedSearchEngine {
    providers: Vec<Box<dyn SearchProvider>>,
    cache: Option<Arc<CacheManager>>,
}

impl CachedSearchEngine {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            cache: None,
        }
    }

    pub fn with_cache(cache: CacheManager) -> Self {
        Self {
            providers: Vec::new(),
            #[allow(clippy::arc_with_non_send_sync)]
            cache: Some(Arc::new(cache)),
        }
    }

    pub fn add_provider(&mut self, provider: Box<dyn SearchProvider>) {
        self.providers.push(provider);
    }

    /// Search with cache-first strategy
    pub async fn search(&self, query: &str) -> Result<Vec<Repository>> {
        // Try query-specific cache first if available
        if let Some(cache) = &self.cache {
            debug!("Checking query cache for: {}", query);
            match cache.get_query_cache::<Repository>(query) {
                Ok(mut results) if !results.is_empty() => {
                    info!("Query cache hit! Found {} results", results.len());
                    // Calculate health metrics for cached results (in case they were cached before health was added)
                    for repo in &mut results {
                        repo.calculate_health();
                    }
                    return Ok(results);
                }
                Ok(_) => debug!("Query cache miss - no results"),
                Err(e) => debug!("Query cache error: {}", e),
            }
        }

        // Cache miss - hit the APIs
        info!("Fetching from providers for query: {}", query);
        let mut results = self.search_providers(query).await?;

        // Calculate health metrics for all results
        for repo in &mut results {
            repo.calculate_health();
        }

        // Store results in query cache
        if let Some(cache) = &self.cache {
            if let Err(e) = cache.set_query_cache(query, &results) {
                debug!("Failed to cache query results: {}", e);
            } else {
                info!("Cached {} repositories for query: {}", results.len(), query);
            }
        }

        Ok(results)
    }

    /// Get repository with cache
    pub async fn get_repository(&self, owner: &str, name: &str) -> Result<Repository> {
        let full_name = format!("{}/{}", owner, name);

        // Try cache first
        if let Some(cache) = &self.cache {
            debug!("Checking cache for repository: {}", full_name);
            // Try all platforms since we don't know which one it's from
            for platform in &["GitHub", "GitLab", "Bitbucket"] {
                if let Ok(mut repo) = cache.get::<Repository>(platform, &full_name) {
                    info!("Cache hit for {}", full_name);
                    repo.calculate_health();
                    return Ok(repo);
                }
            }
        }

        // Cache miss - try all providers until one succeeds
        info!("Fetching {} from provider", full_name);
        let mut last_error = None;

        for provider in &self.providers {
            match provider.get_repository(owner, name).await {
                Ok(mut repo) => {
                    // Calculate health metrics
                    repo.calculate_health();
                    // Cache it
                    if let Some(cache) = &self.cache {
                        if let Err(e) = cache.set(&repo.platform.to_string(), &full_name, &repo) {
                            debug!("Failed to cache {}: {}", full_name, e);
                        }
                    }
                    return Ok(repo);
                }
                Err(e) => {
                    debug!("Provider failed to fetch {}: {}", full_name, e);
                    last_error = Some(e);
                }
            }
        }

        // All providers failed
        Err(last_error.unwrap_or_else(|| crate::Error::ConfigError("No search providers configured".into())))
    }

    /// Search across all providers (without cache)
    async fn search_providers(&self, query: &str) -> Result<Vec<Repository>> {
        use futures::future::join_all;

        let searches: Vec<_> = self
            .providers
            .iter()
            .map(|provider| provider.search(query))
            .collect();

        let results = join_all(searches).await;

        let mut repos = Vec::new();
        for mut r in results.into_iter().flatten() {
            repos.append(&mut r);
        }

        Ok(repos)
    }
}

impl Default for CachedSearchEngine {
    fn default() -> Self {
        Self::new()
    }
}
