pub const CREATE_PROJECT_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS project (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name CHAR(255) NOT NULL UNIQUE,
    created DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_deleted BOOLEAN DEFAULT 0
);
"#;

pub const CREATE_PROJECT_TABLE_TRIGGER_SQL: &str = r#"
CREATE TRIGGER IF NOT EXISTS project_update_timestamp
AFTER UPDATE ON project
FOR EACH ROW
BEGIN
    UPDATE project SET updated = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
"#;

pub const CREATE_TASK_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS task (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    content TEXT DEFAULT '',
    created DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated DATETIME DEFAULT CURRENT_TIMESTAMP,
    status INTEGER NOT NULL DEFAULT '0' CHECK (status IN ('0', '1', '2')),
    project_id INTEGER NOT NULL,
    is_deleted BOOLEAN DEFAULT 0,
    FOREIGN KEY (project_id) REFERENCES project(id)
);
"#;

pub const CREATE_TASK_TABLE_TRIGGER_SQL: &str = r#"
CREATE TRIGGER IF NOT EXISTS task_update_timestamp
AFTER UPDATE ON task
FOR EACH ROW
BEGIN
    UPDATE task SET updated = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
"#;

// ---------------------------------------------------------------------------
// Agent-first migration (v0 -> v1): additive columns, event log, indexes.
// ---------------------------------------------------------------------------

pub const ALTER_TASK_ADD_PRIORITY_SQL: &str =
    "ALTER TABLE task ADD COLUMN priority INTEGER NOT NULL DEFAULT 3;";
pub const ALTER_TASK_ADD_TAGS_SQL: &str =
    "ALTER TABLE task ADD COLUMN tags TEXT NOT NULL DEFAULT '';";
pub const ALTER_TASK_ADD_ASSIGNED_AGENT_SQL: &str =
    "ALTER TABLE task ADD COLUMN assigned_agent TEXT;";
pub const ALTER_TASK_ADD_LEASE_EXPIRES_SQL: &str =
    "ALTER TABLE task ADD COLUMN lease_expires_at INTEGER;";
pub const ALTER_TASK_ADD_REVISION_SQL: &str =
    "ALTER TABLE task ADD COLUMN revision INTEGER NOT NULL DEFAULT 1;";
pub const ALTER_TASK_ADD_IDEMPOTENCY_KEY_SQL: &str =
    "ALTER TABLE task ADD COLUMN idempotency_key TEXT;";

pub const CREATE_TASK_EVENTS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS task_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    project_id INTEGER NOT NULL,
    event TEXT NOT NULL,
    actor TEXT NOT NULL DEFAULT '',
    from_status INTEGER,
    to_status INTEGER,
    metadata TEXT NOT NULL DEFAULT '{}',
    created DATETIME DEFAULT CURRENT_TIMESTAMP
);
"#;

pub const CREATE_TASK_EVENTS_TASK_INDEX_SQL: &str =
    "CREATE INDEX IF NOT EXISTS idx_task_events_task ON task_events(task_id);";
pub const CREATE_TASK_PROJECT_STATUS_INDEX_SQL: &str =
    "CREATE INDEX IF NOT EXISTS idx_task_project_status ON task(project_id, status, is_deleted);";
pub const CREATE_TASK_ASSIGNEE_INDEX_SQL: &str =
    "CREATE INDEX IF NOT EXISTS idx_task_assignee ON task(assigned_agent);";
/// Schema v1 index (kept for the v0 -> v1 migration step).
pub const CREATE_TASK_IDEMPOTENCY_INDEX_SQL: &str = r#"
CREATE UNIQUE INDEX IF NOT EXISTS idx_task_idempotency
  ON task(project_id, idempotency_key)
  WHERE idempotency_key IS NOT NULL AND idempotency_key <> '';
"#;

/// Schema v2: idempotency keys may be reused once the previous task is
/// soft-deleted.
pub const DROP_TASK_IDEMPOTENCY_INDEX_SQL: &str = "DROP INDEX IF EXISTS idx_task_idempotency;";

pub const CREATE_TASK_IDEMPOTENCY_INDEX_V2_SQL: &str = r#"
CREATE UNIQUE INDEX IF NOT EXISTS idx_task_idempotency
  ON task(project_id, idempotency_key)
  WHERE idempotency_key IS NOT NULL AND idempotency_key <> '' AND is_deleted = 0;
"#;

// ---------------------------------------------------------------------------
// Project queries.
// ---------------------------------------------------------------------------

pub const SELECT_PROJECTS_SQL: &str = r#"
SELECT id, name, created, updated FROM project WHERE is_deleted = 0;
"#;

pub const SELECT_PROJECT_SQL: &str = r#"
SELECT id, name, created, updated FROM project WHERE name = ? AND is_deleted = 0;
"#;

pub const CREATE_PROJECT_SQL: &str = r#"
INSERT INTO project (name) VALUES (?);
"#;

/// Looks up a project name across active *and* soft-deleted rows.
pub const SELECT_PROJECT_INCLUDING_DELETED_SQL: &str = r#"
SELECT is_deleted FROM project WHERE name = ?;
"#;

pub const DELETE_PROJECT_SQL: &str = r#"
UPDATE project SET is_deleted = 1 WHERE name = ?;
"#;

