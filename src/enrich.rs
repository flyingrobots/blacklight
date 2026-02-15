use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Shared progress counters updated live during enrichment.
#[derive(Debug)]
pub struct EnrichProgress {
    pub total: AtomicUsize,
    pub done: AtomicUsize,
    pub failed: AtomicUsize,
}

impl Default for EnrichProgress {
    fn default() -> Self {
        Self {
            total: AtomicUsize::new(0),
            done: AtomicUsize::new(0),
            failed: AtomicUsize::new(0),
        }
    }
}

#[derive(Debug)]
pub struct EnrichConfig {
    pub db_path: PathBuf,
    pub limit: Option<usize>,
    pub concurrency: usize,
    pub force: bool,
    /// Base URL for the Ollama API.
    pub ollama_url: String,
    /// Ollama model name (empty = auto-detect).
    pub ollama_model: String,
    /// Google API key for Gemini (empty = not configured).
    pub google_api_key: String,
    /// Confidence threshold for auto-approving enrichments.
    pub auto_approve_threshold: f64,
    /// Backend preference: "auto", "ollama", "gemini", "claude-cli".
    pub preferred_backend: String,
    pub progress: Option<Arc<EnrichProgress>>,
    pub cancel_flag: Option<Arc<AtomicBool>>,
    pub log_lines: Option<Arc<Mutex<Vec<String>>>>,
}

/// Helper to push a log line to a shared buffer.
fn push_log(log: &Option<Arc<Mutex<Vec<String>>>>, msg: String) {
    if let Some(ref buf) = log {
        let mut lines = buf.lock().unwrap();
        lines.push(msg);
    }
}

#[derive(Clone, Debug, Default, Serialize)]
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
pub struct EnrichmentResult {
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<TagResult>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TagResult {
    pub tag: String,
    pub confidence: f64,
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
        // Find the highest byte index <= max that is a character boundary
        let mut index = max;
        while index > 0 && !s.is_char_boundary(index) {
            index -= 1;
        }
        format!("{}...", &s[..index])
    }
}

/// Auto-detect available models from a local Ollama instance.
async fn auto_detect_ollama(base_url: &str) -> Option<String> {
    tracing::info!("auto-detecting Ollama models at {base_url}...");
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(2000))
        .build() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("failed to build reqwest client: {e}");
                return None;
            }
        };

    let res = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::info!("Ollama not found or unreachable: {e}");
            return None;
        }
    };

    if !res.status().is_success() {
        tracing::warn!("Ollama returned status {}", res.status());
        return None;
    }

    let data: serde_json::Value = res.json().await.ok()?;
    let models = data["models"].as_array()?;
    tracing::debug!("found {} Ollama models", models.len());

    // Pick the first non-embedding model, preferring popular ones
    let preferred = ["mistral", "llama3", "llama2", "phi3", "gemma"];
    for p in preferred {
        if models.iter().any(|m| m["name"].as_str().unwrap_or("").starts_with(p)) {
            tracing::info!("auto-detected Ollama model: {}", p);
            return Some(p.to_string());
        }
    }

    // Fallback to any model that doesn't look like an embedding model
    let fallback = models.iter()
        .map(|m| m["name"].as_str().unwrap_or(""))
        .find(|name| !name.is_empty() && !name.contains("embed") && !name.contains("nomic"))
        .map(|s| s.to_string());

    if let Some(ref m) = fallback {
        tracing::info!("using fallback Ollama model: {}", m);
    }
    fallback
}

/// Call a local Ollama instance to generate enrichment data.
async fn call_ollama(digest: &str, model: &str, base_url: &str) -> Result<EnrichmentResult> {
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));

    let prompt = r#"You are analyzing a Claude Code session. Based on the session data below, generate a JSON object with:
- "title": A short, descriptive title for this session (under 80 chars). Be specific about what was done.
- "summary": A 1-3 sentence summary of what happened in this session.
- "tags": An array of objects with "tag" (kebab-case label) and "confidence" (0.0-1.0). Include 2-5 relevant tags.

