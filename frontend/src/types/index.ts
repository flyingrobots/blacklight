export interface Paginated<T> {
  items: T[]
  total: number
  limit: number
  offset: number
}

export interface SessionSummary {
  id: string
  project_path: string
  project_slug: string
  first_prompt: string | null
  summary: string | null
  message_count: number | null
  created_at: string
  modified_at: string
  git_branch: string | null
  claude_version: string | null
  is_sidechain: boolean
  outcome: string | null
  brief_summary: string | null
}

export interface SessionOutcome {
  underlying_goal: string | null
  outcome: string | null
  helpfulness: string | null
  session_type: string | null
  primary_success: string | null
  friction_detail: string | null
  brief_summary: string | null
}

export interface SessionDetail {
  id: string
  project_path: string
  project_slug: string
  first_prompt: string | null
  summary: string | null
  message_count: number | null
  created_at: string
  modified_at: string
  git_branch: string | null
  claude_version: string | null
  is_sidechain: boolean
  outcome: SessionOutcome | null
}

export interface ContentBlockDetail {
  block_index: number
  block_type: string
  content: string | null
  tool_name: string | null
  tool_use_id: string | null
  tool_input: string | null
}

export interface MessageDetail {
  id: string
  session_id: string
  parent_id: string | null
  type: string
  timestamp: string
  model: string | null
  stop_reason: string | null
  duration_ms: number | null
  content_blocks: ContentBlockDetail[]
}

export interface ToolCallDetail {
  id: string
  tool_name: string
  timestamp: string
  input: string | null
  output: string | null
}

export interface FileReference {
  file_path: string
  operation: string
  session_id: string
  message_id: string
}

export interface SearchHit {
  hash: string
  kind: string
  snippet: string
  rank: number
  session_id: string | null
  session_summary: string | null
  message_id: string | null
  message_type: string | null
}

export interface AnalyticsOverview {
  total_sessions: number
  total_messages: number
  total_blobs: number
  total_blob_bytes: number
  db_size_bytes: number
  first_session: string | null
  last_session: string | null
}

export interface DailyStats {
  date: string
  message_count: number | null
  session_count: number | null
  tool_call_count: number | null
}

export interface ModelUsage {
  model: string
  input_tokens: number | null
  output_tokens: number | null
  cache_read_tokens: number | null
  cache_creation_tokens: number | null
}

export interface ToolFrequency {
  tool_name: string
  call_count: number
}

export interface ProjectBreakdown {
  project_slug: string
  session_count: number
  message_count: number
  tool_call_count: number
}

export interface OutcomeStats {
  outcome: string
  count: number
}

export interface StorageOverview {
  total_blobs: number
  total_bytes: number
  unique_blobs: number
  total_references: number
  dedup_ratio: number
  by_kind: StorageByKind[]
}

export interface StorageByKind {
  kind: string
  blob_count: number
  total_bytes: number
}

export interface FileProvenance {
  file_path: string
  session_count: number
  operations: string[]
  last_session_id: string | null
}

export interface IndexCoverage {
  source_files: number
  source_bytes: number
  indexed_files: number
  indexed_bytes: number
  file_pct: number
  byte_pct: number
  blobs_stored: number
  blobs_searchable: number
  search_pct: number
  sessions_with_outcomes: number
  total_sessions: number
  outcome_pct: number
  messages_with_content: number
  total_messages: number
  by_kind: CoverageByKind[]
}

export interface CoverageByKind {
  kind: string
  file_count: number
  total_bytes: number
}

export interface ProjectDetail {
  project_slug: string
  project_path: string
  session_count: number
  message_count: number
  tool_call_count: number
  first_session: string | null
  last_session: string | null
  files_touched: number
  top_tools: ToolFrequency[]
}

export interface ContentBlob {
  hash: string
  content: string
  size: number
  kind: string
}

export type IndexerStatus = 'idle' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled'

export interface IndexProgress {
  phase: string
  files_total: number
  files_done: number
  messages_processed: number
  blobs_inserted: number
}

export interface IndexReport {
  sessions_parsed: number
  messages_processed: number
  messages_skipped: number
  parse_errors: number
  blobs_inserted: number
  tool_calls_inserted: number
  tasks_parsed: number
  facets_parsed: number
  plans_parsed: number
  history_entries: number
  files_processed: number
  files_unchanged: number
  elapsed_secs: number
}

export interface IndexerStatusResponse {
  status: IndexerStatus
  progress: IndexProgress
  latest_report: IndexReport | null
  error_message: string | null
}
