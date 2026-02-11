use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Top-level JSONL message (tagged enum via `type` field)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum SessionMessage {
    #[serde(rename = "user")]
    User(MessageEnvelope),
    #[serde(rename = "assistant")]
    Assistant(MessageEnvelope),
    #[serde(rename = "progress")]
    Progress(ProgressEnvelope),
    #[serde(rename = "system")]
    System(SystemEnvelope),
    #[serde(rename = "summary")]
    Summary(SummaryEnvelope),
    #[serde(rename = "file-history-snapshot")]
    FileHistorySnapshot(serde_json::Value),
    #[serde(rename = "queue-operation")]
    QueueOperation(serde_json::Value),
}

// ---------------------------------------------------------------------------
// Message envelope (shared between user + assistant)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageEnvelope {
    pub uuid: String,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub timestamp: String,
    pub cwd: Option<String>,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
    pub version: Option<String>,
    pub slug: Option<String>,
    #[serde(rename = "isSidechain")]
    pub is_sidechain: Option<bool>,
    pub message: MessageContent,
}

// ---------------------------------------------------------------------------
// Message content
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageContent {
    pub role: String,
    pub model: Option<String>,
    pub id: Option<String>,
    pub content: ContentValue,
    pub stop_reason: Option<String>,
    pub usage: Option<Usage>,
}

/// Content can be either a plain string (user text) or an array of content blocks.
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ContentValue {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
}

// ---------------------------------------------------------------------------
// Content blocks (tagged enum via `type`)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: serde_json::Value,
    },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
}

// ---------------------------------------------------------------------------
// Progress envelope
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
pub struct ProgressEnvelope {
    pub uuid: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub timestamp: String,
    #[serde(rename = "toolUseID")]
    pub tool_use_id: Option<String>,
    pub data: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// System envelope
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
pub struct SystemEnvelope {
    pub uuid: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub timestamp: String,
    pub subtype: Option<String>,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<u64>,
    pub content: Option<String>,
}

// ---------------------------------------------------------------------------
// Summary envelope
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
pub struct SummaryEnvelope {
    pub summary: String,
    #[serde(rename = "leafUuid")]
    pub leaf_uuid: Option<String>,
}

// ---------------------------------------------------------------------------
// Session index (from sessions-index.json)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionIndex {
    pub version: Option<u32>,
    pub entries: Vec<SessionIndexEntry>,
    #[serde(rename = "originalPath")]
    pub original_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionIndexEntry {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "fullPath")]
    pub full_path: String,
    #[serde(rename = "fileMtime")]
    pub file_mtime: Option<u64>,
    #[serde(rename = "firstPrompt")]
    pub first_prompt: Option<String>,
    pub summary: Option<String>,
    #[serde(rename = "messageCount")]
    pub message_count: Option<u32>,
    pub created: Option<String>,
    pub modified: Option<String>,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
    #[serde(rename = "projectPath")]
    pub project_path: Option<String>,
    #[serde(rename = "isSidechain")]
    pub is_sidechain: Option<bool>,
}

// ---------------------------------------------------------------------------
// Task record (from tasks/*.json)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskRecord {
    pub id: String,
    pub subject: String,
    pub description: String,
    #[serde(rename = "activeForm")]
    pub active_form: Option<String>,
    pub status: String,
    #[serde(default)]
    pub blocks: Vec<String>,
    #[serde(rename = "blockedBy", default)]
    pub blocked_by: Vec<String>,
}

// ---------------------------------------------------------------------------
// Session facet (from usage-data/facets/*.json)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionFacet {
    #[serde(rename = "session_id")]
    pub session_id: Option<String>,
    pub underlying_goal: Option<String>,
    pub goal_categories: Option<HashMap<String, u32>>,
    pub outcome: Option<String>,
    pub user_satisfaction_counts: Option<HashMap<String, u32>>,
    pub claude_helpfulness: Option<String>,
    pub session_type: Option<String>,
    pub friction_counts: Option<HashMap<String, u32>>,
    pub friction_detail: Option<String>,
    pub primary_success: Option<String>,
    pub brief_summary: Option<String>,
}

// ---------------------------------------------------------------------------
// Stats cache (from stats-cache.json)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
pub struct StatsCache {
    pub version: u32,
    #[serde(rename = "lastComputedDate")]
    pub last_computed_date: Option<String>,
    #[serde(rename = "dailyActivity")]
    pub daily_activity: Vec<DailyActivity>,
    #[serde(rename = "modelUsage")]
    pub model_usage: HashMap<String, ModelUsageStats>,
    #[serde(rename = "totalSessions")]
    pub total_sessions: Option<u64>,
    #[serde(rename = "totalMessages")]
    pub total_messages: Option<u64>,
    #[serde(rename = "longestSession")]
    pub longest_session: Option<LongestSession>,
    #[serde(rename = "firstSessionDate")]
    pub first_session_date: Option<String>,
    #[serde(rename = "hourCounts")]
    pub hour_counts: Option<HashMap<String, u64>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DailyActivity {
    pub date: String,
    #[serde(rename = "messageCount")]
    pub message_count: Option<u64>,
    #[serde(rename = "sessionCount")]
    pub session_count: Option<u64>,
    #[serde(rename = "toolCallCount")]
    pub tool_call_count: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModelUsageStats {
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_creation_tokens: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LongestSession {
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    #[serde(rename = "messageCount")]
    pub message_count: Option<u64>,
    pub project: Option<String>,
}
