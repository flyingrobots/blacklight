use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Discover additional Claude and Gemini data directories.
pub fn discover_extra_sources() -> Vec<(String, PathBuf, crate::config::SourceKind)> {
    let mut extras = Vec::new();

    if let Some(home) = dirs::home_dir() {
        // 1. Claude Desktop: ~/Library/Application Support/Claude/
        let claude_app_support = home.join("Library/Application Support/Claude");
        if claude_app_support.exists() {
            // 1a. local-agent-mode-sessions
            let agent_sessions = claude_app_support.join("local-agent-mode-sessions");
            if agent_sessions.exists() {
                if let Ok(entries) = find_nested_claude_dirs(&agent_sessions) {
                    for path in entries {
                        extras.push(("claude-desktop-agent".to_string(), path, crate::config::SourceKind::Claude));
                    }
                }
            }

            // 1b. claude-code-sessions
            let code_sessions = claude_app_support.join("claude-code-sessions");
            if code_sessions.exists() {
                // These are often nested deeply, but they contain JSON files we can index.
                // We'll add the root and let the scanner find them.
                extras.push(("claude-desktop-code".to_string(), code_sessions, crate::config::SourceKind::Claude));
            }
        }

        // 2. Gemini: ~/.gemini/
        let gemini_dir = home.join(".gemini");
        if gemini_dir.exists() {
            extras.push(("gemini".to_string(), gemini_dir, crate::config::SourceKind::Gemini));
        }

        // 3. Codex: ~/.codex/
        let codex_dir = home.join(".codex");
        if codex_dir.exists() {
            extras.push(("codex".to_string(), codex_dir, crate::config::SourceKind::Codex));
        }
    }

    extras
}

/// Recursively find `.claude` directories under a root (up to 6 levels deep).
fn find_nested_claude_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    find_claude_dirs_recursive(root, 0, 6, &mut results);
    Ok(results)
}

fn find_claude_dirs_recursive(dir: &Path, depth: u32, max_depth: u32, results: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        if name == ".claude" {
            results.push(path);
        } else {
            find_claude_dirs_recursive(&path, depth + 1, max_depth, results);
        }
    }
}

/// Classification of files found under ~/.claude/ or ~/.gemini/
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileKind {
    SessionIndex,   // projects/**/sessions-index.json
    SessionJsonl,   // projects/**/*.jsonl
    TaskJson,       // tasks/**/*.json (not .lock/.highwatermark)
    TodoJson,       // todos/**/*.json
    FacetJson,      // usage-data/facets/**/*.json
    StatsCache,     // stats-cache.json
    HistoryJsonl,   // history.jsonl
    PlanMarkdown,   // plans/**/*.md
    ToolResultTxt,  // projects/**/tool-results/toolu_*.txt
    GeminiSessionJson, // tmp/**/chats/session-*.json
    ClaudeDesktopSessionIndex, // claude-code-sessions/**/local_*.json
    CodexSessionJsonl, // sessions/**/rollout-*.jsonl
}

impl std::fmt::Display for FileKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileKind::SessionIndex => write!(f, "SessionIndex"),
            FileKind::SessionJsonl => write!(f, "SessionJsonl"),
            FileKind::TaskJson => write!(f, "TaskJson"),
            FileKind::TodoJson => write!(f, "TodoJson"),
            FileKind::FacetJson => write!(f, "FacetJson"),
            FileKind::StatsCache => write!(f, "StatsCache"),
            FileKind::HistoryJsonl => write!(f, "HistoryJsonl"),
            FileKind::PlanMarkdown => write!(f, "PlanMarkdown"),
            FileKind::ToolResultTxt => write!(f, "ToolResultTxt"),
            FileKind::GeminiSessionJson => write!(f, "GeminiSessionJson"),
            FileKind::ClaudeDesktopSessionIndex => write!(f, "ClaudeDesktopSessionIndex"),
            FileKind::CodexSessionJsonl => write!(f, "CodexSessionJsonl"),
        }
    }
}

/// A discovered file with its classification and metadata.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub kind: FileKind,
    pub mtime_ms: u64,
    pub size_bytes: u64,
}

