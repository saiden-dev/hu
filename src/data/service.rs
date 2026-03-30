use std::path::Path;

use anyhow::{bail, Result};

use super::config::{self, DataConfig};
use super::db::SqliteStore;
use super::paths;
use super::queries;
use super::sync;
use super::types::{
    start_of_today_ms, BranchStats, DebugError, Message, SearchResult, Session, SyncResult, Todo,
    TodoWithProject, ToolUsageDetail, ToolUsageStats, UsageStats,
};

// --- DB lifecycle ---

#[cfg(not(tarpaulin_include))]
pub fn get_config() -> Result<DataConfig> {
    config::load_data_config()
}

#[cfg(not(tarpaulin_include))]
pub fn open_db() -> Result<SqliteStore> {
    let cfg = get_config()?;
    SqliteStore::open_initialized(&cfg.database)
}

#[cfg(not(tarpaulin_include))]
pub fn ensure_synced(store: &SqliteStore) -> Result<()> {
    let cfg = get_config()?;
    sync::sync_if_needed(&store.conn, &cfg.claude_dir, cfg.auto_sync_interval)?;
    Ok(())
}

// --- Sync ---

#[cfg(not(tarpaulin_include))]
pub fn sync_data(store: &SqliteStore, force: bool) -> Result<Option<SyncResult>> {
    let cfg = get_config()?;

    if force {
        let result = sync::sync_all(&store.conn, &cfg.claude_dir)?;
        return Ok(Some(result));
    }

    let synced = sync::sync_if_needed(&store.conn, &cfg.claude_dir, cfg.auto_sync_interval)?;
    if !synced {
        return Ok(None);
    }

    let result = sync::sync_all(&store.conn, &cfg.claude_dir)?;
    Ok(Some(result))
}

// --- Sessions ---

pub fn get_sessions(
    store: &SqliteStore,
    project: Option<&str>,
    limit: i64,
) -> Result<Vec<Session>> {
    queries::get_sessions(&store.conn, project, limit)
}

pub fn get_session_messages(store: &SqliteStore, id: &str) -> Result<(Session, Vec<Message>)> {
    let session = queries::get_session_by_prefix(&store.conn, id)?
        .or_else(|| queries::get_session_by_id(&store.conn, id).ok().flatten());

    match session {
        Some(s) => {
            let messages = queries::get_messages_by_session(&store.conn, &s.id)?;
            Ok((s, messages))
        }
        None => bail!("Session not found: {id}"),
    }
}

pub fn get_current_session_messages(store: &SqliteStore) -> Result<(Session, Vec<Message>)> {
    let session_id = std::env::var("SESSION_ID").unwrap_or_default();
    if session_id.is_empty() {
        bail!("SESSION_ID environment variable not set");
    }
    let session = queries::get_session_by_id(&store.conn, &session_id)?;
    match session {
        Some(s) => {
            let messages = queries::get_messages_by_session(&store.conn, &s.id)?;
            Ok((s, messages))
        }
        None => bail!("Current session not found: {session_id}"),
    }
}

// --- Stats ---

pub fn get_stats(
    store: &SqliteStore,
    today: bool,
) -> Result<(UsageStats, Vec<super::types::ModelUsage>)> {
    let since = if today {
        Some(start_of_today_ms())
    } else {
        None
    };

    let stats = queries::get_usage_stats(&store.conn, since)?;
    let model_usage = queries::get_model_usage(&store.conn, since)?;
    Ok((stats, model_usage))
}

// --- Todos ---

pub fn get_todos(store: &SqliteStore, status: Option<&str>) -> Result<Vec<Todo>> {
    queries::get_todos(&store.conn, status)
}

pub fn get_pending_todos(
    store: &SqliteStore,
    project: Option<&str>,
) -> Result<Vec<TodoWithProject>> {
    queries::get_pending_todos(&store.conn, project)
}

// --- Search ---

pub fn search_messages(store: &SqliteStore, query: &str, limit: i64) -> Result<Vec<SearchResult>> {
    queries::search_messages(&store.conn, query, limit)
}

