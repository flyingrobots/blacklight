use serde::Serialize;

use crate::enrich::EnrichReport;
use crate::indexer::{IndexProgress, IndexReport};
use crate::server::state::{EnricherStatus, IndexerStatus};

/// Indexer status response.
#[derive(Debug, Serialize)]
pub struct IndexerStatusResponse {
    pub status: IndexerStatus,
    pub progress: IndexProgress,
    pub latest_report: Option<IndexReport>,
    pub error_message: Option<String>,
}

/// Enricher status response.
#[derive(Debug, Serialize)]
pub struct EnricherStatusResponse {
    pub status: EnricherStatus,
    pub sessions_total: usize,
    pub sessions_done: usize,
    pub sessions_failed: usize,
    pub latest_report: Option<EnrichReport>,
    pub error_message: Option<String>,
}

/// Review queue item for pending enrichments.
#[derive(Debug, Serialize)]
pub struct ReviewItem {
    pub session_id: String,
    pub title: String,
    pub summary: String,
    pub enriched_at: String,
    pub model_used: Option<String>,
    pub project_slug: String,
    pub session_created_at: String,
    pub first_prompt: Option<String>,
    pub tags: Vec<SessionTag>,
}

/// Schedule configuration response.
#[derive(Debug, Serialize)]
pub struct ScheduleConfigResponse {
    pub enabled: bool,
    pub interval_minutes: i32,
    pub run_enrichment: bool,
    pub enrichment_concurrency: i32,
    pub updated_at: String,
}

/// Paginated response wrapper.
#[derive(Debug, Serialize)]
pub struct Paginated<T: Serialize> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Session list item.
#[derive(Debug, Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub project_path: String,
    pub project_slug: String,
    pub first_prompt: Option<String>,
    pub summary: Option<String>,
    pub message_count: Option<i64>,
    pub created_at: String,
    pub modified_at: String,
    pub git_branch: Option<String>,
    pub claude_version: Option<String>,
    pub is_sidechain: bool,
    pub outcome: Option<String>,
    pub brief_summary: Option<String>,
    pub enrichment_title: Option<String>,
    pub enrichment_summary: Option<String>,
    pub approval_status: Option<String>,
    pub tags: Vec<SessionTag>,
    pub source_name: Option<String>,
    pub source_kind: Option<String>,
    pub app_version: Option<String>,
    pub fingerprint: Option<String>,
}

/// Tag with confidence score from AI enrichment.
#[derive(Debug, Serialize)]
pub struct SessionTag {
    pub tag: String,
    pub confidence: f64,
}

/// Full session detail.
#[derive(Debug, Serialize)]
pub struct SessionDetail {
    pub id: String,
    pub project_path: String,
    pub project_slug: String,
    pub first_prompt: Option<String>,
    pub summary: Option<String>,
    pub message_count: Option<i64>,
    pub created_at: String,
    pub modified_at: String,
    pub git_branch: Option<String>,
    pub claude_version: Option<String>,
    pub is_sidechain: bool,
    pub outcome: Option<SessionOutcome>,
    pub enrichment_title: Option<String>,
    pub enrichment_summary: Option<String>,
    pub approval_status: Option<String>,
    pub tags: Vec<SessionTag>,
    pub source_name: Option<String>,
    pub source_kind: Option<String>,
    pub app_version: Option<String>,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionOutcome {
    pub underlying_goal: Option<String>,
    pub outcome: Option<String>,
    pub helpfulness: Option<String>,
    pub session_type: Option<String>,
    pub primary_success: Option<String>,
    pub friction_detail: Option<String>,
    pub brief_summary: Option<String>,
}

/// Message in a session thread.
#[derive(Debug, Serialize)]
pub struct MessageDetail {
    pub id: String,
    pub session_id: String,
    pub parent_id: Option<String>,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub timestamp: String,
    pub model: Option<String>,
    pub stop_reason: Option<String>,
    pub duration_ms: Option<i64>,
    pub content_blocks: Vec<ContentBlockDetail>,
}

/// Content block within a message.
#[derive(Debug, Serialize)]
pub struct ContentBlockDetail {
    pub block_index: i64,
    pub block_type: String,
    pub content: Option<String>,
    pub tool_name: Option<String>,
    pub tool_use_id: Option<String>,
    pub tool_input: Option<String>,
}

/// Tool call record.
#[derive(Debug, Serialize)]
pub struct ToolCallDetail {
    pub id: String,
    pub tool_name: String,
    pub timestamp: String,
    pub input: Option<String>,
    pub output: Option<String>,
}

/// File reference record.
#[derive(Debug, Serialize)]
pub struct FileReference {
    pub file_path: String,
    pub operation: String,
    pub session_id: String,
    pub message_id: String,
}

/// Search result enriched with session context.
#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub hash: String,
    pub kind: String,
    pub snippet: String,
    pub rank: f64,
    pub session_id: Option<String>,
    pub session_summary: Option<String>,
    pub message_id: Option<String>,
    pub message_type: Option<String>,
}

