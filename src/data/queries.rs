use anyhow::Result;
use rusqlite::Connection;
use rusqlite::OptionalExtension;

use super::types::*;

pub fn get_sessions(conn: &Connection, project: Option<&str>, limit: i64) -> Result<Vec<Session>> {
    let (sql, params) = match project {
        Some(p) => {
            let pattern = format!("%{p}%");
            (
                "SELECT id, project, display, started_at, message_count, total_cost_usd, git_branch FROM sessions WHERE project LIKE ?1 ORDER BY started_at DESC LIMIT ?2".to_string(),
                vec![rusqlite::types::Value::Text(pattern), rusqlite::types::Value::Integer(limit)],
            )
        }
        None => (
            "SELECT id, project, display, started_at, message_count, total_cost_usd, git_branch FROM sessions ORDER BY started_at DESC LIMIT ?1".to_string(),
            vec![rusqlite::types::Value::Integer(limit)],
        ),
    };

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        Ok(Session {
            id: row.get(0)?,
            project: row.get(1)?,
            display: row.get(2)?,
            started_at: row.get(3)?,
            message_count: row.get(4)?,
            total_cost_usd: row.get(5)?,
            git_branch: row.get(6)?,
        })
    })?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_session_by_prefix(conn: &Connection, prefix: &str) -> Result<Option<Session>> {
    let pattern = format!("{prefix}%");
    Ok(conn.query_row(
        "SELECT id, project, display, started_at, message_count, total_cost_usd, git_branch FROM sessions WHERE id LIKE ?1 ORDER BY started_at DESC LIMIT 1",
        rusqlite::params![pattern],
        |row| {
            Ok(Session {
                id: row.get(0)?,
                project: row.get(1)?,
                display: row.get(2)?,
                started_at: row.get(3)?,
                message_count: row.get(4)?,
                total_cost_usd: row.get(5)?,
                git_branch: row.get(6)?,
            })
        },
    ).optional()?)
}

pub fn get_session_by_id(conn: &Connection, id: &str) -> Result<Option<Session>> {
    Ok(conn.query_row(
        "SELECT id, project, display, started_at, message_count, total_cost_usd, git_branch FROM sessions WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(Session {
                id: row.get(0)?,
                project: row.get(1)?,
                display: row.get(2)?,
                started_at: row.get(3)?,
                message_count: row.get(4)?,
                total_cost_usd: row.get(5)?,
                git_branch: row.get(6)?,
            })
        },
    ).optional()?)
}

