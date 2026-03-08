use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::indexer::db_ops::{self, LineOps, MessageRow, ContentBlockRow, ToolCallRow};
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

#[derive(Debug, Deserialize, Serialize)]
pub struct CodexToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CodexToolResponse {
    #[serde(rename = "toolUseId")]
    pub tool_use_id: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CodexTaskUpdate {
    pub id: String,
    pub subject: String,
    pub description: String,
    pub status: String,
    #[serde(rename = "activeForm")]
    pub active_form: Option<String>,
    #[serde(rename = "blockedBy", default)]
    pub blocked_by: Vec<String>,
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
    let mut last_assistant_msg_id: Option<String> = None;
    let mut last_user_msg_id: Option<String> = None;
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
                    "INSERT OR IGNORE INTO sessions (id, project_path, project_slug, created_at, modified_at, source_file, source_name, source_kind, app_version, index_version)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'codex', ?8, ?9)",
                    rusqlite::params![
                        session_id,
                        project_path,
                        project_slug,
                        meta.timestamp,
                        meta.timestamp,
                        path.to_string_lossy(),
                        source_name,
                        meta.cli_version,
                        crate::INDEX_VERSION,
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

                let role = if msg.msg_type == "response_item" || msg.msg_type == "final" { 
                    last_assistant_msg_id = Some(msg_id.clone());
                    "assistant" 
                } else { 
                    last_user_msg_id = Some(msg_id.clone());
                    "user" 
                };

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
                    ops.fts_entries.push((hash.clone(), "text".into(), content_str));
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

                db_ops::flush_batch(conn, &[ops])?;
                turn_index += 1;
            }
            "tool_call" => {
                if session_id.is_empty() { continue; }
                let call: CodexToolCall = serde_json::from_value(msg.payload)?;
                let parent_id = last_assistant_msg_id.as_ref().cloned().unwrap_or_else(|| format!("{}-init", session_id));

                let input_json = serde_json::to_string(&call.input)?;
                let input_hash = hash_content(&input_json);

                let mut ops = LineOps::default();
                ops.blobs.push((input_hash.clone(), input_json, 0, "tool_input".into()));
                ops.tool_calls.push(ToolCallRow {
                    id: call.id.clone(),
                    message_id: parent_id.clone(),
                    session_id: session_id.clone(),
                    tool_name: call.name.clone(),
                    input_hash: Some(input_hash.clone()),
                    timestamp: msg.timestamp.clone(),
                    fingerprint: None,
                });
                ops.content_blocks.push(ContentBlockRow {
                    message_id: parent_id,
                    block_index: 1000 + (call.id.as_bytes()[0] as i64), // arbitrary stable index
                    block_type: "tool_use".into(),
                    content_hash: None,
                    tool_name: Some(call.name),
                    tool_use_id: Some(call.id),
                    tool_input_hash: Some(input_hash),
                });
                db_ops::flush_batch(conn, &[ops])?;
            }
            "tool_response" => {
                if session_id.is_empty() { continue; }
                let resp: CodexToolResponse = serde_json::from_value(msg.payload)?;
                let parent_id = last_user_msg_id.as_ref().cloned().unwrap_or_else(|| format!("{}-init", session_id));

                let content_json = serde_json::to_string(&resp.content)?;
                let content_hash = hash_content(&content_json);

                let mut ops = LineOps::default();
                ops.blobs.push((content_hash.clone(), content_json, 0, "tool_output".into()));
                ops.tool_output_links.push((resp.tool_use_id.clone(), content_hash.clone()));
                ops.content_blocks.push(ContentBlockRow {
                    message_id: parent_id,
                    block_index: 2000 + (resp.tool_use_id.as_bytes()[0] as i64),
                    block_type: "tool_result".into(),
                    content_hash: Some(content_hash),
                    tool_name: None,
                    tool_use_id: Some(resp.tool_use_id),
                    tool_input_hash: None,
                });
                db_ops::flush_batch(conn, &[ops])?;
            }
            "plan_update" => {
                if session_id.is_empty() { continue; }
                if let Some(plan) = msg.payload.get("plan") {
                    let plan_str = if plan.is_string() { 
                        plan.as_str().unwrap().to_string() 
                    } else { 
                        serde_json::to_string_pretty(plan)? 
                    };
                    
                    let hash = hash_content(&plan_str);
                    let mut ops = LineOps::default();
                    ops.blobs.push((hash.clone(), plan_str.clone(), plan_str.len() as i64, "plan".into()));
                    ops.fts_entries.push((hash.clone(), "plan".into(), plan_str));
                    db_ops::flush_batch(conn, &[ops])?;
                }
            }
            "task_update" => {
                if session_id.is_empty() { continue; }
                let task: CodexTaskUpdate = serde_json::from_value(msg.payload)?;
                db_ops::record_task(
                    conn,
                    &task.id,
                    &session_id,
                    &task.subject,
                    &task.description,
                    task.active_form.as_deref(),
                    &task.status,
                )?;
                for dep in &task.blocked_by {
                    db_ops::record_task_dependency(conn, &session_id, &task.id, dep)?;
                }
            }
            _ => {}
        }
    }

    if !session_id.is_empty() {
        db_ops::update_session_fingerprint(conn, &session_id)?;
    }

    Ok(())
}
