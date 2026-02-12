use anyhow::Result;
use rusqlite::{params, Connection};

use crate::server::responses::{Paginated, ReviewItem, SessionTag};

pub fn list_pending(
    conn: &Connection,
    limit: i64,
    offset: i64,
) -> Result<Paginated<ReviewItem>> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM session_enrichments WHERE approval_status = 'pending_review'",
        [],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT e.session_id, e.title, e.summary, e.enriched_at, e.model_used,
                s.project_slug, s.created_at, s.first_prompt
         FROM session_enrichments e
         JOIN sessions s ON s.id = e.session_id
         WHERE e.approval_status = 'pending_review'
         ORDER BY e.enriched_at DESC
         LIMIT ?1 OFFSET ?2",
    )?;

    let rows: Vec<_> = stmt
        .query_map(params![limit, offset], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut tag_stmt = conn.prepare(
        "SELECT tag, confidence FROM session_tags WHERE session_id = ?1 ORDER BY confidence DESC",
    )?;

    let mut items = Vec::with_capacity(rows.len());
    for (session_id, title, summary, enriched_at, model_used, project_slug, session_created_at, first_prompt) in rows {
        let tags = tag_stmt
            .query_map(params![session_id], |row| {
                Ok(SessionTag {
                    tag: row.get(0)?,
                    confidence: row.get(1)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        items.push(ReviewItem {
            session_id,
            title,
            summary,
            enriched_at,
            model_used,
            project_slug,
            session_created_at,
            first_prompt,
            tags,
        });
    }

    Ok(Paginated {
        items,
        total,
        limit,
        offset,
    })
}

pub fn approve_session(conn: &Connection, session_id: &str) -> Result<bool> {
    let now = chrono::Utc::now().to_rfc3339();
    let rows = conn.execute(
        "UPDATE session_enrichments SET approval_status = 'approved', reviewed_at = ?1
         WHERE session_id = ?2 AND approval_status = 'pending_review'",
        params![now, session_id],
    )?;
    Ok(rows > 0)
}

pub fn reject_session(conn: &Connection, session_id: &str) -> Result<bool> {
    let now = chrono::Utc::now().to_rfc3339();
    let rows = conn.execute(
        "UPDATE session_enrichments SET approval_status = 'rejected', reviewed_at = ?1
         WHERE session_id = ?2 AND approval_status = 'pending_review'",
        params![now, session_id],
    )?;
    Ok(rows > 0)
}

pub fn approve_all(conn: &Connection) -> Result<usize> {
    let now = chrono::Utc::now().to_rfc3339();
    let rows = conn.execute(
        "UPDATE session_enrichments SET approval_status = 'approved', reviewed_at = ?1
         WHERE approval_status = 'pending_review'",
        params![now],
    )?;
    Ok(rows)
}
