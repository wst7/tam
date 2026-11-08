use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::sql::{
    ALTER_TASK_ADD_ASSIGNED_AGENT_SQL, ALTER_TASK_ADD_IDEMPOTENCY_KEY_SQL,
    ALTER_TASK_ADD_LEASE_EXPIRES_SQL, ALTER_TASK_ADD_PRIORITY_SQL, ALTER_TASK_ADD_REVISION_SQL,
    ALTER_TASK_ADD_TAGS_SQL, CREATE_PROJECT_TABLE_SQL, CREATE_PROJECT_TABLE_TRIGGER_SQL,
    CREATE_TASK_ASSIGNEE_INDEX_SQL, CREATE_TASK_EVENTS_TABLE_SQL,
    CREATE_TASK_EVENTS_TASK_INDEX_SQL, CREATE_TASK_IDEMPOTENCY_INDEX_SQL,
    CREATE_TASK_IDEMPOTENCY_INDEX_V2_SQL, CREATE_TASK_PROJECT_STATUS_INDEX_SQL,
    CREATE_TASK_TABLE_SQL, CREATE_TASK_TABLE_TRIGGER_SQL, DROP_TASK_IDEMPOTENCY_INDEX_SQL,
};
use rusqlite::Connection;

use crate::config::get_config;

/// Current schema version (v2: idempotency keys are reusable after a soft
/// delete because the partial unique index excludes `is_deleted = 1` rows).
pub const SCHEMA_VERSION: i64 = 2;

pub fn get_db() -> anyhow::Result<Connection> {
    let c = get_config();

    // If the database file dir does not exist, create it
    if let Some(parent) = Path::new(&c.db_file).parent() {
        fs::create_dir_all(parent)?;
    }
    // Open the database file, it will be created if it does not exist
    let conn = Connection::open(&c.db_file)?;
    // Agent-first runtime pragmas: concurrent claim needs a busy timeout and
    // WAL; FK enforcement keeps task.project_id meaningful.
    conn.busy_timeout(Duration::from_millis(5000))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    Ok(conn)
}

pub fn init_db() -> anyhow::Result<bool> {
    let conn = get_db()?;
    conn.execute(CREATE_PROJECT_TABLE_SQL, ())?;
    conn.execute(CREATE_PROJECT_TABLE_TRIGGER_SQL, ())?;
    conn.execute(CREATE_TASK_TABLE_SQL, ())?;
    conn.execute(CREATE_TASK_TABLE_TRIGGER_SQL, ())?;

    let db_file = get_config().db_file;
    apply_migrations(&conn, Some(Path::new(&db_file)))?;
    Ok(true)
}

/// Additive, idempotent migration from schema v0 to v1.
///
/// - `db_path` is used to create `tam.db3.bak.v1` before the first migration of
///   a non-empty database (`None` skips backups, e.g. for in-memory tests).
/// - Existing rows are never rewritten: new columns get their defaults.
pub fn apply_migrations(conn: &Connection, db_path: Option<&Path>) -> anyhow::Result<bool> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version >= SCHEMA_VERSION {
        return Ok(false);
    }

    // v0 -> v1: additive columns/events/indexes. This is the only step that may
    // create a backup (v1 -> v2 only replaces an index).
    if version < 1 {
        if let Some(path) = db_path {
            if path.exists()
                && (table_row_count(conn, "project")? + table_row_count(conn, "task")?) > 0
            {
                backup_database(conn, path)?;
            }
        }

        let columns = task_columns(conn)?;
        for (name, statement) in [
            ("priority", ALTER_TASK_ADD_PRIORITY_SQL),
            ("tags", ALTER_TASK_ADD_TAGS_SQL),
            ("assigned_agent", ALTER_TASK_ADD_ASSIGNED_AGENT_SQL),
            ("lease_expires_at", ALTER_TASK_ADD_LEASE_EXPIRES_SQL),
            ("revision", ALTER_TASK_ADD_REVISION_SQL),
            ("idempotency_key", ALTER_TASK_ADD_IDEMPOTENCY_KEY_SQL),
        ] {
            if !columns.iter().any(|column| column == name) {
                conn.execute_batch(statement)
                    .map_err(|e| anyhow::anyhow!("failed to add task column '{}': {}", name, e))?;
            }
        }

        conn.execute_batch(CREATE_TASK_EVENTS_TABLE_SQL)?;
        conn.execute_batch(CREATE_TASK_EVENTS_TASK_INDEX_SQL)?;
        conn.execute_batch(CREATE_TASK_PROJECT_STATUS_INDEX_SQL)?;
        conn.execute_batch(CREATE_TASK_ASSIGNEE_INDEX_SQL)?;
        conn.execute_batch(CREATE_TASK_IDEMPOTENCY_INDEX_SQL)?;
    }

    // v1 -> v2: idempotency keys become reusable after a soft delete.
    if version < 2 {
        conn.execute_batch(DROP_TASK_IDEMPOTENCY_INDEX_SQL)?;
        conn.execute_batch(CREATE_TASK_IDEMPOTENCY_INDEX_V2_SQL)?;
    }

    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(true)
}

