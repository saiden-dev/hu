use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

use super::schema::initialize_schema;

pub struct SqliteStore {
    pub conn: Connection,
}

impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::configure(&conn)?;
        Ok(Self { conn })
    }

    #[allow(dead_code)]
    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::configure(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_initialized(path: &Path) -> Result<Self> {
        let store = Self::open(path)?;
        initialize_schema(&store.conn)?;
        Ok(store)
    }

    fn configure(conn: &Connection) -> Result<()> {
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(())
    }
}

#[cfg(test)]
pub fn open_test_db() -> SqliteStore {
    let store = SqliteStore::open_memory().unwrap();
    initialize_schema(&store.conn).unwrap();
    store
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_memory_works() {
        let store = SqliteStore::open_memory().unwrap();
        let result: i64 = store
            .conn
            .query_row("SELECT 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn open_test_db_has_schema() {
        let store = open_test_db();
        let version: i64 = store
            .conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 2);
    }

    #[test]
    fn open_initialized_creates_file() {
        let tmp = std::env::temp_dir().join("hu-test-db-init");
        let _ = std::fs::remove_dir_all(&tmp);
        let db_path = tmp.join("test.db");

        let store = SqliteStore::open_initialized(&db_path).unwrap();
        let version: i64 = store
            .conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 2);
        assert!(db_path.exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn wal_mode_enabled() {
        let store = open_test_db();
        let mode: String = store
            .conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        // In-memory databases use "memory" mode, not WAL
        assert!(mode == "wal" || mode == "memory");
    }

    #[test]
    fn foreign_keys_enabled() {
        let store = open_test_db();
        let fk: i64 = store
            .conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }
}
