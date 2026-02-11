use anyhow::Result;
use rusqlite::{params, Connection};

use crate::server::responses::{
    AnalyticsOverview, CoverageByKind, DailyStats, IndexCoverage, ModelUsage, OutcomeStats,
    ProjectBreakdown, ToolFrequency,
};

pub fn get_overview(conn: &Connection, db_path: &str) -> Result<AnalyticsOverview> {
    let total_sessions: i64 =
        conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;
    let total_messages: i64 =
        conn.query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))?;
    let total_blobs: i64 =
        conn.query_row("SELECT COUNT(*) FROM content_store", [], |row| row.get(0))?;
    let total_blob_bytes: i64 = conn.query_row(
        "SELECT COALESCE(SUM(size), 0) FROM content_store",
        [],
        |row| row.get(0),
    )?;

    let first_session: Option<String> = conn
        .query_row(
            "SELECT MIN(created_at) FROM sessions",
            [],
            |row| row.get(0),
        )
        .unwrap_or(None);
    let last_session: Option<String> = conn
        .query_row(
            "SELECT MAX(modified_at) FROM sessions",
            [],
            |row| row.get(0),
        )
        .unwrap_or(None);

    // Get DB file size
    let db_size_bytes: i64 = if std::path::Path::new(db_path).exists() {
        std::fs::metadata(db_path)
            .map(|m| m.len() as i64)
            .unwrap_or(0)
    } else {
        0
    };

    Ok(AnalyticsOverview {
        total_sessions,
        total_messages,
        total_blobs,
        total_blob_bytes,
        db_size_bytes,
        first_session,
        last_session,
    })
}

