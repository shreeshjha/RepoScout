use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Entry not found: {0}")]
    NotFound(String),

    #[error("Entry expired")]
    Expired,
}

pub type Result<T> = std::result::Result<T, CacheError>;

/// Cache manager using SQLite + FTS5
///
/// SQLite was chosen because:
/// - Zero-config embedded database
/// - FTS5 for fast text search
/// - Battle-tested and reliable
/// - Doesn't require a separate process
pub struct CacheManager {
    conn: Connection,
    ttl_seconds: i64,
}

impl CacheManager {
    pub fn new(db_path: &str, ttl_hours: u64) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Initialize schema on first run
        Self::init_schema(&conn)?;

        Ok(Self {
            conn,
            ttl_seconds: (ttl_hours * 3600) as i64,
        })
    }

    fn init_schema(conn: &Connection) -> SqlResult<()> {
        // Create repositories table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS repositories (
                id INTEGER PRIMARY KEY,
                platform TEXT NOT NULL,
                full_name TEXT NOT NULL,
                data TEXT NOT NULL,
                cached_at INTEGER NOT NULL,
                UNIQUE(platform, full_name)
            )",
            [],
        )?;

        // FTS5 table for full-text search
        // Using external content table to avoid duplicating data
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS repositories_fts
             USING fts5(full_name, description, topics, content='')",
            [],
        )?;

        // Create bookmarks table
        // Stores user's favorite repositories for quick access
        conn.execute(
            "CREATE TABLE IF NOT EXISTS bookmarks (
                id INTEGER PRIMARY KEY,
                platform TEXT NOT NULL,
                full_name TEXT NOT NULL,
                data TEXT NOT NULL,
                bookmarked_at INTEGER NOT NULL,
                tags TEXT,
                notes TEXT,
                UNIQUE(platform, full_name)
            )",
            [],
        )?;

        // Create search history table
        // Tracks previous searches for quick re-run and auto-complete
        conn.execute(
            "CREATE TABLE IF NOT EXISTS search_history (
                id INTEGER PRIMARY KEY,
                query TEXT NOT NULL,
                filters TEXT,
                result_count INTEGER,
                searched_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create index for efficient querying by timestamp
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_search_history_searched_at
             ON search_history(searched_at DESC)",
            [],
        )?;

        // Create query cache table
        // Stores complete search results for exact queries to avoid FTS5 cross-contamination
        conn.execute(
            "CREATE TABLE IF NOT EXISTS query_cache (
                id INTEGER PRIMARY KEY,
                query_hash TEXT NOT NULL UNIQUE,
                query TEXT NOT NULL,
                results TEXT NOT NULL,
                cached_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create index for efficient lookup by query hash
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_query_cache_hash
             ON query_cache(query_hash)",
            [],
        )?;

        Ok(())
    }

    /// Store a repository in cache
    pub fn set<T: Serialize>(&self, platform: &str, full_name: &str, data: &T) -> Result<()> {
        let json = serde_json::to_string(data)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Parse JSON to extract fields for FTS5
        let value: serde_json::Value = serde_json::from_str(&json)?;
        let description = value.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let topics = value.get("topics")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();

        // Delete old entry if exists
        self.conn.execute(
            "DELETE FROM repositories WHERE platform = ?1 AND full_name = ?2",
            params![platform, full_name],
        )?;

        // Insert new entry
        self.conn.execute(
            "INSERT INTO repositories (platform, full_name, data, cached_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![platform, full_name, json, now],
        )?;

        // Update FTS5 index
        let row_id = self.conn.last_insert_rowid();
        self.conn.execute(
            "INSERT INTO repositories_fts (rowid, full_name, description, topics)
             VALUES (?1, ?2, ?3, ?4)",
            params![row_id, full_name, description, topics],
        )?;

        Ok(())
    }

    /// Get a repository from cache
    pub fn get<T: for<'de> Deserialize<'de>>(&self, platform: &str, full_name: &str) -> Result<T> {
        let (data, cached_at): (String, i64) = self
            .conn
            .query_row(
                "SELECT data, cached_at FROM repositories WHERE platform = ?1 AND full_name = ?2",
                params![platform, full_name],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| CacheError::NotFound(full_name.to_string()))?;

        // Check if entry is expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        if now - cached_at > self.ttl_seconds {
            return Err(CacheError::Expired);
        }

        Ok(serde_json::from_str(&data)?)
    }

    /// Search repositories using FTS5
    pub fn search<T: for<'de> Deserialize<'de>>(&self, query: &str, limit: usize) -> Result<Vec<T>> {
        let mut stmt = self.conn.prepare(
            "SELECT r.data FROM repositories r
             INNER JOIN repositories_fts fts ON r.id = fts.rowid
             WHERE repositories_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![query, limit], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            })?
            .filter_map(|r| r.ok())
            .filter_map(|json| serde_json::from_str(&json).ok())
            .collect();

        Ok(results)
    }

    /// Get all cached repositories (useful for offline mode)
    pub fn get_all<T: for<'de> Deserialize<'de>>(&self, limit: usize) -> Result<Vec<T>> {
        let mut stmt = self.conn.prepare(
            "SELECT data FROM repositories ORDER BY cached_at DESC LIMIT ?1",
        )?;

        let results = stmt
            .query_map(params![limit], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            })?
            .filter_map(|r| r.ok())
            .filter_map(|json| serde_json::from_str(&json).ok())
            .collect();

        Ok(results)
    }

    /// Clear all cached data
    pub fn clear(&self) -> Result<()> {
        self.conn.execute("DELETE FROM repositories", [])?;
        self.conn.execute("DELETE FROM search_history", [])?;
        Ok(())
    }

    /// Delete expired entries
    pub fn cleanup_expired(&self) -> Result<usize> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cutoff = now - self.ttl_seconds;

        let deleted = self.conn.execute(
            "DELETE FROM repositories WHERE cached_at < ?1",
            params![cutoff],
        )?;

        Ok(deleted)
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let cutoff = now - self.ttl_seconds;

        // Repository cache stats
        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM repositories", [], |row| row.get(0))?;

        let expired: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM repositories WHERE cached_at < ?1",
            params![cutoff],
            |row| row.get(0),
        )?;

        // Query cache stats
        let query_total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM query_cache", [], |row| row.get(0))?;

        let query_expired: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM query_cache WHERE cached_at < ?1",
            params![cutoff],
            |row| row.get(0),
        )?;

        // Bookmarks count
        let bookmarks: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM bookmarks", [], |row| row.get(0))?;

        // Get database file size
        let page_count: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let page_size: i64 = self
            .conn
            .query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let size_bytes = page_count * page_size;

        Ok(CacheStats {
            total_entries: total as usize,
            expired_entries: expired as usize,
            valid_entries: (total - expired) as usize,
            query_cache_entries: query_total as usize,
            query_cache_expired: query_expired as usize,
            bookmarks_count: bookmarks as usize,
            size_bytes: size_bytes as usize,
        })
    }

    // Bookmark management methods

    /// Add a repository to bookmarks
    pub fn add_bookmark<T: Serialize>(
        &self,
        platform: &str,
        full_name: &str,
        data: &T,
        tags: Option<&str>,
        notes: Option<&str>,
    ) -> Result<()> {
        let json = serde_json::to_string(data)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.conn.execute(
            "INSERT OR REPLACE INTO bookmarks (platform, full_name, data, bookmarked_at, tags, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![platform, full_name, json, now, tags, notes],
        )?;

        Ok(())
    }

    /// Remove a bookmark
    pub fn remove_bookmark(&self, platform: &str, full_name: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM bookmarks WHERE platform = ?1 AND full_name = ?2",
            params![platform, full_name],
        )?;
        Ok(())
    }

    /// Check if a repository is bookmarked
    pub fn is_bookmarked(&self, platform: &str, full_name: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM bookmarks WHERE platform = ?1 AND full_name = ?2",
            params![platform, full_name],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get all bookmarks
    pub fn get_bookmarks<T: for<'de> Deserialize<'de>>(&self) -> Result<Vec<T>> {
        let mut stmt = self.conn.prepare(
            "SELECT data FROM bookmarks ORDER BY bookmarked_at DESC",
        )?;

        let results = stmt
            .query_map([], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            })?
            .filter_map(|r| r.ok())
            .filter_map(|json| serde_json::from_str(&json).ok())
            .collect();

        Ok(results)
    }

    /// Get bookmarks with metadata (tags, notes)
    pub fn get_bookmarks_with_metadata(&self) -> Result<Vec<BookmarkEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT platform, full_name, data, bookmarked_at, tags, notes
             FROM bookmarks ORDER BY bookmarked_at DESC",
        )?;

        let results = stmt
            .query_map([], |row| {
                Ok(BookmarkEntry {
                    platform: row.get(0)?,
                    full_name: row.get(1)?,
                    data: row.get(2)?,
                    bookmarked_at: row.get(3)?,
                    tags: row.get(4)?,
                    notes: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Clear all bookmarks
    pub fn clear_bookmarks(&self) -> Result<()> {
        self.conn.execute("DELETE FROM bookmarks", [])?;
        Ok(())
    }

    /// Get bookmark count
    pub fn bookmark_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM bookmarks", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    // ===== Search History Methods =====

    /// Add a search to history
    /// Duplicate queries update the timestamp instead of creating new entries
    pub fn add_search_history(
        &self,
        query: &str,
        filters: Option<&str>,
        result_count: Option<i64>,
    ) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Check if this exact query already exists
        let existing: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM search_history WHERE query = ?1 ORDER BY searched_at DESC LIMIT 1",
                params![query],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            // Update existing entry with new timestamp and result count
            self.conn.execute(
                "UPDATE search_history SET searched_at = ?1, result_count = ?2, filters = ?3 WHERE id = ?4",
                params![now, result_count, filters, id],
            )?;
        } else {
            // Insert new entry
            self.conn.execute(
                "INSERT INTO search_history (query, filters, result_count, searched_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![query, filters, result_count, now],
            )?;
        }

        // Limit history to last 100 searches
        self.conn.execute(
            "DELETE FROM search_history WHERE id IN (
                SELECT id FROM search_history ORDER BY searched_at DESC LIMIT -1 OFFSET 100
            )",
            [],
        )?;

        Ok(())
    }

    /// Get recent search history (most recent first)
    pub fn get_search_history(&self, limit: usize) -> Result<Vec<SearchHistoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, query, filters, result_count, searched_at
             FROM search_history ORDER BY searched_at DESC LIMIT ?1",
        )?;

        let results = stmt
            .query_map(params![limit as i64], |row| {
                Ok(SearchHistoryEntry {
                    id: row.get(0)?,
                    query: row.get(1)?,
                    filters: row.get(2)?,
                    result_count: row.get(3)?,
                    searched_at: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Search within history (for auto-complete)
    pub fn search_history(&self, term: &str, limit: usize) -> Result<Vec<SearchHistoryEntry>> {
        let pattern = format!("%{}%", term);
        let mut stmt = self.conn.prepare(
            "SELECT id, query, filters, result_count, searched_at
             FROM search_history WHERE query LIKE ?1
             ORDER BY searched_at DESC LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![pattern, limit as i64], |row| {
                Ok(SearchHistoryEntry {
                    id: row.get(0)?,
                    query: row.get(1)?,
                    filters: row.get(2)?,
                    result_count: row.get(3)?,
                    searched_at: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Delete a specific search history entry
    pub fn delete_search_history(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM search_history WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Clear all search history
    pub fn clear_search_history(&self) -> Result<()> {
        self.conn.execute("DELETE FROM search_history", [])?;
        Ok(())
    }

    /// Get search history count
    pub fn search_history_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM search_history", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    // ===== Query Cache Methods =====

    /// Generate a stable hash for a query string
    fn hash_query(query: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get cached search results for an exact query
    pub fn get_query_cache<T: for<'de> Deserialize<'de>>(&self, query: &str) -> Result<Vec<T>> {
        let query_hash = Self::hash_query(query);

        let (results_json, cached_at): (String, i64) = self
            .conn
            .query_row(
                "SELECT results, cached_at FROM query_cache WHERE query_hash = ?1",
                params![query_hash],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| CacheError::NotFound(query.to_string()))?;

        // Check if entry is expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        if now - cached_at > self.ttl_seconds {
            // Delete expired entry
            self.conn.execute(
                "DELETE FROM query_cache WHERE query_hash = ?1",
                params![query_hash],
            )?;
            return Err(CacheError::Expired);
        }

        let results: Vec<T> = serde_json::from_str(&results_json)?;
        Ok(results)
    }

    /// Store search results for a specific query
    pub fn set_query_cache<T: Serialize>(&self, query: &str, results: &[T]) -> Result<()> {
        let query_hash = Self::hash_query(query);
        let results_json = serde_json::to_string(results)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.conn.execute(
            "INSERT OR REPLACE INTO query_cache (query_hash, query, results, cached_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![query_hash, query, results_json, now],
        )?;

        Ok(())
    }

    /// Clear all query cache entries
    pub fn clear_query_cache(&self) -> Result<()> {
        self.conn.execute("DELETE FROM query_cache", [])?;
        Ok(())
    }

    /// Clean up expired query cache entries
    pub fn cleanup_expired_query_cache(&self) -> Result<usize> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let deleted = self.conn.execute(
            "DELETE FROM query_cache WHERE cached_at < ?1",
            params![now - self.ttl_seconds],
        )?;

        Ok(deleted)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub valid_entries: usize,
    pub query_cache_entries: usize,
    pub query_cache_expired: usize,
    pub bookmarks_count: usize,
    pub size_bytes: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BookmarkEntry {
    pub platform: String,
    pub full_name: String,
    pub data: String,
    pub bookmarked_at: i64,
    pub tags: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchHistoryEntry {
    pub id: i64,
    pub query: String,
    pub filters: Option<String>,
    pub result_count: Option<i64>,
    pub searched_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestRepo {
        name: String,
        description: Option<String>,
        topics: Vec<String>,
    }

    #[test]
    fn test_cache_set_and_get() {
        let cache = CacheManager::new(":memory:", 24).unwrap();

        let repo = TestRepo {
            name: "test/repo".to_string(),
            description: Some("A test repository".to_string()),
            topics: vec!["rust".to_string(), "test".to_string()],
        };

        cache.set("github", "test/repo", &repo).unwrap();

        let retrieved: TestRepo = cache.get("github", "test/repo").unwrap();
        assert_eq!(repo, retrieved);
    }

    #[test]
    fn test_cache_expiration() {
        let cache = CacheManager::new(":memory:", 0).unwrap(); // 0 hours = immediate expiration

        let repo = TestRepo {
            name: "test/repo".to_string(),
            description: None,
            topics: vec![],
        };

        cache.set("github", "test/repo", &repo).unwrap();

        // Should be expired immediately with 0 TTL
        std::thread::sleep(std::time::Duration::from_secs(1));
        let result: Result<TestRepo> = cache.get("github", "test/repo");
        assert!(matches!(result, Err(CacheError::Expired)));
    }

    #[test]
    fn test_cache_stats() {
        let cache = CacheManager::new(":memory:", 24).unwrap();

        let repo = TestRepo {
            name: "test/repo".to_string(),
            description: None,
            topics: vec![],
        };

        cache.set("github", "test/repo", &repo).unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.total_entries, 1);
    }
}