Return ONLY valid JSON.

Session data:
"#;

    let payload = serde_json::json!({
        "model": model,
        "prompt": format!("{}{}", prompt, digest),
        "stream": false,
        "format": "json"
    });

    let client = reqwest::Client::new();
    let res = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .context("failed to reach Ollama. Is it running on your host?")?;

    if !res.status().is_success() {
        let status = res.status();
        let err_text = res.text().await.unwrap_or_default();
        anyhow::bail!("Ollama returned error {}: {}", status, err_text);
    }

    let response_data: serde_json::Value = res.json().await.context("failed to parse Ollama response")?;
    let text = response_data["response"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Ollama response missing 'response' field"))?;

    let result: EnrichmentResult = serde_json::from_str(text)
        .with_context(|| format!("failed to parse Ollama JSON: {}", text))?;

    Ok(result)
}

/// Call the Gemini API to generate enrichment data from a session digest.
async fn call_gemini(digest: &str, api_key: &str) -> Result<EnrichmentResult> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        api_key
    );

    let prompt = r#"You are analyzing a Claude Code session. Based on the session data below, generate a JSON object with:
- "title": A short, descriptive title for this session (under 80 chars). Be specific about what was done.
- "summary": A 1-3 sentence summary of what happened in this session.
- "tags": An array of objects with "tag" (kebab-case label) and "confidence" (0.0-1.0). Include 2-5 relevant tags like: bug-fix, feature, refactor, debugging, config, docs, testing, frontend, backend, devops, database, api, auth, performance, etc.

Return ONLY valid JSON, no markdown fences or extra text.

Session data:
"#;

    let payload = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": format!("{}{}", prompt, digest)
            }]
        }],
        "generationConfig": {
            "response_mime_type": "application/json"
        }
    });

    let client = reqwest::Client::new();
    let res = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .context("failed to send request to Gemini API")?;

    if !res.status().is_success() {
        let status = res.status();
        let err_text = res.text().await.unwrap_or_default();
        anyhow::bail!("Gemini API returned error {}: {}", status, err_text);
    }

    let response_data: serde_json::Value = res.json().await.context("failed to parse Gemini API response as JSON")?;
    
    // Extract text from Gemini response: candidates[0].content.parts[0].text
    let text = response_data["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Gemini response missing text part: {:?}", response_data))?;

    let result: EnrichmentResult = serde_json::from_str(text)
        .with_context(|| format!("failed to parse Gemini output as JSON: {}", text))?;

    Ok(result)
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
        .args(["-p", &full_prompt, "--output-format", "json", "--dangerously-skip-permissions"])
        .output()
        .await
        .context("failed to spawn claude CLI — is it installed and on PATH?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("claude CLI exited with {}: {stderr}", output.status);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if stdout.trim().is_empty() {
        anyhow::bail!("claude CLI returned empty stdout. stderr: {}", stderr);
    }

    // --output-format json produces JSONL: one JSON object per line.
    // Lines include {"type":"system",...}, {"type":"assistant",...}, {"type":"result",...}.
    // We need the "result" line and its "result" field which contains the actual text.
    let text = parse_claude_jsonl_output(&stdout)?;

    // Try to extract JSON from the text (Claude sometimes wraps in markdown fences)
    let json_str = extract_json(&text);

    let result: EnrichmentResult = serde_json::from_str(json_str)
        .with_context(|| format!("failed to parse claude output as JSON: {json_str}. Full stdout: {stdout}"))?;

    Ok(result)
}