pub fn get_daily_stats(
    conn: &Connection,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<DailyStats>> {
    let (sql, param_count) = match (from, to) {
        (Some(_), Some(_)) => (
            "SELECT date, message_count, session_count, tool_call_count
             FROM daily_stats WHERE date >= ?1 AND date <= ?2 ORDER BY date",
            2,
        ),
        (Some(_), None) => (
            "SELECT date, message_count, session_count, tool_call_count
             FROM daily_stats WHERE date >= ?1 ORDER BY date",
            1,
        ),
        (None, Some(_)) => (
            "SELECT date, message_count, session_count, tool_call_count
             FROM daily_stats WHERE date <= ?1 ORDER BY date",
            1,
        ),
        (None, None) => (
            "SELECT date, message_count, session_count, tool_call_count
             FROM daily_stats ORDER BY date",
            0,
        ),
    };

    let mut stmt = conn.prepare(sql)?;

    let rows = match param_count {
        0 => stmt.query_map([], map_daily_row)?,
        1 => {
            let p = from.or(to).unwrap();
            stmt.query_map(params![p], map_daily_row)?
        }
        _ => stmt.query_map(params![from.unwrap(), to.unwrap()], map_daily_row)?,
    };

    let items = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(items)
}

fn map_daily_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DailyStats> {
    Ok(DailyStats {
        date: row.get(0)?,
        message_count: row.get(1)?,
        session_count: row.get(2)?,
        tool_call_count: row.get(3)?,
    })
}

pub fn get_model_usage(conn: &Connection) -> Result<Vec<ModelUsage>> {
    let mut stmt = conn.prepare(
        "SELECT model, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens
         FROM model_usage
         ORDER BY COALESCE(input_tokens, 0) + COALESCE(output_tokens, 0) DESC",
    )?;

    let items = stmt
        .query_map([], |row| {
            Ok(ModelUsage {
                model: row.get(0)?,
                input_tokens: row.get(1)?,
                output_tokens: row.get(2)?,
                cache_read_tokens: row.get(3)?,
                cache_creation_tokens: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(items)
}

pub fn get_tool_frequency(conn: &Connection, limit: i64) -> Result<Vec<ToolFrequency>> {
    let mut stmt = conn.prepare(
        "SELECT tool_name, COUNT(*) as cnt
         FROM tool_calls
         GROUP BY tool_name
         ORDER BY cnt DESC
         LIMIT ?1",
    )?;

    let items = stmt
        .query_map(params![limit], |row| {
            Ok(ToolFrequency {
                tool_name: row.get(0)?,
                call_count: row.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(items)
}

pub fn get_project_breakdown(conn: &Connection) -> Result<Vec<ProjectBreakdown>> {
    // Use subqueries to avoid cross-product explosion from multi-table JOINs
    let mut stmt = conn.prepare(
        "SELECT s.project_slug,
                COUNT(*) as session_count,
                (SELECT COUNT(*) FROM messages m
                 WHERE m.session_id IN (SELECT id FROM sessions WHERE project_slug = s.project_slug)
                ) as message_count,
                (SELECT COUNT(*) FROM tool_calls tc
                 WHERE tc.session_id IN (SELECT id FROM sessions WHERE project_slug = s.project_slug)
                ) as tool_call_count
         FROM sessions s
         GROUP BY s.project_slug
         ORDER BY session_count DESC",
    )?;

    let items = stmt
        .query_map([], |row| {
            Ok(ProjectBreakdown {
                project_slug: row.get(0)?,
                session_count: row.get(1)?,
                message_count: row.get(2)?,
                tool_call_count: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(items)
}

pub fn get_coverage(
    conn: &Connection,
    source_dir: Option<&std::path::Path>,
) -> Result<IndexCoverage> {
    // Source file stats — scan the filesystem if a source dir is provided
    let (source_files, source_bytes) = if let Some(dir) = source_dir {
        match crate::indexer::scanner::scan(dir) {
            Ok(entries) => {
                let count = entries.len() as i64;
                let bytes: i64 = entries.iter().map(|e| e.size_bytes as i64).sum();
                (count, bytes)
            }
            Err(_) => (0i64, 0i64),
        }
    } else {
        // Fall back to indexed_files count as estimate
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM indexed_files", [], |row| row.get(0))?;
        let bytes: i64 = conn.query_row(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM indexed_files",
            [],
            |row| row.get(0),
        )?;
        (count, bytes)
    };

    // Indexed file stats (from indexed_files table)
    let indexed_files: i64 =
        conn.query_row("SELECT COUNT(*) FROM indexed_files", [], |row| row.get(0))?;
    let indexed_bytes: i64 = conn.query_row(
        "SELECT COALESCE(SUM(size_bytes), 0) FROM indexed_files",
        [],
        |row| row.get(0),
    )?;

    // Content store vs FTS5
    let blobs_stored: i64 =
        conn.query_row("SELECT COUNT(*) FROM content_store", [], |row| row.get(0))?;
    let blobs_searchable: i64 =
        conn.query_row("SELECT COUNT(*) FROM fts_content", [], |row| row.get(0))?;

    // Sessions / outcomes
    let total_sessions: i64 =
        conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;
    let sessions_with_outcomes: i64 = conn.query_row(
        "SELECT COUNT(*) FROM session_outcomes",
        [],
        |row| row.get(0),
    )?;

    // Messages with content
    let total_messages: i64 =
        conn.query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))?;
    let messages_with_content: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT message_id) FROM content_blocks",
        [],
        |row| row.get(0),
    )?;

    // Breakdown by file kind (infer from path patterns)
    let mut stmt = conn.prepare(
        "SELECT
           CASE
             WHEN file_path LIKE '%.jsonl' AND file_path LIKE '%/projects/%' THEN 'Session JSONL'
             WHEN file_path LIKE '%sessions-index.json' THEN 'Session Index'
             WHEN file_path LIKE '%/tasks/%.json' THEN 'Tasks'
             WHEN file_path LIKE '%/facets/%.json' THEN 'Facets'
             WHEN file_path LIKE '%stats-cache.json' THEN 'Stats Cache'
             WHEN file_path LIKE '%/plans/%.md' THEN 'Plans'
             WHEN file_path LIKE '%history.jsonl' THEN 'History'
             ELSE 'Other'
           END as kind,
           COUNT(*),
           COALESCE(SUM(size_bytes), 0)
         FROM indexed_files
         GROUP BY kind
         ORDER BY SUM(size_bytes) DESC",
    )?;
    let by_kind = stmt
        .query_map([], |row| {
            Ok(CoverageByKind {
                kind: row.get(0)?,
                file_count: row.get(1)?,
                total_bytes: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let pct = |n: i64, d: i64| -> f64 {
        if d == 0 {
            0.0
        } else {
            (n as f64 / d as f64) * 100.0
        }
    };

    Ok(IndexCoverage {
        source_files,
        source_bytes,
        indexed_files,
        indexed_bytes,
        file_pct: pct(indexed_files, source_files),
        byte_pct: pct(indexed_bytes, source_bytes),
        blobs_stored,
        blobs_searchable,
        search_pct: pct(blobs_searchable, blobs_stored),
        sessions_with_outcomes,
        total_sessions,
        outcome_pct: pct(sessions_with_outcomes, total_sessions),
        messages_with_content,
        total_messages,
        by_kind,
    })
}

pub fn get_outcome_distribution(conn: &Connection) -> Result<Vec<OutcomeStats>> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(outcome, 'unknown') as outcome, COUNT(*) as cnt
         FROM session_outcomes
         GROUP BY outcome
         ORDER BY cnt DESC",
    )?;

    let items = stmt
        .query_map([], |row| {
            Ok(OutcomeStats {
                outcome: row.get(0)?,
                count: row.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(items)
}
