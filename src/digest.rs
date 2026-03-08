use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use std::path::PathBuf;
use std::sync::Arc;

use crate::enrich;
use crate::db::DbPool;

#[derive(Debug)]
pub struct DigestConfig {
    pub db_path: PathBuf,
    pub ollama_url: String,
    pub ollama_model: String,
    pub google_api_key: String,
    pub preferred_backend: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct WeeklyDigest {
    pub id: i64,
    pub start_date: String,
    pub end_date: String,
    pub content: String,
    pub session_count: i32,
    pub success_count: i32,
    pub failed_count: i32,
    pub partial_count: i32,
    pub abandoned_count: i32,
    pub message_count: i32,
    pub created_at: String,
}

pub async fn generate_weekly_digest(pool: Arc<DbPool>, config: DigestConfig, start_date: String, end_date: String) -> Result<WeeklyDigest> {
    let start_date_clone = start_date.clone();
    let end_date_clone = end_date.clone();

    // 1. Gather stats and context using pool.call
    let (stats, context) = pool.call(move |conn| -> Result<((i32, i32, i32, i32, i32, i32), String)> {
        // Gather stats
        let stats = conn.query_row(
            "SELECT 
                COUNT(s.id),
                SUM(CASE WHEN o.outcome = 'success' THEN 1 ELSE 0 END),
                SUM(CASE WHEN o.outcome = 'failed' THEN 1 ELSE 0 END),
                SUM(CASE WHEN o.outcome = 'partial' THEN 1 ELSE 0 END),
                SUM(CASE WHEN o.outcome = 'abandoned' THEN 1 ELSE 0 END),
                SUM(s.message_count)
             FROM sessions s
             LEFT JOIN session_outcomes o ON o.session_id = s.id
             WHERE s.created_at >= ?1 AND s.created_at < ?2",
            params![start_date_clone, end_date_clone],
            |row| Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, Option<i32>>(1)?.unwrap_or(0),
                row.get::<_, Option<i32>>(2)?.unwrap_or(0),
                row.get::<_, Option<i32>>(3)?.unwrap_or(0),
                row.get::<_, Option<i32>>(4)?.unwrap_or(0),
                row.get::<_, Option<i32>>(5)?.unwrap_or(0),
            ))
        )?;

        if stats.0 == 0 {
            anyhow::bail!("No sessions found for the period {} to {}", start_date_clone, end_date_clone);
        }

        // Gather summaries
        let mut stmt = conn.prepare(
            "SELECT s.project_slug, COALESCE(e.title, s.id) as title, COALESCE(e.summary, s.summary) as summary, o.outcome, o.reason_code
             FROM sessions s
             LEFT JOIN session_enrichments e ON e.session_id = s.id
             LEFT JOIN session_outcomes o ON o.session_id = s.id
             WHERE s.created_at >= ?1 AND s.created_at < ?2
             ORDER BY s.created_at ASC
             LIMIT 50"
        )?;

        let summaries: Vec<String> = stmt.query_map(params![start_date_clone, end_date_clone], |row| {
            let project: String = row.get(0)?;
            let title: String = row.get(1)?;
            let summary: Option<String> = row.get(2)?;
            let outcome: Option<String> = row.get(3)?;
            let reason: Option<String> = row.get(4)?;
            
            Ok(format!(
                "Project: {}\nTitle: {}\nSummary: {}\nOutcome: {} {}\n---\n",
                project, title, summary.unwrap_or_default(), 
                outcome.unwrap_or_else(|| "unlabeled".to_string()),
                reason.map(|r| format!("({})", r)).unwrap_or_default()
            ))
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        Ok((stats, summaries.join("\n")))
    }).await?;

    let (session_count, success_count, failed_count, partial_count, abandoned_count, message_count) = stats;

    // 2. Call LLM (async)
    let prompt = format!(
        "You are an engineering lead reviewing a week of AI-assisted coding work.
Period: {} to {}
Stats: {} sessions, {} successes, {} failures, {} partials, {} abandoned.
Total messages: {}

Review the following session summaries and write a 'Weekly Decision Digest'.
Focus on:
1. High-level themes and major accomplishments.
2. Recurring failure patterns or bottlenecks.
3. Recommendations for the next week.

Format as clean Markdown. Use headers, lists, and bold text for emphasis.

Session Summaries:
{}
",
        start_date, end_date, session_count, success_count, failed_count, partial_count, abandoned_count, message_count, context
    );

    let content = call_llm_for_digest(&config, &prompt).await?;

    // 3. Store in DB using pool.write
    let content_clone = content.clone();
    let now = chrono::Utc::now().to_rfc3339();
    let now_clone = now.clone();
    let start_date_db = start_date.clone();
    let end_date_db = end_date.clone();

    let id = pool.write(move |conn| -> Result<i64> {
        conn.execute(
            "INSERT INTO weekly_digests (start_date, end_date, content, session_count, success_count, failed_count, partial_count, abandoned_count, message_count, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![start_date_db, end_date_db, content_clone, session_count, success_count, failed_count, partial_count, abandoned_count, message_count, now_clone],
        )?;
        Ok(conn.last_insert_rowid())
    }).await?;

    Ok(WeeklyDigest {
        id,
        start_date,
        end_date,
        content,
        session_count,
        success_count,
        failed_count,
        partial_count,
        abandoned_count,
        message_count,
        created_at: now,
    })
}

async fn call_llm_for_digest(config: &DigestConfig, prompt: &str) -> Result<String> {
    let google_key = std::env::var("GOOGLE_API_KEY").ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| if config.google_api_key.is_empty() { None } else { Some(config.google_api_key.clone()) });
    
