use anyhow::{Context, Result};
use serde::{Serialize, Deserialize};
use ts_rs::TS;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::watch;

use change::FileStatus;
use scanner::FileKind;
use crate::config::{SourceKind, BackupMode};
use crate::server::state::IndexerState;
use crate::notifications::{self, NotificationLevel, NotificationSender};
use crate::indexer::provider::SourceProvider;
use crate::indexer::providers::{ClaudeProvider, GeminiProvider, CodexProvider};

pub mod change;
pub mod codex;
pub mod db_ops;
pub mod file_paths;
pub mod gemini;
pub mod handlers;
pub mod jsonl;
pub mod provider;
pub mod providers;
pub mod router;
pub mod scanner;
pub mod sessions;
pub mod structured;

/// Configuration for an index run.
pub struct IndexConfig {
    pub sources: Vec<(String, PathBuf, SourceKind, Option<String>)>,
    pub db_path: PathBuf,
    pub backup_dir: PathBuf,
    pub backup_mode: BackupMode,
    pub full: bool,
    pub verbose: bool,
    /// Directories to skip during scanning.
    pub skip_dirs: Vec<String>,
    /// Shared progress tracker (updated during indexing).
    pub progress_tx: Option<watch::Sender<IndexerState>>,
    /// Cancellation flag (checked between phases and files).
    pub cancel_flag: Option<Arc<AtomicBool>>,
    /// Pause flag (spins between phases and files while true).
    pub pause_flag: Option<Arc<AtomicBool>>,
    /// Notification channel for pushing warnings/info to the frontend.
    pub notify_tx: Option<NotificationSender>,
}

/// Live progress information updated during indexing.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct IndexProgress {
    pub phase: String,
    pub files_total: usize,
    pub files_done: usize,
    pub messages_processed: usize,
    pub blobs_inserted: usize,
}

/// Report produced after indexing completes.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct IndexReport {
    pub sessions_parsed: usize,
    pub messages_processed: usize,
    pub messages_skipped: usize,
    pub parse_errors: usize,
    pub blobs_inserted: usize,
    pub tool_calls_inserted: usize,
    pub tasks_parsed: usize,
    pub facets_parsed: usize,
    pub plans_parsed: usize,
    pub history_entries: usize,
    pub files_processed: usize,
    pub files_unchanged: usize,
    pub elapsed_secs: f64,
}

impl std::fmt::Display for IndexReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Indexing complete in {:.1}s", self.elapsed_secs)?;
        writeln!(f, "  Sessions:       {}", self.sessions_parsed)?;
        writeln!(f, "  Messages:       {} processed, {} skipped, {} errors",
            self.messages_processed, self.messages_skipped, self.parse_errors)?;
        writeln!(f, "  Blobs:          {} new", self.blobs_inserted)?;
        writeln!(f, "  Tool calls:     {}", self.tool_calls_inserted)?;
        writeln!(f, "  Tasks:          {}", self.tasks_parsed)?;
        writeln!(f, "  Facets:         {}", self.facets_parsed)?;
        writeln!(f, "  Plans:          {}", self.plans_parsed)?;
        writeln!(f, "  History:        {}", self.history_entries)?;
        writeln!(f, "  Files:          {} processed, {} unchanged",
            self.files_processed, self.files_unchanged)?;
        Ok(())
    }
}

fn update_progress(progress_tx: &Option<watch::Sender<IndexerState>>, f: impl Fn(&mut IndexProgress)) {
    if let Some(tx) = progress_tx {
        tx.send_modify(|s| f(&mut s.progress));
    }
}

fn is_cancelled(cancel_flag: &Option<Arc<AtomicBool>>) -> bool {
    cancel_flag
        .as_ref()
        .is_some_and(|f| f.load(Ordering::Relaxed))
}

