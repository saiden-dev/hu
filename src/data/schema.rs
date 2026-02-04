use anyhow::Result;
use rusqlite::Connection;

const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    project TEXT NOT NULL,
    display TEXT,
    started_at INTEGER NOT NULL,
    message_count INTEGER DEFAULT 0,
    total_cost_usd REAL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project);
CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    parent_id TEXT,
    role TEXT NOT NULL,
    content TEXT,
    model TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cost_usd REAL,
    duration_ms INTEGER,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at);

CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content,
    content=messages,
    content_rowid=rowid
);

CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content)
        VALUES('delete', OLD.rowid, OLD.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content)
        VALUES('delete', OLD.rowid, OLD.content);
    INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;

CREATE TABLE IF NOT EXISTS todos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    content TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    active_form TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);
CREATE INDEX IF NOT EXISTS idx_todos_session ON todos(session_id);
CREATE INDEX IF NOT EXISTS idx_todos_status ON todos(status);

CREATE TABLE IF NOT EXISTS tool_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    message_id TEXT,
    tool_name TEXT NOT NULL,
    input_json TEXT,
    output_json TEXT,
    duration_ms INTEGER,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);
CREATE INDEX IF NOT EXISTS idx_tool_usage_session ON tool_usage(session_id);
CREATE INDEX IF NOT EXISTS idx_tool_usage_tool ON tool_usage(tool_name);

CREATE TABLE IF NOT EXISTS sync_state (
    source TEXT PRIMARY KEY,
    last_sync_at INTEGER NOT NULL,
    last_modified_at INTEGER,
    checksum TEXT
);
"#;

const MIGRATION_V2: &str = r#"
ALTER TABLE sessions ADD COLUMN git_branch TEXT;
CREATE INDEX IF NOT EXISTS idx_sessions_git_branch ON sessions(git_branch);
"#;

struct Migration {
    version: i64,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: MIGRATION_V1,
    },
    Migration {
        version: 2,
        sql: MIGRATION_V2,
    },
];

pub fn get_schema_version(conn: &Connection) -> Result<i64> {
    // Check if schema_version table exists
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_version'",
        [],
        |row| row.get(0),
    )?;

    if !exists {
        return Ok(0);
    }

    let version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;
    Ok(version)
}

pub fn run_migrations(conn: &Connection) -> Result<()> {
    let current = get_schema_version(conn)?;

    for migration in MIGRATIONS {
        if migration.version > current {
            conn.execute_batch(migration.sql)?;

            let now = chrono::Utc::now().timestamp_millis();
            conn.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![migration.version, now],
            )?;
        }
    }

    Ok(())
}

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    run_migrations(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_memory() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .unwrap();
        conn
    }

    #[test]
    fn schema_version_no_table() {
        let conn = open_memory();
        assert_eq!(get_schema_version(&conn).unwrap(), 0);
    }

    #[test]
    fn initialize_creates_tables() {
        let conn = open_memory();
        initialize_schema(&conn).unwrap();

        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 2);

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"sessions".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"todos".to_string()));
        assert!(tables.contains(&"tool_usage".to_string()));
        assert!(tables.contains(&"sync_state".to_string()));
        assert!(tables.contains(&"schema_version".to_string()));
    }

    #[test]
    fn initialize_is_idempotent() {
        let conn = open_memory();
        initialize_schema(&conn).unwrap();
        initialize_schema(&conn).unwrap();
        assert_eq!(get_schema_version(&conn).unwrap(), 2);
    }

    #[test]
    fn migration_v2_adds_git_branch() {
        let conn = open_memory();
        initialize_schema(&conn).unwrap();

        // Verify git_branch column exists by inserting with it
        conn.execute(
            "INSERT INTO sessions (id, project, started_at, git_branch) VALUES ('s1', '/p', 100, 'main')",
            [],
        )
        .unwrap();

        let branch: Option<String> = conn
            .query_row(
                "SELECT git_branch FROM sessions WHERE id = 's1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(branch, Some("main".to_string()));
    }

    #[test]
    fn fts_trigger_inserts() {
        let conn = open_memory();
        initialize_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO sessions (id, project, started_at) VALUES ('s1', '/p', 100)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO messages (id, session_id, role, content, created_at) VALUES ('m1', 's1', 'user', 'hello world', 100)",
            [],
        )
        .unwrap();

        let fts_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'hello'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(fts_count, 1);
    }

    #[test]
    fn fts_trigger_deletes() {
        let conn = open_memory();
        initialize_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO sessions (id, project, started_at) VALUES ('s1', '/p', 100)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO messages (id, session_id, role, content, created_at) VALUES ('m1', 's1', 'user', 'unique_test_word', 100)",
            [],
        )
        .unwrap();

        // Delete should propagate to FTS
        conn.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
        conn.execute("DELETE FROM messages WHERE id = 'm1'", [])
            .unwrap();

        let fts_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'unique_test_word'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(fts_count, 0);
    }

    #[test]
    fn schema_version_tracks_migrations() {
        let conn = open_memory();
        initialize_schema(&conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2); // v1 + v2
    }
}
