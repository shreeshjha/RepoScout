use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Repository model - the star of the show
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub platform: Platform,
    pub full_name: String,
    pub description: Option<String>,
    pub url: String,
    pub homepage_url: Option<String>,
    pub stars: u32,
    pub forks: u32,
    pub watchers: u32,
    pub open_issues: u32,
    pub language: Option<String>,
    pub topics: Vec<String>,
    pub license: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pushed_at: DateTime<Utc>,
    pub size: u64,
    pub default_branch: String,
    pub is_archived: bool,
    pub is_private: bool,
}

/// Which platform this repo lives on
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Platform {
    GitHub,
    GitLab,
    Bitbucket,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::GitHub => write!(f, "GitHub"),
            Platform::GitLab => write!(f, "GitLab"),
            Platform::Bitbucket => write!(f, "Bitbucket"),
        }
    }
}

/// Search query with all the bells and whistles
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub query: String,
    pub platforms: Vec<Platform>,
    pub language: Option<String>,
    pub min_stars: Option<u32>,
    pub max_stars: Option<u32>,
    pub sort_by: SortBy,
    pub limit: usize,
}

/// How we want results sorted
#[derive(Debug, Clone, Copy, Default)]
pub enum SortBy {
    #[default]
    Relevance,
    Stars,
    Forks,
    Updated,
    Created,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            platforms: vec![Platform::GitHub], // GitHub by default because let's be honest
            language: None,
            min_stars: None,
            max_stars: None,
            sort_by: SortBy::default(),
            limit: 30,
        }
    }
}
