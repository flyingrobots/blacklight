use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;

use crate::content;
use crate::models::{SessionFacet, StatsCache, TaskRecord};

use super::scanner::FileEntry;

// ---------------------------------------------------------------------------
// Tasks
// ---------------------------------------------------------------------------

/// Parse task JSON files and populate the tasks + task_dependencies tables.
/// session_id is derived from the parent directory name (a UUID).
pub fn parse_tasks(conn: &Connection, files: &[FileEntry]) -> Result<usize> {
    let mut count = 0;
    let tx = conn.unchecked_transaction()?;

    {
        let mut task_stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO tasks (id, session_id, subject, description, active_form, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        let mut dep_stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO task_dependencies (session_id, task_id, depends_on)
             VALUES (?1, ?2, ?3)",
        )?;

        for file in files {
            let session_id = file
                .path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".into());

            let data = match std::fs::read_to_string(&file.path) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("failed to read {}: {e}", file.path.display());
                    continue;
                }
            };

            let task: TaskRecord = match serde_json::from_str(&data) {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("failed to parse {}: {e}", file.path.display());
                    continue;
                }
            };

            task_stmt.execute(params![
                task.id,
                session_id,
                task.subject,
                task.description,
                task.active_form,
                task.status,
            ])?;

            for dep in &task.blocked_by {
                dep_stmt.execute(params![session_id, task.id, dep])?;
            }

            count += 1;
        }
    }

    tx.commit()?;
    tracing::info!("parsed {count} tasks");
    Ok(count)
}

// ---------------------------------------------------------------------------
// Facets (session outcomes)
// ---------------------------------------------------------------------------

/// Parse facet JSON files and populate session_outcomes, outcome_categories, outcome_friction.
pub fn parse_facets(conn: &Connection, files: &[FileEntry]) -> Result<usize> {
    let mut count = 0;
    let tx = conn.unchecked_transaction()?;

    {
        let mut outcome_stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO session_outcomes
             (session_id, underlying_goal, outcome, helpfulness, session_type, primary_success, friction_detail, brief_summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        let mut cat_stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO outcome_categories (session_id, category, count)
             VALUES (?1, ?2, ?3)",
        )?;
        let mut friction_stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO outcome_friction (session_id, friction_type, count)
             VALUES (?1, ?2, ?3)",
        )?;

        for file in files {
            let data = match std::fs::read_to_string(&file.path) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("failed to read {}: {e}", file.path.display());
                    continue;
                }
            };

            let facet: SessionFacet = match serde_json::from_str(&data) {
                Ok(f) => f,
                Err(e) => {
                    tracing::warn!("failed to parse {}: {e}", file.path.display());
                    continue;
                }
            };

            // session_id: from the JSON field, or derive from filename
            let session_id = facet
                .session_id
                .clone()
                .or_else(|| {
                    file.path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                })
                .unwrap_or_else(|| "unknown".into());

            outcome_stmt.execute(params![
                session_id,
                facet.underlying_goal,
                facet.outcome,
                facet.claude_helpfulness,
                facet.session_type,
                facet.primary_success,
                facet.friction_detail,
                facet.brief_summary,
            ])?;

            if let Some(cats) = &facet.goal_categories {
                for (category, cnt) in cats {
                    cat_stmt.execute(params![session_id, category, cnt])?;
                }
            }

            if let Some(friction) = &facet.friction_counts {
                for (friction_type, cnt) in friction {
                    friction_stmt.execute(params![session_id, friction_type, cnt])?;
                }
            }

            count += 1;
        }
    }

    tx.commit()?;
    tracing::info!("parsed {count} facets");
    Ok(count)
}

// ---------------------------------------------------------------------------
// Stats cache
// ---------------------------------------------------------------------------

/// Parse stats-cache.json and populate daily_stats + model_usage tables.
pub fn parse_stats_cache(conn: &Connection, path: &Path) -> Result<()> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let stats: StatsCache = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    let tx = conn.unchecked_transaction()?;

    {
        let mut daily_stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO daily_stats (date, message_count, session_count, tool_call_count)
             VALUES (?1, ?2, ?3, ?4)",
        )?;
        for day in &stats.daily_activity {
            daily_stmt.execute(params![
                day.date,
                day.message_count,
                day.session_count,
                day.tool_call_count,
            ])?;
        }
    }

    {
        let mut model_stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO model_usage (model, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for (model, usage) in &stats.model_usage {
            model_stmt.execute(params![
                model,
                usage.input_tokens,
                usage.output_tokens,
                usage.cache_read_tokens,
                usage.cache_creation_tokens,
            ])?;
        }
    }

    tx.commit()?;
    tracing::info!(
        "parsed stats-cache: {} days, {} models",
        stats.daily_activity.len(),
        stats.model_usage.len()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Plans (markdown)
// ---------------------------------------------------------------------------

/// Parse plan markdown files into content_store + FTS.
pub fn parse_plans(conn: &Connection, files: &[FileEntry]) -> Result<usize> {
    let mut count = 0;
    let tx = conn.unchecked_transaction()?;

    {
        let mut blob_stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO content_store (hash, content, size, kind) VALUES (?1, ?2, ?3, ?4)",
        )?;
        let mut fts_check = tx.prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM fts_content WHERE hash = ?1)",
        )?;
        let mut fts_stmt = tx.prepare_cached(
            "INSERT INTO fts_content (hash, kind, content) VALUES (?1, ?2, ?3)",
        )?;

        for file in files {
            let data = match std::fs::read_to_string(&file.path) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("failed to read {}: {e}", file.path.display());
                    continue;
                }
            };

            if data.trim().is_empty() {
                continue;
            }

            let hash = content::hash_content(&data);
            blob_stmt.execute(params![hash, data, data.len() as i64, "plan"])?;

            let exists: bool = fts_check.query_row(params![&hash], |row| row.get(0))?;
            if !exists {
                fts_stmt.execute(params![hash, "plan", data])?;
            }

            count += 1;
        }
    }

    tx.commit()?;
    tracing::info!("parsed {count} plans");
    Ok(count)
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