/// Parse JSONL output from `claude --output-format json`.
/// Finds the `{"type":"result",...}` line and extracts its `result` field.
fn parse_claude_jsonl_output(stdout: &str) -> Result<String> {
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            if val.get("type").and_then(|t| t.as_str()) == Some("result") {
                if let Some(result) = val.get("result").and_then(|r| r.as_str()) {
                    return Ok(result.to_string());
                }
            }
        }
    }

    // Fallback: maybe it's a single JSON object (older CLI versions)
    if let Ok(wrapper) = serde_json::from_str::<serde_json::Value>(stdout) {
        if let Some(result) = wrapper.get("result").and_then(|r| r.as_str()) {
            return Ok(result.to_string());
        }
    }

    // Last resort: treat entire stdout as the response text
    Ok(stdout.to_string())
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
fn store_enrichment_internal(
    conn: &Connection,
    session_id: &str,
    result: &EnrichmentResult,
    model_used: &str,
    auto_approve_threshold: f64,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();

    let all_high_confidence = !result.tags.is_empty()
        && result.tags.iter().all(|t| t.confidence >= auto_approve_threshold);

    let (approval_status, reviewed_at) = if all_high_confidence {
        ("approved", Some(now.as_str()))
    } else {
        ("pending_review", None)
    };

    conn.execute(
        "INSERT OR REPLACE INTO session_enrichments
         (session_id, title, summary, enriched_at, model_used, approval_status, reviewed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            session_id,
            result.title,
            result.summary,
            now,
            model_used,
            approval_status,
            reviewed_at,
        ],
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

/// Count enrichments pending human review.
pub fn pending_review_count(conn: &Connection) -> Result<i64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM session_enrichments WHERE approval_status = 'pending_review'",
        [],
        |row| row.get(0),
    )?;
    Ok(count)
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

    // Resolve enrichment backend settings.
    // Env vars override config values for secrets.
    let google_key = std::env::var("GOOGLE_API_KEY").ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            if config.google_api_key.is_empty() { None } else { Some(config.google_api_key.clone()) }
        });
    let mut ollama_model = std::env::var("OLLAMA_MODEL").ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            if config.ollama_model.is_empty() { None } else { Some(config.ollama_model.clone()) }
        });

    let ollama_url = config.ollama_url.clone();
    let auto_approve_threshold = config.auto_approve_threshold;
    let preferred_backend = config.preferred_backend.clone();

    // Resolve which backend to use based on preferred_backend
    match preferred_backend.as_str() {
        "ollama" => {
            if ollama_model.is_none() {
                ollama_model = auto_detect_ollama(&ollama_url).await;
            }
        }
        "gemini" => {
            // Force gemini — ollama_model stays None
            ollama_model = None;
        }
        "claude-cli" => {
            // Force claude-cli — clear both
            ollama_model = None;
            // google_key is already set but won't be used since we check ollama first
        }
        _ => {
            // "auto" — try ollama auto-detect if not explicitly configured
            if ollama_model.is_none() {
                ollama_model = auto_detect_ollama(&ollama_url).await;
            }
        }
    }

    if let Some(ref model) = ollama_model {
        tracing::info!("using local Ollama ({}) for enrichment", model);
    } else if preferred_backend == "claude-cli" {
        tracing::info!("using Claude CLI for enrichment");
    } else if google_key.is_some() {
        tracing::info!("using Gemini 2.0 Flash for enrichment");
    } else {
        tracing::info!("using Claude CLI for enrichment");
    }

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
    let enriched = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);

    // Update shared progress with total (so UI can show it immediately)
    if let Some(ref p) = config.progress {
        p.total.store(total_work, Ordering::Relaxed);
        p.done.store(0, Ordering::Relaxed);
        p.failed.store(0, Ordering::Relaxed);
    }

    let progress_ref = &config.progress;
    let cancel_ref = &config.cancel_flag;
    let log_ref = &config.log_lines;
    let ollama_url_ref = &ollama_url;
    let threshold_ref = &auto_approve_threshold;

    // Process concurrently
    stream::iter(work.into_iter().enumerate())
        .map(|(_, (session_id, digest))| {
            let conn_ref = &conn;
            let enriched_ref = &enriched;
            let failed_ref = &failed;
            let google_key_inner = google_key.clone();
            let ollama_model_inner = ollama_model.clone();
            let preferred_backend_inner = preferred_backend.clone();
            async move {
                // Check cancel flag before starting each item
                if let Some(flag) = cancel_ref {
                    if flag.load(Ordering::Relaxed) {
                        return;
                    }
                }

                let (result, model_name) = if preferred_backend_inner == "claude-cli" {
                    (call_claude(&digest).await, "claude-cli".to_string())
                } else if let Some(ref model) = ollama_model_inner {
                    (call_ollama(&digest, model, ollama_url_ref).await, model.clone())
                } else if let Some(ref key) = google_key_inner {
                    (call_gemini(&digest, key).await, "gemini-2.0-flash".to_string())
                } else {
                    (call_claude(&digest).await, "claude-cli".to_string())
                };

                match result {
                    Ok(res) => {
                        // Store under lock
                        let db = conn_ref.lock().unwrap();
                        if let Err(e) = store_enrichment_internal(&db, &session_id, &res, &model_name, *threshold_ref) {
                            let msg = format!("session {session_id}: DB store failed: {e:#}");
                            tracing::error!("{msg}");
                            push_log(log_ref, msg);
                            failed_ref.fetch_add(1, Ordering::Relaxed);
                            if let Some(ref p) = progress_ref {
                                p.failed.fetch_add(1, Ordering::Relaxed);
                            }
                            return;
                        }
                        drop(db);

                        let tags_str: String = res
                            .tags
                            .iter()
                            .map(|t| format!("{} {:.2}", t.tag, t.confidence))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let n = enriched_ref.fetch_add(1, Ordering::Relaxed) + 1;
                        if let Some(ref p) = progress_ref {
                            p.done.fetch_add(1, Ordering::Relaxed);
                        }
                        let msg = format!(
                            "[{n}/{total_work}] Enriched ({}): \"{}\" ({tags_str})",
                            model_name, res.title
                        );
                        eprintln!("{msg}");
                        push_log(log_ref, msg);
                    }
                    Err(e) => {
                        failed_ref.fetch_add(1, Ordering::Relaxed);
                        if let Some(ref p) = progress_ref {
                            p.failed.fetch_add(1, Ordering::Relaxed);
                        }
                        let n_failed = failed_ref.load(Ordering::Relaxed);
                        let n_done = enriched_ref.load(Ordering::Relaxed);
                        let msg = format!("[{}/{total_work}] Failed: {session_id} — {e:#}", n_done + n_failed);
                        tracing::error!("session {session_id}: enrichment failed: {e:#}");
                        eprintln!("{msg}");
                        push_log(log_ref, msg);
                    }
                }
            }
        })
        .buffer_unordered(config.concurrency)
        .collect::<Vec<()>>()
        .await;

    Ok(EnrichReport {
        enriched: enriched.load(Ordering::Relaxed),
        skipped,
        failed: failed.load(Ordering::Relaxed),
        total_candidates,
    })
}

