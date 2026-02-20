# LLM Token Use: An Empirical Analysis of Agentic Coding Costs

## Executive Summary

We analyzed 4.5 months of agentic coding sessions (October 2025 – February 2026) across Claude, Gemini, and Codex using [Blacklight](https://github.com/neuroglyph/blacklight), a tool that indexes every conversation, tool call, and file operation stored locally by these LLM coding agents. The dataset spans **1,091 sessions**, **291,265 messages**, and **367 MB of deduplicated content** across 784 Claude sessions and 202 Gemini sessions.

### Key Finding

The dominant cost in agentic coding is not individual file reads or test runs. It is **context compounding** — tool outputs that enter the conversation and are replayed on every subsequent API call for the remainder of the session. A single large file read early in a long session can cost more in compounded replay than hundreds of small reads in short sessions.

### Headline Numbers

- **Read tool calls account for 96.2 GB of "context burden"** (output bytes × remaining messages in session) — more than 6× all other tools combined.
- **The top 3 sessions (out of 715 measured) account for 23% of all lifetime context burden.** The top 30 account for 53.6%. Cost follows a steep power law.
- A **dynamic read size cap** (tightening as sessions grow longer) would reduce Read burden by **54.5%**.
- **Capping session length at 500 messages** would reduce it by **58.9%**.
- **Both interventions combined yield a 75.1% reduction** in Read context burden.

---

## 1. Background: What Is a "Token" and Why Does It Matter?

### 1.1 Tokens Are Subword Chunks

LLMs process text as "tokens" — subword units produced by a tokenizer (typically Byte Pair Encoding). A token is not a word and not a character, but something in between.

| Input | Approximate Tokens |
|-------|-------------------|
| `hello` | 1 token |
| `tokenization` | 2–3 tokens |
| Whitespace / indentation | Tokens (this is why code is expensive) |
| `{}();` | Each punctuation char can be its own token |
| A typical English word | ~1 token |
| 1 line of code | ~10–15 tokens |
| An image/screenshot | ~1,600 tokens (fixed visual grid) |

**Rule of thumb:** ~4 characters ≈ 1 token in English prose. Code is worse due to syntax characters and indentation.

### 1.2 The Real Cost: Context Replay

LLMs are stateless. Every API call sends the **entire conversation history** — system prompt, all prior messages, all tool results — as input. This means:

- Turn 1: 5K tokens (system prompt + first message)
- Turn 2: 10K tokens (turn 1 replayed + new work)
- Turn 3: 15K tokens
- Turn N: 5K × N tokens

A 500-line file read (~5,000 tokens) at turn 3 of a 20-turn session is replayed 17 times. That single read effectively costs 5,000 × 17 = **85,000 tokens**.

This is **context compounding**, and it is the dominant cost driver in agentic coding.

### 1.3 "Token" Is Not Comparable Across Vendors

Different LLMs use different tokenizers with different vocabularies. A "token" in Claude is not the same unit as a "token" in Gemini or Codex. For cross-vendor comparison, we use **bytes of tool output** as a vendor-neutral proxy, and **context burden** (bytes × remaining messages) as the primary metric.

---

## 2. The Database: What Blacklight Stores

### 2.1 Data Sources

Blacklight indexes conversation data from three LLM coding agents:

| Agent | Storage Location | Format | Sessions |
|-------|-----------------|--------|----------|
| Claude Code | `~/.claude/projects/**/*.jsonl` | JSONL (one JSON object per line) | 784 |
| Gemini CLI | `~/.gemini/tmp/<hash>/*.jsonl` | JSONL | 202 |
| Codex CLI | `~/.codex/sessions/` | JSONL | (not yet indexed) |

Each session is a continuous conversation between the user and the LLM. A `/clear` command (or equivalent) ends one session and starts a new one.

### 2.2 What Gets Captured

Every session file contains a stream of messages, each typed:

- **`user`** — user messages, including tool results returned to the LLM
- **`assistant`** — LLM responses containing text, tool calls, and thinking blocks
- **`system`** — session events (timing, configuration)
- **`progress`** — real-time tool execution status
- **`file-history-snapshot`** — file state tracking for undo

### 2.3 Blacklight's Schema

Blacklight parses these streams into a normalized SQLite database:

| Table | Purpose |
|-------|---------|
| `sessions` | Session metadata (project path, timestamps, message count, git branch) |
| `messages` | Individual messages with type, timestamp, model, stop reason, turn index |
| `content_store` | BLAKE3-hashed content blobs with size and kind (text, tool_input, tool_output, thinking) |
| `tool_calls` | Denormalized tool invocations: tool name, input hash, output hash, timestamp |
| `file_references` | File path → operation (read/write/edit) → session/message mapping |
| `content_blocks` | Message content broken into typed blocks (text, tool_use, tool_result, thinking) |
| `blob_references` | Maps content blobs back to messages and contexts |

**Key design decision:** Content is deduplicated via BLAKE3 hashing. If the same file is read 100 times with identical content, the blob is stored once. The `tool_calls` table references it by hash, preserving the *count* and *timing* of reads without duplicating storage.

### 2.4 Scale

| Metric | Value |
|--------|-------|
| Database size | 1.3 GB |
| Total sessions | 1,091 |
| Total messages | 291,265 |
| Total content blobs | 229,585 |
| Total content size (deduplicated) | 367.2 MB |
| Date range | October 6, 2025 – February 20, 2026 |
| Claude sessions | 784 |
| Gemini sessions | 202 |

---

## 3. Discovery: How We Found the Problems

### 3.1 Methodology

We queried the Blacklight database with progressively targeted SQL queries, following a pattern of:

1. **Broad measurement** — aggregate tool usage counts and bytes
2. **Hypothesis formation** — identify candidate anti-patterns
3. **Targeted measurement** — write specific queries to test each hypothesis
4. **Validation / rejection** — check whether measurements support the hypothesis
5. **Counterfactual analysis** — simulate interventions and estimate impact

Several initial hypotheses were **rejected** after measurement correction (see Section 8). This is a feature, not a bug — it prevented us from building solutions to nonexistent problems.

### 3.2 The Key Metric: Context Burden

Raw bytes of tool output is a misleading metric. A 50KB file read at the end of a session (with 2 messages remaining) costs far less than a 5KB read at the beginning (with 500 messages remaining).

**Context Burden** accounts for this:

```
Context Burden = output_bytes × messages_remaining_in_session
```

This weights early-session, large outputs heavily (they compound across many turns) and discounts late-session outputs (minimal compounding).

The query:

```sql
WITH msg_ord AS (
  SELECT id AS message_id, session_id,
         ROW_NUMBER() OVER (PARTITION BY session_id ORDER BY timestamp) AS msg_idx,
         COUNT(*) OVER (PARTITION BY session_id) AS msg_total
  FROM messages
),
tc AS (
  SELECT tc.tool_name, tc.session_id,
         cs_out.size AS out_bytes,
         (mo.msg_total - mo.msg_idx + 1) AS msgs_remaining
  FROM tool_calls tc
  JOIN msg_ord mo ON mo.message_id = tc.message_id
  JOIN content_store cs_out ON cs_out.hash = tc.output_hash
)
SELECT tool_name,
       COUNT(*) AS calls,
       ROUND(SUM(CAST(out_bytes AS REAL) * msgs_remaining)
             / 1024.0/1024.0/1024.0, 2) AS burden_gb
FROM tc
GROUP BY tool_name
ORDER BY burden_gb DESC;
```

---

## 4. Key Findings

### 4.1 Tool Usage Overview

| Tool | Calls | Raw Output MB | Avg Output Bytes |
|------|-------|--------------|-----------------|
| Read | 29,638 | 225.25 | 7,969 |
| Bash | 24,673 | 37.21 | 1,581 |
| Edit | 15,001 | 2.45 | 171 |
| Grep | 5,979 | 5.17 | 907 |
| Glob | 3,166 | 3.74 | 1,239 |
| Task (subagent) | 2,282 | 11.90 | 5,466 |
| Write | 2,411 | 1.03 | 447 |

**Read dominates** — 63% of all tool output bytes, from one tool.

### 4.2 Context Burden by Tool

| Tool | Calls | Burden (GB) | Avg Burden per Call (MB) |
|------|-------|-------------|------------------------|
| **Read** | **29,225** | **96.22** | **3.37** |
| Bash | 19,956 | 14.54 | 0.75 |
| Task (subagent) | 2,199 | 4.98 | 2.32 |
| TaskOutput | 344 | 2.00 | 5.96 |
| Grep | 4,263 | 2.12 | 0.51 |
| Glob | 2,128 | 1.54 | 0.74 |
| Edit | 8,813 | 0.87 | 0.10 |

Read's burden (96.2 GB) is **6.6× larger** than the next tool (Bash at 14.5 GB). Everything else is rounding error.

### 4.3 Burden Is Concentrated in a Few Sessions

| Bucket | % of Total Burden |
|--------|------------------|
| Top 3 sessions (of 715) | 23.0% |
| Top 10 sessions | 36.3% |
| Top 30 sessions | 53.6% |

The cost distribution follows a steep power law. The single worst session (`1db1df52`, git-warp project) has 12.7 GB of context burden — 8.1% of everything, ever.

That session's signature:
- 255 Edit→Bash ping-pong cycles
- ~5,900 messages
- Multiple 30–85 KB file reads early in the session, replayed across thousands of subsequent messages

### 4.4 Read Size Distribution

| Read Size | Calls | Raw MB | % of Read Burden |
|-----------|-------|--------|-----------------|
| < 5 KB | 17,333 | 36.66 | (low burden) |
| 5–10 KB | 5,762 | 38.98 | |
| 10–20 KB | 3,728 | 49.34 | |
| 20–40 KB | 1,695 | 44.61 | |
| **40 KB+** | **707** | **55.66** | **21.2% of burden** |
| **20 KB+** | **2,402** | **100.27** | **43.7% of burden** |

The fattest 2.4% of reads (40 KB+) produce 24% of raw bytes and 21.2% of burden. The top 8.2% (20 KB+) produce 43.7% of burden.

### 4.5 Surgical vs. Full-File Reads

| Read Type | Calls | Avg Output | Total MB |
|-----------|-------|------------|----------|
| Full-file read (no offset/limit) | 8,427 | 9,340 bytes | 75.06 |
| Surgical read (with offset/limit) | 6,049 | 3,573 bytes | 20.61 |

58% of reads are full-file. Surgical reads average 2.6× smaller. The capability to read ranges already exists — it's just underused.

### 4.6 File Re-Read Patterns

**Within sessions:** Thousands of cases where the same file is read multiple times in a single conversation:

| Re-reads of Same File | Sessions with This Pattern |
|-----------------------|---------------------------|
| 2× | 2,817 |
| 3× | 1,320 |
| 4× | 629 |
| 5× | 348 |
| 10+ | 45 |
| 20+ | 6 |
| 68× (max) | 1 |

**Across sessions:** The worst offender, `WarpGraph.js`, has been read **1,053 times across 85 sessions**, with a context burden of **1.74 GB** from that one file alone.

### 4.7 The Top 10 Files by Context Burden

| File | Reads | Burden (MB) | Max Read Size |
|------|-------|-------------|---------------|
| WarpGraph.js | 722 | 1,744 | 100 KB |
| index.d.ts | 105 | 761 | 96 KB |
| bin/warp-graph.js | 442 | 648 | 100 KB |
| ROADMAP.md | 184 | 482 | 97 KB |
| **seek-demo.gif** | **4** | **395** | **1,302 KB** |
| GitGraphAdapter.js | 92 | 456 | 35 KB |
| Mesh.vue | 170 | 416 | 59 KB |
| JoinReducer.js | 81 | 395 | 34 KB |
| PatchBuilderV2.js | 86 | 367 | 34 KB |
| warp-graph-viewer/bin/warp-graph.js | 65 | 257 | 58 KB |

Note `seek-demo.gif`: read only **4 times** but each read was 1.3 MB of binary GIF data. Those 4 reads generated more burden than hundreds of normal code reads because the payload was enormous and it happened early in long sessions.

### 4.8 Session Depth Statistics

| Metric | Value |
|--------|-------|
| Average messages per session | 266.7 |
| Average tool calls per session | 103.0 |
| Maximum messages in one session | 6,004 |
| Maximum tool calls in one session | 2,285 |

### 4.9 Non-Edit Reads (Exploration Cost)

64.5% of file reads in a session do not lead to an edit of that file. This is not strictly "waste" — reading `package.json` or test fixtures informs edits to other files — but it represents context acquisition cost that could potentially be replaced by cheaper representations (outlines, indexes, summaries).

### 4.10 Edit→Bash Ping-Pong Loops

Sessions with extreme Edit→Bash cycling (edit code, run test, edit code, run test, ...):

| Edit→Bash Cycles | Sessions |
|-------------------|----------|
| 10+ | Many |
| 30+ | 15 |
| 50+ | 3 |
| **255** (max) | **1** |

The worst session (`1db1df52`) had 255 edit→test cycles. Each cycle adds tool output to a context that was already thousands of messages deep, creating compounding burden with every iteration.

### 4.11 Subagent Output Explosions

The Task tool (subagent) has a bimodal output distribution:

| Output Size | Count | Total MB |
|-------------|-------|----------|
| < 500 bytes | 514 | 0.22 |
| 500 B – 2 KB | 517 | 0.64 |
| 2–10 KB | 684 | 3.24 |
| **10–25 KB** | **434** | **6.33** |
| **25–50 KB** | **49** | **1.42** |
| **50 KB+** | **1** | **0.05** |

When TaskOutput (the mechanism for reading subagent results back into the main context) is examined, the picture is worse: **71% of TaskOutput calls (244 of 344) return 25 KB+**, totaling 7.65 MB.

The pattern: research/planning subagents return their entire analysis as the result. Code-writing subagents return short confirmations. The difference is entirely in the prompt — "work silently" produces ~460 bytes; "provide detailed specifications" produces ~40 KB.

### 4.12 Gemini Data Quality Issue (Resolved)

All 18,913 Gemini tool output records originally had `size = 0` in the content_store despite containing actual content data (verified by checking `LENGTH(content)`). This was an ingestion bug — the size field was not being populated during Gemini session parsing.

**Fix applied:** `UPDATE content_store SET size = LENGTH(CAST(content AS BLOB)) WHERE size = 0 AND content IS NOT NULL`. After backfill, Gemini tool outputs total **121.4 MB** across 18,912 calls. Cross-LLM burden analysis is now possible.

### 4.13 Gemini Context Burden (Post-Fix)

With corrected sizes, Gemini's burden profile:

| Tool | Calls | Burden (GB) | Avg Raw Bytes | Avg Msgs Remaining |
|------|-------|-------------|---------------|-------------------|
| run_shell_command | 5,484 | 5.99 | 10,289 | 142 |
| read_file | 4,025 | 5.24 | 10,833 | 153 |
| read_many_files | 75 | 0.90 | **105,913** | 113 |
| read_multiple_files | 181 | 0.56 | **23,844** | 174 |
| read_media_file | 10 | 0.50 | **551,627** | 91 |
| search_file_content | 327 | 0.51 | 11,689 | 173 |

Key observations:

- **Gemini's `read_file` averages 10.8 KB** per read — larger than Claude's 8.0 KB. Gemini reads bigger chunks.
- **`read_many_files` averages 105 KB per call** — batch reading that dumps multiple files into one response. This is 13× larger than a single read. Batch read tools may amplify burden.
- **`read_media_file` averages 551 KB per call** — Gemini's image reading tool produces enormous outputs. 10 media reads at ~550 KB each = 5.5 MB of raw output, plus compounding.
- **`run_shell_command` is Gemini's #1 burden source** (5.99 GB), not file reading. Gemini appears to run more and bigger shell commands than Claude, averaging 10.3 KB per command output vs. Claude's 1.6 KB.
- Gemini's total tool burden (~14.7 GB across top tools) is lower than Claude's (~130 GB) primarily because Gemini sessions are shorter (202 sessions vs 784) and have fewer messages remaining on average (~150 vs ~400).

### 4.14 Cross-LLM Tool Comparison

| Metric | Claude | Gemini |
|--------|--------|--------|
| Sessions | 784 | 202 |
| Messages | 155,936 | 20,909 |
| Tool output volume | 329.4 MB | 121.4 MB |
| Top burden tool | Read (96.2 GB) | run_shell_command (6.0 GB) |
| #2 burden tool | Bash (14.5 GB) | read_file (5.2 GB) |
| #3 burden tool | Task/subagent (5.0 GB) | read_many_files (0.9 GB) |
| Batch read tool | N/A | read_multiple_files (181 calls), read_many_files (75 calls) |
| Avg read size | 8.0 KB | 10.8 KB |
| Avg shell output | 1.6 KB | 10.3 KB |

Both agents show file reading as a dominant cost, but with different profiles:
- **Claude's problem is Read compounding in long sessions** (high msgs_remaining averaging 441).
- **Gemini's problem is larger shell outputs** (6.4× bigger than Claude's) and batch read tools that dump multiple files at once (105 KB average for `read_many_files`).
- Gemini's `read_media_file` tool (for images) averages **551 KB per call** — confirming that image/binary reads are an issue across LLMs, not just Claude.

