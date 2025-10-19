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
        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM repositories", [], |row| row.get(0))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let cutoff = now - self.ttl_seconds;

        let expired: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM repositories WHERE cached_at < ?1",
            params![cutoff],
            |row| row.get(0),
        )?;

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
            size_bytes: size_bytes as usize,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub valid_entries: usize,
    pub size_bytes: usize,
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
