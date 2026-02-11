use crate::content;
use crate::models::{ContentBlock, ContentValue, MessageEnvelope, SummaryEnvelope, SystemEnvelope};

use super::db_ops::{ContentBlockRow, FileRefRow, LineOps, MessageRow, ToolCallRow};
use super::file_paths::ToolUseTracker;

/// Fast byte-level check to skip progress and queue-operation messages without any JSON parsing.
pub fn is_skippable(line: &str) -> bool {
    line.contains("\"type\":\"progress\"")
        || line.contains("\"type\": \"progress\"")
        || line.contains("\"type\":\"queue-operation\"")
        || line.contains("\"type\": \"queue-operation\"")
}

/// Handle an assistant message. Returns LineOps with all DB operations needed.
pub fn handle_assistant(
    envelope: &MessageEnvelope,
    tracker: &mut ToolUseTracker,
) -> LineOps {
    let mut ops = LineOps::default();
    let msg_id = &envelope.uuid;
    let session_id = &envelope.session_id;
    let timestamp = &envelope.timestamp;

    ops.message = Some(MessageRow {
        id: msg_id.clone(),
        session_id: session_id.clone(),
        parent_id: envelope.parent_uuid.clone(),
        msg_type: "assistant".into(),
        timestamp: timestamp.clone(),
        model: envelope.message.model.clone(),
        stop_reason: envelope.message.stop_reason.clone(),
        cwd: envelope.cwd.clone(),
        git_branch: envelope.git_branch.clone(),
        duration_ms: None,
    });

    if let ContentValue::Blocks(blocks) = &envelope.message.content {
        for (idx, block) in blocks.iter().enumerate() {
            match block {
                ContentBlock::Text { text } => {
                    let hash = content::hash_content(text);
                    if content::should_dedup(text) {
                        ops.blobs.push((
                            hash.clone(),
                            text.clone(),
                            text.len() as i64,
                            "text".into(),
                        ));
                        ops.blob_refs.push((
                            hash.clone(),
                            msg_id.clone(),
                            "response_text".into(),
                        ));
                        ops.fts_entries.push((
                            hash.clone(),
                            "text".into(),
                            text.clone(),
                        ));
                    }
                    ops.content_blocks.push(ContentBlockRow {
                        message_id: msg_id.clone(),
                        block_index: idx as i64,
                        block_type: "text".into(),
                        content_hash: if content::should_dedup(text) {
                            Some(hash)
                        } else {
                            None
                        },
                        tool_name: None,
                        tool_use_id: None,
                        tool_input_hash: None,
                    });
                }
                ContentBlock::ToolUse { id, name, input } => {
                    let input_str = serde_json::to_string(input).unwrap_or_default();
                    let input_hash = content::hash_content(&input_str);

                    if content::should_dedup(&input_str) {
                        ops.blobs.push((
                            input_hash.clone(),
                            input_str.clone(),
                            input_str.len() as i64,
                            "tool_input".into(),
                        ));
                    }

                    ops.content_blocks.push(ContentBlockRow {
                        message_id: msg_id.clone(),
                        block_index: idx as i64,
                        block_type: "tool_use".into(),
                        content_hash: None,
                        tool_name: Some(name.clone()),
                        tool_use_id: Some(id.clone()),
                        tool_input_hash: if content::should_dedup(&input_str) {
                            Some(input_hash.clone())
                        } else {
                            None
                        },
                    });

                    ops.tool_calls.push(ToolCallRow {
                        id: id.clone(),
                        message_id: msg_id.clone(),
                        session_id: session_id.clone(),
                        tool_name: name.clone(),
                        input_hash: if content::should_dedup(&input_str) {
                            Some(input_hash)
                        } else {
                            None
                        },
                        timestamp: timestamp.clone(),
                    });

                    tracker.track_tool_use(id, name, input);
                }
                ContentBlock::Thinking { thinking } => {
                    let hash = content::hash_content(thinking);
                    if content::should_dedup(thinking) {
                        ops.blobs.push((
                            hash.clone(),
                            thinking.clone(),
                            thinking.len() as i64,
                            "thinking".into(),
                        ));
                    }
                    // No FTS for thinking blocks
                    ops.content_blocks.push(ContentBlockRow {
                        message_id: msg_id.clone(),
                        block_index: idx as i64,
                        block_type: "thinking".into(),
                        content_hash: if content::should_dedup(thinking) {
                            Some(hash)
                        } else {
                            None
                        },
                        tool_name: None,
                        tool_use_id: None,
                        tool_input_hash: None,
                    });
                }
                ContentBlock::ToolResult { .. } => {
                    // ToolResult appears in user messages, not assistant messages
                }
            }
        }
    }

    ops
}