// Public wrappers for use by the single-session enrichment API endpoint.
pub fn build_session_digest_pub(conn: &Connection, session_id: &str) -> Result<Option<String>> {
    build_session_digest(conn, session_id)
}

pub async fn call_model_pub(digest: &str, ollama_url: &str) -> Result<(EnrichmentResult, String)> {
    if let Some(model) = std::env::var("OLLAMA_MODEL").ok().filter(|s| !s.trim().is_empty()) {
        let res = call_ollama(digest, &model, ollama_url).await?;
        Ok((res, model))
    } else if let Some(model) = auto_detect_ollama(ollama_url).await {
        let res = call_ollama(digest, &model, ollama_url).await?;
        Ok((res, model))
    } else if let Some(key) = std::env::var("GOOGLE_API_KEY").ok().filter(|s| !s.trim().is_empty()) {
        let res = call_gemini(digest, &key).await?;
        Ok((res, "gemini-2.0-flash".to_string()))
    } else {
        let res = call_claude(digest).await?;
        Ok((res, "claude-cli".to_string()))
    }
}

pub fn store_enrichment_pub(
    conn: &Connection,
    session_id: &str,
    result: &EnrichmentResult,
    model_name: &str,
    auto_approve_threshold: f64,
) -> Result<()> {
    store_enrichment_internal(conn, session_id, result, model_name, auto_approve_threshold)
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
