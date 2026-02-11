use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const BUF_SIZE: usize = 64 * 1024; // 64KB

/// Streaming JSONL reader with byte offset tracking and seek support.
pub struct JsonlReader {
    reader: BufReader<File>,
    byte_offset: u64,
    line_number: u64,
    path: PathBuf,
}

impl JsonlReader {
    /// Open a JSONL file and seek to the given byte offset.
    pub fn open(path: &Path, start_offset: u64) -> Result<Self> {
        let mut file = File::open(path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        if start_offset > 0 {
            file.seek(SeekFrom::Start(start_offset))
                .with_context(|| format!("failed to seek to offset {start_offset} in {}", path.display()))?;
        }

        let reader = BufReader::with_capacity(BUF_SIZE, file);

        Ok(Self {
            reader,
            byte_offset: start_offset,
            line_number: 0,
            path: path.to_path_buf(),
        })
    }

    /// Read the next non-empty line. Returns (line_content, byte_offset_after_line).
    /// Returns None at EOF.
    pub fn next_line(&mut self) -> Result<Option<(String, u64)>> {
        let mut buf = String::new();
        loop {
            buf.clear();
            let bytes_read = self
                .reader
                .read_line(&mut buf)
                .with_context(|| format!("failed to read line from {}", self.path.display()))?;

            if bytes_read == 0 {
                return Ok(None); // EOF
            }

            self.byte_offset += bytes_read as u64;
            self.line_number += 1;

            let trimmed = buf.trim();
            if trimmed.is_empty() {
                continue; // Skip empty lines
            }

            if self.line_number.is_multiple_of(1000) {
                tracing::debug!(
                    "{}:{} offset={}",
                    self.path.display(),
                    self.line_number,
                    self.byte_offset
                );
            }

            return Ok(Some((trimmed.to_string(), self.byte_offset)));
        }
    }

    /// Current byte offset into the file.
    pub fn byte_offset(&self) -> u64 {
        self.byte_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_all_lines() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, r#"{{"type":"user","data":"hello"}}"#).unwrap();
        writeln!(f, r#"{{"type":"assistant","data":"world"}}"#).unwrap();
        f.flush().unwrap();

        let mut reader = JsonlReader::open(f.path(), 0).unwrap();
        let (line1, _) = reader.next_line().unwrap().unwrap();
        assert!(line1.contains("user"));
        let (line2, _) = reader.next_line().unwrap().unwrap();
        assert!(line2.contains("assistant"));
        assert!(reader.next_line().unwrap().is_none());
    }

    #[test]
    fn test_skip_empty_lines() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, r#"{{"line":1}}"#).unwrap();
        writeln!(f).unwrap(); // empty line
        writeln!(f, r#"{{"line":2}}"#).unwrap();
        f.flush().unwrap();

        let mut reader = JsonlReader::open(f.path(), 0).unwrap();
        let (line1, _) = reader.next_line().unwrap().unwrap();
        assert!(line1.contains("\"line\":1"));
        let (line2, _) = reader.next_line().unwrap().unwrap();
        assert!(line2.contains("\"line\":2"));
        assert!(reader.next_line().unwrap().is_none());
    }

    #[test]
    fn test_seek_to_offset() {
        let mut f = NamedTempFile::new().unwrap();
        let line1 = r#"{"line":1}"#;
        let line2 = r#"{"line":2}"#;
        writeln!(f, "{line1}").unwrap();
        writeln!(f, "{line2}").unwrap();
        f.flush().unwrap();

        // Offset past the first line (line1 + newline)
        let offset = (line1.len() + 1) as u64;
        let mut reader = JsonlReader::open(f.path(), offset).unwrap();
        let (line, _) = reader.next_line().unwrap().unwrap();
        assert!(line.contains("\"line\":2"));
        assert!(reader.next_line().unwrap().is_none());
    }

    #[test]
    fn test_byte_offset_tracking() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, r#"{{"a":1}}"#).unwrap(); // 8 chars + newline = 9 bytes
        writeln!(f, r#"{{"b":2}}"#).unwrap();
        f.flush().unwrap();

        let mut reader = JsonlReader::open(f.path(), 0).unwrap();
        assert_eq!(reader.byte_offset(), 0);
        let (_, offset_after) = reader.next_line().unwrap().unwrap();
        assert!(offset_after > 0);
    }
}
