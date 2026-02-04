use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

use super::paths;
use super::types::{HistoryEntry, MessageEntry, SyncResult, TodoEntry};

pub fn get_last_sync_time(conn: &Connection, source: &str) -> Result<i64> {
    let result = conn.query_row(
        "SELECT last_sync_at FROM sync_state WHERE source = ?1",
        rusqlite::params![source],
        |row| row.get(0),
    );
    match result {
        Ok(val) => Ok(val),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
        Err(e) => Err(e.into()),
    }
}

pub fn update_sync_state(conn: &Connection, source: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "INSERT OR REPLACE INTO sync_state (source, last_sync_at, last_modified_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![source, now, now],
    )?;
    Ok(())
}

pub fn needs_sync(conn: &Connection, source: &str, interval_secs: u64) -> Result<bool> {
    if interval_secs == 0 {
        return Ok(false);
    }
    let last = get_last_sync_time(conn, source)?;
    let now = chrono::Utc::now().timestamp_millis();
    let interval_ms = interval_secs as i64 * 1000;
    Ok(now - last > interval_ms)
}

pub fn sync_history(conn: &Connection, claude_dir: &Path) -> Result<usize> {
    let path = paths::history_path(claude_dir);
    if !path.exists() {
        update_sync_state(conn, "history")?;
        return Ok(0);
    }

    let content = std::fs::read_to_string(&path)?;
    let entries: Vec<HistoryEntry> = paths::parse_jsonl(&content);
    let mut count = 0;

    for entry in &entries {
        let (session_id, project, display, timestamp) = match (
            entry.session_id.as_deref(),
            entry.project.as_deref(),
            entry.timestamp,
        ) {
            (Some(id), Some(proj), Some(ts)) => (id, proj, entry.display.as_deref(), ts as i64),
            _ => continue,
        };

        let changed = conn.execute(
            "INSERT OR IGNORE INTO sessions (id, project, display, started_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![session_id, project, display, timestamp],
        )?;
        if changed > 0 {
            count += 1;
        }
    }

    update_sync_state(conn, "history")?;
    Ok(count)
}

pub fn sync_sessions(conn: &Connection, claude_dir: &Path) -> Result<usize> {
    let projects = paths::list_project_dirs(claude_dir)?;
    let mut total = 0;

    for project in &projects {
        let sessions = paths::list_session_files(&project.dir)?;
        for session_file in &sessions {
            total += sync_session_file(conn, &project.path, session_file)?;
        }
    }

    update_sync_state(conn, "sessions")?;
    Ok(total)
}

