use anyhow::Result;
use rusqlite::{params, Connection};

use crate::server::responses::{ProjectDetail, ToolFrequency};

pub fn get_projects(conn: &Connection) -> Result<Vec<ProjectDetail>> {
    // Pre-aggregate each table separately via CTEs, then join on project_slug.
    // This avoids the cross-product explosion of multi-table JOINs (66s → 0.2s).
    let mut stmt = conn.prepare(
        "WITH sess AS (
           SELECT project_slug, project_path,
                  COUNT(*) as session_count,
                  MIN(created_at) as first_session,
                  MAX(modified_at) as last_session
           FROM sessions
           GROUP BY project_slug
         ),
         msg AS (
           SELECT s.project_slug, COUNT(*) as message_count
           FROM messages m
           JOIN sessions s ON s.id = m.session_id
           GROUP BY s.project_slug
         ),
         tc AS (
           SELECT s.project_slug, COUNT(*) as tool_call_count
           FROM tool_calls t
           JOIN sessions s ON s.id = t.session_id
           GROUP BY s.project_slug
         ),
         fr AS (
           SELECT s.project_slug, COUNT(DISTINCT f.file_path) as files_touched
           FROM file_references f
           JOIN sessions s ON s.id = f.session_id
           GROUP BY s.project_slug
         )
         SELECT sess.project_slug, sess.project_path, sess.session_count,
                COALESCE(msg.message_count, 0),
                COALESCE(tc.tool_call_count, 0),
                sess.first_session, sess.last_session,
                COALESCE(fr.files_touched, 0)
         FROM sess
         LEFT JOIN msg ON msg.project_slug = sess.project_slug
         LEFT JOIN tc ON tc.project_slug = sess.project_slug
         LEFT JOIN fr ON fr.project_slug = sess.project_slug
         ORDER BY COALESCE(msg.message_count, 0) DESC",
    )?;

    let projects = stmt
        .query_map([], |row| {
            Ok(ProjectDetail {
                project_slug: row.get(0)?,
                project_path: row.get(1)?,
                session_count: row.get(2)?,
                message_count: row.get(3)?,
                tool_call_count: row.get(4)?,
                first_session: row.get(5)?,
                last_session: row.get(6)?,
                files_touched: row.get(7)?,
                top_tools: Vec::new(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    // Fetch top 5 tools per project
    let mut tool_stmt = conn.prepare(
        "SELECT tc.tool_name, COUNT(*) as cnt
         FROM tool_calls tc
         JOIN sessions s ON s.id = tc.session_id
         WHERE s.project_slug = ?1
         GROUP BY tc.tool_name
         ORDER BY cnt DESC
         LIMIT 5",
    )?;

    let mut result = Vec::with_capacity(projects.len());
    for mut project in projects {
        let tools = tool_stmt
            .query_map(params![project.project_slug], |row| {
                Ok(ToolFrequency {
                    tool_name: row.get(0)?,
                    call_count: row.get(1)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        project.top_tools = tools;
        result.push(project);
    }

    Ok(result)
}