/// Overview stats for the analytics dashboard.
#[derive(Debug, Serialize)]
pub struct AnalyticsOverview {
    pub total_sessions: i64,
    pub total_messages: i64,
    pub total_blobs: i64,
    pub total_blob_bytes: i64,
    pub db_size_bytes: i64,
    pub first_session: Option<String>,
    pub last_session: Option<String>,
}

/// Daily activity stats.
#[derive(Debug, Serialize)]
pub struct DailyStats {
    pub date: String,
    pub message_count: Option<i64>,
    pub session_count: Option<i64>,
    pub tool_call_count: Option<i64>,
}

/// Model usage breakdown.
#[derive(Debug, Serialize)]
pub struct ModelUsage {
    pub model: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_creation_tokens: Option<i64>,
}

/// Tool frequency stats.
#[derive(Debug, Serialize)]
pub struct ToolFrequency {
    pub tool_name: String,
    pub call_count: i64,
}

/// Per-project breakdown.
#[derive(Debug, Serialize)]
pub struct ProjectBreakdown {
    pub project_slug: String,
    pub session_count: i64,
    pub message_count: i64,
    pub tool_call_count: i64,
}

/// Per-LLM breakdown (source_kind).
#[derive(Debug, Serialize)]
pub struct LlmBreakdown {
    pub source_kind: String,
    pub session_count: i64,
    pub message_count: i64,
    pub tool_call_count: i64,
}

/// Rich per-project detail for the Projects page.
#[derive(Debug, Serialize)]
pub struct ProjectDetail {
    pub project_slug: String,
    pub project_path: String,
    pub session_count: i64,
    pub message_count: i64,
    pub tool_call_count: i64,
    pub first_session: Option<String>,
    pub last_session: Option<String>,
    pub files_touched: i64,
    pub top_tools: Vec<ToolFrequency>,
}

/// Outcome distribution.
#[derive(Debug, Serialize)]
pub struct OutcomeStats {
    pub outcome: String,
    pub count: i64,
}

/// Storage overview.
#[derive(Debug, Serialize)]
pub struct StorageOverview {
    pub total_blobs: i64,
    pub total_bytes: i64,
    pub unique_blobs: i64,
    pub total_references: i64,
    pub dedup_ratio: f64,
    pub by_kind: Vec<StorageByKind>,
}

#[derive(Debug, Serialize)]
pub struct StorageByKind {
    pub kind: String,
    pub blob_count: i64,
    pub total_bytes: i64,
}

/// Index coverage — what % of the source data is indexed and searchable.
#[derive(Debug, Serialize)]
pub struct IndexCoverage {
    /// Files discovered by the scanner on disk
    pub source_files: i64,
    /// Total bytes of source files on disk
    pub source_bytes: i64,
    /// Files that have been indexed
    pub indexed_files: i64,
    /// Total bytes of indexed files
    pub indexed_bytes: i64,
    /// % of source files indexed (0-100)
    pub file_pct: f64,
    /// % of source bytes indexed (0-100)
    pub byte_pct: f64,
    /// Blobs in content_store
    pub blobs_stored: i64,
    /// Blobs indexed in FTS5 (searchable)
    pub blobs_searchable: i64,
    /// % of blobs that are searchable (0-100)
    pub search_pct: f64,
    /// Sessions with outcome data
    pub sessions_with_outcomes: i64,
    /// Total sessions
    pub total_sessions: i64,
    /// % of sessions with outcomes (0-100)
    pub outcome_pct: f64,
    /// Messages with at least one content block
    pub messages_with_content: i64,
    /// Total messages
    pub total_messages: i64,
    /// Breakdown of indexed files by kind
    pub by_kind: Vec<CoverageByKind>,
}

#[derive(Debug, Serialize)]
pub struct CoverageByKind {
    pub kind: String,
    pub file_count: i64,
    pub total_bytes: i64,
}

/// File provenance entry.
#[derive(Debug, Serialize)]
pub struct FileProvenance {
    pub file_path: String,
    pub session_count: i64,
    pub operations: Vec<String>,
    pub last_session_id: Option<String>,
}
