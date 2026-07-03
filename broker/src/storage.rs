//! `broker.sqlite` state store — the broker's access clock and working-set
//! membership.
//!
//! Schema (approved plan WP0):
//! - `knowledge_item(path PK, first_seen, last_accessed, in_working_set, always_include)`
//! - `access_log(id, path, ts, source)`
//!
//! Timestamps are unix epoch seconds. The store records observations only;
//! it never touches memory files (no-deletion constraint).

use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// Ordered, idempotent migrations. Adding a migration appends to this list;
/// applied versions are tracked in `schema_version`.
const MIGRATIONS: &[&str] = &[
    // v1: initial schema
    "CREATE TABLE IF NOT EXISTS knowledge_item (
        path TEXT PRIMARY KEY,
        first_seen INTEGER NOT NULL,
        last_accessed INTEGER,
        in_working_set INTEGER NOT NULL DEFAULT 0,
        always_include INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE IF NOT EXISTS access_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        path TEXT NOT NULL,
        ts INTEGER NOT NULL,
        source TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_access_log_path_ts ON access_log(path, ts);",
];

/// A row in `knowledge_item`.
#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeItem {
    pub path: String,
    pub first_seen: i64,
    pub last_accessed: Option<i64>,
    pub in_working_set: bool,
    pub always_include: bool,
}

pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Open (or create) the state DB at `path` and apply pending migrations.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    /// In-memory store — used by tests and the migration dry-run.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Self, StorageError> {
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);",
        )?;
        let mut store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Apply pending migrations. Idempotent.
    fn migrate(&mut self) -> Result<(), StorageError> {
        let current: i64 = self
            .conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .optional()?
            .flatten()
            .unwrap_or(0);
        for (idx, sql) in MIGRATIONS.iter().enumerate() {
            let version = (idx + 1) as i64;
            if version > current {
                let tx = self.conn.transaction()?;
                tx.execute_batch(sql)?;
                tx.execute("INSERT INTO schema_version (version) VALUES (?1)", [version])?;
                tx.commit()?;
            }
        }
        Ok(())
    }

    /// Migration dry-run: apply the full migration chain against a throwaway
    /// in-memory database and report the resulting schema version.
    pub fn migration_dry_run() -> Result<i64, StorageError> {
        let store = Self::open_in_memory()?;
        store
            .conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .map_err(Into::into)
    }

    /// Record an access observation (broker-mediated read). Creates the item
    /// row when unseen and appends to the audit log.
    pub fn record_access(&mut self, path: &str, ts: i64, source: &str) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO knowledge_item (path, first_seen, last_accessed)
             VALUES (?1, ?2, ?2)
             ON CONFLICT(path) DO UPDATE SET
               last_accessed = MAX(COALESCE(knowledge_item.last_accessed, 0), excluded.last_accessed)",
            params![path, ts],
        )?;
        tx.execute(
            "INSERT INTO access_log (path, ts, source) VALUES (?1, ?2, ?3)",
            params![path, ts, source],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Upsert working-set membership / pin flags without touching the clock.
    pub fn set_membership(
        &self,
        path: &str,
        in_working_set: bool,
        always_include: bool,
        now: i64,
    ) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO knowledge_item (path, first_seen, in_working_set, always_include)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(path) DO UPDATE SET
               in_working_set = excluded.in_working_set,
               always_include = excluded.always_include",
            params![path, now, in_working_set, always_include],
        )?;
        Ok(())
    }

    pub fn get_item(&self, path: &str) -> Result<Option<KnowledgeItem>, StorageError> {
        self.conn
            .query_row(
                "SELECT path, first_seen, last_accessed, in_working_set, always_include
                 FROM knowledge_item WHERE path = ?1",
                [path],
                |r| {
                    Ok(KnowledgeItem {
                        path: r.get(0)?,
                        first_seen: r.get(1)?,
                        last_accessed: r.get(2)?,
                        in_working_set: r.get::<_, i64>(3)? != 0,
                        always_include: r.get::<_, i64>(4)? != 0,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn all_items(&self) -> Result<Vec<KnowledgeItem>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT path, first_seen, last_accessed, in_working_set, always_include
             FROM knowledge_item ORDER BY path",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(KnowledgeItem {
                    path: r.get(0)?,
                    first_seen: r.get(1)?,
                    last_accessed: r.get(2)?,
                    in_working_set: r.get::<_, i64>(3)? != 0,
                    always_include: r.get::<_, i64>(4)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn access_log_count(&self, path: &str) -> Result<i64, StorageError> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM access_log WHERE path = ?1",
                [path],
                |r| r.get(0),
            )
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_dry_run_reaches_latest_version() {
        let version = Storage::migration_dry_run().unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
    }

    #[test]
    fn migrate_is_idempotent_on_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broker.sqlite");
        {
            Storage::open(&path).unwrap();
        }
        // Reopening applies no duplicate migrations and does not error.
        let store = Storage::open(&path).unwrap();
        assert!(store.all_items().unwrap().is_empty());
    }

    #[test]
    fn record_access_creates_item_and_appends_log() {
        let mut store = Storage::open_in_memory().unwrap();
        store.record_access("episodes/2026-06-01.md", 1_000, "broker").unwrap();
        store.record_access("episodes/2026-06-01.md", 2_000, "mcp").unwrap();

        let item = store.get_item("episodes/2026-06-01.md").unwrap().unwrap();
        assert_eq!(item.first_seen, 1_000, "first_seen keeps the original ts");
        assert_eq!(item.last_accessed, Some(2_000), "last_accessed advances");
        assert_eq!(store.access_log_count("episodes/2026-06-01.md").unwrap(), 2);
    }

    #[test]
    fn record_access_never_moves_clock_backwards() {
        let mut store = Storage::open_in_memory().unwrap();
        store.record_access("a.md", 5_000, "broker").unwrap();
        store.record_access("a.md", 1_000, "broker").unwrap();
        let item = store.get_item("a.md").unwrap().unwrap();
        assert_eq!(item.last_accessed, Some(5_000));
    }

    #[test]
    fn membership_flags_roundtrip() {
        let store = Storage::open_in_memory().unwrap();
        store.set_membership("IDENTITY.md", true, true, 100).unwrap();
        let item = store.get_item("IDENTITY.md").unwrap().unwrap();
        assert!(item.in_working_set);
        assert!(item.always_include);
        assert_eq!(item.last_accessed, None, "membership does not touch the clock");
    }
}
