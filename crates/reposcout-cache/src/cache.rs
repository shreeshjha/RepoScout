use rusqlite::{Connection, Result};

/// Cache manager using SQLite + FTS5
///
/// SQLite was chosen because:
/// - Zero-config embedded database
/// - FTS5 for fast text search
/// - Battle-tested and reliable
/// - Doesn't require a separate process
pub struct CacheManager {
    conn: Connection,
}

impl CacheManager {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Initialize schema on first run
        Self::init_schema(&conn)?;

        Ok(Self { conn })
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        // Create repositories table with FTS5 index
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
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS repositories_fts
             USING fts5(full_name, description, topics)",
            [],
        )?;

        Ok(())
    }

    // TODO: Add actual cache operations
    // - get, set, delete
    // - search using FTS5
    // - TTL-based expiration
}