/// Default directories to skip entirely during scanning.
pub const DEFAULT_SKIP_DIRS: &[&str] = &[
    "cache",
    "statsig",
    "shell-snapshots",
    "session-env",
    "ide",
    "paste-cache",
    "debug",
    "telemetry",
];

/// Files to skip by exact name.
const SKIP_FILES: &[&str] = &[".DS_Store", "settings.json"];

/// File extensions to skip.
const SKIP_EXTENSIONS: &[&str] = &["lock", "highwatermark"];

/// Recursively scan the given root directory, classify all files, and return sorted entries.
/// Uses the default skip_dirs list.
pub fn scan(root: &Path) -> Result<Vec<FileEntry>> {
    let default_skip: Vec<String> = DEFAULT_SKIP_DIRS.iter().map(|s| (*s).to_string()).collect();
    scan_with_skip_dirs(root, &default_skip)
}

/// Recursively scan the given root directory with custom skip_dirs.
pub fn scan_with_skip_dirs(root: &Path, skip_dirs: &[String]) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    walk_dir(root, root, skip_dirs, &mut entries)?;
    entries.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.path.cmp(&b.path)));

    // Log counts per kind
    let mut counts = std::collections::HashMap::new();
    for entry in &entries {
        *counts.entry(entry.kind.to_string()).or_insert(0u64) += 1;
    }
    for (kind, count) in &counts {
        tracing::info!("scan: {kind} = {count} files");
    }
    tracing::info!("scan: {} total files", entries.len());

    Ok(entries)
}

fn walk_dir(root: &Path, dir: &Path, skip_dirs: &[String], entries: &mut Vec<FileEntry>) -> Result<()> {
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            tracing::warn!("permission denied: {}", dir.display());
            return Ok(());
        }
        Err(e) => {
            return Err(e)
                .with_context(|| format!("failed to read directory {}", dir.display()));
        }
    };

    for entry in read_dir {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let path = entry.path();
        let file_type = entry.file_type().with_context(|| {
            format!("failed to get file type for {}", path.display())
        })?;

        if file_type.is_dir() {
            let dir_name = entry.file_name();
            let dir_name_str = dir_name.to_string_lossy();
            if skip_dirs.iter().any(|s| s == dir_name_str.as_ref()) {
                tracing::debug!("skipping directory: {}", path.display());
                continue;
            }
            walk_dir(root, &path, skip_dirs, entries)?;
        } else if file_type.is_file() {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // Skip by exact name
            if SKIP_FILES.contains(&file_name_str.as_ref()) {
                continue;
            }

            // Skip by extension
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                if SKIP_EXTENSIONS.contains(&ext_str.as_ref()) {
                    continue;
                }
            }

            if let Some(kind) = classify(root, &path, &file_name_str) {
                let metadata = entry.metadata().with_context(|| {
                    format!("failed to get metadata for {}", path.display())
                })?;
                let mtime_ms = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let size_bytes = metadata.len();

                entries.push(FileEntry {
                    path,
                    kind,
                    mtime_ms,
                    size_bytes,
                });
            }
        }
    }

    Ok(())
}

