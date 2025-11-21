// SQLite-based caching layer
// Keeps API calls down and makes offline mode possible

pub mod cache;

pub use cache::{BookmarkEntry, CacheError, CacheManager, CacheStats, SearchHistoryEntry};
