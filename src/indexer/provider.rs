use anyhow::Result;
use rusqlite::Connection;
use crate::indexer::scanner::{FileEntry, FileKind};
use crate::indexer::redact::Redactor;

/// Stats returned after processing a single file.
#[derive(Clone, Debug, Default)]
pub struct ProcessStats {
    pub messages_processed: usize,
    pub messages_skipped: usize,
    pub parse_errors: usize,
    pub blobs_inserted: usize,
    pub tool_calls_inserted: usize,
    pub sessions_parsed: usize,
}

pub trait SourceProvider: Send + Sync {
    fn name(&self) -> &'static str;
    
    /// Returns true if this provider can handle the given file kind.
    fn can_handle(&self, kind: &FileKind) -> bool;

    /// Process metadata/index files.
    fn process_metadata(&self, _conn: &mut Connection, _entry: &FileEntry) -> Result<usize> {
        Ok(0)
    }

    /// Process content files (streaming or full).
    fn process_content(
        &self, 
        _conn: &mut Connection, 
        entry: &FileEntry, 
        _start_offset: u64,
        _redactor: Option<&Redactor>
    ) -> Result<(ProcessStats, u64)> {
        Ok((ProcessStats::default(), entry.size_bytes))
    }
}
