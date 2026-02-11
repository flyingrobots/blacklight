use anyhow::Result;
use rusqlite::{params, Connection};

use crate::server::responses::{Paginated, SearchHit};

pub fn search_content(
    conn: &Connection,
    query: &str,
    kind: Option<&str>,
    _project: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Paginated<SearchHit>> {
    // Build the FTS5 match expression
    let match_expr = if let Some(k) = kind {
        format!("kind:{k} AND content:{query}")
    } else {
        query.to_string()
    };

    // Count total matches
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM fts_content WHERE fts_content MATCH ?1",
            params![match_expr],
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
         ORDER BY rank
         LIMIT ?2 OFFSET ?3",
    )?;

    let items = stmt
        .query_map(params![match_expr, limit, offset], |row| {
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
