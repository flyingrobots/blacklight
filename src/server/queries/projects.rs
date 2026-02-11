use anyhow::Result;
use rusqlite::{params, Connection};

use crate::server::responses::{ProjectDetail, ToolFrequency};

pub fn get_projects(conn: &Connection) -> Result<Vec<ProjectDetail>> {
    let mut stmt = conn.prepare(
        "SELECT s.project_slug,
                s.project_path,
                COUNT(DISTINCT s.id) as session_count,
                COUNT(DISTINCT m.id) as message_count,
                COUNT(DISTINCT tc.id) as tool_call_count,
                MIN(s.created_at) as first_session,
                MAX(s.modified_at) as last_session,
                COUNT(DISTINCT fr.file_path) as files_touched
         FROM sessions s
         LEFT JOIN messages m ON m.session_id = s.id
         LEFT JOIN tool_calls tc ON tc.session_id = s.id
         LEFT JOIN file_references fr ON fr.session_id = s.id
         GROUP BY s.project_slug
         ORDER BY message_count DESC",
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
                top_tools: Vec::new(), // populated below
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
