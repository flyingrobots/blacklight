pub mod config;
pub mod content;
pub mod db;
pub mod enrich;
pub mod indexer;
pub mod models;
pub mod notifications;
pub mod server;

/// Current version of the indexing logic. Increment to trigger re-index suggestions.
pub const INDEX_VERSION: i32 = 1;
/// Current version of the enrichment logic. Increment to trigger re-enrichment suggestions.
pub const ENRICH_VERSION: i32 = 1;