fn sync_session_file(
    conn: &Connection,
    project_path: &str,
    session_file: &paths::SessionFile,
) -> Result<usize> {
    let content = std::fs::read_to_string(&session_file.path)?;
    let entries: Vec<MessageEntry> = paths::parse_jsonl(&content);
    let session_id = &session_file.session_id;

    let tx = conn.unchecked_transaction()?;

    // Ensure session exists
    let now = chrono::Utc::now().timestamp_millis();
    tx.execute(
        "INSERT OR IGNORE INTO sessions (id, project, started_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![session_id, project_path, now],
    )?;

    // Extract git_branch from first entry that has one
    let git_branch = entries.iter().find_map(|e| e.git_branch.as_ref());
    if let Some(branch) = git_branch {
        tx.execute(
            "UPDATE sessions SET git_branch = ?1 WHERE id = ?2 AND git_branch IS NULL",
            rusqlite::params![branch, session_id],
        )?;
    }

    let mut msg_count = 0;

    for entry in &entries {
        let uuid = match entry.uuid.as_deref() {
            Some(id) => id,
            None => continue,
        };

        let msg = match entry.message.as_ref() {
            Some(m) => m,
            None => continue,
        };

        let timestamp_str = match entry.timestamp.as_deref() {
            Some(ts) => ts,
            None => continue,
        };

        let created_at = parse_timestamp(timestamp_str);
        let role = msg.role.as_deref().unwrap_or("unknown");

        let content_str = msg.content.as_ref().map(|c| c.as_string());
        let model = msg.model.as_deref();
        let input_tokens = msg.usage.as_ref().and_then(|u| u.input_tokens);
        let output_tokens = msg.usage.as_ref().and_then(|u| u.output_tokens);
        let cost_usd = entry.cost_usd;
        let duration_ms = entry.duration_ms;
        let parent_id = entry.parent_uuid.as_deref();

        let changed = tx.execute(
            "INSERT OR IGNORE INTO messages (id, session_id, parent_id, role, content, model, input_tokens, output_tokens, cost_usd, duration_ms, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![uuid, session_id, parent_id, role, content_str, model, input_tokens, output_tokens, cost_usd, duration_ms, created_at],
        )?;
        if changed > 0 {
            msg_count += 1;
        }

        // Extract tool usage from assistant messages
        if role == "assistant" {
            if let Some(content) = msg.content.as_ref() {
                for block in content.tool_use_blocks() {
                    let tool_name = match block.name.as_deref() {
                        Some(n) => n,
                        None => continue,
                    };
                    let input_json = block.input.as_ref().map(|v| v.to_string());
                    tx.execute(
                        "INSERT OR IGNORE INTO tool_usage (session_id, message_id, tool_name, input_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                        rusqlite::params![session_id, uuid, tool_name, input_json, created_at],
                    )?;
                }
            }
        }
    }

    // Update session stats
    tx.execute(
        "UPDATE sessions SET message_count = (SELECT COUNT(*) FROM messages WHERE session_id = ?1), total_cost_usd = (SELECT COALESCE(SUM(cost_usd), 0) FROM messages WHERE session_id = ?1) WHERE id = ?1",
        rusqlite::params![session_id],
    )?;

    tx.commit()?;
    Ok(msg_count)
}

fn parse_timestamp(ts: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0)
}

pub fn sync_todos(conn: &Connection, claude_dir: &Path) -> Result<usize> {
    let dir = paths::todos_dir(claude_dir);
    if !dir.exists() {
        update_sync_state(conn, "todos")?;
        return Ok(0);
    }

    let mut count = 0;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".json") {
            continue;
        }

        let session_id = name.trim_end_matches(".json");
        let content = std::fs::read_to_string(entry.path())?;
        let todos: Vec<TodoEntry> = match serde_json::from_str(&content) {
            Ok(t) => t,
            Err(_) => continue,
        };

        // Delete existing todos for this session, then re-insert
        conn.execute(
            "DELETE FROM todos WHERE session_id = ?1",
            rusqlite::params![session_id],
        )?;

        for todo in &todos {
            let todo_content = match todo.content.as_deref() {
                Some(c) => c,
                None => continue,
            };
            let status = todo.status.as_deref().unwrap_or("pending");
            let active_form = todo.active_form.as_deref();

            conn.execute(
                "INSERT INTO todos (session_id, content, status, active_form) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![session_id, todo_content, status, active_form],
            )?;
            count += 1;
        }
    }

    update_sync_state(conn, "todos")?;
    Ok(count)
}

pub fn sync_all(conn: &Connection, claude_dir: &Path) -> Result<SyncResult> {
    let history = sync_history(conn, claude_dir)?;
    let messages = sync_sessions(conn, claude_dir)?;
    let todos = sync_todos(conn, claude_dir)?;
    Ok(SyncResult {
        history,
        messages,
        todos,
    })
}