/// Spin-wait while pause_flag is true. Breaks early if cancel_flag fires.
fn wait_if_paused(config: &IndexConfig) {
    if let Some(pause) = &config.pause_flag {
        while pause.load(Ordering::Relaxed) {
            if is_cancelled(&config.cancel_flag) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

fn notify_warn(config: &IndexConfig, msg: impl Into<String>) {
    if let Some(tx) = &config.notify_tx {
        notifications::notify(tx, NotificationLevel::Warn, msg);
    }
}

fn notify_info(config: &IndexConfig, msg: impl Into<String>) {
    if let Some(tx) = &config.notify_tx {
        notifications::notify(tx, NotificationLevel::Info, msg);
    }
}

/// Main entry point: run the full indexing pipeline.
pub fn run_index(config: IndexConfig) -> Result<IndexReport> {
    let start = Instant::now();
    let mut report = IndexReport::default();
    let cancel_flag = config.cancel_flag.clone();

    // Register providers
    let providers: Vec<Box<dyn SourceProvider>> = vec![
        Box::new(ClaudeProvider),
        Box::new(GeminiProvider),
        Box::new(CodexProvider),
    ];

    update_progress(&config.progress_tx, |p| p.phase = "Scanning".to_string());

    tracing::info!("indexing {} sources → {}", config.sources.len(), config.db_path.display());
    notify_info(&config, "Indexing started");

    // 0. Initialize backup git repo if it doesn't exist
    if let Err(e) = ensure_backup_repo(&config.backup_dir) {
        tracing::warn!("failed to initialize backup git repo: {e}");
    }

    // 1. Open database
    let conn = crate::db::open(&config.db_path)
        .context("failed to open database")?;

    // 2. Scan filesystem for all sources
    let mut manifest_with_source = Vec::new();

    for (name, path, kind, cas_prefix) in &config.sources {
        if !path.exists() {
            tracing::warn!("source path {} does not exist, skipping", path.display());
            continue;
        }

        tracing::info!("scanning source: {} ({})", name, path.display());
        match scanner::scan_with_skip_dirs(path, &config.skip_dirs) {
            Ok(entries) => {
                for entry in entries {
                    manifest_with_source.push((name.clone(), kind.clone(), cas_prefix.clone(), entry));
                }
            }
            Err(e) => {
                let msg = format!("Failed to scan {}: {e}", path.display());
                tracing::warn!("{msg}");
                notify_warn(&config, msg);
            }
        }
    }

    // Sort combined manifest by kind then path
    manifest_with_source.sort_by(|(_, _, _, a), (_, _, _, b)| a.kind.cmp(&b.kind).then_with(|| a.path.cmp(&b.path)));

    wait_if_paused(&config);
    if is_cancelled(&cancel_flag) {
        return Ok(report);
    }

    update_progress(&config.progress_tx, |p| p.phase = "Detecting changes".to_string());

    // 3. Change detection
    let manifest: Vec<_> = manifest_with_source.iter().map(|(_, _, _, entry)| entry.clone()).collect();
    let plan = if config.full {
        tracing::info!("full re-index requested, treating all files as new");
        change::ProcessingPlan {
            to_process: manifest
                .into_iter()
                .map(|entry| (entry, FileStatus::New))
                .collect(),
            unchanged_count: 0,
            deleted_paths: Vec::new(),
        }
    } else {
        change::detect_changes(&conn, &manifest)
            .context("failed to detect changes")?
    };

    report.files_unchanged = plan.unchanged_count;
    
    // Map status back to our source-aware manifest
    let mut work_queue = Vec::new();
    for (entry, status) in plan.to_process {
        if let Some((name, kind, cas_prefix, _)) = manifest_with_source.iter().find(|(_, _, _, e)| e.path == entry.path) {
            work_queue.push((name.clone(), kind.clone(), cas_prefix.clone(), entry, status));
        }
    }

    tracing::info!(
        "{} files to process, {} unchanged, {} deleted",
        work_queue.len(),
        plan.unchanged_count,
        plan.deleted_paths.len(),
    );

    // Partition files into phases
    let mut metadata_files = Vec::new();
    let mut content_files = Vec::new();
    let mut structured_files = Vec::new();

    for item in &work_queue {
        let kind = &item.3.kind;
        match kind {
            FileKind::SessionIndex | FileKind::ClaudeDesktopSessionIndex => metadata_files.push(item),
            FileKind::SessionJsonl | FileKind::GeminiSessionJson | FileKind::CodexSessionJsonl => content_files.push(item),
            FileKind::TaskJson | FileKind::FacetJson | FileKind::StatsCache | FileKind::HistoryJsonl | FileKind::PlanMarkdown => structured_files.push(item),
            _ => {}
        }
    }

    // Phase 1: Metadata files
    update_progress(&config.progress_tx, |p| {
        p.phase = "Phase 1: Metadata indexes".to_string();
        p.files_total = metadata_files.len();
        p.files_done = 0;
    });

    for (source_name, kind, _, entry, _) in &metadata_files {
        for provider in &providers {
            if provider.can_handle(&entry.kind) {
                match provider.process_metadata(&conn, entry) {
                    Ok(count) => {
                        report.sessions_parsed += count;
                        conn.execute(
                            "UPDATE sessions SET source_name = ?1, source_kind = ?2 WHERE source_file = ?3",
                            rusqlite::params![source_name, kind.to_string().to_lowercase(), entry.path.to_string_lossy()],
                        ).ok();
                    }
                    Err(e) => {
                        let msg = format!("Failed to parse metadata {}: {e}", entry.path.display());
                        tracing::warn!("{msg}");
                        notify_warn(&config, msg);
                    }
                }
            }
        }
        update_progress(&config.progress_tx, |p| p.files_done += 1);
        wait_if_paused(&config);
        if is_cancelled(&cancel_flag) { return Ok(report); }
    }

    // Phase 2: Content files
    update_progress(&config.progress_tx, |p| {
        p.phase = "Phase 2: Content files".to_string();
        p.files_total = content_files.len();
        p.files_done = 0;
    });

    for (source_name, _kind, cas_prefix, entry, status) in content_files {
        let start_offset = match status {
            FileStatus::Modified { last_byte_offset } => *last_byte_offset,
            _ => 0,
        };

        for provider in &providers {
            if provider.can_handle(&entry.kind) {
                match provider.process_content(&conn, entry, start_offset) {
                    Ok((stats, final_offset)) => {
                        report.messages_processed += stats.messages_processed;
                        report.messages_skipped += stats.messages_skipped;
                        report.parse_errors += stats.parse_errors;
                        report.blobs_inserted += stats.blobs_inserted;
                        report.tool_calls_inserted += stats.tool_calls_inserted;
                        report.sessions_parsed += stats.sessions_parsed;

                        conn.execute(
                            "UPDATE messages SET source_name = ?1 WHERE session_id IN (SELECT id FROM sessions WHERE source_file = ?2) AND source_name IS NULL",
                            rusqlite::params![source_name, entry.path.to_string_lossy()],
                        ).ok();

                        let actual_prefix = cas_prefix.as_deref().unwrap_or(source_name);
                        if let Err(e) = backup_source_file(&conn, &entry.path, &config.backup_dir, config.backup_mode, actual_prefix) {
                            tracing::warn!("failed to backup {}: {e}", entry.path.display());
                        }

                        change::mark_indexed(&conn, &entry.path.to_string_lossy(), entry.mtime_ms, entry.size_bytes, final_offset)?;
                        report.files_processed += 1;
                    }
                    Err(e) => {
                        let msg = format!("Failed to process {}: {e:#}", entry.path.display());
                        tracing::warn!("{msg}");
                        notify_warn(&config, msg);
                    }
                }
            }
        }

        update_progress(&config.progress_tx, |p| {
            p.files_done += 1;
            p.messages_processed = report.messages_processed;
            p.blobs_inserted = report.blobs_inserted;
        });
        wait_if_paused(&config);
        if is_cancelled(&cancel_flag) { return Ok(report); }
    }

    // Phase 3: Structured data
    update_progress(&config.progress_tx, |p| {
        p.phase = "Phase 3: Structured data".to_string();
    });

    let task_entries: Vec<_> = structured_files.iter().filter(|(_, _, _, e, _)| e.kind == FileKind::TaskJson).map(|(_, _, _, e, _)| e.clone()).collect();
    if !task_entries.is_empty() {
        if let Ok(count) = structured::parse_tasks(&conn, &task_entries) { report.tasks_parsed = count; }
    }

    let facet_entries: Vec<_> = structured_files.iter().filter(|(_, _, _, e, _)| e.kind == FileKind::FacetJson).map(|(_, _, _, e, _)| e.clone()).collect();
    if !facet_entries.is_empty() {
        if let Ok(count) = structured::parse_facets(&conn, &facet_entries) { report.facets_parsed = count; }
    }

    if let Some((_, _, _, entry, _)) = structured_files.iter().find(|(_, _, _, e, _)| e.kind == FileKind::StatsCache) {
        let _ = structured::parse_stats_cache(&conn, &entry.path);
    }

    let plan_entries: Vec<_> = structured_files.iter().filter(|(_, _, _, e, _)| e.kind == FileKind::PlanMarkdown).map(|(_, _, _, e, _)| e.clone()).collect();
    if !plan_entries.is_empty() {
        if let Ok(count) = structured::parse_plans(&conn, &plan_entries) { report.plans_parsed = count; }
    }

    if let Some((_, _, _, entry, _)) = structured_files.iter().find(|(_, _, _, e, _)| e.kind == FileKind::HistoryJsonl) {
        if let Ok(count) = structured::parse_history(&conn, &entry.path) { report.history_entries = count; }
    }

    for (_, _, _, entry, _) in structured_files {
        change::mark_indexed(&conn, &entry.path.to_string_lossy(), entry.mtime_ms, entry.size_bytes, entry.size_bytes)?;
        report.files_processed += 1;
    }

    report.elapsed_secs = start.elapsed().as_secs_f64();
    update_progress(&config.progress_tx, |p| {
        p.phase = "Done".to_string();
        p.messages_processed = report.messages_processed;
        p.blobs_inserted = report.blobs_inserted;
    });

    notify_info(&config, format!("Indexing complete: {} sessions, {} messages in {:.1}s", report.sessions_parsed, report.messages_processed, report.elapsed_secs));
    Ok(report)
}

fn ensure_backup_repo(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }

    if !path.join(".git").exists() {
        tracing::info!("initializing backup git repo in {}", path.display());
        std::process::Command::new("git").arg("init").current_dir(path).output().context("failed to git init backup repo")?;
        std::process::Command::new("git").args(["cas", "vault", "init"]).current_dir(path).output().context("failed to init git-cas vault")?;
    }

    Ok(())
}

pub fn backup_source_file(conn: &rusqlite::Connection, path: &Path, backup_dir: &Path, mode: BackupMode, source_prefix: &str) -> Result<()> {
    let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
    let base_slug = if file_name.starts_with("session-") {
        file_name.strip_suffix(".json").unwrap_or(file_name).to_string()
    } else if file_name.starts_with("rollout-") {
        file_name.strip_suffix(".jsonl").unwrap_or(file_name).to_string()
    } else {
        file_name.to_string()
    };

    let slug = format!("{}:{}", source_prefix, base_slug);

    match mode {
        BackupMode::GitCas => {
            let mut attempts = 0;
            let max_attempts = 3;
            let mut last_err = String::new();

            while attempts < max_attempts {
                let output_res = std::process::Command::new("git")
                    .args(["cas", "store", &path.to_string_lossy(), "--slug", &slug, "--tree"])
                    .current_dir(backup_dir)
                    .output();

                match output_res {
                    Ok(output) if output.status.success() => {
                        let manifest_json = String::from_utf8_lossy(&output.stdout);
                        let manifest: serde_json::Value = serde_json::from_str(&manifest_json).context("failed to parse git-cas manifest")?;
                        
                        let content_hash = manifest["hash"].as_str().or_else(|| manifest["oid"].as_str()).unwrap_or("unknown").to_string();
                        let file_size = std::fs::metadata(path)?.len();

                        let session_id = if file_name.starts_with("session-") {
                            file_name.strip_prefix("session-").and_then(|s| s.strip_suffix(".json"))
                        } else if file_name.starts_with("rollout-") {
                            file_name.strip_prefix("rollout-").and_then(|s| {
                                let s = s.strip_suffix(".jsonl").unwrap_or(s);
                                if s.len() >= 36 { Some(&s[s.len()-36..]) } else { None }
                            })
                        } else { None };

                        if let Some(sid) = session_id {
                            crate::indexer::db_ops::record_backup(conn, sid, &path.to_string_lossy(), &content_hash, file_size)?;
                        }
                        return Ok(());
                    }
                    Ok(output) => {
                        let err = String::from_utf8_lossy(&output.stderr);
                        if err.contains("'cas' is not a git command") {
                            tracing::warn!("git-cas not found, falling back to simple backup for {}", path.display());
                            return backup_source_file(conn, path, backup_dir, BackupMode::Simple, source_prefix);
                        }
                        last_err = err.to_string();
                        attempts += 1;
                        if attempts < max_attempts {
                            std::thread::sleep(std::time::Duration::from_millis(100 * attempts as u64));
                        }
                    }
                    Err(e) => {
                        tracing::warn!("failed to execute git command: {e}. falling back to simple backup");
                        return backup_source_file(conn, path, backup_dir, BackupMode::Simple, source_prefix);
                    }
                }
            }
            anyhow::bail!("git cas store failed after {} attempts: {}", max_attempts, last_err);
        }
        BackupMode::Simple => {
            let content = std::fs::read(path)?;
            let hash = crate::content::hash_content_bytes(&content);
            let size = content.len() as u64;

            if !backup_dir.exists() { std::fs::create_dir_all(backup_dir)?; }

            let backup_path = backup_dir.join(&hash);
            if !backup_path.exists() { std::fs::write(&backup_path, content)?; }

            let session_id = if file_name.starts_with("session-") {
                file_name.strip_prefix("session-").and_then(|s| s.strip_suffix(".json"))
            } else if file_name.starts_with("rollout-") {
                file_name.strip_prefix("rollout-").and_then(|s| {
                    let s = s.strip_suffix(".jsonl").unwrap_or(s);
                    if s.len() >= 36 { Some(&s[s.len()-36..]) } else { None }
                })
            } else { None };

            if let Some(sid) = session_id {
                crate::indexer::db_ops::record_backup(conn, sid, &path.to_string_lossy(), &hash, size)?;
            }
        }
    }

    Ok(())
}
