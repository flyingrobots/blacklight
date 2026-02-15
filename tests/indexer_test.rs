//! Integration tests for the indexer pipeline.
//!
//! All content uses public domain text (Shakespeare, Dickinson, Whitman)
//! in fixtures that match the exact structure of real ~/.claude/ data files.

use blacklight::db;
use blacklight::indexer::{self, IndexConfig};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Public domain text corpus (substituted for real session content)
// ---------------------------------------------------------------------------

const HAMLET_SOLILOQUY: &str = "To be, or not to be, that is the question: \
Whether 'tis nobler in the mind to suffer the slings and arrows of outrageous fortune, \
or to take arms against a sea of troubles and by opposing end them. To die — to sleep, \
no more; and by a sleep to say we end the heartache and the thousand natural shocks \
that flesh is heir to: 'tis a consummation devoutly to be wished.";

const WHITMAN_GRASS: &str = "I believe a leaf of grass is no less than the journey-work of the stars, \
and the pismire is equally perfect, and a grain of sand, and the egg of the wren, \
and the tree-toad is a chef-d'oeuvre for the highest, and the running blackberry would adorn \
the parlors of heaven.";

const DICKINSON_HOPE: &str = "Hope is the thing with feathers that perches in the soul, \
and sings the tune without the words, and never stops at all, and sweetest in the gale is heard; \
and sore must be the storm that could abash the little bird that kept so many warm.";

const TWAIN_RIVER: &str = "The Mississippi is well worth reading about. It is not a commonplace river, \
but on the contrary is in all ways remarkable. Considering the Missouri its main branch, \
it is the longest river in the world — four thousand three hundred miles. It drains the water \
of twenty-eight states and territories; it draws its sources from Delaware, and makes the boundary \
of Idaho upon the other end.";

const POE_RAVEN: &str = "Once upon a midnight dreary, while I pondered, weak and weary, \
over many a quaint and curious volume of forgotten lore — while I nodded, nearly napping, \
suddenly there came a tapping, as of some one gently rapping, rapping at my chamber door.";

// ---------------------------------------------------------------------------
// Fixture builders — matching exact real ~/.claude/ field structure
// ---------------------------------------------------------------------------

fn create_file(root: &std::path::Path, rel_path: &str, content: &str) {
    let path = root.join(rel_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

/// sessions-index.json matching real structure
fn sessions_index(entries: &[(&str, &str, &str, &str, &str)]) -> String {
    // entries: (session_id, first_prompt, summary, project_path, git_branch)
    let entry_strs: Vec<String> = entries
        .iter()
        .map(|(id, prompt, summary, proj_path, branch)| {
            format!(
                r#"{{
                    "sessionId": "{id}",
                    "fullPath": "/fake/{id}.jsonl",
                    "fileMtime": 1769327772261,
                    "firstPrompt": "{prompt}",
                    "summary": "{summary}",
                    "messageCount": 28,
                    "created": "2026-01-13T18:28:12.082Z",
                    "modified": "2026-01-13T18:42:55.749Z",
                    "gitBranch": "{branch}",
                    "projectPath": "{proj_path}",
                    "isSidechain": false
                }}"#
            )
        })
        .collect();

    let proj_path = entries.first().map(|e| e.3).unwrap_or("/unknown");
    format!(
        r#"{{"version": 1, "entries": [{}], "originalPath": "{}"}}"#,
        entry_strs.join(","),
        proj_path,
    )
}

