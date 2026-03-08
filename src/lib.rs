pub mod config;
pub mod content;
pub mod db;
pub mod enrich;
pub mod error;
pub mod indexer;
pub mod models;
pub mod notifications;
pub mod server;

/// Current version of the indexing logic. Increment to trigger re-index suggestions.
pub const INDEX_VERSION: i32 = 1;
/// Current version of the enrichment logic. Increment to trigger re-enrichment suggestions.
pub const ENRICH_VERSION: i32 = 1;

#[cfg(test)]
mod tests {
    use crate::server::responses::*;
    use crate::server::state::*;
    use crate::indexer::{IndexProgress, IndexReport};
    use crate::enrich::EnrichReport;
    use ts_rs::TS;

    #[test]
    fn export_bindings() {
        // This test will export TS bindings when run.
        // We just need to touch one of the types or call a method if needed,
        // but TS-RS usually exports on derive. 
        // Actually, calling .export() is cleaner.
        
        SessionSummary::export().expect("failed to export SessionSummary");
        SessionDetail::export().expect("failed to export SessionDetail");
        MessageDetail::export().expect("failed to export MessageDetail");
        IndexerStatusResponse::export().expect("failed to export IndexerStatusResponse");
        IndexerState::export().expect("failed to export IndexerState");
        IndexProgress::export().expect("failed to export IndexProgress");
        IndexReport::export().expect("failed to export IndexReport");
        IndexRun::export().expect("failed to export IndexRun");
        EnricherStatusResponse::export().expect("failed to export EnricherStatusResponse");
        EnrichReport::export().expect("failed to export EnrichReport");
        MigrationStatusResponse::export().expect("failed to export MigrationStatusResponse");
        MigrationProgress::export().expect("failed to export MigrationProgress");
    }
}