---

## 5. The Plan: Two Interventions

Based on counterfactual analysis, two interventions dominate all others.

### 5.1 Intervention 1: SafeRead (Dynamic Read Size Cap)

**Mechanism:** Replace or gate the native Read tool with a policy layer that limits how much content enters the conversation.

**Rules (v1):**

1. **Ban binary extensions outright:**
   `.gif`, `.png`, `.jpg`, `.jpeg`, `.pdf`, `.zip`, `.wasm`, `.bin`, `.sqlite`, `.mp4`, `.mov`, `.ico`
   Response: refuse the read; suggest `ls -lh` or `file` for metadata, or a dedicated image viewer.

2. **Ban generated/build output paths:**
   `bin/`, `dist/`, `build/`, `out/`, `.next/`, `target/`
   Response: redirect to the corresponding source file.

3. **Dynamic size cap based on session depth:**

   | Session Stage | Messages Remaining | Max Read Output |
   |--------------|-------------------|----------------|
   | Early | < 100 remaining | 20 KB |
   | Mid | 100–500 remaining | 10 KB |
   | Late | > 500 remaining | 4 KB |

   When a read exceeds the cap, return instead:
   - A **file outline** (exported symbols, function signatures, class shapes, line ranges)
   - A **jump table** ("function `merge()` is at lines 247–298")
   - Instructions to request a specific range if more detail is needed