// --- Tools ---

pub fn get_tool_stats(store: &SqliteStore) -> Result<Vec<ToolUsageStats>> {
    queries::get_tool_stats(&store.conn)
}

pub fn get_tool_detail(store: &SqliteStore, name: &str) -> Result<Vec<ToolUsageDetail>> {
    queries::get_tool_detail(&store.conn, name)
}

// --- Errors ---

pub fn scan_debug_errors(claude_dir: &Path, recent_days: u32) -> Result<Vec<DebugError>> {
    let dir = paths::debug_dir(claude_dir);
    if !dir.exists() {
        return Ok(vec![]);
    }

    let cutoff = chrono::Utc::now().timestamp() - (recent_days as i64 * 86400);
    let error_patterns =
        regex::Regex::new(r"(?i)(error|failed|exception|warning|ENOENT|EACCES|EPERM)")?;

    let mut errors = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }

        let metadata = entry.metadata()?;
        let modified = metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        if modified < cutoff {
            continue;
        }

        let content = std::fs::read_to_string(&path)?;
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        for (i, line) in content.lines().enumerate() {
            if error_patterns.is_match(line) && seen.insert(line.to_string()) {
                errors.push(DebugError {
                    file: filename.clone(),
                    line: i + 1,
                    content: line.to_string(),
                    timestamp: modified,
                });
            }
        }
    }

    errors.truncate(50);
    Ok(errors)
}

// --- Branches ---

pub fn get_branch_stats(
    store: &SqliteStore,
    branch: Option<&str>,
    limit: i64,
) -> Result<Vec<BranchStats>> {
    queries::get_branch_stats(&store.conn, branch, limit)
}