/// Handle a user message. Returns LineOps with all DB operations needed.
pub fn handle_user(
    envelope: &MessageEnvelope,
    tracker: &mut ToolUseTracker,
) -> LineOps {
    let mut ops = LineOps::default();
    let msg_id = &envelope.uuid;
    let session_id = &envelope.session_id;

    ops.message = Some(MessageRow {
        id: msg_id.clone(),
        session_id: session_id.clone(),
        parent_id: envelope.parent_uuid.clone(),
        msg_type: "user".into(),
        timestamp: envelope.timestamp.clone(),
        model: None,
        stop_reason: None,
        cwd: envelope.cwd.clone(),
        git_branch: envelope.git_branch.clone(),
        duration_ms: None,
    });

    match &envelope.message.content {
        ContentValue::Text(text) => {
            let hash = content::hash_content(text);
            if content::should_dedup(text) {
                ops.blobs.push((
                    hash.clone(),
                    text.clone(),
                    text.len() as i64,
                    "user_text".into(),
                ));
                ops.blob_refs.push((
                    hash.clone(),
                    msg_id.clone(),
                    "user_prompt".into(),
                ));
                ops.fts_entries.push((
                    hash.clone(),
                    "user_text".into(),
                    text.clone(),
                ));
            }
        }
        ContentValue::Blocks(blocks) => {
            for (idx, block) in blocks.iter().enumerate() {
                match block {
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content: result_content,
                    } => {
                        let content_str = serde_json::to_string(result_content).unwrap_or_default();
                        let hash = content::hash_content(&content_str);
                        let stored = content::should_dedup(&content_str);

                        if stored {
                            ops.blobs.push((
                                hash.clone(),
                                content_str.clone(),
                                content_str.len() as i64,
                                "tool_output".into(),
                            ));
                            ops.blob_refs.push((
                                hash.clone(),
                                msg_id.clone(),
                                "tool_result".into(),
                            ));
                            ops.fts_entries.push((
                                hash.clone(),
                                "tool_output".into(),
                                content_str,
                            ));

                            // Only link output when blob exists (FK constraint)
                            ops.tool_output_links.push((
                                tool_use_id.clone(),
                                hash.clone(),
                            ));
                        }

                        // Check ToolUseTracker for file reference
                        if let Some((_tool_name, file_path, operation)) =
                            tracker.resolve_tool_result(tool_use_id)
                        {
                            if stored {
                                ops.file_refs.push(FileRefRow {
                                    file_path,
                                    content_hash: hash.clone(),
                                    session_id: session_id.clone(),
                                    message_id: msg_id.clone(),
                                    operation,
                                });
                            }
                        }

                        ops.content_blocks.push(ContentBlockRow {
                            message_id: msg_id.clone(),
                            block_index: idx as i64,
                            block_type: "tool_result".into(),
                            content_hash: if stored { Some(hash) } else { None },
                            tool_name: None,
                            tool_use_id: Some(tool_use_id.clone()),
                            tool_input_hash: None,
                        });
                    }
                    ContentBlock::Text { text } => {
                        let hash = content::hash_content(text);
                        if content::should_dedup(text) {
                            ops.blobs.push((
                                hash.clone(),
                                text.clone(),
                                text.len() as i64,
                                "user_text".into(),
                            ));
                            ops.fts_entries.push((
                                hash.clone(),
                                "user_text".into(),
                                text.clone(),
                            ));
                        }
                        ops.content_blocks.push(ContentBlockRow {
                            message_id: msg_id.clone(),
                            block_index: idx as i64,
                            block_type: "text".into(),
                            content_hash: if content::should_dedup(text) {
                                Some(hash)
                            } else {
                                None
                            },
                            tool_name: None,
                            tool_use_id: None,
                            tool_input_hash: None,
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    ops
}

/// Handle a system message. Returns LineOps.
pub fn handle_system(envelope: &SystemEnvelope) -> LineOps {
    let mut ops = LineOps {
        message: Some(MessageRow {
            id: envelope.uuid.clone(),
            session_id: envelope.session_id.clone(),
            parent_id: None,
            msg_type: "system".into(),
            timestamp: envelope.timestamp.clone(),
            model: None,
            stop_reason: None,
            cwd: None,
            git_branch: None,
            duration_ms: envelope.duration_ms,
        }),
        ..Default::default()
    };

    if let Some(content) = &envelope.content {
        let hash = content::hash_content(content);
        if content::should_dedup(content) {
            ops.blobs.push((
                hash.clone(),
                content.clone(),
                content.len() as i64,
                "system".into(),
            ));
            ops.fts_entries.push((hash, "system".into(), content.clone()));
        }
    }

    ops
}

/// Handle a summary message. Returns LineOps.
/// session_id must be provided from the file context since SummaryEnvelope doesn't contain it.
pub fn handle_summary(envelope: &SummaryEnvelope, session_id: &str) -> LineOps {
    let mut ops = LineOps::default();

    let leaf_uuid = envelope
        .leaf_uuid
        .as_deref()
        .unwrap_or("unknown");
    let synthetic_id = format!("summary-{leaf_uuid}");

    ops.message = Some(MessageRow {
        id: synthetic_id.clone(),
        session_id: session_id.into(),
        parent_id: None,
        msg_type: "summary".into(),
        timestamp: String::new(), // summaries don't have timestamps
        model: None,
        stop_reason: None,
        cwd: None,
        git_branch: None,
        duration_ms: None,
    });

    let hash = content::hash_content(&envelope.summary);
    if content::should_dedup(&envelope.summary) {
        ops.blobs.push((
            hash.clone(),
            envelope.summary.clone(),
            envelope.summary.len() as i64,
            "summary".into(),
        ));
        ops.blob_refs.push((
            hash.clone(),
            synthetic_id,
            "summary".into(),
        ));
        ops.fts_entries.push((hash, "summary".into(), envelope.summary.clone()));
    }

    ops
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ContentValue, MessageContent};

    fn make_assistant_envelope(blocks: Vec<ContentBlock>) -> MessageEnvelope {
        MessageEnvelope {
            uuid: "msg-001".into(),
            parent_uuid: None,
            session_id: "sess-001".into(),
            timestamp: "2024-01-01T00:00:00Z".into(),
            cwd: Some("/home/user".into()),
            git_branch: Some("main".into()),
            version: None,
            slug: None,
            is_sidechain: None,
            message: MessageContent {
                role: "assistant".into(),
                model: Some("claude-3".into()),
                id: None,
                content: ContentValue::Blocks(blocks),
                stop_reason: Some("end_turn".into()),
                usage: None,
            },
        }
    }

    #[test]
    fn test_is_skippable() {
        assert!(is_skippable(r#"{"type":"progress","uuid":"abc"}"#));
        assert!(is_skippable(r#"{"type": "progress", "uuid":"abc"}"#));
        assert!(is_skippable(r#"{"type":"queue-operation","data":{}}"#));
        assert!(!is_skippable(r#"{"type":"assistant","uuid":"abc"}"#));
        assert!(!is_skippable(r#"{"type":"user","uuid":"abc"}"#));
    }

    #[test]
    fn test_handle_assistant_text() {
        let text = "A".repeat(300); // Over dedup threshold
        let envelope = make_assistant_envelope(vec![ContentBlock::Text { text: text.clone() }]);
        let mut tracker = ToolUseTracker::new();

        let ops = handle_assistant(&envelope, &mut tracker);
        assert!(ops.message.is_some());
        assert_eq!(ops.content_blocks.len(), 1);
        assert_eq!(ops.content_blocks[0].block_type, "text");
        assert_eq!(ops.blobs.len(), 1);
        assert_eq!(ops.fts_entries.len(), 1);
    }

    #[test]
    fn test_handle_assistant_tool_use() {
        let envelope = make_assistant_envelope(vec![ContentBlock::ToolUse {
            id: "toolu_123".into(),
            name: "Read".into(),
            input: serde_json::json!({"file_path": "/src/main.rs"}),
        }]);
        let mut tracker = ToolUseTracker::new();

        let ops = handle_assistant(&envelope, &mut tracker);
        assert_eq!(ops.tool_calls.len(), 1);
        assert_eq!(ops.tool_calls[0].tool_name, "Read");

        // Tracker should have recorded the tool use
        let resolved = tracker.resolve_tool_result("toolu_123");
        assert!(resolved.is_some());
    }

    #[test]
    fn test_handle_assistant_thinking_no_fts() {
        let thinking = "T".repeat(300);
        let envelope =
            make_assistant_envelope(vec![ContentBlock::Thinking { thinking: thinking.clone() }]);
        let mut tracker = ToolUseTracker::new();

        let ops = handle_assistant(&envelope, &mut tracker);
        assert_eq!(ops.content_blocks.len(), 1);
        assert_eq!(ops.content_blocks[0].block_type, "thinking");
        assert_eq!(ops.blobs.len(), 1);
        assert!(ops.fts_entries.is_empty()); // No FTS for thinking
    }

    #[test]
    fn test_handle_summary() {
        let summary = "S".repeat(300);
        let envelope = SummaryEnvelope {
            summary: summary.clone(),
            leaf_uuid: Some("leaf-001".into()),
        };

        let ops = handle_summary(&envelope, "sess-001");
        assert!(ops.message.is_some());
        let msg = ops.message.unwrap();
        assert_eq!(msg.id, "summary-leaf-001");
        assert_eq!(msg.session_id, "sess-001");
        assert_eq!(ops.fts_entries.len(), 1);
    }
}