pub fn sync_if_needed(conn: &Connection, claude_dir: &Path, interval_secs: u64) -> Result<bool> {
    let any_needed = needs_sync(conn, "history", interval_secs)?
        || needs_sync(conn, "sessions", interval_secs)?
        || needs_sync(conn, "todos", interval_secs)?;

    if any_needed {
        sync_all(conn, claude_dir)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::db::open_test_db;

    #[test]
    fn get_last_sync_no_record() {
        let store = open_test_db();
        assert_eq!(get_last_sync_time(&store.conn, "history").unwrap(), 0);
    }

    #[test]
    fn update_and_get_sync_time() {
        let store = open_test_db();
        update_sync_state(&store.conn, "history").unwrap();
        let time = get_last_sync_time(&store.conn, "history").unwrap();
        assert!(time > 0);
    }

    #[test]
    fn needs_sync_zero_interval() {
        let store = open_test_db();
        assert!(!needs_sync(&store.conn, "test", 0).unwrap());
    }

    #[test]
    fn needs_sync_no_prior() {
        let store = open_test_db();
        assert!(needs_sync(&store.conn, "test", 300).unwrap());
    }

    #[test]
    fn needs_sync_recent() {
        let store = open_test_db();
        update_sync_state(&store.conn, "test").unwrap();
        assert!(!needs_sync(&store.conn, "test", 300).unwrap());
    }

    #[test]
    fn sync_history_missing_file() {
        let store = open_test_db();
        let result = sync_history(&store.conn, Path::new("/nonexistent")).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn sync_history_from_fixture() {
        let store = open_test_db();
        let tmp = std::env::temp_dir().join("hu-test-sync-history");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let jsonl = r#"{"display":"test session","timestamp":1700000000000,"project":"/home/user/proj","sessionId":"sess-001"}
{"display":"another","timestamp":1700001000000,"project":"/home/user/proj2","sessionId":"sess-002"}
"#;
        std::fs::write(tmp.join("history.jsonl"), jsonl).unwrap();

        let count = sync_history(&store.conn, &tmp).unwrap();
        assert_eq!(count, 2);

        // Verify data
        let session_count: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(session_count, 2);

        // Idempotent
        let count2 = sync_history(&store.conn, &tmp).unwrap();
        assert_eq!(count2, 0);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sync_history_skips_incomplete() {
        let store = open_test_db();
        let tmp = std::env::temp_dir().join("hu-test-sync-hist-skip");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let jsonl = r#"{"display":"no session id","timestamp":1700000000000,"project":"/proj"}
{"display":"no timestamp","project":"/proj","sessionId":"s1"}
{"display":"ok","timestamp":1700000000000,"project":"/proj","sessionId":"s2"}
"#;
        std::fs::write(tmp.join("history.jsonl"), jsonl).unwrap();

        let count = sync_history(&store.conn, &tmp).unwrap();
        assert_eq!(count, 1);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sync_sessions_from_fixture() {
        let store = open_test_db();
        let tmp = std::env::temp_dir().join("hu-test-sync-sessions");
        let _ = std::fs::remove_dir_all(&tmp);

        let proj_dir = tmp.join("projects").join("-home-user-proj");
        std::fs::create_dir_all(&proj_dir).unwrap();

        let jsonl = r#"{"uuid":"m1","type":"user","timestamp":"2024-01-01T00:00:00Z","gitBranch":"main","message":{"role":"user","content":"hello","usage":{"input_tokens":10,"output_tokens":0}}}
{"uuid":"m2","parentUuid":"m1","type":"assistant","timestamp":"2024-01-01T00:00:01Z","message":{"role":"assistant","content":[{"type":"text","text":"hi"},{"type":"tool_use","name":"Read","input":{"path":"/tmp"}}],"model":"claude-sonnet-4-5-20251101","usage":{"input_tokens":10,"output_tokens":50}},"costUSD":0.001,"durationMs":500}
"#;
        std::fs::write(proj_dir.join("sess-001.jsonl"), jsonl).unwrap();

        let count = sync_sessions(&store.conn, &tmp).unwrap();
        assert_eq!(count, 2);

        // Check session was created with git_branch
        let branch: Option<String> = store
            .conn
            .query_row(
                "SELECT git_branch FROM sessions WHERE id = 'sess-001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(branch, Some("main".to_string()));

        // Check tool usage
        let tool_count: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM tool_usage", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tool_count, 1);

        // Check message stats updated
        let msg_count: i64 = store
            .conn
            .query_row(
                "SELECT message_count FROM sessions WHERE id = 'sess-001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(msg_count, 2);

        // Idempotent
        let count2 = sync_sessions(&store.conn, &tmp).unwrap();
        assert_eq!(count2, 0);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sync_sessions_skips_non_message_entries() {
        let store = open_test_db();
        let tmp = std::env::temp_dir().join("hu-test-sync-skip-types");
        let _ = std::fs::remove_dir_all(&tmp);

        let proj_dir = tmp.join("projects").join("-home-user-proj");
        std::fs::create_dir_all(&proj_dir).unwrap();

        let jsonl = r#"{"uuid":"m1","type":"summary","timestamp":"2024-01-01T00:00:00Z"}
{"uuid":"m2","type":"user","timestamp":"2024-01-01T00:00:01Z","message":{"role":"user","content":"hello"}}
"#;
        std::fs::write(proj_dir.join("sess-002.jsonl"), jsonl).unwrap();

        let count = sync_sessions(&store.conn, &tmp).unwrap();
        assert_eq!(count, 1); // Only the message with message body

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sync_todos_missing_dir() {
        let store = open_test_db();
        let result = sync_todos(&store.conn, Path::new("/nonexistent")).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn sync_todos_from_fixture() {
        let store = open_test_db();
        let tmp = std::env::temp_dir().join("hu-test-sync-todos");
        let _ = std::fs::remove_dir_all(&tmp);

        // Need a session first
        store
            .conn
            .execute(
                "INSERT INTO sessions (id, project, started_at) VALUES ('sess-t1', '/proj', 100)",
                [],
            )
            .unwrap();

        let todos_dir = tmp.join("todos");
        std::fs::create_dir_all(&todos_dir).unwrap();

        let json = r#"[
            {"content":"Fix bug","status":"pending","activeForm":"Fixing bug"},
            {"content":"Add tests","status":"completed"}
        ]"#;
        std::fs::write(todos_dir.join("sess-t1.json"), json).unwrap();

        let count = sync_todos(&store.conn, &tmp).unwrap();
        assert_eq!(count, 2);

        let todo_count: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM todos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(todo_count, 2);

        // Re-sync replaces (delete + insert)
        let count2 = sync_todos(&store.conn, &tmp).unwrap();
        assert_eq!(count2, 2);
        let todo_count2: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM todos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(todo_count2, 2);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sync_todos_skips_invalid_json() {
        let store = open_test_db();
        let tmp = std::env::temp_dir().join("hu-test-sync-todos-bad");
        let _ = std::fs::remove_dir_all(&tmp);

        let todos_dir = tmp.join("todos");
        std::fs::create_dir_all(&todos_dir).unwrap();
        std::fs::write(todos_dir.join("bad-session.json"), "not json").unwrap();

        let count = sync_todos(&store.conn, &tmp).unwrap();
        assert_eq!(count, 0);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sync_all_from_fixture() {
        let store = open_test_db();
        let tmp = std::env::temp_dir().join("hu-test-sync-all");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        // Empty claude dir
        let result = sync_all(&store.conn, &tmp).unwrap();
        assert_eq!(result.history, 0);
        assert_eq!(result.messages, 0);
        assert_eq!(result.todos, 0);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sync_if_needed_fresh_db() {
        let store = open_test_db();
        let tmp = std::env::temp_dir().join("hu-test-sync-needed");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let synced = sync_if_needed(&store.conn, &tmp, 300).unwrap();
        assert!(synced);

        // After sync, should not need again
        let synced2 = sync_if_needed(&store.conn, &tmp, 300).unwrap();
        assert!(!synced2);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn parse_timestamp_valid() {
        let ts = parse_timestamp("2024-01-01T00:00:00Z");
        assert!(ts > 0);
    }

    #[test]
    fn parse_timestamp_invalid() {
        let ts = parse_timestamp("not a date");
        assert_eq!(ts, 0);
    }

    #[test]
    fn sync_todos_skips_non_json() {
        let store = open_test_db();
        let tmp = std::env::temp_dir().join("hu-test-sync-todos-nonjson");
        let _ = std::fs::remove_dir_all(&tmp);

        let todos_dir = tmp.join("todos");
        std::fs::create_dir_all(&todos_dir).unwrap();
        std::fs::write(todos_dir.join("notes.txt"), "not a todo file").unwrap();

        let count = sync_todos(&store.conn, &tmp).unwrap();
        assert_eq!(count, 0);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sync_sessions_empty_projects() {
        let store = open_test_db();
        let tmp = std::env::temp_dir().join("hu-test-sync-empty-proj");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("projects")).unwrap();

        let count = sync_sessions(&store.conn, &tmp).unwrap();
        assert_eq!(count, 0);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
