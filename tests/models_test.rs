use blacklight::models::*;

#[test]
fn test_deserialize_user_message() {
    let json = r#"{
        "type": "user",
        "uuid": "u-123",
        "sessionId": "sess-1",
        "timestamp": "2026-01-15T10:00:00Z",
        "cwd": "/Users/test/project",
        "gitBranch": "main",
        "message": {
            "role": "user",
            "content": "Hello, can you help me?"
        }
    }"#;

    let msg: SessionMessage = serde_json::from_str(json).unwrap();
    match msg {
        SessionMessage::User(env) => {
            assert_eq!(env.uuid, "u-123");
            assert_eq!(env.session_id, "sess-1");
            assert_eq!(env.cwd, Some("/Users/test/project".to_string()));
            assert_eq!(env.git_branch, Some("main".to_string()));
            match env.message.content {
                ContentValue::Text(t) => assert_eq!(t, "Hello, can you help me?"),
                _ => panic!("expected text content"),
            }
        }
        _ => panic!("expected User variant"),
    }
}

#[test]
fn test_deserialize_assistant_message_with_blocks() {
    let json = r#"{
        "type": "assistant",
        "uuid": "a-456",
        "sessionId": "sess-1",
        "timestamp": "2026-01-15T10:00:01Z",
        "message": {
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {"type": "thinking", "thinking": "Let me think about this..."},
                {"type": "text", "text": "Here's what I found."},
                {"type": "tool_use", "id": "toolu_abc", "name": "Read", "input": {"file_path": "/src/main.rs"}}
            ],
            "stop_reason": "tool_use",
            "usage": {
                "input_tokens": 1000,
                "output_tokens": 500
            }
        }
    }"#;

    let msg: SessionMessage = serde_json::from_str(json).unwrap();
    match msg {
        SessionMessage::Assistant(env) => {
            assert_eq!(env.uuid, "a-456");
            assert_eq!(env.message.model, Some("claude-sonnet-4-5-20250929".to_string()));
            assert_eq!(env.message.stop_reason, Some("tool_use".to_string()));
            match &env.message.content {
                ContentValue::Blocks(blocks) => {
                    assert_eq!(blocks.len(), 3);
                    match &blocks[0] {
                        ContentBlock::Thinking { thinking } => {
                            assert_eq!(thinking, "Let me think about this...");
                        }
                        _ => panic!("expected Thinking block"),
                    }
                    match &blocks[1] {
                        ContentBlock::Text { text } => {
                            assert_eq!(text, "Here's what I found.");
                        }
                        _ => panic!("expected Text block"),
                    }
                    match &blocks[2] {
                        ContentBlock::ToolUse { id, name, input } => {
                            assert_eq!(id, "toolu_abc");
                            assert_eq!(name, "Read");
                            assert_eq!(input["file_path"], "/src/main.rs");
                        }
                        _ => panic!("expected ToolUse block"),
                    }
                }
                _ => panic!("expected Blocks content"),
            }
            let usage = env.message.usage.unwrap();
            assert_eq!(usage.input_tokens, Some(1000));
            assert_eq!(usage.output_tokens, Some(500));
        }
        _ => panic!("expected Assistant variant"),
    }
}

#[test]
fn test_deserialize_user_message_with_tool_result() {
    let json = r#"{
        "type": "user",
        "uuid": "u-789",
        "sessionId": "sess-1",
        "timestamp": "2026-01-15T10:00:02Z",
        "message": {
            "role": "user",
            "content": [
                {"type": "tool_result", "tool_use_id": "toolu_abc", "content": "fn main() { ... }"}
            ]
        }
    }"#;

    let msg: SessionMessage = serde_json::from_str(json).unwrap();
    match msg {
        SessionMessage::User(env) => {
            match &env.message.content {
                ContentValue::Blocks(blocks) => {
                    assert_eq!(blocks.len(), 1);
                    match &blocks[0] {
                        ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                        } => {
                            assert_eq!(tool_use_id, "toolu_abc");
                            assert_eq!(content, "fn main() { ... }");
                        }
                        _ => panic!("expected ToolResult block"),
                    }
                }
                _ => panic!("expected Blocks content"),
            }
        }
        _ => panic!("expected User variant"),
    }
}