**Counterfactual impact:**

| Cap Strategy | Burden After | Reduction |
|-------------|-------------|-----------|
| Flat 2 KB cap | 21.1 GB | 78.0% |
| Flat 5 KB cap | 40.4 GB | 58.0% |
| Flat 10 KB cap | 58.8 GB | 38.9% |
| Flat 20 KB cap | 76.1 GB | 20.9% |
| **Dynamic cap (4/10/20 KB)** | **43.8 GB** | **54.5%** |

The dynamic cap is the best balance: generous early (allowing exploration) and strict late (preventing compounding catastrophes).

### 5.2 Intervention 2: Session Length Management

**Mechanism:** Prevent sessions from growing into "infinite money furnaces" by detecting runaway sessions and forcing context resets.

**Tripwires (trigger when any threshold is exceeded):**

- `messages_in_session > 500`
- `edit_bash_transitions > 30` (ping-pong loop detection)
- `tool_calls_since_last_user_message > 80` (stuck-in-a-loop detection)
- Any single tool output > 20 KB after the session is already > 300 messages

**Actions when tripped:**

1. Write/update a `WORKING_STATE.md` file capturing:
   - Current task and hypothesis
   - Files modified and their states
   - Next 3 planned actions
   - Key findings so far
2. Recommend or force a `/clear` (context reset)
3. Next session begins by reading `WORKING_STATE.md` (~200–500 tokens) instead of replaying thousands of messages