/// Restores a soft-deleted project; the update trigger refreshes `updated`.
pub const RESTORE_PROJECT_SQL: &str = r#"
UPDATE project SET is_deleted = 0 WHERE name = ?;
"#;

/// Number of active (non-deleted) tasks attached to a project.
pub const COUNT_ACTIVE_TASKS_SQL: &str = r#"
SELECT COUNT(*) FROM task WHERE project_id = ? AND is_deleted = 0;
"#;

pub const UPDATE_PROJECT_SQL: &str = r#"
UPDATE project SET name = ? WHERE name = ?;
"#;

// ---------------------------------------------------------------------------
// Task queries. The full projection is shared by the human and agent paths.
// Columns: 0 id, 1 project_id, 2 project name, 3 title, 4 content, 5 status,
//          6 priority, 7 tags, 8 assigned_agent, 9 lease_expires_at,
//          10 revision, 11 created, 12 updated.
// ---------------------------------------------------------------------------

pub const SELECT_ALL_TASKS_SQL: &str = r#"
SELECT t.id, t.project_id, p.name, t.title, t.content, t.status, t.priority, t.tags,
       t.assigned_agent, t.lease_expires_at, t.revision, t.created, t.updated
FROM task t JOIN project p ON p.id = t.project_id
WHERE t.project_id = ? AND t.is_deleted = 0
ORDER BY t.created DESC;
"#;

pub const SELECT_TASK_SQL: &str = r#"
SELECT t.id, t.project_id, p.name, t.title, t.content, t.status, t.priority, t.tags,
       t.assigned_agent, t.lease_expires_at, t.revision, t.created, t.updated
FROM task t JOIN project p ON p.id = t.project_id
WHERE t.id = ? AND t.is_deleted = 0;
"#;

pub const SELECT_TASK_BY_IDEMPOTENCY_SQL: &str = r#"
SELECT id FROM task
WHERE project_id = ? AND idempotency_key = ? AND is_deleted = 0;
"#;

/// Next claimable candidate: highest priority then oldest; skips Done and
/// tasks under a valid lease (`lease_expires_at > now`).
pub const SELECT_NEXT_TASK_SQL: &str = r#"
SELECT t.id FROM task t
WHERE t.project_id = ? AND t.is_deleted = 0 AND t.status != 2
  AND (t.lease_expires_at IS NULL OR t.lease_expires_at <= ?)
ORDER BY t.priority ASC, t.created ASC, t.id ASC
LIMIT 1;
"#;

pub const INSERT_TASK_FULL_SQL: &str = r#"
INSERT INTO task (title, content, project_id, priority, tags, assigned_agent,
                  lease_expires_at, revision, idempotency_key)
VALUES (?, ?, ?, ?, ?, NULL, NULL, 1, ?);
"#;

pub const UPDATE_TASK_FIELDS_SQL: &str = r#"
UPDATE task
SET title = ?, content = ?, status = ?, priority = ?, tags = ?,
    assigned_agent = ?, lease_expires_at = ?, revision = revision + 1
WHERE id = ?;
"#;

pub const UPDATE_TASK_CLAIM_SQL: &str = r#"
UPDATE task
SET status = 1, assigned_agent = ?, lease_expires_at = ?, revision = revision + 1
WHERE id = ?;
"#;

pub const UPDATE_TASK_HEARTBEAT_SQL: &str = r#"
UPDATE task SET lease_expires_at = ?, revision = revision + 1 WHERE id = ?;
"#;

pub const UPDATE_TASK_RELEASE_SQL: &str = r#"
UPDATE task
SET assigned_agent = NULL, lease_expires_at = NULL,
    status = CASE WHEN status = 1 THEN 0 ELSE status END,
    revision = revision + 1
WHERE id = ?;
"#;

pub const UPDATE_TASK_COMPLETE_SQL: &str = r#"
UPDATE task
SET status = 2, assigned_agent = NULL, lease_expires_at = NULL, revision = revision + 1
WHERE id = ?;
"#;

/// Soft delete with revision bump (agent `task del`).
pub const UPDATE_TASK_SOFT_DELETE_SQL: &str = r#"
UPDATE task SET is_deleted = 1, revision = revision + 1 WHERE id = ?;
"#;

/// Full projection for a single task, including soft-deleted rows (used to
/// return the deleted task and to look up rows after deletion).
pub const SELECT_TASK_INCLUDING_DELETED_SQL: &str = r#"
SELECT t.id, t.project_id, p.name, t.title, t.content, t.status, t.priority, t.tags,
       t.assigned_agent, t.lease_expires_at, t.revision, t.created, t.updated
FROM task t JOIN project p ON p.id = t.project_id
WHERE t.id = ?;
"#;

// ---------------------------------------------------------------------------
// Task event log.
// ---------------------------------------------------------------------------

pub const INSERT_TASK_EVENT_SQL: &str = r#"
INSERT INTO task_events (task_id, project_id, event, actor, from_status, to_status, metadata)
VALUES (?, ?, ?, ?, ?, ?, ?);
"#;

pub const SELECT_TASK_EVENTS_SQL: &str = r#"
SELECT id, task_id, project_id, event, actor, from_status, to_status, metadata, created
FROM task_events WHERE task_id = ? ORDER BY id ASC;
"#;