fn task_columns(conn: &Connection) -> anyhow::Result<Vec<String>> {
    let mut stmt = conn.prepare("PRAGMA table_info(task)")?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(columns)
}

fn table_row_count(conn: &Connection, table: &str) -> anyhow::Result<i64> {
    let sql = format!("SELECT COUNT(*) FROM {}", table);
    Ok(conn.query_row(&sql, [], |row| row.get(0))?)
}

/// Copy the database to `<db>.bak.v1` (never overwriting an existing backup).
/// A checkpoint first folds any WAL content into the main file so the copy is
/// a complete snapshot.
fn backup_database(conn: &Connection, db_path: &Path) -> anyhow::Result<PathBuf> {
    let backup_path = PathBuf::from(format!("{}.bak.v1", db_path.display()));
    if backup_path.exists() {
        return Ok(backup_path);
    }
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    fs::copy(db_path, &backup_path)
        .map_err(|e| anyhow::anyhow!("failed to back up database to {:?}: {}", backup_path, e))?;
    Ok(backup_path)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sql::{
        CREATE_PROJECT_TABLE_SQL, CREATE_PROJECT_TABLE_TRIGGER_SQL, CREATE_TASK_TABLE_SQL,
        CREATE_TASK_TABLE_TRIGGER_SQL,
    };
    use std::sync::atomic::{AtomicU32, Ordering};

    static NEXT_ID: AtomicU32 = AtomicU32::new(0);

    fn temp_db_path(tag: &str) -> PathBuf {
        let unique = format!(
            "tam-db-migration-{}-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::SeqCst),
            tag
        );
        std::env::temp_dir().join(unique).join("tam.db3")
    }

    fn create_legacy_db(path: &Path) -> Connection {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let conn = Connection::open(path).unwrap();
        conn.execute(CREATE_PROJECT_TABLE_SQL, ()).unwrap();
        conn.execute(CREATE_PROJECT_TABLE_TRIGGER_SQL, ()).unwrap();
        conn.execute(CREATE_TASK_TABLE_SQL, ()).unwrap();
        conn.execute(CREATE_TASK_TABLE_TRIGGER_SQL, ()).unwrap();
        conn.execute("INSERT INTO project (name) VALUES ('legacy')", ())
            .unwrap();
        conn.execute(
            "INSERT INTO task (title, project_id) VALUES ('first', 1)",
            (),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO task (title, project_id) VALUES ('second', 1)",
            (),
        )
        .unwrap();
        conn
    }

    fn table_exists(conn: &Connection, table: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
            (table,),
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
            > 0
    }

    fn index_exists(conn: &Connection, index: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?",
            (index,),
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
            > 0
    }

    #[test]
    fn migrates_legacy_schema_and_creates_backup() {
        let path = temp_db_path("legacy");
        let conn = create_legacy_db(&path);

        let applied = apply_migrations(&conn, Some(&path)).expect("migration should succeed");
        assert!(applied);

        let columns = task_columns(&conn).unwrap();
        for column in [
            "priority",
            "tags",
            "assigned_agent",
            "lease_expires_at",
            "revision",
            "idempotency_key",
        ] {
            assert!(
                columns.iter().any(|c| c == column),
                "missing column {column}"
            );
        }
        assert!(table_exists(&conn, "task_events"));
        for index in [
            "idx_task_project_status",
            "idx_task_assignee",
            "idx_task_idempotency",
        ] {
            assert!(index_exists(&conn, index), "missing index {index}");
        }

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2, "fresh/legacy databases migrate to schema v2");

        // The idempotency index must ignore soft-deleted rows so a key can be
        // reused after a task was deleted.
        let index_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'idx_task_idempotency'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            index_sql.contains("is_deleted = 0"),
            "idempotency index must exclude soft-deleted rows: {index_sql}"
        );

        // Rows are preserved and get the documented defaults.
        let task_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM task", [], |row| row.get(0))
            .unwrap();
        assert_eq!(task_count, 2);
        let (priority, revision, tags, assigned): (i64, i64, String, Option<String>) = conn
            .query_row(
                "SELECT priority, revision, tags, assigned_agent FROM task WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(priority, 3);
        assert_eq!(revision, 1);
        assert_eq!(tags, "");
        assert_eq!(assigned, None);

        // The backup was written before the migration and has the same rows.
        let backup = PathBuf::from(format!("{}.bak.v1", path.display()));
        assert!(backup.exists(), "backup {:?} missing", backup);
        let backup_conn = Connection::open(&backup).unwrap();
        let backup_tasks: i64 = backup_conn
            .query_row("SELECT COUNT(*) FROM task", [], |row| row.get(0))
            .unwrap();
        assert_eq!(backup_tasks, 2);

        // Events are writable after migration.
        conn.execute(
            "INSERT INTO task_events (task_id, project_id, event, actor) VALUES (1, 1, 'created', 'human')",
            (),
        )
        .unwrap();

        // A soft-deleted task must not block reusing its idempotency key.
        conn.execute(
            "INSERT INTO task (title, project_id, idempotency_key) VALUES ('keyed', 1, 'reuse-me')",
            (),
        )
        .unwrap();
        let keyed_id = conn.last_insert_rowid();
        conn.execute("UPDATE task SET is_deleted = 1 WHERE id = ?1", (keyed_id,))
            .unwrap();
        conn.execute(
            "INSERT INTO task (title, project_id, idempotency_key) VALUES ('keyed-again', 1, 'reuse-me')",
            (),
        )
        .expect("a soft-deleted task must not reserve the idempotency key");

        drop(conn);
        drop(backup_conn);
        cleanup(&path);
    }

    #[test]
    fn migration_is_idempotent() {
        let path = temp_db_path("idempotent");
        let conn = create_legacy_db(&path);

        assert!(apply_migrations(&conn, Some(&path)).unwrap());
        let second = apply_migrations(&conn, Some(&path)).unwrap();
        assert!(!second, "second migration must be a no-op");

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
        let task_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM task", [], |row| row.get(0))
            .unwrap();
        assert_eq!(task_count, 2);

        drop(conn);
        cleanup(&path);
    }

    #[test]
    fn migrates_in_memory_without_backup() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(CREATE_PROJECT_TABLE_SQL, ()).unwrap();
        conn.execute(CREATE_PROJECT_TABLE_TRIGGER_SQL, ()).unwrap();
        conn.execute(CREATE_TASK_TABLE_SQL, ()).unwrap();
        conn.execute(CREATE_TASK_TABLE_TRIGGER_SQL, ()).unwrap();

        assert!(apply_migrations(&conn, None).unwrap());
        assert!(table_exists(&conn, "task_events"));
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
    }

    #[test]
    fn empty_legacy_db_migration_needs_no_backup() {
        let path = temp_db_path("empty");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let conn = Connection::open(&path).unwrap();
        conn.execute(CREATE_PROJECT_TABLE_SQL, ()).unwrap();
        conn.execute(CREATE_PROJECT_TABLE_TRIGGER_SQL, ()).unwrap();
        conn.execute(CREATE_TASK_TABLE_SQL, ()).unwrap();
        conn.execute(CREATE_TASK_TABLE_TRIGGER_SQL, ()).unwrap();

        assert!(apply_migrations(&conn, Some(&path)).unwrap());
        assert!(!PathBuf::from(format!("{}.bak.v1", path.display())).exists());

        drop(conn);
        cleanup(&path);
    }

    /// Build a database in the shape produced by schema v1 (no v2 index fix).
    fn create_v1_db(path: &Path) -> Connection {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let conn = Connection::open(path).unwrap();
        conn.execute(CREATE_PROJECT_TABLE_SQL, ()).unwrap();
        conn.execute(CREATE_PROJECT_TABLE_TRIGGER_SQL, ()).unwrap();
        conn.execute(CREATE_TASK_TABLE_SQL, ()).unwrap();
        conn.execute(CREATE_TASK_TABLE_TRIGGER_SQL, ()).unwrap();
        for statement in [
            ALTER_TASK_ADD_PRIORITY_SQL,
            ALTER_TASK_ADD_TAGS_SQL,
            ALTER_TASK_ADD_ASSIGNED_AGENT_SQL,
            ALTER_TASK_ADD_LEASE_EXPIRES_SQL,
            ALTER_TASK_ADD_REVISION_SQL,
            ALTER_TASK_ADD_IDEMPOTENCY_KEY_SQL,
        ] {
            conn.execute_batch(statement).unwrap();
        }
        conn.execute_batch(CREATE_TASK_EVENTS_TABLE_SQL).unwrap();
        conn.execute_batch(CREATE_TASK_EVENTS_TASK_INDEX_SQL)
            .unwrap();
        conn.execute_batch(CREATE_TASK_PROJECT_STATUS_INDEX_SQL)
            .unwrap();
        conn.execute_batch(CREATE_TASK_ASSIGNEE_INDEX_SQL).unwrap();
        // The original v1 index did not exclude soft-deleted rows.
        conn.execute_batch(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_task_idempotency ON task(project_id, idempotency_key) WHERE idempotency_key IS NOT NULL AND idempotency_key <> '';",
        )
        .unwrap();
        conn.execute("INSERT INTO project (name) VALUES ('legacy-v1')", ())
            .unwrap();
        conn.execute(
            "INSERT INTO task (title, project_id) VALUES ('v1 task', 1)",
            (),
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 1_i64).unwrap();
        conn
    }

    #[test]
    fn migrates_v1_to_v2_index_without_new_backup() {
        let path = temp_db_path("v1-to-v2");
        let conn = create_v1_db(&path);

        let applied = apply_migrations(&conn, Some(&path)).unwrap();
        assert!(applied, "v1 -> v2 migration must run");

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
        let index_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'idx_task_idempotency'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            index_sql.contains("is_deleted = 0"),
            "v2 index must exclude soft-deleted rows: {index_sql}"
        );
        // A v1 -> v2 upgrade only replaces an index, so no second backup.
        assert!(!PathBuf::from(format!("{}.bak.v1", path.display())).exists());

        drop(conn);
        cleanup(&path);
    }

    fn cleanup(path: &Path) {
        for suffix in ["", "-wal", "-shm", ".bak.v1"] {
            let _ = fs::remove_file(format!("{}{}", path.display(), suffix));
        }
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}
