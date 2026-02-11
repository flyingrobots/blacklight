use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Classification of files found under ~/.claude/
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

/// Directories to skip entirely during scanning.
const SKIP_DIRS: &[&str] = &[
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
pub fn scan(root: &Path) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    walk_dir(root, root, &mut entries)?;
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

fn walk_dir(root: &Path, dir: &Path, entries: &mut Vec<FileEntry>) -> Result<()> {
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
            if SKIP_DIRS.contains(&dir_name_str.as_ref()) {
                tracing::debug!("skipping directory: {}", path.display());
                continue;
            }
            walk_dir(root, &path, entries)?;
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
