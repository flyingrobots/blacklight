pub mod change;
pub mod db_ops;
pub mod file_paths;
pub mod handlers;
pub mod jsonl;
pub mod router;
pub mod scanner;
pub mod sessions;
pub mod structured;

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Instant;

use change::FileStatus;
use scanner::FileKind;

/// Configuration for an index run.
pub struct IndexConfig {
    pub claude_dir: PathBuf,
    pub db_path: PathBuf,
    pub full: bool,
    pub verbose: bool,
}

/// Report produced after indexing completes.
#[derive(Debug, Default)]
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

/// Main entry point: run the full indexing pipeline.
pub fn run_index(config: IndexConfig) -> Result<IndexReport> {
    let start = Instant::now();
    let mut report = IndexReport::default();

    tracing::info!("indexing {} → {}", config.claude_dir.display(), config.db_path.display());

    // 1. Open database
    let conn = crate::db::open(&config.db_path)
        .context("failed to open database")?;

    // 2. Scan filesystem
    let manifest = scanner::scan(&config.claude_dir)
        .with_context(|| format!("failed to scan {}", config.claude_dir.display()))?;

    // 3. Change detection (or treat all as New if --full)
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
    tracing::info!(
        "{} files to process, {} unchanged, {} deleted",
        plan.to_process.len(),
        plan.unchanged_count,
        plan.deleted_paths.len(),
    );

    // Partition files by kind
    let mut session_indexes = Vec::new();
    let mut session_jsonls = Vec::new();
    let mut task_files = Vec::new();
    let mut facet_files = Vec::new();
    let mut stats_cache_path: Option<(scanner::FileEntry, FileStatus)> = None;
    let mut history_path: Option<(scanner::FileEntry, FileStatus)> = None;
    let mut plan_files = Vec::new();

    for (entry, status) in plan.to_process {
        match &entry.kind {
            FileKind::SessionIndex => session_indexes.push((entry, status)),
            FileKind::SessionJsonl => session_jsonls.push((entry, status)),
            FileKind::TaskJson => task_files.push((entry, status)),
            FileKind::FacetJson => facet_files.push((entry, status)),
            FileKind::StatsCache => stats_cache_path = Some((entry, status)),
            FileKind::HistoryJsonl => history_path = Some((entry, status)),
            FileKind::PlanMarkdown => plan_files.push((entry, status)),
            FileKind::TodoJson => { /* TODO: implement todo parsing */ }
            FileKind::ToolResultTxt => { /* TODO: implement tool result parsing */ }
        }
    }

    // Phase 1: Parse sessions-index.json files (must come first for FK ordering)
    tracing::info!("phase 1: parsing {} session index files", session_indexes.len());
    for (entry, _status) in &session_indexes {
        match sessions::parse_session_index(&conn, &entry.path) {
            Ok(count) => report.sessions_parsed += count,
            Err(e) => tracing::warn!("failed to parse {}: {e}", entry.path.display()),
        }
    }

    // Phase 2: Stream JSONL files
    tracing::info!("phase 2: streaming {} JSONL files", session_jsonls.len());
    for (entry, status) in &session_jsonls {
        let start_offset = match status {
            FileStatus::Modified { last_byte_offset } => *last_byte_offset,
            _ => 0,
        };

        match router::process_jsonl(&conn, &entry.path, start_offset, config.verbose) {
            Ok((stats, final_offset)) => {
                report.messages_processed += stats.messages_processed;
                report.messages_skipped += stats.messages_skipped;
                report.parse_errors += stats.parse_errors;
                report.blobs_inserted += stats.blobs_inserted;
                report.tool_calls_inserted += stats.tool_calls_inserted;

                change::mark_indexed(
                    &conn,
                    &entry.path.to_string_lossy(),
                    entry.mtime_ms,
                    entry.size_bytes,
                    final_offset,
                )?;
                report.files_processed += 1;
            }
            Err(e) => {
                tracing::warn!("failed to process {}: {e:#}", entry.path.display());
            }
        }
    }

    // Phase 3: Structured data
    tracing::info!("phase 3: parsing structured data");

    // Tasks
    if !task_files.is_empty() {
        let entries: Vec<_> = task_files.iter().map(|(e, _)| e.clone()).collect();
        match structured::parse_tasks(&conn, &entries) {
            Ok(count) => report.tasks_parsed = count,
            Err(e) => tracing::warn!("failed to parse tasks: {e}"),
        }
    }

    // Facets
    if !facet_files.is_empty() {
        let entries: Vec<_> = facet_files.iter().map(|(e, _)| e.clone()).collect();
        match structured::parse_facets(&conn, &entries) {
            Ok(count) => report.facets_parsed = count,
            Err(e) => tracing::warn!("failed to parse facets: {e}"),
        }
    }

    // Stats cache
    if let Some((entry, _)) = &stats_cache_path {
        match structured::parse_stats_cache(&conn, &entry.path) {
            Ok(()) => {}
            Err(e) => tracing::warn!("failed to parse stats-cache: {e}"),
        }
    }

    // Plans
    if !plan_files.is_empty() {
        let entries: Vec<_> = plan_files.iter().map(|(e, _)| e.clone()).collect();
        match structured::parse_plans(&conn, &entries) {
            Ok(count) => report.plans_parsed = count,
            Err(e) => tracing::warn!("failed to parse plans: {e}"),
        }
    }

    // History
    if let Some((entry, _)) = &history_path {
        match structured::parse_history(&conn, &entry.path) {
            Ok(count) => report.history_entries = count,
            Err(e) => tracing::warn!("failed to parse history: {e}"),
        }
    }

    // Mark all non-JSONL processed files as indexed
    for (entry, _) in session_indexes
        .iter()
        .chain(task_files.iter())
        .chain(facet_files.iter())
        .chain(plan_files.iter())
        .chain(stats_cache_path.as_ref().into_iter())
        .chain(history_path.as_ref().into_iter())
    {
        change::mark_indexed(
            &conn,
            &entry.path.to_string_lossy(),
            entry.mtime_ms,
            entry.size_bytes,
            entry.size_bytes, // For non-JSONL, offset = full file size
        )?;
        report.files_processed += 1;
    }

    report.elapsed_secs = start.elapsed().as_secs_f64();

    // Log DB size
    let db_size = std::fs::metadata(&config.db_path)
        .map(|m| m.len())
        .unwrap_or(0);
    tracing::info!("database size: {:.1} MB", db_size as f64 / 1_048_576.0);

    Ok(report)
}
