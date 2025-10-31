// Core business logic lives here - the brain of the operation
pub mod config;
pub mod error;
pub mod export;
pub mod health;
pub mod models;
pub mod providers;
pub mod search;
pub mod search_with_cache;

pub use config::Config;
pub use error::Error;
pub use export::{ExportFormat, Exporter};
pub use health::{HealthCalculator, HealthMetrics, HealthStatus, MaintenanceLevel};
pub use search_with_cache::CachedSearchEngine;

/// Result type alias because typing Result<T, Error> everywhere is tedious
pub type Result<T> = std::result::Result<T, Error>;
