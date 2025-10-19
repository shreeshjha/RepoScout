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
            cache: Some(Arc::new(cache)),
        }
    }

    pub fn add_provider(&mut self, provider: Box<dyn SearchProvider>) {
        self.providers.push(provider);
    }

    /// Search with cache-first strategy
    pub async fn search(&self, query: &str) -> Result<Vec<Repository>> {
        // Try cache first if available
        if let Some(cache) = &self.cache {
            debug!("Checking cache for query: {}", query);
            match cache.search::<Repository>(query, 100) {
                Ok(results) if !results.is_empty() => {
                    info!("Cache hit! Found {} results", results.len());
                    return Ok(results);
                }
                Ok(_) => debug!("Cache miss - no results"),
                Err(e) => debug!("Cache error: {}", e),
            }
        }

        // Cache miss - hit the APIs
        info!("Fetching from providers");
        let results = self.search_providers(query).await?;

        // Store results in cache
        if let Some(cache) = &self.cache {
            for repo in &results {
                if let Err(e) = cache.set(&repo.platform.to_string(), &repo.full_name, repo) {
                    debug!("Failed to cache {}: {}", repo.full_name, e);
                }
            }
            info!("Cached {} repositories", results.len());
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
                if let Ok(repo) = cache.get::<Repository>(platform, &full_name) {
                    info!("Cache hit for {}", full_name);
                    return Ok(repo);
                }
            }
        }

        // Cache miss - fetch from first provider (usually GitHub)
        if let Some(provider) = self.providers.first() {
            info!("Fetching {} from provider", full_name);
            let repo = provider.get_repository(owner, name).await?;

            // Cache it
            if let Some(cache) = &self.cache {
                if let Err(e) = cache.set(&repo.platform.to_string(), &full_name, &repo) {
                    debug!("Failed to cache {}: {}", full_name, e);
                }
            }

            Ok(repo)
        } else {
            Err(crate::Error::ConfigError("No search providers configured".into()))
        }
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
        for result in results {
            if let Ok(mut r) = result {
                repos.append(&mut r);
            }
        }

        Ok(repos)
    }
}

impl Default for CachedSearchEngine {
    fn default() -> Self {
        Self::new()
    }
}
