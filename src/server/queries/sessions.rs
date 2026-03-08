use anyhow::Result;
use rusqlite::{params, Connection};
use std::collections::HashMap;

use crate::db::query_builder::QueryBuilder;
use crate::server::responses::{
    Paginated, SessionDetail, SessionOutcome, SessionSummary, SessionTag,
    ToolCallDetail, FileReference,
};

pub fn list_sessions(
    conn: &Connection,
    project: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Paginated<SessionSummary>> {
    let mut qb = QueryBuilder::new(
        "SELECT s.id, s.project_path, s.project_slug,
                COALESCE(
                    CASE WHEN e.approval_status = 'approved' THEN e.title END,
                    s.first_prompt,
                    (SELECT substr(cs.content, 1, 200)
                     FROM messages m2
                     JOIN content_blocks cb ON cb.message_id = m2.id
                     JOIN content_store cs ON cs.hash = cb.content_hash
                     WHERE m2.session_id = s.id AND m2.type = 'user'
                     ORDER BY m2.timestamp ASC
                     LIMIT 1)
                ) as first_prompt,
                s.summary,
                (SELECT COUNT(*) FROM messages m WHERE m.session_id = s.id) as message_count,
                s.created_at, s.modified_at, s.git_branch,
                s.claude_version, s.is_sidechain,
                o.outcome, o.brief_summary,
                e.title, e.summary, e.approval_status,
                s.source_name, s.source_kind, s.app_version, s.fingerprint
         FROM sessions s
         LEFT JOIN session_outcomes o ON o.session_id = s.id
         LEFT JOIN session_enrichments e ON e.session_id = s.id"
    );

    if let Some(p) = project {
        qb = qb.r#where("s.project_slug = ?", Box::new(p.to_string()));
    }
    if let Some(f) = from {
        qb = qb.r#where("s.created_at >= ?", Box::new(f.to_string()));
    }
    if let Some(t) = to {
        qb = qb.r#where("s.created_at <= ?", Box::new(t.to_string()));
    }

    // 1. Get total count
    let mut count_sql = "SELECT COUNT(*) FROM sessions s".to_string();
    let mut count_params: Vec<Box<dyn rusqlite::ToSql + Send>> = Vec::new();
    let mut where_clauses = Vec::new();
    if let Some(p) = project {
        where_clauses.push(format!("s.project_slug = ?{}", count_params.len() + 1));
        count_params.push(Box::new(p.to_string()));
    }
    if let Some(f) = from {
        where_clauses.push(format!("s.created_at >= ?{}", count_params.len() + 1));
        count_params.push(Box::new(f.to_string()));
    }
    if let Some(t) = to {
        where_clauses.push(format!("s.created_at <= ?{}", count_params.len() + 1));
        count_params.push(Box::new(t.to_string()));
    }
    if !where_clauses.is_empty() {
        count_sql.push_str(" WHERE ");
        count_sql.push_str(&where_clauses.join(" AND "));
    }

    let total: i64 = conn.query_row(
        &count_sql,
        rusqlite::params_from_iter(count_params.iter().map(|p| p.as_ref())),
        |row| row.get(0),
    )?;

    // 2. Fetch sessions
    let (sql, params) = qb.order_by("s.modified_at DESC")
        .limit(limit)
        .offset(offset)
        .build_with_limit();

    let mut stmt = conn.prepare(&sql)?;
    let session_rows = stmt.query_map(rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
        Ok((
            row.get::<_, String>(0)?, // id
            row.get::<_, String>(1)?, // project_path
            row.get::<_, String>(2)?, // project_slug
            row.get::<_, Option<String>>(3)?, // first_prompt
            row.get::<_, Option<String>>(4)?, // summary
            row.get::<_, Option<i64>>(5)?, // message_count
            row.get::<_, String>(6)?, // created_at
            row.get::<_, String>(7)?, // modified_at
            row.get::<_, Option<String>>(8)?, // git_branch
            row.get::<_, Option<String>>(9)?, // claude_version
            row.get::<_, Option<i64>>(10)?.unwrap_or(0) != 0, // is_sidechain
            row.get::<_, Option<String>>(11)?, // outcome
            row.get::<_, Option<String>>(12)?, // brief_summary
            row.get::<_, Option<String>>(13)?, // enrichment_title
            row.get::<_, Option<String>>(14)?, // enrichment_summary
            row.get::<_, Option<String>>(15)?, // approval_status
            row.get::<_, Option<String>>(16)?, // source_name
            row.get::<_, Option<String>>(17)?, // source_kind
            row.get::<_, Option<String>>(18)?, // app_version
            row.get::<_, Option<String>>(19)?, // fingerprint
        ))
    })?.collect::<std::result::Result<Vec<_>, _>>()?;

    if session_rows.is_empty() {
        return Ok(Paginated {
            items: vec![],
            total,
            limit, offset,
        });
    }

    let session_ids: Vec<String> = session_rows.iter().map(|(id, ..)| id.clone()).collect();
    let id_placeholders = session_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect::<Vec<_>>().join(",");

    // 3. Fetch tags for ALL sessions in one go
    let tag_sql = format!(
        "SELECT session_id, tag, confidence FROM session_tags WHERE session_id IN ({}) ORDER BY confidence DESC",
        id_placeholders
    );
    let mut tag_stmt = conn.prepare(&tag_sql)?;
    let tag_rows = tag_stmt.query_map(rusqlite::params_from_iter(session_ids), |row| {
        Ok((
            row.get::<_, String>(0)?,
            SessionTag {
                tag: row.get(1)?,
                confidence: row.get(2)?,
            }
        ))
    })?.collect::<std::result::Result<Vec<_>, _>>()?;

    let mut tags_by_session: HashMap<String, Vec<SessionTag>> = HashMap::new();
    for (sid, tag) in tag_rows {
        tags_by_session.entry(sid).or_default().push(tag);
    }

    // 4. Assemble
    let mut items = Vec::with_capacity(session_rows.len());
    for (id, project_path, project_slug, first_prompt, summary, message_count,
         created_at, modified_at, git_branch, claude_version, is_sidechain,
         outcome, brief_summary, enrichment_title, enrichment_summary, approval_status,
         source_name, source_kind, app_version, fingerprint) in session_rows
    {
        let tags = tags_by_session.remove(&id).unwrap_or_default();
        items.push(SessionSummary {
            id, project_path, project_slug, first_prompt, summary, message_count,
            created_at, modified_at, git_branch, claude_version, is_sidechain,
            outcome, brief_summary, enrichment_title, enrichment_summary,
            approval_status, tags,
            source_name, source_kind, app_version, fingerprint,
        });
    }

    Ok(Paginated {
        items,
        total,
        limit,
        offset,
    })
}