#[test]
fn test_deserialize_progress_message() {
    let json = r#"{
        "type": "progress",
        "uuid": "p-100",
        "sessionId": "sess-1",
        "timestamp": "2026-01-15T10:00:03Z",
        "toolUseID": "toolu_abc",
        "data": {"type": "tool_status", "tool": "Read"}
    }"#;

    let msg: SessionMessage = serde_json::from_str(json).unwrap();
    match msg {
        SessionMessage::Progress(env) => {
            assert_eq!(env.uuid, "p-100");
            assert_eq!(env.tool_use_id, Some("toolu_abc".to_string()));
            let data = env.data.unwrap();
            assert_eq!(data["type"], "tool_status");
            assert_eq!(data["tool"], "Read");
        }
        _ => panic!("expected Progress variant"),
    }
}

#[test]
fn test_deserialize_system_message() {
    let json = r#"{
        "type": "system",
        "uuid": "s-200",
        "sessionId": "sess-1",
        "timestamp": "2026-01-15T10:01:00Z",
        "subtype": "turn_duration",
        "durationMs": 3500
    }"#;

    let msg: SessionMessage = serde_json::from_str(json).unwrap();
    match msg {
        SessionMessage::System(env) => {
            assert_eq!(env.uuid, "s-200");
            assert_eq!(env.subtype, Some("turn_duration".to_string()));
            assert_eq!(env.duration_ms, Some(3500));
        }
        _ => panic!("expected System variant"),
    }
}

#[test]
fn test_deserialize_summary_message() {
    let json = r#"{
        "type": "summary",
        "summary": "The user asked for help with authentication.",
        "leafUuid": "a-456"
    }"#;

    let msg: SessionMessage = serde_json::from_str(json).unwrap();
    match msg {
        SessionMessage::Summary(env) => {
            assert_eq!(env.summary, "The user asked for help with authentication.");
            assert_eq!(env.leaf_uuid, Some("a-456".to_string()));
        }
        _ => panic!("expected Summary variant"),
    }
}

#[test]
fn test_deserialize_file_history_snapshot() {
    let json = r#"{
        "type": "file-history-snapshot",
        "trackedFileBackups": {}
    }"#;

    let msg: SessionMessage = serde_json::from_str(json).unwrap();
    match msg {
        SessionMessage::FileHistorySnapshot(_) => {}
        _ => panic!("expected FileHistorySnapshot variant"),
    }
}

#[test]
fn test_deserialize_queue_operation() {
    let json = r#"{
        "type": "queue-operation",
        "operation": "enqueue",
        "data": {}
    }"#;

    let msg: SessionMessage = serde_json::from_str(json).unwrap();
    match msg {
        SessionMessage::QueueOperation(_) => {}
        _ => panic!("expected QueueOperation variant"),
    }
}

#[test]
fn test_unknown_fields_ignored() {
    let json = r#"{
        "type": "user",
        "uuid": "u-999",
        "sessionId": "sess-1",
        "timestamp": "2026-01-15T10:00:00Z",
        "unknownField": "should be ignored",
        "anotherUnknown": 42,
        "message": {
            "role": "user",
            "content": "test",
            "extraField": true
        }
    }"#;

    let msg: SessionMessage = serde_json::from_str(json).unwrap();
    match msg {
        SessionMessage::User(env) => {
            assert_eq!(env.uuid, "u-999");
        }
        _ => panic!("expected User variant"),
    }
}

#[test]
fn test_missing_optional_fields() {
    let json = r#"{
        "type": "user",
        "uuid": "u-min",
        "sessionId": "sess-1",
        "timestamp": "2026-01-15T10:00:00Z",
        "message": {
            "role": "user",
            "content": "hello"
        }
    }"#;

    let msg: SessionMessage = serde_json::from_str(json).unwrap();
    match msg {
        SessionMessage::User(env) => {
            assert!(env.parent_uuid.is_none());
            assert!(env.cwd.is_none());
            assert!(env.git_branch.is_none());
            assert!(env.version.is_none());
            assert!(env.slug.is_none());
            assert!(env.is_sidechain.is_none());
            assert!(env.message.model.is_none());
            assert!(env.message.usage.is_none());
            assert!(env.message.stop_reason.is_none());
        }
        _ => panic!("expected User variant"),
    }
}

#[test]
fn test_deserialize_session_index() {
    let json = r#"{
        "version": 1,
        "entries": [
            {
                "sessionId": "sess-abc",
                "fullPath": "/Users/test/.claude/projects/abc/sess-abc.jsonl",
                "fileMtime": 1707000000000,
                "firstPrompt": "Help me fix the auth bug",
                "summary": "Fixed authentication issue",
                "messageCount": 42,
                "created": "2026-01-15T10:00:00Z",
                "modified": "2026-01-15T11:00:00Z",
                "gitBranch": "fix-auth",
                "projectPath": "/Users/test/project",
                "isSidechain": false
            }
        ],
        "originalPath": "/Users/test/.claude/projects/abc"
    }"#;

    let index: SessionIndex = serde_json::from_str(json).unwrap();
    assert_eq!(index.version, Some(1));
    assert_eq!(index.entries.len(), 1);
    let entry = &index.entries[0];
    assert_eq!(entry.session_id, "sess-abc");
    assert_eq!(entry.first_prompt, Some("Help me fix the auth bug".to_string()));
    assert_eq!(entry.message_count, Some(42));
    assert_eq!(entry.git_branch, Some("fix-auth".to_string()));
}

