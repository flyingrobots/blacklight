use anyhow::Result;
use futures::stream::{self, StreamExt};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::enrich::{self, EnrichProgress, push_log};

#[derive(Debug)]
pub struct ClassifierConfig {
    pub db_path: PathBuf,
    pub limit: Option<usize>,
    pub concurrency: usize,
    pub force: bool,
    pub ollama_url: String,
    pub ollama_model: String,
    pub google_api_key: String,
    pub preferred_backend: String,
    pub progress: Option<Arc<EnrichProgress>>,
    pub cancel_flag: Option<Arc<AtomicBool>>,
    pub log_lines: Option<Arc<Mutex<Vec<String>>>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct ClassifierReport {
    pub classified: usize,
    pub skipped: usize,
    pub failed: usize,
    pub total_candidates: usize,
}

#[derive(Debug, Deserialize)]
pub struct ClassificationResult {
    pub outcome: String,
    pub reason_code: Option<String>,
}

pub async fn run_classify(config: ClassifierConfig) -> Result<ClassifierReport> {
    let conn = crate::db::open(&config.db_path)?;

    // Find sessions without outcome OR not user-labeled (or all if --force)
    let mut session_ids: Vec<String> = {
        let query = if config.force {
            "SELECT id FROM sessions ORDER BY modified_at DESC"
        } else {
            "SELECT s.id FROM sessions s
             LEFT JOIN session_outcomes o ON o.session_id = s.id
             WHERE o.session_id IS NULL OR o.is_user_labeled = 0
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
        return Ok(ClassifierReport {
            total_candidates,
            ..Default::default()
        });
    }

    // Resolve backend settings (reuse enrich logic)
    let google_key = std::env::var("GOOGLE_API_KEY").ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| if config.google_api_key.is_empty() { None } else { Some(config.google_api_key.clone()) });
    
    let mut ollama_model = std::env::var("OLLAMA_MODEL").ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| if config.ollama_model.is_empty() { None } else { Some(config.ollama_model.clone()) });

    if ollama_model.is_none() && (config.preferred_backend == "ollama" || config.preferred_backend == "auto") {
        ollama_model = enrich::auto_detect_ollama_pub(&config.ollama_url).await;
    }

    // Build digests
    let mut work: Vec<(String, String)> = Vec::new();
    let mut skipped = 0;
    for sid in &session_ids {
        match enrich::build_session_digest_pub(&conn, sid)? {
            Some(digest) => work.push((sid.clone(), digest)),
            None => skipped += 1,
        }
    }

