use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::indexer::db_ops::{self, LineOps, MessageRow, ContentBlockRow, ToolCallRow};
use crate::content::hash_content;
use crate::server::state::{DbPool, MigrationState};
use crate::config::BackupMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct GeminiSession {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "projectHash")]
    pub project_hash: String,
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
    pub messages: Vec<GeminiMessage>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeminiMessage {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub content: GeminiContent,
    #[serde(rename = "toolCalls")]
    pub tool_calls: Option<Vec<GeminiToolCall>>,
    pub thoughts: Option<Vec<GeminiThought>>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GeminiContent {
    Text(String),
    Blocks(Vec<serde_json::Value>),
}

impl GeminiContent {
    pub fn as_text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Blocks(blocks) => {
                let mut full_text = String::new();
                for block in blocks {
                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                        full_text.push_str(text);
                    }
                }
                full_text
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Text(s) => s.is_empty(),
            Self::Blocks(blocks) => blocks.is_empty(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Text(s) => s.len(),
            Self::Blocks(_) => self.as_text().len(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeminiToolCall {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeminiThought {
    pub subject: String,
    pub description: String,
    pub timestamp: String,
}

/// Process a Gemini session JSON file.
pub fn process_gemini_session(
    conn: &Connection,
    path: &Path,
    source_name: &str,
) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read Gemini session: {}", path.display()))?;
    
    let session: GeminiSession = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse Gemini session: {}", path.display()))?;

    // 1. Ensure session exists in DB
    conn.execute(
        "INSERT OR IGNORE INTO sessions (id, project_path, project_slug, created_at, modified_at, source_file, source_name, source_kind, index_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'gemini', ?8)",
        rusqlite::params![
            session.session_id,
            session.project_hash,
            &session.project_hash[0..8], // slug
            session.start_time,
            session.last_updated,
            path.to_string_lossy(),
            source_name,
            crate::INDEX_VERSION,
        ],
    )?;

    let mut batch = Vec::new();

    for (idx, msg) in session.messages.into_iter().enumerate() {
        let mut ops = LineOps::default();
        
        // Calculate message fingerprint
        let mut fp_hasher = blake3::Hasher::new();
        fp_hasher.update(msg.msg_type.as_bytes());
        fp_hasher.update(msg.timestamp.as_bytes());
        let content_text = msg.content.as_text();
        fp_hasher.update(content_text.as_bytes());

        let msg_row = MessageRow {
            id: msg.id.clone(),
            session_id: session.session_id.clone(),
            parent_id: None,
            msg_type: msg.msg_type.clone(),
            timestamp: msg.timestamp.clone(),
            model: msg.model.clone(),
            stop_reason: None,
            cwd: None,
            git_branch: None,
            duration_ms: None,
            turn_index: Some(idx as i32),
            source_name: Some(source_name.to_string()),
            fingerprint: None,
        };

        // Text content
        if !msg.content.is_empty() {
            let hash = hash_content(&content_text);
            let size = content_text.len() as i64;
            ops.blobs.push((hash.clone(), content_text.clone(), size, "text".into()));
            ops.blob_refs.push((hash.clone(), msg.id.clone(), "response_text".into()));
            ops.fts_entries.push((hash.clone(), "text".into(), content_text));
            ops.content_blocks.push(ContentBlockRow {
                message_id: msg.id.clone(),
                block_index: 0,
                block_type: "text".into(),
                content_hash: Some(hash),
                tool_name: None,
                tool_use_id: None,
                tool_input_hash: None,
            });
        }

        // Thoughts
        if let Some(thoughts) = msg.thoughts {
            for (t_idx, thought) in thoughts.into_iter().enumerate() {
                let thought_json = serde_json::to_string(&thought)?;
                let hash = hash_content(&thought_json);
                fp_hasher.update(hash.as_bytes());
                
                ops.blobs.push((hash.clone(), thought_json, thought.description.len() as i64, "thinking".into()));
                ops.blob_refs.push((hash.clone(), msg.id.clone(), "thinking".into()));
                ops.content_blocks.push(ContentBlockRow {
                    message_id: msg.id.clone(),
                    block_index: (t_idx + 1) as i64,
                    block_type: "thinking".into(),
                    content_hash: Some(hash),
                    tool_name: None,
                    tool_use_id: None,
                    tool_input_hash: None,
                });
            }
        }

        // Tool calls
        if let Some(tool_calls) = msg.tool_calls {
            for (tc_idx, tc) in tool_calls.into_iter().enumerate() {
                let input_json = serde_json::to_string(&tc.args)?;
                let input_hash = hash_content(&input_json);
                
                let output_hash = if let Some(res) = tc.result {
                    let out_json = serde_json::to_string(&res)?;
                    let h = hash_content(&out_json);
                    ops.blobs.push((h.clone(), out_json, 0, "tool_output".into()));
                    Some(h)
                } else {
                    None
                };

                // Tool call fingerprint
                let mut tc_fp = blake3::Hasher::new();
                tc_fp.update(tc.name.as_bytes());
                tc_fp.update(input_hash.as_bytes());
                if let Some(h) = &output_hash { tc_fp.update(h.as_bytes()); }
                let tc_fingerprint = tc_fp.finalize().to_hex().to_string();
                fp_hasher.update(tc_fingerprint.as_bytes());

                ops.tool_calls.push(ToolCallRow {
                    id: tc.id.clone(),
                    message_id: msg.id.clone(),
                    session_id: session.session_id.clone(),
                    tool_name: tc.name.clone(),
                    input_hash: Some(input_hash.clone()),
                    timestamp: msg.timestamp.clone(),
                    fingerprint: Some(tc_fingerprint),
                });

                if let Some(h) = output_hash {
                    ops.tool_output_links.push((tc.id.clone(), h));
                }

                ops.blobs.push((input_hash.clone(), input_json, 0, "tool_input".into()));
                ops.content_blocks.push(ContentBlockRow {
                    message_id: msg.id.clone(),
                    block_index: (1000 + tc_idx) as i64,
                    block_type: "tool_use".into(),
                    content_hash: None,
                    tool_name: Some(tc.name),
                    tool_use_id: Some(tc.id),
                    tool_input_hash: Some(input_hash),
                });
            }
        }

        let mut final_msg = msg_row;
        final_msg.fingerprint = Some(fp_hasher.finalize().to_hex().to_string());
        ops.message = Some(final_msg);
        batch.push(ops);
    }

    db_ops::flush_batch(conn, &batch)?;
    db_ops::update_session_fingerprint(conn, &session.session_id)?;

    Ok(())
}

pub async fn run_v4_migration(
    db: Arc<DbPool>,
    state: Arc<tokio::sync::Mutex<MigrationState>>,
    backup_dir: PathBuf,
    backup_mode: BackupMode,
) -> Result<()> {
    // 1. Get all sessions that need backup or fingerprinting
    let sessions = db.call(|conn| {
        let mut stmt = conn.prepare("SELECT id, source_file, source_kind FROM sessions")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }).await?;

    {
        let guard = state.lock().await;
        let mut p = guard.progress.lock().unwrap();
        p.total_sessions = sessions.len();
    }

    // 2. Iterate and process
    for (id, source_file, kind) in sessions {
        let path = PathBuf::from(&source_file);
        
        // Backup if it exists and we haven't already
        if path.exists() {
            let db_inner = db.clone();
            let id_inner = id.clone();
            let path_inner = path.clone();
            let backup_dir_inner = backup_dir.clone();
            
            db_inner.call(move |conn| {
                // Check if already backed up
                let exists: bool = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM session_backups WHERE session_id = ?1)",
                    rusqlite::params![id_inner],
                    |row| row.get(0)
                )?;

                if !exists {
                    if kind.as_deref() == Some("gemini") || kind.as_deref() == Some("codex") || source_file.contains(".gemini") || source_file.contains(".codex") {
                         let source_name = kind.as_deref().unwrap_or("unknown");
                         crate::indexer::backup_source_file(conn, &path_inner, &backup_dir_inner, backup_mode, source_name)?;
                    }
                }
                Ok(())
            }).await?;

            let guard = state.lock().await;
            let mut p = guard.progress.lock().unwrap();
            p.backed_up += 1;
        }

        // Always update fingerprint (Merkle root)
        let db_inner = db.clone();
        let id_inner = id.clone();
        db_inner.call(move |conn| {
            db_ops::update_session_fingerprint(conn, &id_inner)?;
            Ok(())
        }).await?;

        {
            let guard = state.lock().await;
            let mut p = guard.progress.lock().unwrap();
            p.fingerprints_updated += 1;
        }
    }

    Ok(())
}
