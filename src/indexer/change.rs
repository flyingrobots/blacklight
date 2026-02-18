use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use super::scanner::FileEntry;

/// Status of a file relative to the last index run.
#[derive(Debug, Clone)]
pub enum FileStatus {
    /// File has never been indexed.
    New,
    /// File has been modified since last index. Carries the byte offset where we left off.
    Modified { last_byte_offset: u64 },
    /// File is unchanged since last index.
    Unchanged,
}

/// The processing plan produced by change detection.
pub struct ProcessingPlan {
    pub to_process: Vec<(FileEntry, FileStatus)>,
    pub unchanged_count: usize,
    pub deleted_paths: Vec<String>,
}

/// Compare scanned manifest against the indexed_files table and produce a processing plan.
pub fn detect_changes(conn: &Connection, manifest: &[FileEntry]) -> Result<ProcessingPlan> {
    let mut to_process = Vec::new();
    let mut unchanged_count = 0;

    let mut stmt = conn.prepare(
        "SELECT mtime_ms, size_bytes, last_byte_offset FROM indexed_files WHERE file_path = ?1",
    )?;

    for entry in manifest {
        let path_str = entry.path.to_string_lossy();
        let row = stmt
            .query_row(params![path_str.as_ref()], |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, u64>(1)?,
                    row.get::<_, u64>(2)?,
                ))
            })
            .optional();

        match row {
            Ok(Some((prev_mtime, prev_size, last_offset))) => {
                if entry.mtime_ms == prev_mtime && entry.size_bytes == prev_size {
                    unchanged_count += 1;
                } else {
                    to_process.push((
                        entry.clone(),
                        FileStatus::Modified {
                            last_byte_offset: last_offset,
                        },
                    ));
                }
            }
            Ok(None) => {
                to_process.push((entry.clone(), FileStatus::New));
            }
            Err(_) => {
                to_process.push((entry.clone(), FileStatus::New));
            }
        }
    }

    // Detect deleted files: paths in indexed_files that are not in the manifest
    let manifest_paths: std::collections::HashSet<String> = manifest
        .iter()
        .map(|e| e.path.to_string_lossy().to_string())
        .collect();

    let mut all_stmt =
        conn.prepare("SELECT file_path FROM indexed_files")?;
    let deleted_paths: Vec<String> = all_stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .filter(|p| !manifest_paths.contains(p))
        .collect();

    if !deleted_paths.is_empty() {
        tracing::info!("detected {} deleted files", deleted_paths.len());
    }

    Ok(ProcessingPlan {
        to_process,
        unchanged_count,
        deleted_paths,
    })
}

/// Record that a file has been successfully indexed.
pub fn mark_indexed(
    conn: &Connection,
    path: &str,
    mtime_ms: u64,
    size_bytes: u64,
    byte_offset: u64,
) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO indexed_files (file_path, mtime_ms, size_bytes, last_byte_offset, indexed_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        params![path, mtime_ms, size_bytes, byte_offset],
    )
    .context("failed to mark file as indexed")?;
    Ok(())
}

/// Helper trait for optional query results.
trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::indexer::scanner::FileKind;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn test_entry(path: &str, mtime: u64, size: u64) -> FileEntry {
        FileEntry {
            path: PathBuf::from(path),
            kind: FileKind::SessionJsonl,
            mtime_ms: mtime,
            size_bytes: size,
        }
    }

    #[test]
    fn test_all_new_files() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        let manifest = vec![test_entry("/a.jsonl", 1000, 500)];
        let plan = detect_changes(&conn, &manifest).unwrap();

        assert_eq!(plan.to_process.len(), 1);
        assert_eq!(plan.unchanged_count, 0);
        assert!(matches!(plan.to_process[0].1, FileStatus::New));
    }

    #[test]
    fn test_unchanged_files() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        mark_indexed(&conn, "/a.jsonl", 1000, 500, 500).unwrap();

        let manifest = vec![test_entry("/a.jsonl", 1000, 500)];
        let plan = detect_changes(&conn, &manifest).unwrap();

        assert_eq!(plan.to_process.len(), 0);
        assert_eq!(plan.unchanged_count, 1);
    }

    #[test]
    fn test_modified_files() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        mark_indexed(&conn, "/a.jsonl", 1000, 500, 300).unwrap();

        let manifest = vec![test_entry("/a.jsonl", 2000, 800)];
        let plan = detect_changes(&conn, &manifest).unwrap();

        assert_eq!(plan.to_process.len(), 1);
        assert!(matches!(
            plan.to_process[0].1,
            FileStatus::Modified { last_byte_offset: 300 }
        ));
    }

    #[test]
    fn test_deleted_files() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        mark_indexed(&conn, "/a.jsonl", 1000, 500, 500).unwrap();
        mark_indexed(&conn, "/b.jsonl", 1000, 500, 500).unwrap();

        let manifest = vec![test_entry("/a.jsonl", 1000, 500)];
        let plan = detect_changes(&conn, &manifest).unwrap();

        assert_eq!(plan.deleted_paths.len(), 1);
        assert_eq!(plan.deleted_paths[0], "/b.jsonl");
    }
}