    let total_work = work.len();
    let conn = Mutex::new(conn);
    let classified = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);

    if let Some(ref p) = config.progress {
        p.total.store(total_work, Ordering::Relaxed);
        p.done.store(0, Ordering::Relaxed);
        p.failed.store(0, Ordering::Relaxed);
    }

    let progress_ref = &config.progress;
    let cancel_ref = &config.cancel_flag;
    let log_ref = &config.log_lines;
    let ollama_url_ref = &config.ollama_url;

    stream::iter(work.into_iter().enumerate())
        .map(|(_, (session_id, digest))| {
            let conn_ref = &conn;
            let classified_ref = &classified;
            let failed_ref = &failed;
            let google_key_inner = google_key.clone();
            let ollama_model_inner = ollama_model.clone();
            let preferred_backend_inner = config.preferred_backend.clone();
            async move {
                if let Some(flag) = cancel_ref {
                    if flag.load(Ordering::Relaxed) { return; }
                }

                let result = if preferred_backend_inner == "claude-cli" {
                    call_classifier_claude(&digest).await
                } else if let Some(ref model) = ollama_model_inner {
                    call_classifier_ollama(&digest, model, ollama_url_ref).await
                } else if let Some(ref key) = google_key_inner {
                    call_classifier_gemini(&digest, key).await
                } else {
                    call_classifier_claude(&digest).await
                };

                match result {
                    Ok(res) => {
                        let db = conn_ref.lock().unwrap();
                        let store_res = db.execute(
                            "INSERT OR REPLACE INTO session_outcomes (session_id, outcome, reason_code, is_user_labeled)
                             VALUES (?1, ?2, ?3, 0)",
                            params![session_id, res.outcome, res.reason_code],
                        );
                        if let Err(e) = store_res {
                            let msg = format!("session {session_id}: DB store failed: {e:#}");
                            push_log(log_ref, msg);
                            failed_ref.fetch_add(1, Ordering::Relaxed);
                            if let Some(ref p) = progress_ref { p.failed.fetch_add(1, Ordering::Relaxed); }
                            return;
                        }
                        drop(db);

                        let n = classified_ref.fetch_add(1, Ordering::Relaxed) + 1;
                        if let Some(ref p) = progress_ref { p.done.fetch_add(1, Ordering::Relaxed); }
                        let msg = format!("[{n}/{total_work}] Classified {session_id}: {}", res.outcome);
                        push_log(log_ref, msg);
                    }
                    Err(e) => {
                        failed_ref.fetch_add(1, Ordering::Relaxed);
                        if let Some(ref p) = progress_ref { p.failed.fetch_add(1, Ordering::Relaxed); }
                        let msg = format!("session {session_id}: classification failed: {e:#}");
                        push_log(log_ref, msg);
                    }
                }
            }
        })
        .buffer_unordered(config.concurrency)
        .collect::<Vec<()>>()
        .await;

    Ok(ClassifierReport {
        classified: classified.load(Ordering::Relaxed),
        skipped,
        failed: failed.load(Ordering::Relaxed),
        total_candidates,
    })
}

const CLASSIFIER_PROMPT: &str = r#"Analyze the following AI coding session and classify its outcome.
Return a JSON object with:
- "outcome": one of ["success", "partial", "failed", "abandoned"]
- "reason_code": if not success, one of ["repro_missing", "context_drift", "tool_misuse", "dependency_trap", "unknown"], else null

Definitions:
- success: The user's primary goal was achieved.
- partial: Some progress was made, but the task wasn't fully finished or has minor issues.
- failed: The AI couldn't solve the problem, got stuck in a loop, or produced incorrect code.
- abandoned: The user stopped the session early without a clear conclusion.

Return ONLY valid JSON.

Session data:
"#;

async fn call_classifier_ollama(digest: &str, model: &str, base_url: &str) -> Result<ClassificationResult> {
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let payload = serde_json::json!({
        "model": model,
        "prompt": format!("{}{}", CLASSIFIER_PROMPT, digest),
        "stream": false,
        "format": "json"
    });
    let client = reqwest::Client::new();
    let res = client.post(&url).json(&payload).send().await?;
    let response_data: serde_json::Value = res.json().await?;
    let text = response_data["response"].as_str().ok_or_else(|| anyhow::anyhow!("missing response"))?;
    Ok(serde_json::from_str(text)?)
}

async fn call_classifier_gemini(digest: &str, api_key: &str) -> Result<ClassificationResult> {
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}", api_key);
    let payload = serde_json::json!({
        "contents": [{"parts": [{"text": format!("{}{}", CLASSIFIER_PROMPT, digest)}]}],
        "generationConfig": { "response_mime_type": "application/json" }
    });
    let client = reqwest::Client::new();
    let res = client.post(&url).json(&payload).send().await?;
    let response_data: serde_json::Value = res.json().await?;
    let text = response_data["candidates"][0]["content"]["parts"][0]["text"].as_str().ok_or_else(|| anyhow::anyhow!("missing text"))?;
    Ok(serde_json::from_str(text)?)
}

async fn call_classifier_claude(digest: &str) -> Result<ClassificationResult> {
    let full_prompt = format!("{CLASSIFIER_PROMPT}{digest}");
    let output = tokio::process::Command::new("claude")
        .args(["-p", &full_prompt, "--output-format", "json", "--dangerously-skip-permissions"])
        .output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let text = enrich::parse_claude_jsonl_output_pub(&stdout)?;
    let json_str = enrich::extract_json_pub(&text);
    Ok(serde_json::from_str(json_str)?)
}
