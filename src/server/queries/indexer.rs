use anyhow::Result;
use rusqlite::{params, Connection};

use crate::server::responses::{IndexRun, Paginated};

pub fn list_runs(
    conn: &mut Connection,
    limit: i64,
    offset: i64,
) -> Result<Paginated<IndexRun>> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM index_runs",
        [],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, started_at, finished_at, status, is_full,
                files_scanned, files_processed, files_unchanged,
                sessions_parsed, messages_processed, blobs_inserted,
                errors, error_message
         FROM index_runs
         ORDER BY started_at DESC
         LIMIT ?1 OFFSET ?2",
    )?;

    let items = stmt
        .query_map(params![limit, offset], |row| {
            Ok(IndexRun {
                id: row.get(0)?,
                started_at: row.get(1)?,
                finished_at: row.get(2)?,
                status: row.get(3)?,
                is_full: row.get::<_, i32>(4)? != 0,
                files_scanned: row.get(5)?,
                files_processed: row.get(6)?,
                files_unchanged: row.get(7)?,
                sessions_parsed: row.get(8)?,
                messages_processed: row.get(9)?,
                blobs_inserted: row.get(10)?,
                errors: row.get(11)?,
                error_message: row.get(12)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(Paginated {
        items,
        total,
        limit,
        offset,
    })
}
