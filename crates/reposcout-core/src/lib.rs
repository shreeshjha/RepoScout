// Core business logic lives here - the brain of the operation
pub mod config;
pub mod error;
pub mod models;
pub mod providers;
pub mod search;

pub use config::Config;
pub use error::Error;

/// Result type alias because typing Result<T, Error> everywhere is tedious
pub type Result<T> = std::result::Result<T, Error>;
