use anyhow::Result;
use rusqlite::{params, Connection};

use crate::server::responses::{Paginated, SearchHit};

pub fn search_content(
    conn: &Connection,
    query: &str,
    kind: Option<&str>,
    project: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Paginated<SearchHit>> {
    // Sanitize query: wrap in double quotes and escape internal quotes to prevent
    // FTS5 from interpreting special characters (like :) as column filters or operators.
    let escaped_query = query.replace('\"', "\"\"");
    let quoted_query = format!("\"{}\"", escaped_query);

    // Build the FTS5 match expression
    let match_expr = if let Some(k) = kind {
        format!("kind:{k} AND content:{quoted_query}")
    } else {
        quoted_query
    };

    // Count total matches (including optional project filter).
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM (
                SELECT 1
                FROM fts_content f
                LEFT JOIN blob_references br ON br.hash = f.hash
                LEFT JOIN messages m ON m.id = br.message_id
                LEFT JOIN sessions s ON s.id = m.session_id
                WHERE fts_content MATCH ?1
                  AND (?2 IS NULL OR s.project_slug = ?2)
             )",
            params![&match_expr, project],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Search with enrichment: join through blob_references to get session/message context
    let mut stmt = conn.prepare(
        "SELECT f.hash, f.kind,
                snippet(fts_content, 2, '<mark>', '</mark>', '...', 64) as snippet,
                bm25(fts_content) as rank,
                br.message_id,
                m.type,
                m.session_id,
                s.summary
         FROM fts_content f
         LEFT JOIN blob_references br ON br.hash = f.hash
         LEFT JOIN messages m ON m.id = br.message_id
         LEFT JOIN sessions s ON s.id = m.session_id
         WHERE fts_content MATCH ?1
           AND (?4 IS NULL OR s.project_slug = ?4)
         ORDER BY rank
         LIMIT ?2 OFFSET ?3",
    )?;

    let items = stmt
        .query_map(params![&match_expr, limit, offset, project], |row| {
            Ok(SearchHit {
                hash: row.get(0)?,
                kind: row.get(1)?,
                snippet: row.get(2)?,
                rank: row.get(3)?,
                message_id: row.get(4)?,
                message_type: row.get(5)?,
                session_id: row.get(6)?,
                session_summary: row.get(7)?,
            })
        })
        .map_err(|e| anyhow::anyhow!("FTS5 search failed (check query syntax): {e}"))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("FTS5 search failed: {e}"))?;

    Ok(Paginated {
        items,
        total,
        limit,
        offset,
    })
}