pub fn get_messages_by_session(conn: &Connection, session_id: &str) -> Result<Vec<Message>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, parent_id, role, content, model, input_tokens, output_tokens, cost_usd, duration_ms, created_at FROM messages WHERE session_id = ?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(rusqlite::params![session_id], |row| {
        Ok(Message {
            id: row.get(0)?,
            session_id: row.get(1)?,
            parent_id: row.get(2)?,
            role: row.get(3)?,
            content: row.get(4)?,
            model: row.get(5)?,
            input_tokens: row.get(6)?,
            output_tokens: row.get(7)?,
            cost_usd: row.get(8)?,
            duration_ms: row.get(9)?,
            created_at: row.get(10)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn search_messages(conn: &Connection, query: &str, limit: i64) -> Result<Vec<SearchResult>> {
    let pattern = format!("%{query}%");
    let mut stmt = conn.prepare(
        "SELECT m.id, m.session_id, m.role, m.content, m.model, m.created_at, s.project FROM messages m JOIN sessions s ON m.session_id = s.id WHERE m.content LIKE ?1 ORDER BY m.created_at DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(rusqlite::params![pattern, limit], |row| {
        Ok(SearchResult {
            id: row.get(0)?,
            session_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            model: row.get(4)?,
            created_at: row.get(5)?,
            project: row.get(6)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_todos(conn: &Connection, status: Option<&str>) -> Result<Vec<Todo>> {
    let (sql, params) = match status {
        Some(s) => (
            "SELECT id, session_id, content, status, active_form FROM todos WHERE status = ?1 ORDER BY id DESC".to_string(),
            vec![rusqlite::types::Value::Text(s.to_string())],
        ),
        None => (
            "SELECT id, session_id, content, status, active_form FROM todos ORDER BY id DESC"
                .to_string(),
            vec![],
        ),
    };

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        Ok(Todo {
            id: row.get(0)?,
            session_id: row.get(1)?,
            content: row.get(2)?,
            status: row.get(3)?,
            active_form: row.get(4)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_pending_todos(conn: &Connection, project: Option<&str>) -> Result<Vec<TodoWithProject>> {
    let (sql, params) = match project {
        Some(p) => {
            let pattern = format!("%{p}%");
            (
                "SELECT t.id, t.session_id, t.content, t.status, t.active_form, s.project FROM todos t JOIN sessions s ON t.session_id = s.id WHERE t.status != 'completed' AND s.project LIKE ?1 ORDER BY t.id DESC".to_string(),
                vec![rusqlite::types::Value::Text(pattern)],
            )
        }
        None => (
            "SELECT t.id, t.session_id, t.content, t.status, t.active_form, s.project FROM todos t JOIN sessions s ON t.session_id = s.id WHERE t.status != 'completed' ORDER BY t.id DESC".to_string(),
            vec![],
        ),
    };

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        Ok(TodoWithProject {
            id: row.get(0)?,
            session_id: row.get(1)?,
            content: row.get(2)?,
            status: row.get(3)?,
            active_form: row.get(4)?,
            project: row.get(5)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_usage_stats(conn: &Connection, since: Option<i64>) -> Result<UsageStats> {
    let total_sessions: i64 = conn.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?;

    let (total_messages, total_cost, total_input_tokens, total_output_tokens) = match since {
        Some(ts) => {
            let msgs: i64 = conn.query_row(
                "SELECT COUNT(*) FROM messages WHERE created_at >= ?1",
                rusqlite::params![ts],
                |r| r.get(0),
            )?;
            let cost: f64 = conn.query_row(
                "SELECT COALESCE(SUM(cost_usd), 0) FROM messages WHERE created_at >= ?1",
                rusqlite::params![ts],
                |r| r.get(0),
            )?;
            let input: i64 = conn.query_row(
                "SELECT COALESCE(SUM(input_tokens), 0) FROM messages WHERE created_at >= ?1",
                rusqlite::params![ts],
                |r| r.get(0),
            )?;
            let output: i64 = conn.query_row(
                "SELECT COALESCE(SUM(output_tokens), 0) FROM messages WHERE created_at >= ?1",
                rusqlite::params![ts],
                |r| r.get(0),
            )?;
            (msgs, cost, input, output)
        }
        None => {
            let msgs: i64 = conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?;
            let cost: f64 =
                conn.query_row("SELECT COALESCE(SUM(cost_usd), 0) FROM messages", [], |r| {
                    r.get(0)
                })?;
            let input: i64 = conn.query_row(
                "SELECT COALESCE(SUM(input_tokens), 0) FROM messages",
                [],
                |r| r.get(0),
            )?;
            let output: i64 = conn.query_row(
                "SELECT COALESCE(SUM(output_tokens), 0) FROM messages",
                [],
                |r| r.get(0),
            )?;
            (msgs, cost, input, output)
        }
    };

    Ok(UsageStats {
        total_sessions,
        total_messages,
        total_cost,
        total_input_tokens,
        total_output_tokens,
    })
}

pub fn get_model_usage(conn: &Connection, since: Option<i64>) -> Result<Vec<ModelUsage>> {
    let (sql, params): (String, Vec<rusqlite::types::Value>) = match since {
        Some(ts) => (
            "SELECT model, COUNT(*) as count, COALESCE(SUM(cost_usd), 0) as cost, COALESCE(SUM(input_tokens), 0) as input_tokens, COALESCE(SUM(output_tokens), 0) as output_tokens FROM messages WHERE model IS NOT NULL AND created_at >= ?1 GROUP BY model ORDER BY count DESC".to_string(),
            vec![rusqlite::types::Value::Integer(ts)],
        ),
        None => (
            "SELECT model, COUNT(*) as count, COALESCE(SUM(cost_usd), 0) as cost, COALESCE(SUM(input_tokens), 0) as input_tokens, COALESCE(SUM(output_tokens), 0) as output_tokens FROM messages WHERE model IS NOT NULL GROUP BY model ORDER BY count DESC".to_string(),
            vec![],
        ),
    };

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        Ok(ModelUsage {
            model: row.get(0)?,
            count: row.get(1)?,
            cost: row.get(2)?,
            input_tokens: row.get(3)?,
            output_tokens: row.get(4)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_tool_stats(conn: &Connection) -> Result<Vec<ToolUsageStats>> {
    let mut stmt = conn.prepare(
        "SELECT tool_name, COUNT(*) as count, MAX(created_at) as last_used FROM tool_usage GROUP BY tool_name ORDER BY count DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ToolUsageStats {
            tool_name: row.get(0)?,
            count: row.get(1)?,
            last_used: row.get(2)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_tool_detail(conn: &Connection, tool_name: &str) -> Result<Vec<ToolUsageDetail>> {
    let mut stmt = conn.prepare(
        "SELECT tu.tool_name, tu.session_id, s.project, tu.created_at FROM tool_usage tu JOIN sessions s ON tu.session_id = s.id WHERE tu.tool_name = ?1 ORDER BY tu.created_at DESC LIMIT 20",
    )?;
    let rows = stmt.query_map(rusqlite::params![tool_name], |row| {
        Ok(ToolUsageDetail {
            tool_name: row.get(0)?,
            session_id: row.get(1)?,
            project: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_branch_stats(
    conn: &Connection,
    branch_filter: Option<&str>,
    limit: i64,
) -> Result<Vec<BranchStats>> {
    let (sql, params): (String, Vec<rusqlite::types::Value>) = match branch_filter {
        Some(b) => {
            let pattern = format!("%{b}%");
            (
                "SELECT git_branch, COUNT(*) as session_count, GROUP_CONCAT(id) as session_ids, MAX(started_at) as last_activity, SUM(message_count) as total_messages, SUM(total_cost_usd) as total_cost, project FROM sessions WHERE git_branch IS NOT NULL AND git_branch LIKE ?1 GROUP BY git_branch, project ORDER BY last_activity DESC LIMIT ?2".to_string(),
                vec![rusqlite::types::Value::Text(pattern), rusqlite::types::Value::Integer(limit)],
            )
        }
        None => (
            "SELECT git_branch, COUNT(*) as session_count, GROUP_CONCAT(id) as session_ids, MAX(started_at) as last_activity, SUM(message_count) as total_messages, SUM(total_cost_usd) as total_cost, project FROM sessions WHERE git_branch IS NOT NULL GROUP BY git_branch, project ORDER BY last_activity DESC LIMIT ?1".to_string(),
            vec![rusqlite::types::Value::Integer(limit)],
        ),
    };

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        Ok(BranchStats {
            git_branch: row.get(0)?,
            session_count: row.get(1)?,
            session_ids: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            last_activity: row.get(3)?,
            total_messages: row.get(4)?,
            total_cost: row.get(5)?,
            project: row.get(6)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_period_usage(conn: &Connection, since: i64) -> Result<PeriodUsage> {
    let row = conn.query_row(
        "SELECT COUNT(*) as messages, COALESCE(SUM(input_tokens), 0) as input_tokens, COALESCE(SUM(output_tokens), 0) as output_tokens FROM messages WHERE created_at >= ?1",
        rusqlite::params![since],
        |row| {
            Ok(PeriodUsage {
                messages: row.get(0)?,
                input_tokens: row.get(1)?,
                output_tokens: row.get(2)?,
            })
        },
    )?;
    Ok(row)
}

pub fn get_period_model_usage(conn: &Connection, since: i64) -> Result<Vec<ModelTokenUsage>> {
    let mut stmt = conn.prepare(
        "SELECT model, COALESCE(SUM(input_tokens), 0) as input_tokens, COALESCE(SUM(output_tokens), 0) as output_tokens FROM messages WHERE model IS NOT NULL AND created_at >= ?1 GROUP BY model",
    )?;
    let rows = stmt.query_map(rusqlite::params![since], |row| {
        Ok(ModelTokenUsage {
            model: row.get(0)?,
            input_tokens: row.get(1)?,
            output_tokens: row.get(2)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// Extra types used only by queries

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PeriodUsage {
    pub messages: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ModelTokenUsage {
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::db::open_test_db;

    fn seed_data(conn: &Connection) {
        conn.execute_batch(
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
        .unwrap();
    }

    #[test]
    fn get_sessions_all() {
        let store = open_test_db();
        seed_data(&store.conn);
        let sessions = get_sessions(&store.conn, None, 20).unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, "s2"); // Most recent first
    }

    #[test]
    fn get_sessions_filtered() {
        let store = open_test_db();
        seed_data(&store.conn);
        let sessions = get_sessions(&store.conn, Some("proj2"), 20).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "s2");
    }

    #[test]
    fn get_sessions_limited() {
        let store = open_test_db();
        seed_data(&store.conn);
        let sessions = get_sessions(&store.conn, None, 1).unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn get_sessions_empty() {
        let store = open_test_db();
        let sessions = get_sessions(&store.conn, None, 20).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn get_session_by_prefix_found() {
        let store = open_test_db();
        seed_data(&store.conn);
        let session = get_session_by_prefix(&store.conn, "s1").unwrap();
        assert!(session.is_some());
        assert_eq!(session.unwrap().id, "s1");
    }

    #[test]
    fn get_session_by_prefix_not_found() {
        let store = open_test_db();
        seed_data(&store.conn);
        let session = get_session_by_prefix(&store.conn, "zzz").unwrap();
        assert!(session.is_none());
    }

    #[test]
    fn get_session_by_id_found() {
        let store = open_test_db();
        seed_data(&store.conn);
        let session = get_session_by_id(&store.conn, "s1").unwrap();
        assert!(session.is_some());
    }

    #[test]
    fn get_session_by_id_not_found() {
        let store = open_test_db();
        let session = get_session_by_id(&store.conn, "nonexistent").unwrap();
        assert!(session.is_none());
    }

    #[test]
    fn get_messages_by_session_found() {
        let store = open_test_db();
        seed_data(&store.conn);
        let msgs = get_messages_by_session(&store.conn, "s1").unwrap();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[1].role, "assistant");
    }

    #[test]
    fn get_messages_by_session_empty() {
        let store = open_test_db();
        let msgs = get_messages_by_session(&store.conn, "nonexistent").unwrap();
        assert!(msgs.is_empty());
    }

    #[test]
    fn search_messages_found() {
        let store = open_test_db();
        seed_data(&store.conn);
        let results = search_messages(&store.conn, "search test", 50).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "m3");
    }

    #[test]
    fn search_messages_not_found() {
        let store = open_test_db();
        seed_data(&store.conn);
        let results = search_messages(&store.conn, "nonexistent_xyz", 50).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_messages_empty_db() {
        let store = open_test_db();
        let results = search_messages(&store.conn, "test", 50).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn get_todos_all() {
        let store = open_test_db();
        seed_data(&store.conn);
        let todos = get_todos(&store.conn, None).unwrap();
        assert_eq!(todos.len(), 3);
    }

    #[test]
    fn get_todos_filtered() {
        let store = open_test_db();
        seed_data(&store.conn);
        let todos = get_todos(&store.conn, Some("pending")).unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].content, "Fix bug");
    }

    #[test]
    fn get_todos_empty() {
        let store = open_test_db();
        let todos = get_todos(&store.conn, None).unwrap();
        assert!(todos.is_empty());
    }

    #[test]
    fn get_pending_todos_all() {
        let store = open_test_db();
        seed_data(&store.conn);
        let todos = get_pending_todos(&store.conn, None).unwrap();
        assert_eq!(todos.len(), 2); // pending + in_progress
    }

    #[test]
    fn get_pending_todos_filtered() {
        let store = open_test_db();
        seed_data(&store.conn);
        let todos = get_pending_todos(&store.conn, Some("proj2")).unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].content, "Review PR");
    }

    #[test]
    fn get_pending_todos_empty() {
        let store = open_test_db();
        let todos = get_pending_todos(&store.conn, None).unwrap();
        assert!(todos.is_empty());
    }

    #[test]
    fn get_usage_stats_all() {
        let store = open_test_db();
        seed_data(&store.conn);
        let stats = get_usage_stats(&store.conn, None).unwrap();
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.total_messages, 4);
        assert!(stats.total_cost > 0.0);
        assert!(stats.total_input_tokens > 0);
    }

    #[test]
    fn get_usage_stats_since() {
        let store = open_test_db();
        seed_data(&store.conn);
        let stats = get_usage_stats(&store.conn, Some(1700000500000)).unwrap();
        assert_eq!(stats.total_sessions, 2); // sessions always counted fully
        assert_eq!(stats.total_messages, 1); // only m4
    }

    #[test]
    fn get_usage_stats_empty() {
        let store = open_test_db();
        let stats = get_usage_stats(&store.conn, None).unwrap();
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.total_cost, 0.0);
    }

    #[test]
    fn get_model_usage_all() {
        let store = open_test_db();
        seed_data(&store.conn);
        let usage = get_model_usage(&store.conn, None).unwrap();
        assert_eq!(usage.len(), 1); // Only assistant msgs have model
        assert_eq!(usage[0].model, "claude-sonnet-4-5-20251101");
    }

    #[test]
    fn get_model_usage_empty() {
        let store = open_test_db();
        let usage = get_model_usage(&store.conn, None).unwrap();
        assert!(usage.is_empty());
    }

    #[test]
    fn get_model_usage_since() {
        let store = open_test_db();
        seed_data(&store.conn);
        // After all messages
        let usage = get_model_usage(&store.conn, Some(9999999999999)).unwrap();
        assert!(usage.is_empty());
    }

    #[test]
    fn get_tool_stats_found() {
        let store = open_test_db();
        seed_data(&store.conn);
        let stats = get_tool_stats(&store.conn).unwrap();
        assert_eq!(stats.len(), 2); // Read and Edit
        assert_eq!(stats[0].tool_name, "Read"); // Most used
        assert_eq!(stats[0].count, 2);
    }

    #[test]
    fn get_tool_stats_empty() {
        let store = open_test_db();
        let stats = get_tool_stats(&store.conn).unwrap();
        assert!(stats.is_empty());
    }

    #[test]
    fn get_tool_detail_found() {
        let store = open_test_db();
        seed_data(&store.conn);
        let detail = get_tool_detail(&store.conn, "Read").unwrap();
        assert_eq!(detail.len(), 2);
        assert_eq!(detail[0].project, "/home/user/proj");
    }

    #[test]
    fn get_tool_detail_not_found() {
        let store = open_test_db();
        seed_data(&store.conn);
        let detail = get_tool_detail(&store.conn, "NonexistentTool").unwrap();
        assert!(detail.is_empty());
    }

    #[test]
    fn get_branch_stats_all() {
        let store = open_test_db();
        seed_data(&store.conn);
        let stats = get_branch_stats(&store.conn, None, 20).unwrap();
        assert_eq!(stats.len(), 2);
    }

    #[test]
    fn get_branch_stats_filtered() {
        let store = open_test_db();
        seed_data(&store.conn);
        let stats = get_branch_stats(&store.conn, Some("feature"), 20).unwrap();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].git_branch, "feature/x");
    }

    #[test]
    fn get_branch_stats_limited() {
        let store = open_test_db();
        seed_data(&store.conn);
        let stats = get_branch_stats(&store.conn, None, 1).unwrap();
        assert_eq!(stats.len(), 1);
    }

    #[test]
    fn get_branch_stats_empty() {
        let store = open_test_db();
        let stats = get_branch_stats(&store.conn, None, 20).unwrap();
        assert!(stats.is_empty());
    }

    #[test]
    fn get_period_usage_found() {
        let store = open_test_db();
        seed_data(&store.conn);
        let usage = get_period_usage(&store.conn, 0).unwrap();
        assert_eq!(usage.messages, 4);
        assert!(usage.input_tokens > 0);
    }

    #[test]
    fn get_period_usage_empty() {
        let store = open_test_db();
        let usage = get_period_usage(&store.conn, 0).unwrap();
        assert_eq!(usage.messages, 0);
    }

    #[test]
    fn get_period_model_usage_found() {
        let store = open_test_db();
        seed_data(&store.conn);
        let usage = get_period_model_usage(&store.conn, 0).unwrap();
        assert_eq!(usage.len(), 1);
    }

    #[test]
    fn get_period_model_usage_empty() {
        let store = open_test_db();
        let usage = get_period_model_usage(&store.conn, 0).unwrap();
        assert!(usage.is_empty());
    }

    #[test]
    fn period_usage_default() {
        let p = PeriodUsage::default();
        assert_eq!(p.messages, 0);
    }

    #[test]
    fn model_token_usage_default() {
        let m = ModelTokenUsage::default();
        assert_eq!(m.model, "");
    }
}
