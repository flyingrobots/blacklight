use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug)]
pub struct EnrichConfig {
    pub db_path: PathBuf,
    pub limit: Option<usize>,
    pub concurrency: usize,
    pub force: bool,
}

#[derive(Debug, Default)]
pub struct EnrichReport {
    pub enriched: usize,
    pub skipped: usize,
    pub failed: usize,
    pub total_candidates: usize,
}

impl fmt::Display for EnrichReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Enrichment complete:")?;
        writeln!(f, "  enriched:   {}", self.enriched)?;
        writeln!(f, "  skipped:    {}", self.skipped)?;
        writeln!(f, "  failed:     {}", self.failed)?;
        writeln!(f, "  candidates: {}", self.total_candidates)
    }
}

#[derive(Debug, Deserialize)]
struct EnrichmentResult {
    title: String,
    summary: String,
    #[serde(default)]
    tags: Vec<TagResult>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TagResult {
    tag: String,
    confidence: f64,
}

/// Build a text digest for a session to send to Claude for enrichment.
fn build_session_digest(conn: &Connection, session_id: &str) -> Result<Option<String>> {
    // Get session metadata
    let mut stmt = conn.prepare(
        "SELECT s.project_slug, s.git_branch, s.created_at, s.modified_at,
                s.summary, s.first_prompt
         FROM sessions s WHERE s.id = ?1",
    )?;

    struct SessionMeta {
        project_slug: String,
        git_branch: Option<String>,
        created_at: String,
        modified_at: String,
        summary: Option<String>,
        first_prompt: Option<String>,
    }

    let meta: Option<SessionMeta> = stmt
        .query_row(params![session_id], |row| {
            Ok(SessionMeta {
                project_slug: row.get(0)?,
                git_branch: row.get(1)?,
                created_at: row.get(2)?,
                modified_at: row.get(3)?,
                summary: row.get(4)?,
                first_prompt: row.get(5)?,
            })
        })
        .optional()?;

    let meta = match meta {
        Some(m) => m,
        None => return Ok(None),
    };

    let mut digest = String::with_capacity(8000);
    digest.push_str(&format!("Project: {}\n", meta.project_slug));
    if let Some(ref branch) = meta.git_branch {
        digest.push_str(&format!("Branch: {branch}\n"));
    }
    digest.push_str(&format!("Created: {}\n", meta.created_at));
    digest.push_str(&format!("Modified: {}\n", meta.modified_at));

    if let Some(ref s) = meta.summary {
        digest.push_str(&format!("Existing summary: {s}\n"));
    }
    if let Some(ref fp) = meta.first_prompt {
        let truncated = truncate_str(fp, 500);
        digest.push_str(&format!("First prompt: {truncated}\n"));
    }
    digest.push('\n');

    // Get first 20 messages with content
    let mut msg_stmt = conn.prepare(
        "SELECT m.type, cs.content
         FROM messages m
         JOIN content_blocks cb ON cb.message_id = m.id
         LEFT JOIN content_store cs ON cs.hash = cb.content_hash
         WHERE m.session_id = ?1
         ORDER BY m.timestamp ASC, cb.block_index ASC
         LIMIT 40",
    )?;

    let messages: Vec<(String, Option<String>)> = msg_stmt
        .query_map(params![session_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut msg_count = 0;
    for (msg_type, content) in &messages {
        if digest.len() > 7500 {
            break;
        }
        if msg_count >= 20 {
            break;
        }
        let text = match content {
            Some(c) => truncate_str(c, 500),
            None => continue,
        };
        digest.push_str(&format!("[{msg_type}] {text}\n"));
        msg_count += 1;
    }

    // Get tool names used
    let mut tool_stmt = conn.prepare(
        "SELECT DISTINCT tool_name FROM tool_calls WHERE session_id = ?1 ORDER BY tool_name",
    )?;
    let tools: Vec<String> = tool_stmt
        .query_map(params![session_id], |row| row.get(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    if !tools.is_empty() {
        digest.push_str(&format!("\nTools used: {}\n", tools.join(", ")));
    }

    Ok(Some(digest))
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..s.floor_char_boundary(max)])
    }
}

/// Call the claude CLI to generate enrichment data from a session digest.
async fn call_claude(digest: &str) -> Result<EnrichmentResult> {
    let prompt = r#"You are analyzing a Claude Code session. Based on the session data below, generate a JSON object with:
- "title": A short, descriptive title for this session (under 80 chars). Be specific about what was done.
- "summary": A 1-3 sentence summary of what happened in this session.
- "tags": An array of objects with "tag" (kebab-case label) and "confidence" (0.0-1.0). Include 2-5 relevant tags like: bug-fix, feature, refactor, debugging, config, docs, testing, frontend, backend, devops, database, api, auth, performance, etc.

Return ONLY valid JSON, no markdown fences or extra text.

Session data:
"#;

    let full_prompt = format!("{prompt}{digest}");

    let output = tokio::process::Command::new("claude")
        .args(["-p", &full_prompt, "--output-format", "json"])
        .output()
        .await
        .context("failed to spawn claude CLI — is it installed and on PATH?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("claude CLI exited with {}: {stderr}", output.status);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // With --output-format json, claude returns a JSON object with a "result" field
    // Try parsing that first, then fall back to raw parsing
    let text = if let Ok(wrapper) = serde_json::from_str::<serde_json::Value>(&stdout) {
        if let Some(result) = wrapper.get("result").and_then(|r| r.as_str()) {
            result.to_string()
        } else {
            stdout.to_string()
        }
    } else {
        stdout.to_string()
    };

    // Try to extract JSON from the text (Claude sometimes wraps in markdown fences)
    let json_str = extract_json(&text);

    let result: EnrichmentResult = serde_json::from_str(json_str)
        .with_context(|| format!("failed to parse claude output as JSON: {json_str}"))?;

    Ok(result)
}

/// Extract a JSON object from text that may contain markdown fences.
fn extract_json(text: &str) -> &str {
    let trimmed = text.trim();
    // Strip markdown code fences
    if let Some(rest) = trimmed.strip_prefix("```json") {
        if let Some(inner) = rest.strip_suffix("```") {
            return inner.trim();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("```") {
        if let Some(inner) = rest.strip_suffix("```") {
            return inner.trim();
        }
    }
    // Find the first { and last }
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return &trimmed[start..=end];
        }
    }
    trimmed
}

/// Store enrichment results into the database.
fn store_enrichment(
    conn: &Connection,
    session_id: &str,
    result: &EnrichmentResult,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT OR REPLACE INTO session_enrichments (session_id, title, summary, enriched_at, model_used)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![session_id, result.title, result.summary, now, "claude-cli"],
    )?;

    // Delete existing tags for this session (in case of re-enrichment)
    conn.execute(
        "DELETE FROM session_tags WHERE session_id = ?1",
        params![session_id],
    )?;

    let mut tag_stmt = conn.prepare(
        "INSERT INTO session_tags (session_id, tag, confidence) VALUES (?1, ?2, ?3)",
    )?;
    for tag in &result.tags {
        tag_stmt.execute(params![session_id, tag.tag, tag.confidence])?;
    }

    Ok(())
}

/// Run the enrichment pipeline.
pub async fn run_enrich(config: EnrichConfig) -> Result<EnrichReport> {
    let conn = crate::db::open(&config.db_path)?;

    // Find un-enriched sessions (or all sessions if --force)
    let mut session_ids: Vec<String> = {
        let query = if config.force {
            "SELECT id FROM sessions ORDER BY modified_at DESC"
        } else {
            "SELECT s.id FROM sessions s
             LEFT JOIN session_enrichments e ON e.session_id = s.id
             WHERE e.session_id IS NULL
             ORDER BY s.modified_at DESC"
        };
        let mut stmt = conn.prepare(query)?;
        let rows = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows
    };

    let total_candidates = session_ids.len();

    if let Some(limit) = config.limit {
        session_ids.truncate(limit);
    }

    if session_ids.is_empty() {
        tracing::info!("no sessions to enrich");
        return Ok(EnrichReport {
            total_candidates,
            ..Default::default()
        });
    }

    tracing::info!(
        "enriching {} of {} candidate sessions (concurrency={})",
        session_ids.len(),
        total_candidates,
        config.concurrency
    );

    // Build digests synchronously (DB access)
    let mut work: Vec<(String, String)> = Vec::new();
    let mut skipped = 0;
    for sid in &session_ids {
        match build_session_digest(&conn, sid)? {
            Some(digest) => work.push((sid.clone(), digest)),
            None => {
                tracing::warn!("session {sid}: no data, skipping");
                skipped += 1;
            }
        }
    }

    let total_work = work.len();
    let conn = Mutex::new(conn);
    let enriched = std::sync::atomic::AtomicUsize::new(0);
    let failed = std::sync::atomic::AtomicUsize::new(0);

    // Process concurrently
    stream::iter(work.into_iter().enumerate())
        .map(|(i, (session_id, digest))| {
            let conn_ref = &conn;
            let enriched_ref = &enriched;
            let failed_ref = &failed;
            async move {
                match call_claude(&digest).await {
                    Ok(result) => {
                        // Store under lock
                        let db = conn_ref.lock().unwrap();
                        if let Err(e) = store_enrichment(&db, &session_id, &result) {
                            tracing::error!("session {session_id}: DB store failed: {e:#}");
                            failed_ref.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            return;
                        }
                        drop(db);

                        let tags_str: String = result
                            .tags
                            .iter()
                            .map(|t| format!("{} {:.2}", t.tag, t.confidence))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let n = enriched_ref.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                        eprintln!(
                            "[{n}/{total_work}] Enriched: \"{}\" ({tags_str})",
                            result.title
                        );
                    }
                    Err(e) => {
                        tracing::error!("session {session_id}: enrichment failed: {e:#}");
                        let n = failed_ref.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                        eprintln!("[{}/{total_work}] Failed: {session_id} — {e:#}", i + 1 - n + 1);
                    }
                }
            }
        })
        .buffer_unordered(config.concurrency)
        .collect::<Vec<()>>()
        .await;

    Ok(EnrichReport {
        enriched: enriched.load(std::sync::atomic::Ordering::Relaxed),
        skipped,
        failed: failed.load(std::sync::atomic::Ordering::Relaxed),
        total_candidates,
    })
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
