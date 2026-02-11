use anyhow::Result;
use rusqlite::{params, Connection};

use crate::server::responses::{FileProvenance, FileReference, Paginated};

pub fn get_file_references(
    conn: &Connection,
    path: Option<&str>,
    session: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Paginated<FileReference>> {
    let mut where_clauses = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(p) = path {
        where_clauses.push(format!("file_path LIKE ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(format!("%{p}%")));
    }
    if let Some(s) = session {
        where_clauses.push(format!("session_id = ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(s.to_string()));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM file_references {where_sql}");
    let total: i64 = conn.query_row(
        &count_sql,
        rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
        |row| row.get(0),
    )?;

    let query_sql = format!(
        "SELECT file_path, operation, session_id, message_id
         FROM file_references
         {where_sql}
         ORDER BY file_path
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
                Ok(FileReference {
                    file_path: row.get(0)?,
                    operation: row.get(1)?,
                    session_id: row.get(2)?,
                    message_id: row.get(3)?,
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

pub fn get_file_provenance(conn: &Connection, limit: i64) -> Result<Vec<FileProvenance>> {
    let mut stmt = conn.prepare(
        "SELECT file_path,
                COUNT(DISTINCT session_id) as session_count,
                GROUP_CONCAT(DISTINCT operation) as operations,
                MAX(session_id) as last_session_id
         FROM file_references
         GROUP BY file_path
         ORDER BY session_count DESC
         LIMIT ?1",
    )?;

    let items = stmt
        .query_map(params![limit], |row| {
            let ops_str: Option<String> = row.get(2)?;
            let operations = ops_str
                .map(|s| s.split(',').map(String::from).collect())
                .unwrap_or_default();
            Ok(FileProvenance {
                file_path: row.get(0)?,
                session_count: row.get(1)?,
                operations,
                last_session_id: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(items)
}