    let mut ollama_model = std::env::var("OLLAMA_MODEL").ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| if config.ollama_model.is_empty() { None } else { Some(config.ollama_model.clone()) });

    if ollama_model.is_none() && (config.preferred_backend == "ollama" || config.preferred_backend == "auto") {
        ollama_model = enrich::auto_detect_ollama_pub(&config.ollama_url).await;
    }

    if config.preferred_backend == "claude-cli" {
        return call_claude_raw(prompt).await;
    } else if let Some(ref model) = ollama_model {
        return call_ollama_raw(prompt, model, &config.ollama_url).await;
    } else if let Some(ref key) = google_key {
        return call_gemini_raw(prompt, key).await;
    } else {
        return call_claude_raw(prompt).await;
    }
}

async fn call_ollama_raw(prompt: &str, model: &str, base_url: &str) -> Result<String> {
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let payload = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false
    });
    let client = reqwest::Client::new();
    let res = client.post(&url).json(&payload).send().await?;
    let response_data: serde_json::Value = res.json().await?;
    Ok(response_data["response"].as_str().unwrap_or_default().to_string())
}

async fn call_gemini_raw(prompt: &str, api_key: &str) -> Result<String> {
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}", api_key);
    let payload = serde_json::json!({
        "contents": [{"parts": [{"text": prompt}]}]
    });
    let client = reqwest::Client::new();
    let res = client.post(&url).json(&payload).send().await?;
    let response_data: serde_json::Value = res.json().await?;
    let text = response_data["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or_default();
    Ok(text.to_string())
}

async fn call_claude_raw(prompt: &str) -> Result<String> {
    let output = tokio::process::Command::new("claude")
        .args(["-p", prompt, "--dangerously-skip-permissions"])
        .output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.to_string())
}

pub fn list_digests(conn: &mut Connection, limit: i64, offset: i64) -> Result<Vec<WeeklyDigest>> {
    let mut stmt = conn.prepare(
        "SELECT id, start_date, end_date, content, session_count, success_count, failed_count, partial_count, abandoned_count, message_count, created_at
         FROM weekly_digests
         ORDER BY start_date DESC
         LIMIT ?1 OFFSET ?2"
    )?;

    let items = stmt.query_map(params![limit, offset], |row| {
        Ok(WeeklyDigest {
            id: row.get(0)?,
            start_date: row.get(1)?,
            end_date: row.get(2)?,
            content: row.get(3)?,
            session_count: row.get(4)?,
            success_count: row.get(5)?,
            failed_count: row.get(6)?,
            partial_count: row.get(7)?,
            abandoned_count: row.get(8)?,
            message_count: row.get(9)?,
            created_at: row.get(10)?,
        })
    })?.collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(items)
}