pub fn get_session(conn: &Connection, id: &str) -> Result<Option<SessionDetail>> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.project_path, s.project_slug,
                COALESCE(s.first_prompt, (
                    SELECT substr(cs.content, 1, 200)
                    FROM messages m2
                    JOIN content_blocks cb ON cb.message_id = m2.id
                    JOIN content_store cs ON cs.hash = cb.content_hash
                    WHERE m2.session_id = s.id AND m2.type = 'user'
                    ORDER BY m2.timestamp ASC
                    LIMIT 1
                )) as first_prompt,
                s.summary,
                (SELECT COUNT(*) FROM messages m WHERE m.session_id = s.id) as message_count,
                s.created_at, s.modified_at, s.git_branch,
                s.claude_version, s.is_sidechain,
                o.underlying_goal, o.outcome, o.helpfulness, o.session_type,
                o.primary_success, o.friction_detail, o.brief_summary,
                e.title, e.summary, e.approval_status,
                s.source_name, s.source_kind, s.app_version, s.fingerprint
         FROM sessions s
         LEFT JOIN session_outcomes o ON o.session_id = s.id
         LEFT JOIN session_enrichments e ON e.session_id = s.id
         WHERE s.id = ?1",
    )?;

    let result = stmt
        .query_row(params![id], |row| {
            let outcome_text: Option<String> = row.get(12)?;
            let outcome = if outcome_text.is_some() {
                Some(SessionOutcome {
                    underlying_goal: row.get(11)?,
                    outcome: row.get(12)?,
                    helpfulness: row.get(13)?,
                    session_type: row.get(14)?,
                    primary_success: row.get(15)?,
                    friction_detail: row.get(16)?,
                    brief_summary: row.get(17)?,
                })
            } else {
                None
            };

            Ok(SessionDetail {
                id: row.get(0)?,
                project_path: row.get(1)?,
                project_slug: row.get(2)?,
                first_prompt: row.get(3)?,
                summary: row.get(4)?,
                message_count: row.get(5)?,
                created_at: row.get(6)?,
                modified_at: row.get(7)?,
                git_branch: row.get(8)?,
                claude_version: row.get(9)?,
                is_sidechain: row.get::<_, Option<i64>>(10)?.unwrap_or(0) != 0,
                outcome,
                enrichment_title: row.get(18)?,
                enrichment_summary: row.get(19)?,
                approval_status: row.get(20)?,
                tags: Vec::new(), // filled below
                source_name: row.get(21)?,
                source_kind: row.get(22)?,
                app_version: row.get(23)?,
                fingerprint: row.get(24)?,
            })
        })
        .optional()?;

    if let Some(mut detail) = result {
        let mut tag_stmt = conn.prepare(
            "SELECT tag, confidence FROM session_tags WHERE session_id = ?1 ORDER BY confidence DESC",
        )?;
        detail.tags = tag_stmt
            .query_map(params![id], |row| {
                Ok(SessionTag {
                    tag: row.get(0)?,
                    confidence: row.get(1)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(Some(detail))
    } else {
        Ok(None)
    }
}

pub fn get_session_tools(conn: &Connection, session_id: &str) -> Result<Vec<ToolCallDetail>> {
    let mut stmt = conn.prepare(
        "SELECT tc.id, tc.tool_name, tc.timestamp,
                cs_in.content, cs_out.content
         FROM tool_calls tc
         LEFT JOIN content_store cs_in ON cs_in.hash = tc.input_hash
         LEFT JOIN content_store cs_out ON cs_out.hash = tc.output_hash
         WHERE tc.session_id = ?1
         ORDER BY tc.timestamp",
    )?;

    let items = stmt
        .query_map(params![session_id], |row| {
            Ok(ToolCallDetail {
                id: row.get(0)?,
                tool_name: row.get(1)?,
                timestamp: row.get(2)?,
                input: row.get(3)?,
                output: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(items)
}

pub fn get_session_files(conn: &Connection, session_id: &str) -> Result<Vec<FileReference>> {
    let mut stmt = conn.prepare(
        "SELECT file_path, operation, session_id, message_id
         FROM file_references
         WHERE session_id = ?1
         ORDER BY file_path",
    )?;

    let items = stmt
        .query_map(params![session_id], |row| {
            Ok(FileReference {
                file_path: row.get(0)?,
                operation: row.get(1)?,
                session_id: row.get(2)?,
                message_id: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(items)
}

/// Helper: make rusqlite's QueryReturnedNoRows into Option::None
trait OptionalExt<T> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for std::result::Result<T, rusqlite::Error> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