/// Classify a file based on its relative path from root.
fn classify(root: &Path, path: &Path, file_name: &str) -> Option<FileKind> {
    let rel = path.strip_prefix(root).ok()?;
    let rel_str = rel.to_string_lossy();

    // stats-cache.json at root
    if file_name == "stats-cache.json" {
        return Some(FileKind::StatsCache);
    }

    // history.jsonl at root
    if file_name == "history.jsonl" {
        return Some(FileKind::HistoryJsonl);
    }

    // projects/ subtree
    if rel_str.starts_with("projects/") || rel_str.starts_with("projects\\") {
        if file_name == "sessions-index.json" {
            return Some(FileKind::SessionIndex);
        }
        if file_name.ends_with(".jsonl") {
            return Some(FileKind::SessionJsonl);
        }
        if rel_str.contains("tool-results/") && file_name.starts_with("toolu_") && file_name.ends_with(".txt") {
            return Some(FileKind::ToolResultTxt);
        }
        return None;
    }

    // tasks/ subtree
    if rel_str.starts_with("tasks/") || rel_str.starts_with("tasks\\") {
        if file_name.ends_with(".json") {
            return Some(FileKind::TaskJson);
        }
        return None;
    }

    // todos/ subtree
    if rel_str.starts_with("todos/") || rel_str.starts_with("todos\\") {
        if file_name.ends_with(".json") {
            return Some(FileKind::TodoJson);
        }
        return None;
    }

    // usage-data/facets/ subtree
    if (rel_str.starts_with("usage-data/facets/") || rel_str.starts_with("usage-data\\facets\\"))
        && file_name.ends_with(".json")
    {
        return Some(FileKind::FacetJson);
    }

    // plans/ subtree
    if rel_str.starts_with("plans/") || rel_str.starts_with("plans\\") {
        if file_name.ends_with(".md") {
            return Some(FileKind::PlanMarkdown);
        }
        return None;
    }

    // Gemini: tmp/**/chats/session-*.json
    if (rel_str.contains("/chats/") || rel_str.contains("\\chats\\")) && file_name.ends_with(".json") && file_name.starts_with("session-") {
        return Some(FileKind::GeminiSessionJson);
    }

    // Claude Desktop Code Sessions: claude-code-sessions/**/local_*.json
    if (rel_str.contains("/claude-code-sessions/") || rel_str.contains("\\claude-code-sessions\\")) && file_name.ends_with(".json") && file_name.starts_with("local_") {
        return Some(FileKind::ClaudeDesktopSessionIndex);
    }

    // Codex: sessions/**/rollout-*.jsonl
    if (rel_str.contains("/sessions/") || rel_str.contains("\\sessions\\")) && file_name.ends_with(".jsonl") && file_name.starts_with("rollout-") {
        return Some(FileKind::CodexSessionJsonl);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_file(root: &Path, rel_path: &str) {
        let path = root.join(rel_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, "test content").unwrap();
    }

    #[test]
    fn test_scan_classifies_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        create_file(root, "projects/abc/sessions-index.json");
        create_file(root, "projects/abc/session1.jsonl");
        create_file(root, "projects/abc/tool-results/toolu_123.txt");
        create_file(root, "tasks/sess1/task1.json");
        create_file(root, "todos/sess1/todo1.json");
        create_file(root, "usage-data/facets/sess1.json");
        create_file(root, "stats-cache.json");
        create_file(root, "history.jsonl");
        create_file(root, "plans/plan1.md");

        let entries = scan(root).unwrap();
        assert_eq!(entries.len(), 9);

        let kinds: Vec<_> = entries.iter().map(|e| &e.kind).collect();
        assert!(kinds.contains(&&FileKind::SessionIndex));
        assert!(kinds.contains(&&FileKind::SessionJsonl));
        assert!(kinds.contains(&&FileKind::ToolResultTxt));
        assert!(kinds.contains(&&FileKind::TaskJson));
        assert!(kinds.contains(&&FileKind::TodoJson));
        assert!(kinds.contains(&&FileKind::FacetJson));
        assert!(kinds.contains(&&FileKind::StatsCache));
        assert!(kinds.contains(&&FileKind::HistoryJsonl));
        assert!(kinds.contains(&&FileKind::PlanMarkdown));
    }

    #[test]
    fn test_scan_skips_irrelevant() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        create_file(root, "cache/something.json");
        create_file(root, "statsig/data.json");
        create_file(root, "projects/abc/.DS_Store");
        create_file(root, "settings.json");
        create_file(root, "tasks/sess1/task1.lock");
        create_file(root, "tasks/sess1/task1.highwatermark");

        let entries = scan(root).unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_scan_sorted_by_kind_then_path() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        create_file(root, "projects/abc/sessions-index.json");
        create_file(root, "projects/abc/z_session.jsonl");
        create_file(root, "projects/abc/a_session.jsonl");

        let entries = scan(root).unwrap();
        assert_eq!(entries[0].kind, FileKind::SessionIndex);
        // SessionJsonl entries should be sorted by path
        assert!(entries[1].path < entries[2].path);
    }
}