/// User message with plain text content (matching real field set)
fn user_text_msg(uuid: &str, parent: Option<&str>, session_id: &str, text: &str) -> String {
    let parent_val = parent
        .map(|p| format!(r#""{p}""#))
        .unwrap_or_else(|| "null".into());
    format!(
        r#"{{"parentUuid":{parent_val},"isSidechain":false,"userType":"external","cwd":"/Users/test/git/myproject","sessionId":"{session_id}","version":"2.1.19","gitBranch":"main","type":"user","message":{{"role":"user","content":"{text}"}},"slug":"test-session","uuid":"{uuid}","timestamp":"2026-01-13T18:28:15.000Z"}}"#
    )
}

/// User message with tool_result content blocks
fn user_tool_result_msg(
    uuid: &str,
    parent: &str,
    session_id: &str,
    tool_use_id: &str,
    result_text: &str,
) -> String {
    format!(
        r#"{{"parentUuid":"{parent}","isSidechain":false,"userType":"external","cwd":"/Users/test/git/myproject","sessionId":"{session_id}","version":"2.1.19","gitBranch":"main","type":"user","message":{{"role":"user","content":[{{"tool_use_id":"{tool_use_id}","type":"tool_result","content":"{result_text}"}}]}},"slug":"test-session","uuid":"{uuid}","timestamp":"2026-01-13T18:29:00.000Z"}}"#
    )
}

/// Assistant message with text content
fn assistant_text_msg(
    uuid: &str,
    parent: &str,
    session_id: &str,
    text: &str,
    model: &str,
) -> String {
    // Escape the text for JSON
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        r#"{{"parentUuid":"{parent}","isSidechain":false,"userType":"external","cwd":"/Users/test/git/myproject","sessionId":"{session_id}","version":"2.1.19","gitBranch":"main","type":"assistant","message":{{"model":"{model}","id":"msg_01test","type":"message","role":"assistant","content":[{{"type":"text","text":"{escaped}"}}],"stop_reason":"end_turn","usage":{{"input_tokens":1500,"output_tokens":350}}}},"slug":"test-session","uuid":"{uuid}","timestamp":"2026-01-13T18:28:30.000Z"}}"#
    )
}

/// Assistant message with tool_use block
fn assistant_tool_use_msg(
    uuid: &str,
    parent: &str,
    session_id: &str,
    tool_use_id: &str,
    tool_name: &str,
    input_json: &str,
    model: &str,
) -> String {
    format!(
        r#"{{"parentUuid":"{parent}","isSidechain":false,"userType":"external","cwd":"/Users/test/git/myproject","sessionId":"{session_id}","version":"2.1.19","gitBranch":"main","type":"assistant","message":{{"model":"{model}","id":"msg_02test","type":"message","role":"assistant","content":[{{"type":"tool_use","id":"{tool_use_id}","name":"{tool_name}","input":{input_json}}}],"stop_reason":"tool_use","usage":{{"input_tokens":800,"output_tokens":50}}}},"slug":"test-session","uuid":"{uuid}","timestamp":"2026-01-13T18:28:45.000Z"}}"#
    )
}

/// Assistant message with thinking + text blocks
fn assistant_thinking_msg(
    uuid: &str,
    parent: &str,
    session_id: &str,
    thinking: &str,
    text: &str,
    model: &str,
) -> String {
    let escaped_thinking = thinking.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_text = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        r#"{{"parentUuid":"{parent}","isSidechain":false,"userType":"external","cwd":"/Users/test/git/myproject","sessionId":"{session_id}","version":"2.1.19","gitBranch":"main","type":"assistant","message":{{"model":"{model}","id":"msg_03test","type":"message","role":"assistant","content":[{{"type":"thinking","thinking":"{escaped_thinking}"}},{{"type":"text","text":"{escaped_text}"}}],"stop_reason":"end_turn","usage":{{"input_tokens":2000,"output_tokens":500}}}},"slug":"test-session","uuid":"{uuid}","timestamp":"2026-01-13T18:30:00.000Z"}}"#
    )
}

/// Progress message (the kind we skip — ~28% of real lines)
fn progress_msg(uuid: &str, parent: &str, session_id: &str, tool_use_id: &str) -> String {
    format!(
        r#"{{"parentUuid":"{parent}","isSidechain":false,"userType":"external","cwd":"/Users/test/git/myproject","sessionId":"{session_id}","version":"2.1.19","gitBranch":"main","slug":"test-session","type":"progress","data":{{"message":{{"type":"user","message":{{"role":"user","content":[{{"type":"text","text":"searching for files..."}}]}}}}}},"toolUseID":"agent_msg_01test","parentToolUseID":"{tool_use_id}","uuid":"{uuid}","timestamp":"2026-01-13T18:29:05.000Z"}}"#
    )
}

/// System message (turn duration tracking)
fn system_msg(uuid: &str, parent: &str, session_id: &str, duration_ms: u64) -> String {
    format!(
        r#"{{"parentUuid":"{parent}","isSidechain":false,"userType":"external","cwd":"/Users/test/git/myproject","sessionId":"{session_id}","version":"2.1.19","gitBranch":"main","slug":"test-session","type":"system","subtype":"turn_duration","durationMs":{duration_ms},"uuid":"{uuid}","timestamp":"2026-01-13T18:31:00.000Z","isMeta":false}}"#
    )
}

