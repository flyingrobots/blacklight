use anyhow::{Context, Result};
use rusqlite::{params, Connection};

/// Minimum content size (in bytes) to store in the content-addressable blob store.
/// Smaller content is stored inline — the overhead of a hash lookup exceeds savings.
pub const DEDUP_THRESHOLD: usize = 256;

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/// Compute a BLAKE3 hash of the given text, returning a 64-char hex string.
pub fn hash_content(content: &str) -> String {
    blake3::hash(content.as_bytes()).to_hex().to_string()
}

/// Compute a BLAKE3 hash of the given bytes, returning a 64-char hex string.
pub fn hash_content_bytes(content: &[u8]) -> String {
    blake3::hash(content).to_hex().to_string()
}

/// Returns true if content is large enough to be worth deduplicating via the blob store.
pub fn should_dedup(content: &str) -> bool {
    content.len() >= DEDUP_THRESHOLD
}

// ---------------------------------------------------------------------------
// Content store types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ContentBlob {
    pub hash: String,
    pub content: String,
    pub size: i64,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct BlobReference {
    pub hash: String,
    pub message_id: String,
    pub context: String,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub hash: String,
    pub kind: String,
    pub snippet: String,
    pub rank: f64,
}

// ---------------------------------------------------------------------------
// Content store DB operations
// ---------------------------------------------------------------------------

/// Insert a blob into the content store. Returns true if newly inserted, false if it
/// already existed (dedup hit).
pub fn insert_blob(
    conn: &Connection,
    hash: &str,
    content: &str,
    size: i64,
    kind: &str,
) -> Result<bool> {
    let changes = conn
        .execute(
            "INSERT OR IGNORE INTO content_store (hash, content, size, kind) VALUES (?1, ?2, ?3, ?4)",
            params![hash, content, size, kind],
        )
        .context("failed to insert blob")?;
    Ok(changes > 0)
}

/// Retrieve a blob from the content store by hash.
pub fn get_blob(conn: &Connection, hash: &str) -> Result<Option<ContentBlob>> {
    let mut stmt = conn.prepare(
        "SELECT hash, content, size, kind FROM content_store WHERE hash = ?1",
    )?;
    let result = stmt
        .query_row(params![hash], |row| {
            Ok(ContentBlob {
                hash: row.get(0)?,
                content: row.get(1)?,
                size: row.get(2)?,
                kind: row.get(3)?,
            })
        })
        .optional()?;
    Ok(result)
}

/// Check whether a blob exists in the content store.
pub fn blob_exists(conn: &Connection, hash: &str) -> Result<bool> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM content_store WHERE hash = ?1)",
        params![hash],
        |row| row.get(0),
    )?;
    Ok(exists)
}

/// Insert a reference linking a blob to a message.
pub fn insert_blob_reference(
    conn: &Connection,
    hash: &str,
    message_id: &str,
    context: &str,
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO blob_references (hash, message_id, context) VALUES (?1, ?2, ?3)",
        params![hash, message_id, context],
    )
    .context("failed to insert blob reference")?;
    Ok(())
}

/// Get all references for a given blob hash.
pub fn get_blob_references(conn: &Connection, hash: &str) -> Result<Vec<BlobReference>> {
    let mut stmt = conn.prepare(
        "SELECT hash, message_id, context FROM blob_references WHERE hash = ?1",
    )?;
    let refs = stmt
        .query_map(params![hash], |row| {
            Ok(BlobReference {
                hash: row.get(0)?,
                message_id: row.get(1)?,
                context: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(refs)
}

/// Batch insert multiple blobs in a single transaction. Returns the count of newly
/// inserted blobs (excludes dedup hits).
pub fn insert_blobs_batch(conn: &Connection, blobs: &[ContentBlob]) -> Result<usize> {
    let tx = conn.unchecked_transaction()?;
    let mut inserted = 0usize;
    {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO content_store (hash, content, size, kind) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for blob in blobs {
            let changes = stmt.execute(params![blob.hash, blob.content, blob.size, blob.kind])?;
            if changes > 0 {
                inserted += 1;
            }
        }
    }
    tx.commit()?;
    Ok(inserted)
}

// ---------------------------------------------------------------------------
// FTS5 operations
// ---------------------------------------------------------------------------

/// Index content in the FTS5 table. Skips if the hash is already indexed.
pub fn index_content(conn: &Connection, hash: &str, kind: &str, content: &str) -> Result<()> {
    let already_indexed: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM fts_content WHERE hash = ?1)",
        params![hash],
        |row| row.get(0),
    )?;

    if !already_indexed {
        conn.execute(
            "INSERT INTO fts_content (hash, kind, content) VALUES (?1, ?2, ?3)",
            params![hash, kind, content],
        )
        .context("failed to index content in FTS5")?;
    }

    Ok(())
}

/// Search the FTS5 index with BM25 ranking, returning snippets with highlighted matches.
pub fn search(
    conn: &Connection,
    query: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<SearchResult>> {
    let mut stmt = conn
        .prepare(
            "SELECT hash, kind,
                    snippet(fts_content, 2, '<mark>', '</mark>', '...', 64) as snippet,
                    bm25(fts_content) as rank
             FROM fts_content
             WHERE fts_content MATCH ?1
             ORDER BY rank
             LIMIT ?2 OFFSET ?3",
        )
        .context("failed to prepare FTS5 search query")?;

    let results = stmt
        .query_map(params![query, limit, offset], |row| {
            Ok(SearchResult {
                hash: row.get(0)?,
                kind: row.get(1)?,
                snippet: row.get(2)?,
                rank: row.get(3)?,
            })
        })
        .map_err(|e| anyhow::anyhow!("FTS5 search failed (check query syntax): {e}"))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("FTS5 search failed: {e}"))?;

    Ok(results)
}

/// Helper trait to make rusqlite's optional query results ergonomic.
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