/// Parse history.jsonl and index each prompt in content_store + FTS.
pub fn parse_history(conn: &Connection, path: &Path) -> Result<usize> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let mut count = 0;
    let tx = conn.unchecked_transaction()?;

    {
        let mut blob_stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO content_store (hash, content, size, kind) VALUES (?1, ?2, ?3, ?4)",
        )?;
        let mut fts_check = tx.prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM fts_content WHERE hash = ?1)",
        )?;
        let mut fts_stmt = tx.prepare_cached(
            "INSERT INTO fts_content (hash, kind, content) VALUES (?1, ?2, ?3)",
        )?;

        for line in data.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let val: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("history parse error: {e}");
                    continue;
                }
            };

            let display = val
                .get("display")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if display.is_empty() {
                continue;
            }

            let hash = content::hash_content(display);
            blob_stmt.execute(params![hash, display, display.len() as i64, "history_prompt"])?;

            let exists: bool = fts_check.query_row(params![&hash], |row| row.get(0))?;
            if !exists {
                fts_stmt.execute(params![hash, "history_prompt", display])?;
            }

            count += 1;
        }
    }

    tx.commit()?;
    tracing::info!("parsed {count} history entries");
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::indexer::scanner::FileKind;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_entry(path: PathBuf, kind: FileKind) -> FileEntry {
        FileEntry {
            path,
            kind,
            mtime_ms: 0,
            size_bytes: 0,
        }
    }

    #[test]
    fn test_parse_tasks() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        // Create task file: tasks/<session_id>/task.json
        let task_dir = tmp.path().join("tasks").join("sess-001");
        std::fs::create_dir_all(&task_dir).unwrap();
        let task_path = task_dir.join("task1.json");
        let mut f = std::fs::File::create(&task_path).unwrap();
        write!(
            f,
            r#"{{"id":"t1","subject":"Do thing","description":"Details","status":"pending","blockedBy":["t0"]}}"#
        ).unwrap();

        let entries = vec![make_entry(task_path, FileKind::TaskJson)];
        let count = parse_tasks(&conn, &entries).unwrap();
        assert_eq!(count, 1);

        let subject: String = conn
            .query_row(
                "SELECT subject FROM tasks WHERE id = 't1' AND session_id = 'sess-001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(subject, "Do thing");

        let dep: String = conn
            .query_row(
                "SELECT depends_on FROM task_dependencies WHERE task_id = 't1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dep, "t0");
    }

    #[test]
    fn test_parse_facets() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        let facet_path = tmp.path().join("facet.json");
        let mut f = std::fs::File::create(&facet_path).unwrap();
        write!(
            f,
            r#"{{
                "session_id": "sess-001",
                "underlying_goal": "build feature",
                "outcome": "success",
                "goal_categories": {{"coding": 1}},
                "friction_counts": {{"slow": 2}}
            }}"#
        ).unwrap();

        let entries = vec![make_entry(facet_path, FileKind::FacetJson)];
        let count = parse_facets(&conn, &entries).unwrap();
        assert_eq!(count, 1);

        let goal: String = conn
            .query_row(
                "SELECT underlying_goal FROM session_outcomes WHERE session_id = 'sess-001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(goal, "build feature");
    }

    #[test]
    fn test_parse_stats_cache() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        let stats_path = tmp.path().join("stats-cache.json");
        let mut f = std::fs::File::create(&stats_path).unwrap();
        write!(
            f,
            r#"{{
                "version": 1,
                "dailyActivity": [
                    {{"date": "2024-01-01", "messageCount": 10, "sessionCount": 2, "toolCallCount": 5}}
                ],
                "modelUsage": {{
                    "claude-3": {{"input_tokens": 1000, "output_tokens": 500, "cache_read_tokens": 100, "cache_creation_tokens": 50}}
                }}
            }}"#
        ).unwrap();

        parse_stats_cache(&conn, &stats_path).unwrap();

        let msg_count: i64 = conn
            .query_row(
                "SELECT message_count FROM daily_stats WHERE date = '2024-01-01'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(msg_count, 10);
    }

    #[test]
    fn test_parse_plans() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        let plan_path = tmp.path().join("plan.md");
        std::fs::write(&plan_path, "# My Plan\n\nDo some stuff.").unwrap();

        let entries = vec![make_entry(plan_path, FileKind::PlanMarkdown)];
        let count = parse_plans(&conn, &entries).unwrap();
        assert_eq!(count, 1);

        let kind: String = conn
            .query_row(
                "SELECT kind FROM content_store WHERE kind = 'plan'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kind, "plan");
    }

    #[test]
    fn test_parse_history() {
        let tmp = TempDir::new().unwrap();
        let conn = db::open(&tmp.path().join("test.db")).unwrap();

        let history_path = tmp.path().join("history.jsonl");
        let mut f = std::fs::File::create(&history_path).unwrap();
        writeln!(f, r#"{{"display":"fix the bug","timestamp":1704067200000}}"#).unwrap();
        writeln!(f, r#"{{"display":"add feature","timestamp":1704067201000}}"#).unwrap();

        let count = parse_history(&conn, &history_path).unwrap();
        assert_eq!(count, 2);

        let blob_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM content_store WHERE kind = 'history_prompt'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(blob_count, 2);
    }
}
