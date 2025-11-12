use crate::models::Repository;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A portfolio/watchlist containing grouped repositories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: PortfolioColor,
    pub icon: PortfolioIcon,
    pub repos: Vec<WatchedRepo>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Repository with watching metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedRepo {
    pub repo: Repository,
    pub added_at: DateTime<Utc>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    /// Last known state for change detection
    pub last_stars: u32,
    pub last_forks: u32,
    pub last_pushed_at: DateTime<Utc>,
    pub last_checked_at: DateTime<Utc>,
}

/// Visual color for portfolio
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PortfolioColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Pink,
    Gray,
}

impl PortfolioColor {
    pub fn as_str(&self) -> &'static str {
        match self {
            PortfolioColor::Red => "Red",
            PortfolioColor::Orange => "Orange",
            PortfolioColor::Yellow => "Yellow",
            PortfolioColor::Green => "Green",
            PortfolioColor::Blue => "Blue",
            PortfolioColor::Purple => "Purple",
            PortfolioColor::Pink => "Pink",
            PortfolioColor::Gray => "Gray",
        }
    }

    pub fn all() -> Vec<PortfolioColor> {
        vec![
            PortfolioColor::Red,
            PortfolioColor::Orange,
            PortfolioColor::Yellow,
            PortfolioColor::Green,
            PortfolioColor::Blue,
            PortfolioColor::Purple,
            PortfolioColor::Pink,
            PortfolioColor::Gray,
        ]
    }
}

/// Icon for portfolio
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PortfolioIcon {
    Work,       // 💼
    Learning,   // 📚
    Personal,   // 👤
    Stars,      // ⭐
    Bookmark,   // 🔖
    Code,       // 💻
    Tools,      // 🔧
    Rocket,     // 🚀
    Heart,      // ❤️
    Fire,       // 🔥
}

impl PortfolioIcon {
    pub fn as_emoji(&self) -> &'static str {
        match self {
            PortfolioIcon::Work => "💼",
            PortfolioIcon::Learning => "📚",
            PortfolioIcon::Personal => "👤",
            PortfolioIcon::Stars => "⭐",
            PortfolioIcon::Bookmark => "🔖",
            PortfolioIcon::Code => "💻",
            PortfolioIcon::Tools => "🔧",
            PortfolioIcon::Rocket => "🚀",
            PortfolioIcon::Heart => "❤️",
            PortfolioIcon::Fire => "🔥",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PortfolioIcon::Work => "Work",
            PortfolioIcon::Learning => "Learning",
            PortfolioIcon::Personal => "Personal",
            PortfolioIcon::Stars => "Stars",
            PortfolioIcon::Bookmark => "Bookmark",
            PortfolioIcon::Code => "Code",
            PortfolioIcon::Tools => "Tools",
            PortfolioIcon::Rocket => "Rocket",
            PortfolioIcon::Heart => "Heart",
            PortfolioIcon::Fire => "Fire",
        }
    }

    pub fn all() -> Vec<PortfolioIcon> {
        vec![
            PortfolioIcon::Work,
            PortfolioIcon::Learning,
            PortfolioIcon::Personal,
            PortfolioIcon::Stars,
            PortfolioIcon::Bookmark,
            PortfolioIcon::Code,
            PortfolioIcon::Tools,
            PortfolioIcon::Rocket,
            PortfolioIcon::Heart,
            PortfolioIcon::Fire,
        ]
    }
}

/// Updates detected in watched repositories
#[derive(Debug, Clone)]
pub struct RepoUpdate {
    pub repo_full_name: String,
    pub update_type: UpdateType,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateType {
    NewStars(u32),      // Number of new stars
    NewForks(u32),      // Number of new forks
    NewPush,            // Repository was pushed to
    NewRelease(String), // New release version
}

impl UpdateType {
    pub fn description(&self) -> String {
        match self {
            UpdateType::NewStars(count) => format!("+{} stars", count),
            UpdateType::NewForks(count) => format!("+{} forks", count),
            UpdateType::NewPush => "New commits".to_string(),
            UpdateType::NewRelease(version) => format!("Release {}", version),
        }
    }
}

/// Manager for portfolios
pub struct PortfolioManager {
    portfolios: HashMap<String, Portfolio>,
}

impl PortfolioManager {
    pub fn new() -> Self {
        Self {
            portfolios: HashMap::new(),
        }
    }

