use anyhow::Result;
use rusqlite::{params, Connection};

use crate::server::responses::{ContentBlockDetail, MessageDetail, Paginated};

pub fn get_messages(
    conn: &Connection,
    session_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Paginated<MessageDetail>> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM messages WHERE session_id = ?1",
        params![session_id],
        |row| row.get(0),
    )?;

    let mut msg_stmt = conn.prepare(
        "SELECT id, session_id, parent_id, type, timestamp, model, stop_reason, duration_ms
         FROM messages
         WHERE session_id = ?1
         ORDER BY timestamp
         LIMIT ?2 OFFSET ?3",
    )?;

    let mut block_stmt = conn.prepare(
        "SELECT cb.block_index, cb.block_type, cs.content, cb.tool_name,
                cb.tool_use_id, cs_input.content
         FROM content_blocks cb
         LEFT JOIN content_store cs ON cs.hash = cb.content_hash
         LEFT JOIN content_store cs_input ON cs_input.hash = cb.tool_input_hash
         WHERE cb.message_id = ?1
         ORDER BY cb.block_index",
    )?;

    let messages = msg_stmt
        .query_map(params![session_id, limit, offset], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<i64>>(7)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut items = Vec::with_capacity(messages.len());
    for (id, session_id, parent_id, msg_type, timestamp, model, stop_reason, duration_ms) in
        messages
    {
        let blocks = block_stmt
            .query_map(params![id], |row| {
                Ok(ContentBlockDetail {
                    block_index: row.get(0)?,
                    block_type: row.get(1)?,
                    content: row.get(2)?,
                    tool_name: row.get(3)?,
                    tool_use_id: row.get(4)?,
                    tool_input: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        items.push(MessageDetail {
            id,
            session_id,
            parent_id,
            msg_type,
            timestamp,
            model,
            stop_reason,
            duration_ms,
            content_blocks: blocks,
        });
    }

    Ok(Paginated {
        items,
        total,
        limit,
        offset,
    })
}