#[test]
fn test_deserialize_task_record() {
    let json = r#"{
        "id": "1",
        "subject": "Fix the login bug",
        "description": "Users can't login with email",
        "activeForm": "Fixing the login bug",
        "status": "in_progress",
        "blocks": ["2", "3"],
        "blockedBy": []
    }"#;

    let task: TaskRecord = serde_json::from_str(json).unwrap();
    assert_eq!(task.id, "1");
    assert_eq!(task.subject, "Fix the login bug");
    assert_eq!(task.status, "in_progress");
    assert_eq!(task.blocks, vec!["2", "3"]);
    assert!(task.blocked_by.is_empty());
}

#[test]
fn test_deserialize_session_facet() {
    let json = r#"{
        "session_id": "sess-abc",
        "underlying_goal": "Fix authentication",
        "goal_categories": {"bug_fix": 1, "security": 1},
        "outcome": "fully_achieved",
        "claude_helpfulness": "essential",
        "session_type": "single_task",
        "friction_counts": {"wrong_approach": 1},
        "primary_success": "true",
        "brief_summary": "Fixed the auth bug"
    }"#;

    let facet: SessionFacet = serde_json::from_str(json).unwrap();
    assert_eq!(facet.session_id, Some("sess-abc".to_string()));
    assert_eq!(facet.outcome, Some("fully_achieved".to_string()));
    let categories = facet.goal_categories.unwrap();
    assert_eq!(categories["bug_fix"], 1);
    let friction = facet.friction_counts.unwrap();
    assert_eq!(friction["wrong_approach"], 1);
}

#[test]
fn test_deserialize_stats_cache() {
    let json = r#"{
        "version": 2,
        "lastComputedDate": "2026-02-10",
        "dailyActivity": [
            {"date": "2026-02-09", "messageCount": 100, "sessionCount": 5, "toolCallCount": 30}
        ],
        "modelUsage": {
            "claude-sonnet-4-5-20250929": {
                "input_tokens": 1000000,
                "output_tokens": 500000,
                "cache_read_tokens": 3700000000,
                "cache_creation_tokens": 200000
            }
        },
        "totalSessions": 363,
        "totalMessages": 169701,
        "longestSession": {
            "sessionId": "sess-long",
            "messageCount": 2000,
            "project": "echo"
        },
        "firstSessionDate": "2026-01-11",
        "hourCounts": {"10": 500, "14": 800}
    }"#;

    let stats: StatsCache = serde_json::from_str(json).unwrap();
    assert_eq!(stats.version, 2);
    assert_eq!(stats.total_sessions, Some(363));
    assert_eq!(stats.total_messages, Some(169701));
    assert_eq!(stats.daily_activity.len(), 1);
    assert_eq!(stats.daily_activity[0].message_count, Some(100));

    let model = &stats.model_usage["claude-sonnet-4-5-20250929"];
    assert_eq!(model.cache_read_tokens, Some(3_700_000_000));

    let longest = stats.longest_session.unwrap();
    assert_eq!(longest.message_count, Some(2000));
    assert_eq!(longest.project, Some("echo".to_string()));

    let hours = stats.hour_counts.unwrap();
    assert_eq!(hours["14"], 800);
}

#[test]
fn test_deserialize_roundtrip() {
    let json = r#"{
        "type": "user",
        "uuid": "u-rt",
        "sessionId": "sess-1",
        "timestamp": "2026-01-15T10:00:00Z",
        "cwd": "/test",
        "message": {
            "role": "user",
            "content": "roundtrip test"
        }
    }"#;

    let msg: SessionMessage = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&msg).unwrap();
    let msg2: SessionMessage = serde_json::from_str(&serialized).unwrap();

    match (msg, msg2) {
        (SessionMessage::User(a), SessionMessage::User(b)) => {
            assert_eq!(a.uuid, b.uuid);
            assert_eq!(a.session_id, b.session_id);
        }
        _ => panic!("roundtrip should preserve variant"),
    }
}