**Counterfactual impact:**

| Strategy | Burden After | Reduction |
|----------|-------------|-----------|
| Cap sessions at 500 messages | 39.6 GB | 58.9% |

Note: The original measurement (45.0%) undercounted savings by excluding post-500 reads from the baseline side. The corrected query properly accounts for the full tail that would be eliminated.

### 5.3 Combined Impact

| Intervention | Reduction | Mechanism |
|-------------|-----------|-----------|
| SafeRead (dynamic cap) alone | 54.5% | Smaller payloads per read |
| Session cap alone | 58.9% | Fewer messages to compound across |
| **Both combined** | **75.1%** | **Multiplicative — smaller payloads AND fewer messages** |

**From 96.2 GB of Read burden to 24.0 GB.** A 4× reduction.

---

## 6. Implementation Per LLM

### 6.1 Persistent Instructions

Each LLM coding agent has an equivalent mechanism for project-level instructions:

| Agent | Instruction File | Scope |
|-------|-----------------|-------|
| Claude Code | `CLAUDE.md` | Per-project, loaded every session |
| Codex CLI | `AGENTS.md` | Per-project |
| Gemini CLI | `GEMINI.md` | Per-project |

**Rules-only approach (minimum viable):**

Place equivalent instructions in all three files:

```markdown
## Read Policy
- Never read binary files (.gif, .png, .jpg, .pdf, .zip, .wasm). Use `ls -lh` instead.
- Never read build output (bin/, dist/, target/). Find the source file instead.
- Use offset/limit parameters when reading files > 200 lines. Read the outline first.
- Before re-reading a file, check if you already read it this session.

## Test Execution Policy
- Always run tests with: cmd 2>&1 | tee /tmp/test.log | tail -60
- If you need more output, read /tmp/test.log. Do NOT re-run the tests.

## Subagent Policy
- Subagent results must be under 500 bytes. Write detailed output to a file.

## Session Management
- After completing a task, update WORKING_STATE.md with current status.
- If you've been working for many turns, suggest /clear to the user.
```