/// Summary message
fn summary_msg(leaf_uuid: &str, summary_text: &str) -> String {
    let escaped = summary_text.replace('\\', "\\\\").replace('"', "\\\"");
    format!(r#"{{"type":"summary","summary":"{escaped}","leafUuid":"{leaf_uuid}"}}"#)
}

/// Queue operation message (also skipped)
fn queue_op_msg(session_id: &str) -> String {
    format!(
        r#"{{"type":"queue-operation","sessionId":"{session_id}","uuid":"qop-01","timestamp":"2026-01-13T18:28:12.000Z","operation":"enqueue"}}"#
    )
}

/// Task JSON matching real structure
fn task_json(id: &str, subject: &str, desc: &str, status: &str, blocked_by: &[&str]) -> String {
    let deps: Vec<String> = blocked_by.iter().map(|d| format!(r#""{d}""#)).collect();
    format!(
        r#"{{"id":"{id}","subject":"{subject}","description":"{desc}","activeForm":"Working on {subject}","status":"{status}","blocks":[],"blockedBy":[{deps}]}}"#,
        deps = deps.join(","),
    )
}

/// Facet JSON matching real structure
fn facet_json(session_id: &str, goal: &str, outcome: &str, summary: &str) -> String {
    format!(
        r#"{{
            "underlying_goal": "{goal}",
            "goal_categories": {{"code_review": 2, "bug_fix": 1}},
            "outcome": "{outcome}",
            "user_satisfaction_counts": {{"likely_satisfied": 1}},
            "claude_helpfulness": "essential",
            "session_type": "multi_task",
            "friction_counts": {{"slow_response": 1}},
            "friction_detail": "Model took a long time on the initial analysis step.",
            "primary_success": "multi_file_changes",
            "brief_summary": "{summary}",
            "session_id": "{session_id}"
        }}"#
    )
}

/// stats-cache.json matching real structure (with large token counts)
fn stats_cache_json() -> String {
    r#"{
        "version": 2,
        "lastComputedDate": "2026-01-15",
        "dailyActivity": [
            {"date": "2026-01-11", "messageCount": 1262, "sessionCount": 2, "toolCallCount": 362},
            {"date": "2026-01-12", "messageCount": 845, "sessionCount": 5, "toolCallCount": 201},
            {"date": "2026-01-13", "messageCount": 533, "sessionCount": 3, "toolCallCount": 98}
        ],
        "modelUsage": {
            "claude-opus-4-5-20251101": {
                "input_tokens": 2914395,
                "output_tokens": 267945,
                "cache_read_tokens": 3697899746,
                "cache_creation_tokens": 190971744
            },
            "claude-sonnet-4-5-20250929": {
                "input_tokens": 500000,
                "output_tokens": 45000,
                "cache_read_tokens": 12000000,
                "cache_creation_tokens": 800000
            }
        },
        "totalSessions": 363,
        "totalMessages": 169701,
        "longestSession": {
            "sessionId": "longest-sess-001",
            "messageCount": 1553,
            "project": "/Users/test/git/bigproject"
        },
        "firstSessionDate": "2026-01-11T04:20:48.565Z",
        "hourCounts": {"0": 12, "1": 13, "14": 45, "15": 38}
    }"#
    .into()
}

/// history.jsonl entries matching real structure
fn history_entry(display: &str, project: &str, session_id: &str, ts: u64) -> String {
    let escaped = display.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        r#"{{"display":"{escaped}","pastedContents":{{}},"timestamp":{ts},"project":"{project}","sessionId":"{session_id}"}}"#
    )
}

// ---------------------------------------------------------------------------
// Test: Full pipeline with realistic multi-session data
// ---------------------------------------------------------------------------

