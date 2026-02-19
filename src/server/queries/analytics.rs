use anyhow::Result;
use rusqlite::{params, Connection};

use crate::server::responses::{
    AnalyticsOverview, CoverageByKind, DailyStats, IndexCoverage, LlmBreakdown, ModelUsage,
    OutcomeStats, ProjectBreakdown, ToolFrequency,
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

pub fn get_tool_frequency(
    conn: &Connection,
    limit: i64,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<ToolFrequency>> {
    let mut where_clauses = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(f) = from {
        where_clauses.push(format!("timestamp >= ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(f.to_string()));
    }
    if let Some(t) = to {
        where_clauses.push(format!("timestamp <= ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(t.to_string()));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let sql = format!(
        "SELECT tool_name, COUNT(*) as cnt 
         FROM tool_calls 
         {where_sql}
         GROUP BY tool_name 
         ORDER BY cnt DESC 
         LIMIT ?{}",
        params_vec.len() + 1
    );
    params_vec.push(Box::new(limit));

    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())), |row| {
            Ok(ToolFrequency {
                tool_name: row.get(0)?,
                call_count: row.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(items)
}

pub fn get_project_breakdown(
    conn: &Connection,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<ProjectBreakdown>> {
    let mut where_clauses = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(f) = from {
        where_clauses.push(format!("created_at >= ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(f.to_string()));
    }
    if let Some(t) = to {
        where_clauses.push(format!("created_at <= ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(t.to_string()));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let sql = format!(
        "WITH sess AS (
           SELECT project_slug, id
           FROM sessions
           {where_sql}
         ),
         proj_counts AS (
           SELECT project_slug, COUNT(*) as session_count
           FROM sess
           GROUP BY project_slug
         ),
         msg_counts AS (
           SELECT s.project_slug, COUNT(*) as message_count
           FROM messages m
           JOIN sess s ON s.id = m.session_id
           GROUP BY s.project_slug
         ),
         tc_counts AS (
           SELECT s.project_slug, COUNT(*) as tool_call_count
           FROM tool_calls t
           JOIN sess s ON s.id = t.session_id
           GROUP BY s.project_slug
         )
         SELECT pc.project_slug, pc.session_count,
                COALESCE(mc.message_count, 0),
                COALESCE(tc.tool_call_count, 0)
         FROM proj_counts pc
         LEFT JOIN msg_counts mc ON mc.project_slug = pc.project_slug
         LEFT JOIN tc_counts tc ON tc.project_slug = pc.project_slug
         ORDER BY pc.session_count DESC"
    );

    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())), |row| {
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

pub fn get_llm_breakdown(
    conn: &Connection,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<LlmBreakdown>> {
    let mut where_clauses = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(f) = from {
        where_clauses.push(format!("created_at >= ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(f.to_string()));
    }
    if let Some(t) = to {
        where_clauses.push(format!("created_at <= ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(t.to_string()));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let sql = format!(
        "WITH sess AS (
           SELECT COALESCE(source_kind, 'unknown') as source_kind, id
           FROM sessions
           {where_sql}
         ),
         llm_counts AS (
           SELECT source_kind, COUNT(*) as session_count
           FROM sess
           GROUP BY source_kind
         ),
         msg_counts AS (
           SELECT s.source_kind, COUNT(*) as message_count
           FROM messages m
           JOIN sess s ON s.id = m.session_id
           GROUP BY s.source_kind
         ),
         tc_counts AS (
           SELECT s.source_kind, COUNT(*) as tool_call_count
           FROM tool_calls t
           JOIN sess s ON s.id = t.session_id
           GROUP BY s.source_kind
         )
         SELECT lc.source_kind, lc.session_count,
                COALESCE(mc.message_count, 0),
                COALESCE(tc.tool_call_count, 0)
         FROM llm_counts lc
         LEFT JOIN msg_counts mc ON mc.source_kind = lc.source_kind
         LEFT JOIN tc_counts tc ON tc.source_kind = lc.source_kind
         ORDER BY lc.session_count DESC"
    );

    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())), |row| {
            Ok(LlmBreakdown {
                source_kind: row.get(0)?,
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
    source_dirs: &[std::path::PathBuf],
) -> Result<IndexCoverage> {
    // Source file stats — scan all filesystem sources
    let mut source_files = 0i64;
    let mut source_bytes = 0i64;

    for dir in source_dirs {
        if let Ok(entries) = crate::indexer::scanner::scan(dir) {
            source_files += entries.len() as i64;
            source_bytes += entries.iter().map(|e| e.size_bytes as i64).sum::<i64>();
        }
    }

    if source_dirs.is_empty() {
        // Fall back to indexed_files count as estimate if no sources
        source_files = conn.query_row("SELECT COUNT(*) FROM indexed_files", [], |row| row.get(0))?;
        source_bytes = conn.query_row(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM indexed_files",
            [],
            |row| row.get(0),
        )?;
    }

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