**Limitation:** Models often "agree and then ignore" instruction-only rules. Enforcement is stronger.

### 6.2 Enforcement via Hooks (Claude Code)

Claude Code supports [hooks](https://docs.anthropic.com/en/docs/claude-code/hooks) — shell commands that execute in response to events:

- **`PreToolUse`** — runs before a tool call, can block it
- **`PostToolUse`** — runs after, can inspect results
- **`SessionStart`** — runs when a session begins

Example hook configuration (`.claude/hooks.json`):

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Read",
        "command": "python3 scripts/safe_read_gate.py"
      }
    ]
  }
}
```

The gate script can inspect the file path and reject binary reads, enforce size caps, or redirect to an outline tool.

### 6.3 Enforcement via MCP Server (Cross-LLM)

All three agents support MCP (Model Context Protocol) tool servers:

| Agent | MCP Support |
|-------|------------|
| Claude Code | Native MCP tool integration + hook matchers for `mcp__*` tools |
| Codex CLI | MCP configuration documented |
| Gemini CLI | Local/remote MCP server support |

A single MCP server exposing these tools works across all three:

| Tool | Replaces | Behavior |
|------|----------|----------|
| `safe_read(path, intent?)` | `Read` | Enforces size caps, returns outline for large files |
| `file_outline(path)` | Exploratory full-file reads | Returns exports, function signatures, line ranges |
| `read_range(path, start, end)` | Full-file reads for one function | Returns only the specified line range |
| `run_capture(cmd, tail_lines)` | `Bash` for test runs | Tees output to log, returns only tail |
| `state_load()` / `state_save(content)` | Manual breadcrumb management | Reads/writes WORKING_STATE.md |

This is the recommended approach for experiments because it provides **consistent enforcement** across all LLMs and **measurable compliance** (every tool call is logged by Blacklight).

---

## 7. User Experience Implications

### 7.1 What Changes for the User

**SafeRead:** Users will see the LLM working with outlines and targeted reads instead of dumping entire files. The LLM will say things like "I can see from the outline that `merge()` is at lines 247–298, let me read just that section" instead of silently reading 100 KB.

**For large files** the LLM may need 2 tool calls instead of 1 (outline → targeted read), but each call is dramatically smaller and the total context cost is lower.

**Session management:** Users will be prompted to `/clear` more frequently, with the LLM maintaining a `WORKING_STATE.md` that preserves continuity. The experience becomes:

```
You: "Fix the auth bug"
LLM: [reads state file, fixes bug, updates state] "Done. Tests pass."
You: /clear
You: "Now add rate limiting"
LLM: [reads state file, knows the project context, proceeds immediately]
```

Instead of:

```
You: "Fix the auth bug"
LLM: [reads 8 files, fixes bug]
You: "Now add rate limiting"
LLM: [re-reads 5 of the same files because context is getting compressed,
      plus pays for all the auth bug context compounding]
