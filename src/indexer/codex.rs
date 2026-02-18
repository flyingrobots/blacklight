use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::indexer::db_ops::{self, LineOps, MessageRow, ContentBlockRow};
use crate::content::hash_content;

#[derive(Debug, Deserialize, Serialize)]
pub struct CodexMessage {
    pub timestamp: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CodexSessionMeta {
    pub id: String,
    pub timestamp: String,
    pub cwd: Option<String>,
    pub originator: Option<String>,
    pub cli_version: Option<String>,
    pub source: Option<String>,
    pub model_provider: Option<String>,
}

/// Process a Codex session JSONL file.
pub fn process_codex_session(
    conn: &Connection,
    path: &Path,
    source_name: &str,
) -> Result<()> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open Codex session: {}", path.display()))?;
    let reader = std::io::BufReader::new(file);
    let mut session_id = String::new();
    let mut batch = Vec::new();
    let mut turn_index = 0;

    use std::io::BufRead;
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }

        let msg: CodexMessage = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("failed to parse Codex line in {}: {}", path.display(), e);
                continue;
            }
        };

        match msg.msg_type.as_str() {
            "session_meta" => {
                let meta: CodexSessionMeta = serde_json::from_value(msg.payload)?;
                session_id = meta.id.clone();
                
                let project_path = meta.cwd.as_deref().unwrap_or("unknown");
                let project_slug = project_path
                    .rsplit('/')
                    .next()
                    .unwrap_or("unknown")
                    .to_string();

                conn.execute(
                    "INSERT OR IGNORE INTO sessions (id, project_path, project_slug, created_at, modified_at, source_file, source_name, source_kind, app_version)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'codex', ?8)",
                    rusqlite::params![
                        session_id,
                        project_path,
                        project_slug,
                        meta.timestamp,
                        meta.timestamp,
                        path.to_string_lossy(),
                        source_name,
                        meta.cli_version,
                    ],
                )?;
            }
            "response_item" | "request_item" | "commentary" | "final" => {
                if session_id.is_empty() { continue; }

                let mut ops = LineOps::default();
                let msg_id = format!("{}-{}", session_id, turn_index);
                
                let mut fp_hasher = blake3::Hasher::new();
                fp_hasher.update(msg.msg_type.as_bytes());
                fp_hasher.update(msg.timestamp.as_bytes());

                let mut content_str = String::new();
                if let Some(content) = msg.payload.get("content") {
                    if let Some(arr) = content.as_array() {
                        for block in arr {
                            if let Some(text) = block.get("text") {
                                if let Some(s) = text.as_str() {
                                    content_str.push_str(s);
                                }
                            }
                        }
                    } else if let Some(s) = content.as_str() {
                        content_str = s.to_string();
                    }
                }

                fp_hasher.update(content_str.as_bytes());

                let role = if msg.msg_type == "response_item" || msg.msg_type == "final" { "assistant" } else { "user" };

                ops.message = Some(MessageRow {
                    id: msg_id.clone(),
                    session_id: session_id.clone(),
                    parent_id: None,
                    msg_type: role.to_string(),
                    timestamp: msg.timestamp.clone(),
                    model: None,
                    stop_reason: None,
                    cwd: None,
                    git_branch: None,
                    duration_ms: None,
                    turn_index: Some(turn_index),
                    source_name: Some(source_name.to_string()),
                    fingerprint: None,
                });

                if !content_str.is_empty() {
                    let hash = hash_content(&content_str);
                    ops.blobs.push((hash.clone(), content_str.clone(), content_str.len() as i64, "text".into()));
                    ops.blob_refs.push((hash.clone(), msg_id.clone(), "text".into()));
                    ops.content_blocks.push(ContentBlockRow {
                        message_id: msg_id.clone(),
                        block_index: 0,
                        block_type: "text".into(),
                        content_hash: Some(hash),
                        tool_name: None,
                        tool_use_id: None,
                        tool_input_hash: None,
                    });
                }

                if let Some(msg_ref) = &mut ops.message {
                    msg_ref.fingerprint = Some(fp_hasher.finalize().to_hex().to_string());
                }

                batch.push(ops);
                turn_index += 1;
            }
            _ => {}
        }
    }

    if !batch.is_empty() {
        db_ops::flush_batch(conn, &batch)?;
        db_ops::update_session_fingerprint(conn, &session_id)?;
    }

    Ok(())
}
