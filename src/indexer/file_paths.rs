use std::collections::HashMap;

/// Per-session tracker that maps tool_use_id → (tool_name, file_path) for file operations.
#[derive(Default)]
pub struct ToolUseTracker {
    /// tool_use_id → (tool_name, file_path)
    pending: HashMap<String, (String, String)>,
}

impl ToolUseTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Track a tool_use block. Extracts file_path from input for Read/Write/Edit/MultiEdit tools.
    pub fn track_tool_use(
        &mut self,
        tool_use_id: &str,
        tool_name: &str,
        input: &serde_json::Value,
    ) {
        let file_path = input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(fp) = file_path {
            match tool_name {
                "Read" | "Write" | "Edit" | "MultiEdit" => {
                    self.pending
                        .insert(tool_use_id.to_string(), (tool_name.to_string(), fp));
                }
                _ => {}
            }
        }
    }

    /// Resolve a tool_result by its tool_use_id. Returns (tool_name, file_path, operation)
    /// if the original tool_use was a tracked file operation.
    pub fn resolve_tool_result(
        &mut self,
        tool_use_id: &str,
    ) -> Option<(String, String, String)> {
        self.pending.remove(tool_use_id).map(|(name, path)| {
            let operation = match name.as_str() {
                "Read" => "read",
                "Write" => "write",
                "Edit" | "MultiEdit" => "edit",
                _ => "unknown",
            };
            (name, path, operation.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_track_and_resolve() {
        let mut tracker = ToolUseTracker::new();

        tracker.track_tool_use(
            "toolu_123",
            "Read",
            &json!({"file_path": "/src/main.rs"}),
        );

        let result = tracker.resolve_tool_result("toolu_123");
        assert!(result.is_some());
        let (name, path, op) = result.unwrap();
        assert_eq!(name, "Read");
        assert_eq!(path, "/src/main.rs");
        assert_eq!(op, "read");
    }

    #[test]
    fn test_resolve_removes_entry() {
        let mut tracker = ToolUseTracker::new();
        tracker.track_tool_use(
            "toolu_123",
            "Write",
            &json!({"file_path": "/tmp/out.txt"}),
        );

        assert!(tracker.resolve_tool_result("toolu_123").is_some());
        assert!(tracker.resolve_tool_result("toolu_123").is_none());
    }

    #[test]
    fn test_ignores_non_file_tools() {
        let mut tracker = ToolUseTracker::new();
        tracker.track_tool_use(
            "toolu_123",
            "Bash",
            &json!({"command": "ls"}),
        );

        assert!(tracker.resolve_tool_result("toolu_123").is_none());
    }

    #[test]
    fn test_ignores_missing_file_path() {
        let mut tracker = ToolUseTracker::new();
        tracker.track_tool_use(
            "toolu_123",
            "Read",
            &json!({"other_field": "value"}),
        );

        assert!(tracker.resolve_tool_result("toolu_123").is_none());
    }

    #[test]
    fn test_edit_operation() {
        let mut tracker = ToolUseTracker::new();
        tracker.track_tool_use(
            "toolu_456",
            "Edit",
            &json!({"file_path": "/src/lib.rs", "old_string": "a", "new_string": "b"}),
        );

        let (_, _, op) = tracker.resolve_tool_result("toolu_456").unwrap();
        assert_eq!(op, "edit");
    }
}