#[cfg(not(tarpaulin_include))]
pub async fn fetch_pr_info(branch: &str) -> Option<super::types::PrInfo> {
    let output: std::process::Output = tokio::process::Command::new("gh")
        .args([
            "pr",
            "list",
            "--head",
            branch,
            "--json",
            "number,title,state,url",
            "--limit",
            "1",
        ])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let prs: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).ok()?;
    let pr = prs.first()?;

    Some(super::types::PrInfo {
        number: pr.get("number")?.as_i64()?,
        title: pr.get("title")?.as_str()?.to_string(),
        state: pr.get("state")?.as_str()?.to_string(),
        url: pr.get("url")?.as_str()?.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::db::open_test_db;

    fn seed_data(store: &SqliteStore) {
        store
            .conn
            .execute_batch(
                "
            INSERT INTO sessions (id, project, display, started_at, message_count, total_cost_usd, git_branch) VALUES
                ('s1', '/home/user/proj', 'First session', 1700000000000, 3, 0.05, 'main'),
                ('s2', '/home/user/proj2', 'Second session', 1700001000000, 1, 0.01, 'feature/x');

            INSERT INTO messages (id, session_id, role, content, model, input_tokens, output_tokens, cost_usd, created_at) VALUES
                ('m1', 's1', 'user', 'hello world', NULL, 10, 0, NULL, 1700000000000),
                ('m2', 's1', 'assistant', 'hi there', 'claude-sonnet-4-5-20251101', 10, 50, 0.003, 1700000001000),
                ('m3', 's1', 'user', 'search test query', NULL, 15, 0, NULL, 1700000002000),
                ('m4', 's2', 'user', 'other message', NULL, 5, 0, NULL, 1700001000000);

            INSERT INTO todos (session_id, content, status, active_form) VALUES
                ('s1', 'Fix bug', 'pending', 'Fixing bug'),
                ('s1', 'Add tests', 'completed', NULL),
                ('s2', 'Review PR', 'in_progress', 'Reviewing PR');

            INSERT INTO tool_usage (session_id, message_id, tool_name, created_at) VALUES
                ('s1', 'm2', 'Read', 1700000001000),
                ('s1', 'm2', 'Read', 1700000001500),
                ('s1', 'm2', 'Edit', 1700000002000);
            ",
            )
            .expect("seed data should insert");
    }

    // --- Sessions ---

    #[test]
    fn get_sessions_returns_all() {
        let store = open_test_db();
        seed_data(&store);
        let sessions = get_sessions(&store, None, 20).unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn get_sessions_filters_by_project() {
        let store = open_test_db();
        seed_data(&store);
        let sessions = get_sessions(&store, Some("proj2"), 20).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "s2");
    }

    #[test]
    fn get_session_messages_by_prefix() {
        let store = open_test_db();
        seed_data(&store);
        let (session, messages) = get_session_messages(&store, "s1").unwrap();
        assert_eq!(session.id, "s1");
        assert_eq!(messages.len(), 3);
    }

    #[test]
    fn get_session_messages_not_found() {
        let store = open_test_db();
        seed_data(&store);
        let result = get_session_messages(&store, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn get_current_session_no_env() {
        let store = open_test_db();
        // SESSION_ID is not set (or empty), should fail
        std::env::remove_var("SESSION_ID");
        let result = get_current_session_messages(&store);
        assert!(result.is_err());
    }

    #[test]
    fn get_current_session_found() {
        let store = open_test_db();
        seed_data(&store);
        std::env::set_var("SESSION_ID", "s1");
        let (session, messages) = get_current_session_messages(&store).unwrap();
        assert_eq!(session.id, "s1");
        assert_eq!(messages.len(), 3);
        // Clean up env var
        std::env::remove_var("SESSION_ID");
    }

    #[test]
    fn get_current_session_not_found() {
        let store = open_test_db();
        seed_data(&store);
        std::env::set_var("SESSION_ID", "nonexistent");
        let result = get_current_session_messages(&store);
        assert!(result.is_err());
        std::env::remove_var("SESSION_ID");
    }

    // --- Stats ---

    #[test]
    fn get_stats_all_time() {
        let store = open_test_db();
        seed_data(&store);
        let (stats, model_usage) = get_stats(&store, false).unwrap();
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.total_messages, 4);
        assert!(!model_usage.is_empty());
    }

    #[test]
    fn get_stats_today_returns_data() {
        let store = open_test_db();
        seed_data(&store);
        // Data is old (timestamp 1700000000000), "today" will return empty stats
        let (stats, _) = get_stats(&store, true).unwrap();
        assert_eq!(stats.total_messages, 0);
    }

    // --- Todos ---

    #[test]
    fn get_todos_all() {
        let store = open_test_db();
        seed_data(&store);
        let todos = get_todos(&store, None).unwrap();
        assert_eq!(todos.len(), 3);
    }

    #[test]
    fn get_todos_filtered() {
        let store = open_test_db();
        seed_data(&store);
        let todos = get_todos(&store, Some("pending")).unwrap();
        assert_eq!(todos.len(), 1);
    }

    #[test]
    fn get_pending_todos_all() {
        let store = open_test_db();
        seed_data(&store);
        let todos = get_pending_todos(&store, None).unwrap();
        assert_eq!(todos.len(), 2);
    }

    #[test]
    fn get_pending_todos_filtered() {
        let store = open_test_db();
        seed_data(&store);
        let todos = get_pending_todos(&store, Some("proj2")).unwrap();
        assert_eq!(todos.len(), 1);
    }

    // --- Search ---

    #[test]
    fn search_messages_found() {
        let store = open_test_db();
        seed_data(&store);
        let results = search_messages(&store, "search test", 50).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_messages_empty() {
        let store = open_test_db();
        seed_data(&store);
        let results = search_messages(&store, "zzzzz_no_match", 50).unwrap();
        assert!(results.is_empty());
    }

    // --- Tools ---

    #[test]
    fn get_tool_stats_returns_data() {
        let store = open_test_db();
        seed_data(&store);
        let stats = get_tool_stats(&store).unwrap();
        assert_eq!(stats.len(), 2);
    }

    #[test]
    fn get_tool_detail_returns_data() {
        let store = open_test_db();
        seed_data(&store);
        let detail = get_tool_detail(&store, "Read").unwrap();
        assert_eq!(detail.len(), 2);
    }

    #[test]
    fn get_tool_detail_empty() {
        let store = open_test_db();
        seed_data(&store);
        let detail = get_tool_detail(&store, "Nonexistent").unwrap();
        assert!(detail.is_empty());
    }

    // --- Errors ---

    #[test]
    fn scan_debug_errors_missing_dir() {
        let errors = scan_debug_errors(Path::new("/nonexistent"), 7).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn scan_debug_errors_with_fixture() {
        let tmp = std::env::temp_dir().join("hu-svc-test-debug-errors");
        let _ = std::fs::remove_dir_all(&tmp);
        let debug = tmp.join("debug");
        std::fs::create_dir_all(&debug).unwrap();

        std::fs::write(
            debug.join("test.txt"),
            "normal line\nError: something broke\nFailed to connect\nanother normal line\n",
        )
        .unwrap();

        let errors = scan_debug_errors(&tmp, 7).unwrap();
        assert_eq!(errors.len(), 2);
        assert!(errors[0].content.contains("Error"));
        assert!(errors[1].content.contains("Failed"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_debug_errors_deduplication() {
        let tmp = std::env::temp_dir().join("hu-svc-test-debug-dedup");
        let _ = std::fs::remove_dir_all(&tmp);
        let debug = tmp.join("debug");
        std::fs::create_dir_all(&debug).unwrap();

        std::fs::write(debug.join("a.txt"), "Error: same message\n").unwrap();
        std::fs::write(debug.join("b.txt"), "Error: same message\n").unwrap();

        let errors = scan_debug_errors(&tmp, 7).unwrap();
        assert_eq!(errors.len(), 1);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_debug_errors_skips_non_txt() {
        let tmp = std::env::temp_dir().join("hu-svc-test-debug-skip");
        let _ = std::fs::remove_dir_all(&tmp);
        let debug = tmp.join("debug");
        std::fs::create_dir_all(&debug).unwrap();

        std::fs::write(debug.join("test.log"), "Error: in log file\n").unwrap();

        let errors = scan_debug_errors(&tmp, 7).unwrap();
        assert!(errors.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_debug_errors_skips_old_files() {
        let tmp = std::env::temp_dir().join("hu-svc-test-debug-old");
        let _ = std::fs::remove_dir_all(&tmp);
        let debug = tmp.join("debug");
        std::fs::create_dir_all(&debug).unwrap();

        let file_path = debug.join("old.txt");
        std::fs::write(&file_path, "Error: old error\n").unwrap();

        let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 86400);
        let file = std::fs::File::options()
            .write(true)
            .open(&file_path)
            .unwrap();
        file.set_times(std::fs::FileTimes::new().set_modified(old_time))
            .unwrap();

        let errors = scan_debug_errors(&tmp, 7).unwrap();
        assert!(errors.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_debug_errors_max_50() {
        let tmp = std::env::temp_dir().join("hu-svc-test-debug-max");
        let _ = std::fs::remove_dir_all(&tmp);
        let debug = tmp.join("debug");
        std::fs::create_dir_all(&debug).unwrap();

        let mut content = String::new();
        for i in 0..60 {
            content.push_str(&format!("Error: unique error {i}\n"));
        }
        std::fs::write(debug.join("many.txt"), &content).unwrap();

        let errors = scan_debug_errors(&tmp, 7).unwrap();
        assert_eq!(errors.len(), 50);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // --- Branches ---

    #[test]
    fn get_branch_stats_all() {
        let store = open_test_db();
        seed_data(&store);
        let stats = get_branch_stats(&store, None, 20).unwrap();
        assert_eq!(stats.len(), 2);
    }

    #[test]
    fn get_branch_stats_filtered() {
        let store = open_test_db();
        seed_data(&store);
        let stats = get_branch_stats(&store, Some("feature"), 20).unwrap();
        assert_eq!(stats.len(), 1);
    }
}
