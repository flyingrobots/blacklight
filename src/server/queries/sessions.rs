use anyhow::Result;
use rusqlite::{params, Connection};

use crate::server::responses::{
    FileReference, Paginated, SessionDetail, SessionOutcome, SessionSummary, ToolCallDetail,
};

pub fn list_sessions(
    conn: &Connection,
    project: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Paginated<SessionSummary>> {
    let mut where_clauses = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(p) = project {
        where_clauses.push(format!("s.project_slug = ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(p.to_string()));
    }
    if let Some(f) = from {
        where_clauses.push(format!("s.created_at >= ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(f.to_string()));
    }
    if let Some(t) = to {
        where_clauses.push(format!("s.created_at <= ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(t.to_string()));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    // Count total
    let count_sql = format!("SELECT COUNT(*) FROM sessions s {where_sql}");
    let total: i64 = conn.query_row(
        &count_sql,
        rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
        |row| row.get(0),
    )?;

    // Fetch page
    let query_sql = format!(
        "SELECT s.id, s.project_path, s.project_slug, s.first_prompt, s.summary,
                s.message_count, s.created_at, s.modified_at, s.git_branch,
                s.claude_version, s.is_sidechain,
                o.outcome, o.brief_summary
         FROM sessions s
         LEFT JOIN session_outcomes o ON o.session_id = s.id
         {where_sql}
         ORDER BY s.modified_at DESC
         LIMIT ?{} OFFSET ?{}",
        params_vec.len() + 1,
        params_vec.len() + 2
    );
    params_vec.push(Box::new(limit));
    params_vec.push(Box::new(offset));

    let mut stmt = conn.prepare(&query_sql)?;
    let items = stmt
        .query_map(
            rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
            |row| {
                Ok(SessionSummary {
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
                    outcome: row.get(11)?,
                    brief_summary: row.get(12)?,
                })
            },
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(Paginated {
        items,
        total,
        limit,
        offset,
    })
}

pub fn get_session(conn: &Connection, id: &str) -> Result<Option<SessionDetail>> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.project_path, s.project_slug, s.first_prompt, s.summary,
                s.message_count, s.created_at, s.modified_at, s.git_branch,
                s.claude_version, s.is_sidechain,
                o.underlying_goal, o.outcome, o.helpfulness, o.session_type,
                o.primary_success, o.friction_detail, o.brief_summary
         FROM sessions s
         LEFT JOIN session_outcomes o ON o.session_id = s.id
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
            })
        })
        .optional()?;

    Ok(result)
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
