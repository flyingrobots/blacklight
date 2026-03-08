use anyhow::Result;
use rusqlite::{params, Connection};
use std::collections::HashMap;

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

    // 1. Fetch messages for the page
    let mut msg_stmt = conn.prepare(
        "SELECT id, session_id, parent_id, type, timestamp, model, stop_reason, duration_ms
         FROM messages
         WHERE session_id = ?1
         ORDER BY timestamp
         LIMIT ?2 OFFSET ?3",
    )?;

    let messages_rows = msg_stmt
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

    if messages_rows.is_empty() {
        return Ok(Paginated {
            items: vec![],
            total,
            limit,
            offset,
        });
    }

    let message_ids: Vec<String> = messages_rows.iter().map(|(id, ..)| id.clone()).collect();
    let id_placeholders = message_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect::<Vec<_>>().join(",");

    // 2. Fetch all content blocks for these messages in ONE query
    let block_sql = format!(
        "SELECT cb.message_id, cb.block_index, cb.block_type, cs.content, cb.tool_name,
                cb.tool_use_id, cs_input.content
         FROM content_blocks cb
         LEFT JOIN content_store cs ON cs.hash = cb.content_hash
         LEFT JOIN content_store cs_input ON cs_input.hash = cb.tool_input_hash
         WHERE cb.message_id IN ({})
         ORDER BY cb.message_id, cb.block_index",
        id_placeholders
    );

    let mut block_stmt = conn.prepare(&block_sql)?;
    let block_rows = block_stmt.query_map(rusqlite::params_from_iter(message_ids), |row| {
        Ok((
            row.get::<_, String>(0)?,
            ContentBlockDetail {
                block_index: row.get(1)?,
                block_type: row.get(2)?,
                content: row.get(3)?,
                tool_name: row.get(4)?,
                tool_use_id: row.get(5)?,
                tool_input: row.get(6)?,
            }
        ))
    })?.collect::<std::result::Result<Vec<_>, _>>()?;

    // 3. Group blocks by message_id
    let mut blocks_by_msg: HashMap<String, Vec<ContentBlockDetail>> = HashMap::new();
    for (msg_id, block) in block_rows {
        blocks_by_msg.entry(msg_id).or_default().push(block);
    }

    // 4. Assemble final MessageDetail objects
    let mut items = Vec::with_capacity(messages_rows.len());
    for (id, session_id, parent_id, msg_type, timestamp, model, stop_reason, duration_ms) in
        messages_rows
    {
        let content_blocks = blocks_by_msg.remove(&id).unwrap_or_default();
        items.push(MessageDetail {
            id,
            session_id,
            parent_id,
            msg_type,
            timestamp,
            model,
            stop_reason,
            duration_ms,
            content_blocks,
        });
    }

    Ok(Paginated {
        items,
        total,
        limit,
        offset,
    })
}
