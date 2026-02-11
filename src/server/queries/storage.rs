use anyhow::Result;
use rusqlite::Connection;

use crate::server::responses::{StorageByKind, StorageOverview};

pub fn get_storage_overview(conn: &Connection) -> Result<StorageOverview> {
    let unique_blobs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM content_store",
        [],
        |row| row.get(0),
    )?;
    let total_bytes: i64 = conn.query_row(
        "SELECT COALESCE(SUM(size), 0) FROM content_store",
        [],
        |row| row.get(0),
    )?;
    let total_references: i64 = conn.query_row(
        "SELECT COUNT(*) FROM blob_references",
        [],
        |row| row.get(0),
    )?;

    let dedup_ratio = if total_references > 0 {
        1.0 - (unique_blobs as f64 / total_references as f64)
    } else {
        0.0
    };

    let mut stmt = conn.prepare(
        "SELECT COALESCE(kind, 'unknown'), COUNT(*), COALESCE(SUM(size), 0)
         FROM content_store
         GROUP BY kind
         ORDER BY SUM(size) DESC",
    )?;

    let by_kind = stmt
        .query_map([], |row| {
            Ok(StorageByKind {
                kind: row.get(0)?,
                blob_count: row.get(1)?,
                total_bytes: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(StorageOverview {
        total_blobs: unique_blobs,
        total_bytes,
        unique_blobs,
        total_references,
        dedup_ratio,
        by_kind,
    })
}
