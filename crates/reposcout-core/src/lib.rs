// Core business logic lives here - the brain of the operation
pub mod config;
pub mod error;
pub mod export;
pub mod health;
pub mod models;
pub mod packages;
pub mod portfolio;
pub mod providers;
pub mod registries;
pub mod search;
pub mod search_with_cache;
pub mod theme;
pub mod token_store;
pub mod trending;

pub use config::Config;
pub use error::Error;
pub use export::{ExportFormat, Exporter};
pub use health::{HealthCalculator, HealthMetrics, HealthStatus, MaintenanceLevel};
pub use packages::{License, LicenseCompatibility, PackageDetector, PackageInfo, PackageManager};
pub use portfolio::{Portfolio, PortfolioColor, PortfolioIcon, PortfolioManager};
pub use registries::RegistryClient;
pub use search_with_cache::CachedSearchEngine;
pub use theme::{Color, Theme, ThemeColors};
pub use token_store::TokenStore;
pub use trending::{TrendingFilters, TrendingFinder, TrendingPeriod};

// Re-export notification types from API crate
pub use reposcout_api::{Notification, NotificationFilters, NotificationReason};

/// Result type alias because typing Result<T, Error> everywhere is tedious
pub type Result<T> = std::result::Result<T, Error>;
