use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

use crate::config::SqliteConfig;

const MIGRATION_001: &str = include_str!("schema.sql");
const MIGRATION_002: &str = include_str!("enrich_migration.sql");
const MIGRATION_003: &str = include_str!("schedule_migration.sql");

const MIGRATIONS: &[(u32, &str)] = &[(1, MIGRATION_001), (2, MIGRATION_002), (3, MIGRATION_003)];

/// Open or create a SQLite database with default PRAGMA settings.
pub fn open(path: &Path) -> Result<Connection> {
    open_with_config(path, &SqliteConfig::default())
}

/// Open or create a SQLite database with configurable PRAGMA settings.
pub fn open_with_config(path: &Path, sqlite_config: &SqliteConfig) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    let conn = Connection::open(path)
        .with_context(|| format!("failed to open database at {}", path.display()))?;

    // cache_size in KB (negative = KB in SQLite convention)
    let cache_size_kb = sqlite_config.cache_size_mb as i64 * 1000;
    let mmap_size = sqlite_config.mmap_size_mb as i64 * 1_048_576;

    let pragmas = format!(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA cache_size = -{cache_size_kb};
         PRAGMA mmap_size = {mmap_size};"
    );

    conn.execute_batch(&pragmas)
        .context("failed to set database PRAGMAs")?;

    migrate(&conn)?;

    Ok(conn)
}

/// Returns the default database path: ~/.blacklight/blacklight.db
pub fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".blacklight")
        .join("blacklight.db")
}

/// Run pending migrations against the database.
fn migrate(conn: &Connection) -> Result<()> {
    let current_version: u32 =
        conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    for &(version, sql) in MIGRATIONS {
        if version > current_version {
            tracing::info!("running migration v{version}");
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)
                .with_context(|| format!("migration v{version} failed"))?;
            tx.pragma_update(None, "user_version", version)?;
            tx.commit()
                .with_context(|| format!("failed to commit migration v{version}"))?;
            tracing::info!("migration v{version} complete");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_open_creates_db_and_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("nested").join("dir").join("test.db");
        let conn = open(&db_path).unwrap();

        // Verify file was created
        assert!(db_path.exists());

        // Verify WAL mode
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "wal");

        // Verify foreign keys enabled
        let fk: i32 = conn
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }

    #[test]
    fn test_migration_sets_version() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let conn = open(&db_path).unwrap();

        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 3);
    }

    #[test]
    fn test_migration_idempotent() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");

        // Open twice — second open should not re-run migrations
        let _conn1 = open(&db_path).unwrap();
        let conn2 = open(&db_path).unwrap();

        let version: u32 = conn2
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 3);
    }

    #[test]
    fn test_all_tables_created() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let conn = open(&db_path).unwrap();

        let expected_tables = [
            "sessions",
            "messages",
            "content_blocks",
            "tool_calls",
            "content_store",
            "blob_references",
            "file_references",
            "tasks",
            "task_dependencies",
            "session_outcomes",
            "outcome_categories",
            "outcome_friction",
            "daily_stats",
            "model_usage",
            "indexed_files",
            "session_enrichments",
            "session_tags",
            "schedule_config",
        ];

        for table in &expected_tables {
            let exists: bool = conn
                .prepare(&format!(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{table}'"
                ))
                .unwrap()
                .query_row([], |row| row.get::<_, i32>(0))
                .map(|c| c > 0)
                .unwrap();
            assert!(exists, "table {table} should exist");
        }

        // Verify FTS5 virtual table
        let fts_exists: bool = conn
            .prepare(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='fts_content'",
            )
            .unwrap()
            .query_row([], |row| row.get::<_, i32>(0))
            .map(|c| c > 0)
            .unwrap();
        assert!(fts_exists, "fts_content virtual table should exist");
    }

    #[test]
    fn test_default_db_path() {
        let path = default_db_path();
        assert!(path.ends_with(".blacklight/blacklight.db"));
    }
}