```

### 7.2 What Doesn't Change

- The LLM still reads files — it just reads less per call and reads smarter.
- The LLM can still be asked to read a full file explicitly if needed.
- Code quality should not be affected — the LLM has the same information, just delivered more efficiently.

### 7.3 Potential Risks

- **Over-aggressive gating** could prevent the LLM from reading files it genuinely needs. The dynamic cap mitigates this by being generous early.
- **State file quality** depends on the LLM writing useful summaries. If the state file is vague, the next session starts confused. This needs to be tested empirically.
- **Some tasks require broad context** (refactoring across many files). The outline-first approach may add friction for these cases. A "force full read" escape hatch should be available.

---

## 8. Ideas Considered and Rejected (With Evidence)

### 8.1 "Write-Then-Edit Is an Anti-Pattern" — REJECTED

**Hypothesis:** LLMs frequently write files incorrectly and immediately edit them, wasting tokens on the initial write.

**Initial measurement:** 639 Write→Edit sequences across 220 sessions.

**Corrected measurement:** After adding a 5-minute time window (filtering to "write then edit the *same file* within 5 minutes"), the count dropped to **0**.

**Conclusion:** The original 639 cases were "wrote a file at minute 1, edited it at minute 45" — which is normal software development, not an anti-pattern. No intervention needed.

### 8.2 "Read-Then-Edit-Same-File Is Wasteful" — REJECTED

**Hypothesis:** LLMs read entire files just to make small edits, wasting most of the read content.

**Initial measurement:** 27 Read→Edit sequences within 120 seconds (not joined on file path).

**Corrected measurement:** After joining on the same file path, the count dropped to **1**.

**Conclusion:** LLMs almost never read a file and then immediately edit it. They read files to build context for editing *other* files. The fix is not "read less before editing" — it's "acquire context through cheaper means (outlines, indexes)."

### 8.3 "Edit Failure Cascades Are a Major Problem" — REJECTED

**Hypothesis:** Failed edits (non-unique `old_string`) trigger retry cascades on the same file, wasting turns.

**Measurement:** Edit failure rate is 7.3% (640 of 8,813), but failed-edit-then-retry-same-file-within-2-minutes was **0 cases**.

**Conclusion:** Edit failures exist but they're scattered, not clustered into expensive cascades. The 7.3% rate is annoying but not a cost driver.

### 8.4 "Bash Error Rate Is 63.6%" — MEASUREMENT INVALID

**Initial measurement:** Using `LIKE '%error%'` on Bash output text classified 63.6% of sessions as "error-dominant."

**Problem:** This captures PR comments containing the word "error," test output mentioning "error" even when the command succeeded, and many other false positives.

**Attempted fix:** Looked for structured exit code signals (`Exit code 1`, etc.). Found that 100% of Bash outputs came back as "no exit code reported" — the exit code is not consistently captured in the tool output format.

**Conclusion:** Cannot measure Bash error rate accurately with current data. This is a **Blacklight ingestion improvement** — parse and store exit codes as a separate column during indexing.

### 8.5 "AST/LSP Symbol Index" — DEFERRED, NOT REJECTED

**Idea:** Parse project files into an AST-based symbol index, allowing the LLM to look up specific functions/classes by name without reading whole files.

**Assessment:** This is a good idea, but the counterfactual analysis shows that **SafeRead caps and session length management address 75% of the problem without any indexing infrastructure**. Symbol indexing would help with the remaining 25% (the "where is function X?" queries that currently trigger full-file reads), but it's a larger engineering investment.

**Decision:** Build SafeRead and session management first. Add symbol indexing after, once the "dumb waste" is eliminated and the marginal value of indexing can be measured cleanly.

### 8.6 "git-mind Knowledge Graph as Cross-Session State" — DEFERRED, NOT REJECTED

**Idea:** Use [git-mind](https://github.com/neuroglyph/git-mind) (a typed knowledge graph stored in Git) as the state store between sessions, replacing the simple `WORKING_STATE.md` markdown file.

**Assessment:** git-mind provides typed edges, confidence scores, and time-travel — strictly more capable than a markdown file. However:

- It has a **cold start problem** (empty graph provides no value)
- It requires the LLM to learn git-mind's CLI and edge vocabulary
- A markdown file is trivially simple and has no dependencies

**Decision:** Start with `WORKING_STATE.md` for session state. Graduate to git-mind once:

1. Shallow sessions are proven to work
2. The markdown file's limitations become apparent (e.g., it can't represent complex inter-file relationships)
3. git-mind edges can be auto-populated from Blacklight data (eliminating cold start)

### 8.7 "Binary Extension Ban" — LOW ROI

**Counterfactual:** Banning binary extensions (.gif, .png, .jpg, etc.) saves ~0.86 GB of burden — **0.89%** of total (corrected query using LEFT JOIN on input hash to match the full 96.22 GB population).

**However:** While the aggregate savings are small, individual incidents can be catastrophic (the GIF read: 4 reads → 395 MB burden). It should be implemented because it's trivially easy and prevents outlier disasters, even though it doesn't move aggregate numbers. Reading a GIF into context is a policy failure, not a cost optimization — ban it as cheap insurance.

### 8.8 "Build Path Ban" — LOW ROI

**Counterfactual:** Banning reads from build output paths (bin/, dist/, target/) saves ~1.9 GB — **1.98%** of total burden (corrected).

**Decision:** Implement as part of SafeRead rules (trivially easy), but don't expect it to move the needle significantly.

### 8.9 "Blacklight → git-mind Feedback Loop" — INTERESTING, FUTURE WORK

**Idea:** Mine Blacklight's historical data to auto-generate git-mind edges. For example: if the LLM reads files A, B, and C together in 90% of sessions, infer a `depends-on` relationship. Use these inferred relationships to pre-populate git-mind graphs for projects, eliminating cold start.

**Assessment:** This is genuinely interesting and novel, but it's a Phase 2+ idea that depends on:

1. Proving that session state (in any form) improves efficiency
2. Proving that structured state (git-mind) outperforms unstructured (markdown)
3. Having accurate Gemini and Codex data for cross-LLM comparison

**Decision:** Document as future work. Pursue after the foundational experiments are complete.

---

## 9. Research Program

### 9.1 Immediate Next Steps

1. **Enforce SafeRead** — implement as a Claude Code hook or MCP tool
2. **Add runaway session breaker** — detect ping-pong loops and force state-save + clear
3. **Fix Gemini ingestion bug** — populate `content_store.size` from actual content length
4. **Run the A/B experiment:**
   - Control: normal session, no rules
   - Treatment: SafeRead + session management + tee-for-tests
   - 3 LLMs × 5 replicates × 1 macrobenchmark

### 9.2 Metrics

**Primary:**
- Cost per successful task (or burden per successful task)
- Task success rate (tests pass + feature works)

**Secondary:**
- Read calls count and total bytes per task
- Re-read ratio (same file read > 1× per session)
- Max context burden in a single session
- Edit→Bash ping-pong cycle count
- p95 / p99 session burden (tail risk)
- Number of sessions exceeding catastrophic threshold (> 1 GB burden)

### 9.3 Experiment Design

**Macrobenchmark:** A toy repo (~10 files, ~500 LOC) with planted bugs, missing features, and cross-file dependencies. Task sequence: fix bug → add feature → update tests → refactor → ensure all tests pass.

**Conditions:**
- A: Continuous session (baseline)
- B: Continuous session + SafeRead rules + tee-for-tests (rules only)
- C: Shallow sessions + WORKING_STATE.md + SafeRead (full treatment)

**Microbenchmarks:**
- Tail escalation trap (test that fails with error at line ~250 of 400 lines output)
- Re-read trap (bug in one function of an 800-line file)
- Huge file trap (40 KB+ file that needs a small edit)
- Subagent explosion trap (ask for "detailed implementation plan")

---

## 10. Appendix: Notable Data Points

### 10.1 The Session From Hell

Session `1db1df52-af1b-47f6-82c2-0c1917234671` (git-warp project):
- **12.7 GB context burden** (8.1% of all-time total)
- 255 Edit→Bash ping-pong cycles
- ~5,900 messages
- Multiple 30–85 KB reads early, replayed across thousands of subsequent messages
- Appears **13 times** in the top 20 individual tool calls by burden

This single session cost more than 685 other sessions combined.

### 10.2 The GIF Incident

`seek-demo.gif` — a 1.3 MB GIF animation — was read 4 times. Each read dumped 1.3 MB of binary data into the conversation context. With hundreds of messages remaining, those 4 reads generated **395 MB of context burden** — more than most files that were read hundreds of times.

### 10.3 Models Used

| Model | Messages | Sessions |
|-------|----------|----------|
| claude-opus-4-5-20251101 | 59,787 | 234 |
| claude-opus-4-6 | 58,589 | 507 |
| claude-haiku-4-5-20251001 | 28,239 | 299 |
| claude-sonnet-4-5-20250929 | 9,321 | 71 |
| gemini-3-flash-preview | 7,446 | 90 |
| gemini-3-pro-preview | 6,760 | 103 |
| gemini-2.5-pro | 3,369 | 73 |
| gemini-2.5-flash | 3,306 | 72 |

---

*Report generated February 20, 2026. Data source: Blacklight v0.x indexing `~/.claude/` and `~/.gemini/` directories.*
