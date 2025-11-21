// Trending repositories discovery
use crate::{models::Repository, search::SearchProvider, Result};
use chrono::{Duration, Utc};

/// Time range for trending repositories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendingPeriod {
    Daily,
    Weekly,
    Monthly,
}

impl TrendingPeriod {
    /// Get the date range for this period
    pub fn date_range(&self) -> String {
        let now = Utc::now();
        let start_date = match self {
            TrendingPeriod::Daily => now - Duration::days(1),
            TrendingPeriod::Weekly => now - Duration::weeks(1),
            TrendingPeriod::Monthly => now - Duration::days(30),
        };
        format!(">={}", start_date.format("%Y-%m-%d"))
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            TrendingPeriod::Daily => "Today",
            TrendingPeriod::Weekly => "This Week",
            TrendingPeriod::Monthly => "This Month",
        }
    }
}

/// Trending repository filters
#[derive(Debug, Clone, Default)]
pub struct TrendingFilters {
    pub language: Option<String>,
    pub min_stars: Option<u32>,
    pub topic: Option<String>,
}

/// Trending repository finder
pub struct TrendingFinder<'a> {
    providers: Vec<&'a dyn SearchProvider>,
}

impl<'a> TrendingFinder<'a> {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn add_provider(&mut self, provider: &'a dyn SearchProvider) {
        self.providers.push(provider);
    }

    /// Find trending repositories for a given period
    pub async fn find_trending(
        &self,
        period: TrendingPeriod,
        filters: &TrendingFilters,
    ) -> Result<Vec<Repository>> {
        // Build search query for trending
        let mut query_parts = vec!["stars:>100".to_string()]; // Minimum stars threshold

        // Add date filter
        let date_filter = format!("created:{}", period.date_range());
        query_parts.push(date_filter);

        // Add optional filters
        if let Some(ref lang) = filters.language {
            query_parts.push(format!("language:{}", lang));
        }

        if let Some(min_stars) = filters.min_stars {
            query_parts.push(format!("stars:>={}", min_stars));
        }

        if let Some(ref topic) = filters.topic {
            query_parts.push(format!("topic:{}", topic));
        }

        let query = query_parts.join(" ");

        // Search across all providers
        use futures::future::join_all;
        let searches: Vec<_> = self
            .providers
            .iter()
            .map(|provider| provider.search(&query))
            .collect();

        let results = join_all(searches).await;

        let mut repos = Vec::new();
        for mut r in results.into_iter().flatten() {
            repos.append(&mut r);
        }

        // Sort by stars (descending) - these are the "hottest" repos
        repos.sort_by(|a, b| b.stars.cmp(&a.stars));

        // Enrich with velocity calculation (as metadata in description if needed)
        for repo in &mut repos {
            let age_days = (Utc::now() - repo.created_at).num_days() as f64;
            if age_days > 0.0 {
                let velocity = repo.stars as f64 / age_days;
                // Store velocity in a custom field or just use it for sorting
                // For now, repos are already sorted by total stars
                // Could add: repo.star_velocity = Some(velocity);
                let _ = velocity; // Suppress warning for now
            }
        }

        Ok(repos)
    }

    /// Get trending repos with star velocity sorting
    /// This finds repos that have gained stars quickly, not just total stars
    pub async fn find_trending_by_velocity(
        &self,
        period: TrendingPeriod,
        filters: &TrendingFilters,
    ) -> Result<Vec<Repository>> {
        let mut repos = self.find_trending(period, filters).await?;

        // Calculate and sort by velocity (stars per day)
        repos.sort_by(|a, b| {
            let age_a = (Utc::now() - a.created_at).num_days().max(1) as f64;
            let age_b = (Utc::now() - b.created_at).num_days().max(1) as f64;

            let velocity_a = a.stars as f64 / age_a;
            let velocity_b = b.stars as f64 / age_b;

            velocity_b
                .partial_cmp(&velocity_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(repos)
    }
}

impl<'a> Default for TrendingFinder<'a> {
    fn default() -> Self {
        Self::new()
    }
}