    /// Create a new portfolio
    pub fn create_portfolio(
        &mut self,
        name: String,
        description: Option<String>,
        color: PortfolioColor,
        icon: PortfolioIcon,
    ) -> Portfolio {
        let now = Utc::now();
        let id = uuid::Uuid::new_v4().to_string();

        let portfolio = Portfolio {
            id: id.clone(),
            name,
            description,
            color,
            icon,
            repos: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        self.portfolios.insert(id, portfolio.clone());
        portfolio
    }

    /// Get all portfolios
    pub fn list_portfolios(&self) -> Vec<&Portfolio> {
        self.portfolios.values().collect()
    }

    /// Get portfolio by ID
    pub fn get_portfolio(&self, id: &str) -> Option<&Portfolio> {
        self.portfolios.get(id)
    }

    /// Get mutable portfolio by ID
    pub fn get_portfolio_mut(&mut self, id: &str) -> Option<&mut Portfolio> {
        self.portfolios.get_mut(id)
    }

    /// Add repository to portfolio
    pub fn add_repo_to_portfolio(
        &mut self,
        portfolio_id: &str,
        repo: Repository,
        notes: Option<String>,
        tags: Vec<String>,
    ) -> crate::Result<()> {
        let portfolio = self
            .portfolios
            .get_mut(portfolio_id)
            .ok_or_else(|| crate::Error::ConfigError("Portfolio not found".to_string()))?;

        let watched = WatchedRepo {
            last_stars: repo.stars,
            last_forks: repo.forks,
            last_pushed_at: repo.pushed_at,
            last_checked_at: Utc::now(),
            added_at: Utc::now(),
            notes,
            tags,
            repo,
        };

        portfolio.repos.push(watched);
        portfolio.updated_at = Utc::now();
        Ok(())
    }

    /// Remove repository from portfolio
    pub fn remove_repo_from_portfolio(
        &mut self,
        portfolio_id: &str,
        repo_full_name: &str,
    ) -> crate::Result<()> {
        let portfolio = self
            .portfolios
            .get_mut(portfolio_id)
            .ok_or_else(|| crate::Error::ConfigError("Portfolio not found".to_string()))?;

        portfolio
            .repos
            .retain(|r| r.repo.full_name != repo_full_name);
        portfolio.updated_at = Utc::now();
        Ok(())
    }

    /// Delete a portfolio
    pub fn delete_portfolio(&mut self, id: &str) -> crate::Result<()> {
        self.portfolios
            .remove(id)
            .ok_or_else(|| crate::Error::ConfigError("Portfolio not found".to_string()))?;
        Ok(())
    }

    /// Update portfolio metadata
    pub fn update_portfolio(
        &mut self,
        id: &str,
        name: Option<String>,
        description: Option<String>,
        color: Option<PortfolioColor>,
        icon: Option<PortfolioIcon>,
    ) -> crate::Result<()> {
        let portfolio = self
            .portfolios
            .get_mut(id)
            .ok_or_else(|| crate::Error::ConfigError("Portfolio not found".to_string()))?;

        if let Some(n) = name {
            portfolio.name = n;
        }
        if let Some(d) = description {
            portfolio.description = Some(d);
        }
        if let Some(c) = color {
            portfolio.color = c;
        }
        if let Some(i) = icon {
            portfolio.icon = i;
        }
        portfolio.updated_at = Utc::now();
        Ok(())
    }

    /// Check for updates in a watched repository
    pub fn check_for_updates(&mut self, portfolio_id: &str, updated_repo: &Repository) -> Vec<RepoUpdate> {
        let mut updates = Vec::new();

        if let Some(portfolio) = self.portfolios.get_mut(portfolio_id) {
            if let Some(watched) = portfolio
                .repos
                .iter_mut()
                .find(|r| r.repo.full_name == updated_repo.full_name)
            {
                let now = Utc::now();

                // Check for new stars
                if updated_repo.stars > watched.last_stars {
                    let new_stars = updated_repo.stars - watched.last_stars;
                    updates.push(RepoUpdate {
                        repo_full_name: updated_repo.full_name.clone(),
                        update_type: UpdateType::NewStars(new_stars),
                        detected_at: now,
                    });
                    watched.last_stars = updated_repo.stars;
                }

                // Check for new forks
                if updated_repo.forks > watched.last_forks {
                    let new_forks = updated_repo.forks - watched.last_forks;
                    updates.push(RepoUpdate {
                        repo_full_name: updated_repo.full_name.clone(),
                        update_type: UpdateType::NewForks(new_forks),
                        detected_at: now,
                    });
                    watched.last_forks = updated_repo.forks;
                }

                // Check for new pushes
                if updated_repo.pushed_at > watched.last_pushed_at {
                    updates.push(RepoUpdate {
                        repo_full_name: updated_repo.full_name.clone(),
                        update_type: UpdateType::NewPush,
                        detected_at: now,
                    });
                    watched.last_pushed_at = updated_repo.pushed_at;
                }

                // Update the repository data
                watched.repo = updated_repo.clone();
                watched.last_checked_at = now;
            }
        }

        updates
    }

    /// Get total repository count across all portfolios
    pub fn total_repo_count(&self) -> usize {
        self.portfolios.values().map(|p| p.repos.len()).sum()
    }

    /// Find which portfolios contain a specific repository
    pub fn find_repo_portfolios(&self, repo_full_name: &str) -> Vec<&Portfolio> {
        self.portfolios
            .values()
            .filter(|p| p.repos.iter().any(|r| r.repo.full_name == repo_full_name))
            .collect()
    }
}

impl Default for PortfolioManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Portfolio {
    /// Get repository count
    pub fn repo_count(&self) -> usize {
        self.repos.len()
    }

    /// Get total stars across all repos
    pub fn total_stars(&self) -> u32 {
        self.repos.iter().map(|r| r.repo.stars).sum()
    }

    /// Get most recently added repos
    pub fn recent_repos(&self, limit: usize) -> Vec<&WatchedRepo> {
        let mut repos: Vec<_> = self.repos.iter().collect();
        repos.sort_by(|a, b| b.added_at.cmp(&a.added_at));
        repos.into_iter().take(limit).collect()
    }

    /// Get repos sorted by stars
    pub fn top_starred_repos(&self, limit: usize) -> Vec<&WatchedRepo> {
        let mut repos: Vec<_> = self.repos.iter().collect();
        repos.sort_by(|a, b| b.repo.stars.cmp(&a.repo.stars));
        repos.into_iter().take(limit).collect()
    }
}