#[test]
fn test_realistic_multi_session_index() {
    let tmp = TempDir::new().unwrap();
    let claude_dir = tmp.path().join("claude");
    let db_path = tmp.path().join("test.db");

    let sess_a = "aaaaaaaa-1111-2222-3333-444444444444";
    let sess_b = "bbbbbbbb-1111-2222-3333-444444444444";
    let project_dir = "-Users-test-git-myproject";

    // --- Sessions index with two sessions ---
    create_file(
        &claude_dir,
        &format!("projects/{project_dir}/sessions-index.json"),
        &sessions_index(&[
            (
                sess_a,
                "Analyze the river passage and suggest improvements",
                "Analyzed Twain excerpt and proposed structural changes",
                "/Users/test/git/myproject",
                "main",
            ),
            (
                sess_b,
                "Review the poem and extract themes",
                "Reviewed Dickinson and identified key imagery",
                "/Users/test/git/myproject",
                "feature/poetry",
            ),
        ]),
    );

    // --- Session A: multi-turn conversation with tool use ---
    let mut jsonl_a = String::new();

    // Summary appears first (as it can in real data)
    jsonl_a.push_str(&summary_msg(
        "a-msg-04",
        "Analyzed a passage from Mark Twain about the Mississippi River. Discussed the rhetorical structure and suggested modernizing the prose while preserving the original voice.",
    ));
    jsonl_a.push('\n');

    // User asks about the text
    jsonl_a.push_str(&user_text_msg(
        "a-msg-01",
        None,
        sess_a,
        "Please analyze this passage and suggest improvements",
    ));
    jsonl_a.push('\n');

    // Assistant uses Read tool to look at a file
    jsonl_a.push_str(&assistant_tool_use_msg(
        "a-msg-02",
        "a-msg-01",
        sess_a,
        "toolu_01ReadTwain",
        "Read",
        r#"{"file_path": "/Users/test/git/myproject/docs/twain-excerpt.md"}"#,
        "claude-opus-4-5-20251101",
    ));
    jsonl_a.push('\n');

    // Progress message during tool execution (should be skipped)
    jsonl_a.push_str(&progress_msg(
        "a-prog-01",
        "a-msg-02",
        sess_a,
        "toolu_01ReadTwain",
    ));
    jsonl_a.push('\n');

    // Queue operation (also skipped)
    jsonl_a.push_str(&queue_op_msg(sess_a));
    jsonl_a.push('\n');

    // User returns tool result
    jsonl_a.push_str(&user_tool_result_msg(
        "a-msg-03",
        "a-msg-02",
        sess_a,
        "toolu_01ReadTwain",
        TWAIN_RIVER,
    ));
    jsonl_a.push('\n');

    // Assistant responds with thinking + text
    jsonl_a.push_str(&assistant_thinking_msg(
        "a-msg-04",
        "a-msg-03",
        sess_a,
        HAMLET_SOLILOQUY, // thinking content (won't be FTS indexed)
        WHITMAN_GRASS,    // response text (will be FTS indexed)
        "claude-opus-4-5-20251101",
    ));
    jsonl_a.push('\n');

    // System turn duration
    jsonl_a.push_str(&system_msg("a-sys-01", "a-msg-04", sess_a, 15234));
    jsonl_a.push('\n');

    create_file(
        &claude_dir,
        &format!("projects/{project_dir}/{sess_a}.jsonl"),
        &jsonl_a,
    );

    // --- Session B: simple conversation ---
    let mut jsonl_b = String::new();

    jsonl_b.push_str(&user_text_msg(
        "b-msg-01",
        None,
        sess_b,
        "What are the key themes in this poem?",
    ));
    jsonl_b.push('\n');

    jsonl_b.push_str(&assistant_text_msg(
        "b-msg-02",
        "b-msg-01",
        sess_b,
        DICKINSON_HOPE,
        "claude-sonnet-4-5-20250929",
    ));
    jsonl_b.push('\n');

    // Progress (skipped)
    jsonl_b.push_str(&progress_msg("b-prog-01", "b-msg-02", sess_b, "toolu_none"));
    jsonl_b.push('\n');

    jsonl_b.push_str(&system_msg("b-sys-01", "b-msg-02", sess_b, 8750));
    jsonl_b.push('\n');

    jsonl_b.push_str(&summary_msg(
        "b-msg-02",
        "Discussed Emily Dickinson's Hope poem. Identified themes of resilience, nature metaphor, and the persistence of hope through adversity.",
    ));
    jsonl_b.push('\n');

    create_file(
        &claude_dir,
        &format!("projects/{project_dir}/{sess_b}.jsonl"),
        &jsonl_b,
    );

    // --- Subagent file (references parent session A) ---
    let mut subagent_jsonl = String::new();
    subagent_jsonl.push_str(&user_text_msg(
        "sub-msg-01",
        None,
        sess_a, // references parent session
        "Search for all markdown files in the project",
    ));
    subagent_jsonl.push('\n');
    subagent_jsonl.push_str(&assistant_text_msg(
        "sub-msg-02",
        "sub-msg-01",
        sess_a,
        POE_RAVEN, // use Poe as the response text
        "claude-sonnet-4-5-20250929",
    ));
    subagent_jsonl.push('\n');

    create_file(
        &claude_dir,
        &format!("projects/{project_dir}/{sess_a}/subagents/agent-a12b34c.jsonl"),
        &subagent_jsonl,
    );

    // --- Tasks (under tasks/<session_id>/) ---
    create_file(
        &claude_dir,
        &format!("tasks/{sess_a}/1.json"),
        &task_json(
            "1",
            "Analyze prose structure",
            "Read the Twain excerpt and identify rhetorical devices",
            "completed",
            &[],
        ),
    );
    create_file(
        &claude_dir,
        &format!("tasks/{sess_a}/2.json"),
        &task_json(
            "2",
            "Suggest improvements",
            "Propose modernization while preserving voice",
            "completed",
            &["1"],
        ),
    );
    create_file(
        &claude_dir,
        &format!("tasks/{sess_a}/3.json"),
        &task_json(
            "3",
            "Write revised passage",
            "Apply suggested improvements to the text",
            "pending",
            &["2"],
        ),
    );

    // --- Facets ---
    create_file(
        &claude_dir,
        &format!("usage-data/facets/{sess_a}.json"),
        &facet_json(
            sess_a,
            "Analyze and improve a literary passage",
            "fully_achieved",
            "Analyzed Twain excerpt, identified devices, suggested improvements",
        ),
    );
    create_file(
        &claude_dir,
        &format!("usage-data/facets/{sess_b}.json"),
        &facet_json(
            sess_b,
            "Poetry analysis and theme extraction",
            "fully_achieved",
            "Identified themes in Dickinson poem",
        ),
    );

    // --- Stats cache ---
    create_file(&claude_dir, "stats-cache.json", &stats_cache_json());

    // --- Plans ---
    create_file(
        &claude_dir,
        "plans/fix-warpgraph-join-elements.md",
        &format!(
            "# Fix passage analysis pipeline\n\n## Context\n\n{TWAIN_RIVER}\n\n## Approach\n\n1. Parse the passage into sentences\n2. Identify rhetorical devices\n3. Score each device by effectiveness\n4. Suggest alternatives\n"
        ),
    );
    create_file(
        &claude_dir,
        "plans/poetry-theme-extraction.md",
        &format!(
            "# Poetry Theme Extraction\n\n## Overview\n\n{DICKINSON_HOPE}\n\n## Method\n\n1. Identify recurring imagery\n2. Map metaphor chains\n3. Extract thematic statements\n"
        ),
    );

    // --- History ---
    let mut history = String::new();
    history.push_str(&history_entry(
        "analyze this Twain passage for rhetorical devices",
        "/Users/test/git/myproject",
        sess_a,
        1768105248463,
    ));
    history.push('\n');
    history.push_str(&history_entry(
        "what are the key themes in the Dickinson poem?",
        "/Users/test/git/myproject",
        sess_b,
        1768105300000,
    ));
    history.push('\n');
    history.push_str(&history_entry(
        "find all references to nature imagery in the corpus",
        "/Users/test/git/myproject",
        sess_a,
        1768105400000,
    ));
    history.push('\n');
    create_file(&claude_dir, "history.jsonl", &history);

    // --- Files that should be SKIPPED ---
    create_file(&claude_dir, "cache/model-cache.json", "{}");
    create_file(&claude_dir, "statsig/experiments.json", "{}");
    create_file(&claude_dir, "shell-snapshots/snap1.json", "{}");
    create_file(&claude_dir, "session-env/env1.json", "{}");
    create_file(&claude_dir, "ide/settings.json", "{}");
    create_file(&claude_dir, "paste-cache/paste1.txt", "pasted text");
    create_file(&claude_dir, "debug/log.txt", "debug info");
    create_file(&claude_dir, "telemetry/events.json", "{}");
    create_file(&claude_dir, "settings.json", r#"{"theme":"dark"}"#);
    create_file(
        &claude_dir,
        &format!("projects/{project_dir}/.DS_Store"),
        "binary junk",
    );
    create_file(
        &claude_dir,
        &format!("tasks/{sess_a}/1.lock"),
        "locked",
    );
    create_file(
        &claude_dir,
        &format!("tasks/{sess_a}/1.highwatermark"),
        "42",
    );

    // ===== RUN THE INDEXER =====
    let report = indexer::run_index(IndexConfig {
        claude_dir: claude_dir.clone(),
        extra_dirs: vec![],
        db_path: db_path.clone(),
        full: false,
        verbose: true,
        skip_dirs: blacklight::indexer::scanner::DEFAULT_SKIP_DIRS.iter().map(|s| (*s).to_string()).collect(),
        progress: None,
        cancel_flag: None,
        pause_flag: None,
        notify_tx: None,
    })
    .unwrap();

    let conn = db::open(&db_path).unwrap();

    // --- Verify report ---
    assert_eq!(report.sessions_parsed, 2, "should parse 2 sessions from index");
    assert_eq!(report.parse_errors, 0, "no parse errors expected");
    assert_eq!(report.tasks_parsed, 3, "should parse 3 tasks");
    assert_eq!(report.facets_parsed, 2, "should parse 2 facets");
    assert_eq!(report.plans_parsed, 2, "should parse 2 plans");
    assert_eq!(report.history_entries, 3, "should parse 3 history entries");

    // Progress + queue-op messages should be skipped
    assert!(
        report.messages_skipped >= 3,
        "expected at least 3 skipped (progress/queue-op), got {}",
        report.messages_skipped,
    );

    // --- Verify sessions table ---
    let session_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(session_count, 2);

    let slug: String = conn
        .query_row(
            "SELECT project_slug FROM sessions WHERE id = ?1",
            [sess_a],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(slug, "myproject");

    let branch: String = conn
        .query_row(
            "SELECT git_branch FROM sessions WHERE id = ?1",
            [sess_b],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(branch, "feature/poetry");

    // --- Verify messages table ---
    let msg_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))
        .unwrap();
    // Session A: summary + user + tool_use + tool_result + thinking_text + system = 6
    // Session B: user + text + system + summary = 4
    // Subagent: user + text = 2
    // = 12 messages (progress/queue-op excluded)
    assert!(
        msg_count >= 10,
        "expected at least 10 messages, got {msg_count}"
    );

    // Check assistant message has model set
    let model: String = conn
        .query_row(
            "SELECT model FROM messages WHERE id = 'a-msg-04'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(model, "claude-opus-4-5-20251101");

    // Check system message has duration_ms set
    let duration: i64 = conn
        .query_row(
            "SELECT duration_ms FROM messages WHERE id = 'a-sys-01'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(duration, 15234);

    // --- Verify tool_calls table ---
    let tool_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tool_calls", [], |r| r.get(0))
        .unwrap();
    assert!(tool_count >= 1, "expected at least 1 tool call");

    let tool_name: String = conn
        .query_row(
            "SELECT tool_name FROM tool_calls WHERE id = 'toolu_01ReadTwain'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tool_name, "Read");

    // --- Verify content_blocks table ---
    let block_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM content_blocks", [], |r| r.get(0))
        .unwrap();
    assert!(block_count >= 4, "expected at least 4 content blocks");

    // Thinking block should exist
    let thinking_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM content_blocks WHERE block_type = 'thinking'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(thinking_count, 1, "should have 1 thinking block");

    // --- Verify content_store (blobs) ---
    let blob_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM content_store", [], |r| r.get(0))
        .unwrap();
    assert!(blob_count >= 5, "expected at least 5 blobs, got {blob_count}");

    // Thinking content should be stored but NOT in FTS
    let thinking_blob_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM content_store WHERE kind = 'thinking'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(thinking_blob_count, 1);

    // --- Verify FTS index ---
    // Thinking should NOT be in FTS
    let thinking_fts: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM fts_content WHERE kind = 'thinking'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(thinking_fts, 0, "thinking blocks should NOT be FTS indexed");

    // Plans should be in FTS
    let plan_fts: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM fts_content WHERE kind = 'plan'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(plan_fts, 2, "both plans should be FTS indexed");

    // Search for Twain content (in plan)
    let results = blacklight::content::search(&conn, "Mississippi", 10, 0).unwrap();
    assert!(
        !results.is_empty(),
        "FTS search for 'Mississippi' should return results"
    );

    // Search for Dickinson content (in plan)
    let results = blacklight::content::search(&conn, "feathers perches soul", 10, 0).unwrap();
    assert!(
        !results.is_empty(),
        "FTS search for Dickinson poem content should return results"
    );

    // History prompts should be searchable
    let history_fts: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM fts_content WHERE kind = 'history_prompt'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(history_fts, 3);

    // --- Verify file_references (from Read tool) ---
    let file_ref_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM file_references", [], |r| r.get(0))
        .unwrap();
    assert!(
        file_ref_count >= 1,
        "expected at least 1 file reference from Read tool"
    );

    let ref_path: String = conn
        .query_row(
            "SELECT file_path FROM file_references LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ref_path, "/Users/test/git/myproject/docs/twain-excerpt.md");

    // --- Verify tasks table ---
    let task_subjects: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT subject FROM tasks WHERE session_id = ?1 ORDER BY id")
            .unwrap();
        stmt.query_map([sess_a], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    assert_eq!(task_subjects.len(), 3);
    assert_eq!(task_subjects[0], "Analyze prose structure");
    assert_eq!(task_subjects[2], "Write revised passage");

    // Task dependencies
    let dep_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM task_dependencies WHERE session_id = ?1",
            [sess_a],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(dep_count, 2, "tasks 2 and 3 each have a blockedBy dep");

    // --- Verify session_outcomes ---
    let goal: String = conn
        .query_row(
            "SELECT underlying_goal FROM session_outcomes WHERE session_id = ?1",
            [sess_a],
            |r| r.get(0),
        )
        .unwrap();
    assert!(goal.contains("literary passage"));

    // Outcome categories
    let cat_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM outcome_categories WHERE session_id = ?1",
            [sess_a],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(cat_count, 2, "should have code_review + bug_fix categories");

    // --- Verify daily_stats ---
    let days: i64 = conn
        .query_row("SELECT COUNT(*) FROM daily_stats", [], |r| r.get(0))
        .unwrap();
    assert_eq!(days, 3);

    // --- Verify model_usage ---
    let models: i64 = conn
        .query_row("SELECT COUNT(*) FROM model_usage", [], |r| r.get(0))
        .unwrap();
    assert_eq!(models, 2);

    // Check large token count (cache_read can exceed 3.7B)
    let cache_read: i64 = conn
        .query_row(
            "SELECT cache_read_tokens FROM model_usage WHERE model = 'claude-opus-4-5-20251101'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(cache_read, 3697899746);

    // --- Verify indexed_files tracking ---
    let indexed: i64 = conn
        .query_row("SELECT COUNT(*) FROM indexed_files", [], |r| r.get(0))
        .unwrap();
    assert!(
        indexed >= 10,
        "expected 10+ indexed files, got {indexed}"
    );

    // --- Verify skipped directories/files aren't indexed ---
    // Note: stats-cache.json contains "cache" in name but is NOT in the cache/ directory
    let skip_check: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM indexed_files WHERE file_path LIKE '%/cache/%' OR file_path LIKE '%/statsig/%' OR file_path LIKE '%.DS_Store%' OR file_path LIKE '%.lock'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(skip_check, 0, "skipped dirs/files should not be indexed");
}

// ---------------------------------------------------------------------------
// Test: Incremental indexing — second run processes nothing
// ---------------------------------------------------------------------------

#[test]
fn test_incremental_index() {
    let tmp = TempDir::new().unwrap();
    let claude_dir = tmp.path().join("claude");
    let db_path = tmp.path().join("test.db");
    let sess = "cccccccc-1111-2222-3333-444444444444";

    create_file(
        &claude_dir,
        "projects/proj/sessions-index.json",
        &sessions_index(&[(
            sess,
            "initial prompt",
            "session summary",
            "/Users/test/proj",
            "main",
        )]),
    );
    create_file(
        &claude_dir,
        &format!("projects/proj/{sess}.jsonl"),
        &format!(
            "{}\n{}\n",
            user_text_msg("u1", None, sess, "hello"),
            assistant_text_msg("a1", "u1", sess, WHITMAN_GRASS, "claude-opus-4-5-20251101"),
        ),
    );

    // First run
    let r1 = indexer::run_index(IndexConfig {
        claude_dir: claude_dir.clone(),
        extra_dirs: vec![],
        db_path: db_path.clone(),
        full: false,
        verbose: false,
        skip_dirs: blacklight::indexer::scanner::DEFAULT_SKIP_DIRS.iter().map(|s| (*s).to_string()).collect(),
        progress: None,
        cancel_flag: None,
        pause_flag: None,
        notify_tx: None,
    })
    .unwrap();
    assert!(r1.files_processed > 0);
    assert!(r1.messages_processed >= 2);

    // Second run — nothing should change
    let r2 = indexer::run_index(IndexConfig {
        claude_dir,
        extra_dirs: vec![],
        db_path,
        full: false,
        verbose: false,
        skip_dirs: blacklight::indexer::scanner::DEFAULT_SKIP_DIRS.iter().map(|s| (*s).to_string()).collect(),
        progress: None,
        cancel_flag: None,
        pause_flag: None,
        notify_tx: None,
    })
    .unwrap();
    assert_eq!(r2.files_processed, 0, "second run should process 0 files");
    assert!(r2.files_unchanged > 0);
    assert_eq!(r2.messages_processed, 0);
}

// ---------------------------------------------------------------------------
// Test: FTS search returns correct results after indexing
// ---------------------------------------------------------------------------

#[test]
fn test_fts_search_after_indexing() {
    let tmp = TempDir::new().unwrap();
    let claude_dir = tmp.path().join("claude");
    let db_path = tmp.path().join("test.db");

    // Plan with searchable content
    create_file(
        &claude_dir,
        "plans/hamlet-analysis.md",
        &format!("# Hamlet Analysis\n\n{HAMLET_SOLILOQUY}\n\n## Themes\n\nMortality, indecision, the nature of existence."),
    );

    // History with searchable content
    let mut hist = String::new();
    hist.push_str(&history_entry(
        "analyze the soliloquy from Hamlet",
        "/Users/test/git/lit",
        "hist-sess",
        1768105248463,
    ));
    hist.push('\n');
    create_file(&claude_dir, "history.jsonl", &hist);

    let report = indexer::run_index(IndexConfig {
        claude_dir,
        extra_dirs: vec![],
        db_path: db_path.clone(),
        full: false,
        verbose: false,
        skip_dirs: blacklight::indexer::scanner::DEFAULT_SKIP_DIRS.iter().map(|s| (*s).to_string()).collect(),
        progress: None,
        cancel_flag: None,
        pause_flag: None,
        notify_tx: None,
    })
    .unwrap();
    assert_eq!(report.plans_parsed, 1);

    let conn = db::open(&db_path).unwrap();

    // Search for plan content — "heartache" appears in the soliloquy text
    let results = blacklight::content::search(&conn, "heartache mortality", 10, 0).unwrap();
    assert!(!results.is_empty(), "should find Hamlet plan via heartache + mortality");
    assert_eq!(results[0].kind, "plan");

    // Porter stemming: "opposing" should match via stem "oppos"
    let results = blacklight::content::search(&conn, "oppose", 10, 0).unwrap();
    assert!(!results.is_empty(), "stemming should match 'opposing' via 'oppose'");

    // History search
    let results = blacklight::content::search(&conn, "soliloquy Hamlet", 10, 0).unwrap();
    assert!(
        !results.is_empty(),
        "should find history entry about Hamlet soliloquy"
    );
}

// ---------------------------------------------------------------------------
// Test: --full flag forces re-index of unchanged files
// ---------------------------------------------------------------------------

#[test]
fn test_full_reindex() {
    let tmp = TempDir::new().unwrap();
    let claude_dir = tmp.path().join("claude");
    let db_path = tmp.path().join("test.db");
    let sess = "dddddddd-1111-2222-3333-444444444444";

    create_file(
        &claude_dir,
        "projects/proj/sessions-index.json",
        &sessions_index(&[(sess, "prompt", "summary", "/Users/test/proj", "main")]),
    );
    create_file(
        &claude_dir,
        &format!("projects/proj/{sess}.jsonl"),
        &format!("{}\n", user_text_msg("u1", None, sess, "test")),
    );

    // First run
    let r1 = indexer::run_index(IndexConfig {
        claude_dir: claude_dir.clone(),
        extra_dirs: vec![],
        db_path: db_path.clone(),
        full: false,
        verbose: false,
        skip_dirs: blacklight::indexer::scanner::DEFAULT_SKIP_DIRS.iter().map(|s| (*s).to_string()).collect(),
        progress: None,
        cancel_flag: None,
        pause_flag: None,
        notify_tx: None,
    })
    .unwrap();
    assert!(r1.files_processed > 0);

    // Second run with --full
    let r2 = indexer::run_index(IndexConfig {
        claude_dir,
        extra_dirs: vec![],
        db_path,
        full: true,
        verbose: false,
        skip_dirs: blacklight::indexer::scanner::DEFAULT_SKIP_DIRS.iter().map(|s| (*s).to_string()).collect(),
        progress: None,
        cancel_flag: None,
        pause_flag: None,
        notify_tx: None,
    })
    .unwrap();
    assert!(
        r2.files_processed > 0,
        "--full should re-process all files"
    );
    assert_eq!(r2.files_unchanged, 0);
}

// ---------------------------------------------------------------------------
// Test: Summary-first JSONL files (summary before any user/assistant message)
// ---------------------------------------------------------------------------

#[test]
fn test_summary_first_in_jsonl() {
    let tmp = TempDir::new().unwrap();
    let claude_dir = tmp.path().join("claude");
    let db_path = tmp.path().join("test.db");
    let sess = "eeeeeeee-1111-2222-3333-444444444444";

    create_file(
        &claude_dir,
        "projects/proj/sessions-index.json",
        &sessions_index(&[(sess, "prompt", "summary", "/Users/test/proj", "main")]),
    );

    // Summary appears BEFORE any user/assistant message
    let mut jsonl = String::new();
    jsonl.push_str(&summary_msg("leaf-99", "This session explored public domain literature."));
    jsonl.push('\n');
    jsonl.push_str(&user_text_msg("u1", None, sess, "hello"));
    jsonl.push('\n');

    create_file(
        &claude_dir,
        &format!("projects/proj/{sess}.jsonl"),
        &jsonl,
    );

    let report = indexer::run_index(IndexConfig {
        claude_dir,
        extra_dirs: vec![],
        db_path: db_path.clone(),
        full: false,
        verbose: false,
        skip_dirs: blacklight::indexer::scanner::DEFAULT_SKIP_DIRS.iter().map(|s| (*s).to_string()).collect(),
        progress: None,
        cancel_flag: None,
        pause_flag: None,
        notify_tx: None,
    })
    .unwrap();

    // Should succeed without FK errors
    assert_eq!(report.parse_errors, 0);
    assert!(report.messages_processed >= 2);

    // Summary message should have the correct session_id (derived from filename)
    let conn = db::open(&db_path).unwrap();
    let summary_sess: String = conn
        .query_row(
            "SELECT session_id FROM messages WHERE id = 'summary-leaf-99'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(summary_sess, sess);
}
