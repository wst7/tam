use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::json;

use crate::{
    agent::{self, AgentError},
    db,
    model::{Task, TaskEvent, TaskStatus},
    services::project,
    sql::{
        INSERT_TASK_EVENT_SQL, INSERT_TASK_FULL_SQL, SELECT_ALL_TASKS_SQL, SELECT_NEXT_TASK_SQL,
        SELECT_TASK_BY_IDEMPOTENCY_SQL, SELECT_TASK_EVENTS_SQL, SELECT_TASK_INCLUDING_DELETED_SQL,
        SELECT_TASK_SQL, UPDATE_TASK_CLAIM_SQL, UPDATE_TASK_COMPLETE_SQL, UPDATE_TASK_FIELDS_SQL,
        UPDATE_TASK_HEARTBEAT_SQL, UPDATE_TASK_RELEASE_SQL, UPDATE_TASK_SOFT_DELETE_SQL,
    },
};

pub const DEFAULT_LEASE_SECONDS: i64 = 900;
pub const MIN_LEASE_SECONDS: i64 = 5;
pub const MAX_LEASE_SECONDS: i64 = 86_400;

/// Parse a SQLite `DATETIME` value (`YYYY-MM-DD HH:MM:SS`, stored as UTC) into
/// a UTC timestamp. Invalid values are reported as errors; the current time is
/// never used as a fallback.
pub(crate) fn parse_db_datetime(value: &str) -> anyhow::Result<DateTime<Utc>> {
    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| anyhow::anyhow!("invalid database timestamp {:?}: {}", value, e))?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

/// Canonicalize a task title coming from the CLI or the TUI: surrounding
/// whitespace is ignored and blank titles are rejected.
fn normalize_task_title(raw: &str) -> anyhow::Result<String> {
    let title = raw.trim();
    if title.is_empty() {
        return Err(agent::to_anyhow(AgentError::invalid(
            "task title cannot be empty",
        )));
    }
    Ok(title.to_string())
}

/// Canonicalize tags: lowercase, trimmed, non-empty, deduplicated, comma joined.
pub fn normalize_tags(tags: &[String]) -> String {
    let mut normalized: Vec<String> = Vec::new();
    for tag in tags {
        for part in tag.split(',') {
            let part = part.trim().to_lowercase();
            if !part.is_empty() && !normalized.contains(&part) {
                normalized.push(part);
            }
        }
    }
    normalized.join(",")
}

pub fn tags_of(task: &Task) -> Vec<String> {
    if task.tags.is_empty() {
        Vec::new()
    } else {
        task.tags.split(',').map(|tag| tag.to_string()).collect()
    }
}

fn validate_priority(priority: i32) -> anyhow::Result<i32> {
    if (1..=5).contains(&priority) {
        Ok(priority)
    } else {
        Err(agent::to_anyhow(AgentError::invalid(format!(
            "priority must be between 1 and 5, got {}",
            priority
        ))))
    }
}

fn validate_lease(seconds: i64) -> anyhow::Result<i64> {
    if (MIN_LEASE_SECONDS..=MAX_LEASE_SECONDS).contains(&seconds) {
        Ok(seconds)
    } else {
        Err(agent::to_anyhow(AgentError::invalid(format!(
            "lease must be between {} and {} seconds, got {}",
            MIN_LEASE_SECONDS, MAX_LEASE_SECONDS, seconds
        ))))
    }
}

fn not_found_task(id: i32) -> anyhow::Error {
    agent::to_anyhow(AgentError::not_found(format!("task {} not found", id)))
}

fn conflict(message: impl Into<String>) -> anyhow::Error {
    agent::to_anyhow(AgentError::conflict(message))
}

fn is_lease_valid(task: &Task, now: i64) -> bool {
    task.assigned_agent.is_some()
        && task
            .lease_expires_at
            .map(|expires| expires > now)
            .unwrap_or(false)
}

/// Raw task row before timestamp parsing.
struct RawTask {
    id: i32,
    project_id: i32,
    project: String,
    title: String,
    content: String,
    status: TaskStatus,
    priority: i32,
    tags: String,
    assigned_agent: Option<String>,
    lease_expires_at: Option<i64>,
    revision: i32,
    created: String,
    updated: String,
}

impl RawTask {
    fn into_task(self) -> anyhow::Result<Task> {
        Ok(Task {
            id: Some(self.id),
            project_id: self.project_id,
            project: self.project,
            title: self.title,
            content: self.content,
            status: self.status,
            priority: self.priority,
            tags: self.tags,
            assigned_agent: self.assigned_agent,
            lease_expires_at: self.lease_expires_at,
            revision: self.revision,
            created: parse_db_datetime(&self.created)?,
            updated: parse_db_datetime(&self.updated)?,
        })
    }
}

fn map_raw_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawTask> {
    Ok(RawTask {
        id: row.get(0)?,
        project_id: row.get(1)?,
        project: row.get(2)?,
        title: row.get(3)?,
        content: row.get(4)?,
        status: row.get(5)?,
        priority: row.get(6)?,
        tags: row.get(7)?,
        assigned_agent: row.get(8)?,
        lease_expires_at: row.get(9)?,
        revision: row.get(10)?,
        created: row.get(11)?,
        updated: row.get(12)?,
    })
}

/// Load a task by id, optionally rejecting tasks outside `project_scope`.
fn query_task_conn(conn: &Connection, id: i32) -> anyhow::Result<Task> {
    let mut stmt = conn.prepare(SELECT_TASK_SQL)?;
    let raw = stmt.query_row((id,), map_raw_task).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => not_found_task(id),
        other => anyhow::anyhow!("failed to query task {}: {}", id, other),
    })?;
    raw.into_task()
}

fn query_task_any_conn(conn: &Connection, id: i32) -> anyhow::Result<Task> {
    let mut stmt = conn.prepare(SELECT_TASK_INCLUDING_DELETED_SQL)?;
    let raw = stmt.query_row((id,), map_raw_task).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => not_found_task(id),
        other => anyhow::anyhow!("failed to query task {}: {}", id, other),
    })?;
    raw.into_task()
}

fn ensure_scope(task: &Task, project_scope: Option<i32>) -> anyhow::Result<()> {
    if let Some(project_id) = project_scope {
        if task.project_id != project_id {
            return Err(not_found_task(task.id.unwrap_or_default()));
        }
    }
    Ok(())
}

fn load_scoped(conn: &Connection, id: i32, project_scope: Option<i32>) -> anyhow::Result<Task> {
    let task = query_task_conn(conn, id)?;
    ensure_scope(&task, project_scope)?;
    Ok(task)
}

fn insert_event(
    conn: &Connection,
    task: &Task,
    event: &str,
    actor: &str,
    from_status: Option<TaskStatus>,
    to_status: Option<TaskStatus>,
    metadata: serde_json::Value,
) -> anyhow::Result<()> {
    conn.execute(
        INSERT_TASK_EVENT_SQL,
        params![
            task.id.unwrap_or_default(),
            task.project_id,
            event,
            actor,
            from_status.map(|status| status as i32),
            to_status.map(|status| status as i32),
            metadata.to_string()
        ],
    )?;
    Ok(())
}

fn fetch_tasks(project_id: i32) -> anyhow::Result<Vec<Task>> {
    let conn = db::get_db()?;
    let mut stmt = conn.prepare(SELECT_ALL_TASKS_SQL)?;
    let rows = stmt
        .query_map((project_id,), map_raw_task)?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    rows.into_iter().map(RawTask::into_task).collect()
}

fn current_project_id() -> anyhow::Result<i32> {
    let project = project::get_current_project()?;
    project.id.ok_or_else(|| {
        anyhow::anyhow!("No current project set. Use `tam project use <name>` to set a project.")
    })
}

// ---------------------------------------------------------------------------
// Human-facing operations (kept source-compatible for the TUI and old CLI).
// ---------------------------------------------------------------------------

/// Human/TUI task creation: `actor` is stored in the event log.
pub fn add_task(title: String, p_id: Option<i32>, actor: &str) -> anyhow::Result<Task> {
    let project_id = match p_id {
        Some(id) => id,
        None => current_project_id()?,
    };
    let (task, _) = create_task(
        project_id,
        CreateTaskInput {
            title,
            description: None,
            priority: None,
            tags: Vec::new(),
            idempotency_key: None,
        },
        actor,
    )?;
    Ok(task)
}

pub fn add(title: String, p_id: Option<i32>) -> anyhow::Result<bool> {
    add_task(title, p_id, "human")?;
    Ok(true)
}

pub fn list(project_id: i32, status: Option<TaskStatus>) -> anyhow::Result<Vec<Task>> {
    let tasks = fetch_tasks(project_id)?;
    Ok(match status {
        Some(status) => tasks
            .into_iter()
            .filter(|task| task.status == status)
            .collect(),
        None => tasks,
    })
}

pub fn start(id: i32) -> anyhow::Result<bool> {
    let mut conn = db::get_db()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let task = query_task_conn(&tx, id)?;
    if task.status == TaskStatus::InProgress {
        return Err(anyhow::anyhow!(
            "Task '{}' is already in progress.",
            task.title
        ));
    }
    tx.execute(
        UPDATE_TASK_FIELDS_SQL,
        params![
            task.title,
            task.content,
            TaskStatus::InProgress,
            task.priority,
            task.tags,
            task.assigned_agent,
            task.lease_expires_at,
            id
        ],
    )?;
    let updated = query_task_conn(&tx, id)?;
    insert_event(
        &tx,
        &updated,
        "updated",
        "human",
        Some(task.status),
        Some(TaskStatus::InProgress),
        json!({ "op": "start" }),
    )?;
    tx.commit()?;
    Ok(true)
}

pub fn done(id: i32) -> anyhow::Result<bool> {
    complete_task(id, None, None)?;
    Ok(true)
}

pub fn edit(id: i32, title: String) -> anyhow::Result<bool> {
    let title = normalize_task_title(&title)?;
    let mut conn = db::get_db()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let task = query_task_conn(&tx, id)?;
    tx.execute(
        UPDATE_TASK_FIELDS_SQL,
        params![
            title,
            task.content,
            task.status,
            task.priority,
            task.tags,
            task.assigned_agent,
            task.lease_expires_at,
            id
        ],
    )?;
    let updated = query_task_conn(&tx, id)?;
    insert_event(
        &tx,
        &updated,
        "updated",
        "human",
        Some(task.status),
        Some(task.status),
        json!({ "fields": ["title"] }),
    )?;
    tx.commit()?;
    Ok(true)
}

/// Soft delete inside a transaction, appending a `deleted` event. Returns the
/// deleted task (loaded including the soft-deleted row).
pub fn delete_task_scoped(
    id: i32,
    project_scope: Option<i32>,
    actor: &str,
) -> anyhow::Result<Task> {
    let mut conn = db::get_db()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let task = load_scoped(&tx, id, project_scope)?;
    tx.execute(UPDATE_TASK_SOFT_DELETE_SQL, (id,))?;
    let deleted = query_task_any_conn(&tx, id)?;
    insert_event(
        &tx,
        &deleted,
        "deleted",
        actor,
        Some(task.status),
        Some(task.status),
        json!({}),
    )?;
    tx.commit()?;
    Ok(deleted)
}

pub fn delete(id: i32) -> anyhow::Result<bool> {
    delete_task_scoped(id, None, "human")?;
    Ok(true)
}

// ---------------------------------------------------------------------------
// Agent-facing operations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct CreateTaskInput {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<i32>,
    pub tags: Vec<String>,
    pub idempotency_key: Option<String>,
}

/// Create a task. When `idempotency_key` already exists in the project, the
/// existing task is returned with `deduplicated = true` instead of inserting.
pub fn create_task(
    project_id: i32,
    input: CreateTaskInput,
    actor: &str,
) -> anyhow::Result<(Task, bool)> {
    let title = normalize_task_title(&input.title)?;
    let description = input.description.unwrap_or_default();
    let priority = match input.priority {
        Some(priority) => validate_priority(priority)?,
        None => 3,
    };
    let tags = normalize_tags(&input.tags);
    let idempotency_key = input
        .idempotency_key
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty());

    let conn = db::get_db()?;
    if let Some(key) = &idempotency_key {
        if let Some(id) = find_task_id_by_key(&conn, project_id, key)? {
            return Ok((query_task_conn(&conn, id)?, true));
        }
    }

    let inserted = conn.execute(
        INSERT_TASK_FULL_SQL,
        params![
            title,
            description,
            project_id,
            priority,
            tags,
            idempotency_key
        ],
    );
    match inserted {
        Ok(_) => {}
        Err(rusqlite::Error::SqliteFailure(error, _))
            if error.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            // Lost a race with the same idempotency key: return the winner.
            if let Some(key) = &idempotency_key {
                if let Some(id) = find_task_id_by_key(&conn, project_id, key)? {
                    return Ok((query_task_conn(&conn, id)?, true));
                }
            }
            return Err(anyhow::anyhow!("failed to create task: {}", error));
        }
        Err(error) => return Err(anyhow::anyhow!("failed to create task: {}", error)),
    }

    let id = conn.last_insert_rowid() as i32;
    let task = query_task_conn(&conn, id)?;
    insert_event(
        &conn,
        &task,
        "created",
        actor,
        None,
        Some(TaskStatus::Todo),
        json!({ "priority": task.priority, "tags": task.tags }),
    )?;
    Ok((task, false))
}

fn find_task_id_by_key(
    conn: &Connection,
    project_id: i32,
    key: &str,
) -> anyhow::Result<Option<i32>> {
    Ok(conn
        .query_row(SELECT_TASK_BY_IDEMPOTENCY_SQL, (project_id, key), |row| {
            row.get(0)
        })
        .optional()?)
}

/// `--assigned` filter for `task list`.
#[derive(Debug, Clone)]
pub enum AssignedFilter {
    /// `--assigned me` resolved to a concrete agent id.
    Agent(String),
    /// `--assigned unassigned`
    Unassigned,
}

#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
    pub status: Option<TaskStatus>,
    pub assigned: Option<AssignedFilter>,
    pub priority: Option<i32>,
    pub tags: Vec<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
}

pub fn list_filtered(project_id: i32, filter: &TaskFilter) -> anyhow::Result<(Vec<Task>, bool)> {
    let mut tasks = fetch_tasks(project_id)?;

    if let Some(status) = filter.status {
        tasks.retain(|task| task.status == status);
    }
    match &filter.assigned {
        Some(AssignedFilter::Agent(agent)) => {
            tasks.retain(|task| task.assigned_agent.as_deref() == Some(agent.as_str()));
        }
        Some(AssignedFilter::Unassigned) => {
            tasks.retain(|task| task.assigned_agent.is_none());
        }
        None => {}
    }
    if let Some(priority) = filter.priority {
        let priority = validate_priority(priority)?;
        tasks.retain(|task| task.priority == priority);
    }
    if !filter.tags.is_empty() {
        let wanted = filter
            .tags
            .iter()
            .map(|tag| tag.trim().to_lowercase())
            .filter(|tag| !tag.is_empty())
            .collect::<Vec<_>>();
        tasks.retain(|task| {
            let task_tags = tags_of(task);
            wanted.iter().all(|wanted| task_tags.contains(wanted))
        });
    }
    if let Some(search) = &filter.search {
        let needle = search.trim().to_lowercase();
        if !needle.is_empty() {
            tasks.retain(|task| {
                task.title.to_lowercase().contains(&needle)
                    || task.content.to_lowercase().contains(&needle)
            });
        }
    }

    let limit = filter.limit.unwrap_or(100).clamp(1, 1000);
    let truncated = tasks.len() > limit;
    tasks.truncate(limit);
    Ok((tasks, truncated))
}

pub fn get_task(id: i32, project_scope: Option<i32>) -> anyhow::Result<Task> {
    let conn = db::get_db()?;
    load_scoped(&conn, id, project_scope)
}

#[derive(Debug, Clone, Default)]
pub struct TaskUpdateInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<i32>,
    pub tags: Option<Vec<String>>,
}

impl TaskUpdateInput {
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.status.is_none()
            && self.priority.is_none()
            && self.tags.is_none()
    }

    fn changed_fields(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.title.is_some() {
            fields.push("title");
        }
        if self.description.is_some() {
            fields.push("description");
        }
        if self.status.is_some() {
            fields.push("status");
        }
        if self.priority.is_some() {
            fields.push("priority");
        }
        if self.tags.is_some() {
            fields.push("tags");
        }
        fields
    }
}

pub fn update_task(
    id: i32,
    project_scope: Option<i32>,
    input: TaskUpdateInput,
    actor: &str,
) -> anyhow::Result<Task> {
    if input.is_empty() {
        return Err(agent::to_anyhow(AgentError::invalid(
            "task update requires at least one of --title/--description/--status/--priority/--tags",
        )));
    }

    let mut conn = db::get_db()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let task = load_scoped(&tx, id, project_scope)?;

    let title = match &input.title {
        Some(title) => normalize_task_title(title)?,
        None => task.title.clone(),
    };
    let content = input
        .description
        .clone()
        .unwrap_or_else(|| task.content.clone());
    let status = input.status.unwrap_or(task.status);
    let priority = match input.priority {
        Some(priority) => validate_priority(priority)?,
        None => task.priority,
    };
    let tags = match &input.tags {
        Some(tags) => normalize_tags(tags),
        None => task.tags.clone(),
    };
    // Lease integrity: any agent write against another agent's valid lease is a
    // conflict, not just `--status done`. Humans (no agent identity) may still
    // operate on a leased task. The check runs before any mutation, so a
    // conflict leaves the row (and revision) untouched.
    let now = Utc::now().timestamp();
    if actor != "human"
        && is_lease_valid(&task, now)
        && task.assigned_agent.as_deref() != Some(actor)
    {
        return Err(lease_conflict(id, &task));
    }
    let becomes_done = status == TaskStatus::Done;

    // D3: todo/done are unleased states; only in-progress keeps the lease.
    let (assigned_agent, lease_expires_at) = match status {
        TaskStatus::Todo | TaskStatus::Done => (None, None),
        TaskStatus::InProgress => (task.assigned_agent.clone(), task.lease_expires_at),
    };
    let fields = input.changed_fields();
    let event = if task.status != TaskStatus::Done && becomes_done {
        "completed"
    } else {
        "updated"
    };

    tx.execute(
        UPDATE_TASK_FIELDS_SQL,
        params![
            title,
            content,
            status,
            priority,
            tags,
            assigned_agent,
            lease_expires_at,
            id
        ],
    )?;
    let updated = query_task_conn(&tx, id)?;
    insert_event(
        &tx,
        &updated,
        event,
        actor,
        Some(task.status),
        Some(status),
        json!({ "fields": fields }),
    )?;
    tx.commit()?;
    Ok(updated)
}

fn lease_conflict(id: i32, task: &Task) -> anyhow::Error {
    let expires = agent::timestamp_opt(task.lease_expires_at).unwrap_or_default();
    agent::to_anyhow(
        AgentError::conflict(format!(
            "task {} is leased by {} until {}",
            id,
            task.assigned_agent.clone().unwrap_or_default(),
            expires
        ))
        .with_details(json!({
            "task_id": id,
            "assigned_agent": task.assigned_agent,
            "lease_expires_at": agent::timestamp_opt(task.lease_expires_at),
        })),
    )
}

pub fn claim_task(
    id: i32,
    project_scope: Option<i32>,
    agent: &str,
    lease_seconds: i64,
    force: bool,
) -> anyhow::Result<Task> {
    let lease_seconds = validate_lease(lease_seconds)?;
    let mut conn = db::get_db()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let task = load_scoped(&tx, id, project_scope)?;
    if task.status == TaskStatus::Done {
        return Err(conflict(format!(
            "task {} is done and cannot be claimed",
            id
        )));
    }

    let now = Utc::now().timestamp();
    if is_lease_valid(&task, now) && !force && task.assigned_agent.as_deref() != Some(agent) {
        return Err(lease_conflict(id, &task));
    }

    let expires_at = now + lease_seconds;
    let previous_agent = task.assigned_agent.clone();
    tx.execute(UPDATE_TASK_CLAIM_SQL, params![agent, expires_at, id])?;
    let updated = query_task_conn(&tx, id)?;
    insert_event(
        &tx,
        &updated,
        "claimed",
        agent,
        Some(task.status),
        Some(TaskStatus::InProgress),
        json!({ "lease_seconds": lease_seconds, "force": force, "previous_agent": previous_agent }),
    )?;
    tx.commit()?;
    Ok(updated)
}

pub fn heartbeat_task(
    id: i32,
    project_scope: Option<i32>,
    agent: &str,
    lease_seconds: i64,
) -> anyhow::Result<Task> {
    let lease_seconds = validate_lease(lease_seconds)?;
    let mut conn = db::get_db()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let task = load_scoped(&tx, id, project_scope)?;
    if task.status == TaskStatus::Done {
        return Err(conflict(format!("task {} is done", id)));
    }
    if task.assigned_agent.as_deref() != Some(agent) {
        return Err(conflict(format!("task {} is not leased by {}", id, agent)));
    }

    let expires_at = Utc::now().timestamp() + lease_seconds;
    tx.execute(UPDATE_TASK_HEARTBEAT_SQL, params![expires_at, id])?;
    let updated = query_task_conn(&tx, id)?;
    insert_event(
        &tx,
        &updated,
        "heartbeat",
        agent,
        Some(task.status),
        Some(task.status),
        json!({ "lease_seconds": lease_seconds, "lease_expires_at": agent::timestamp_opt(Some(expires_at)) }),
    )?;
    tx.commit()?;
    Ok(updated)
}

pub fn release_task(
    id: i32,
    project_scope: Option<i32>,
    agent: Option<&str>,
    force: bool,
) -> anyhow::Result<Task> {
    let mut conn = db::get_db()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let task = load_scoped(&tx, id, project_scope)?;

    let holder = task
        .assigned_agent
        .clone()
        .ok_or_else(|| conflict(format!("task {} is not leased", id)))?;
    if !force && agent != Some(holder.as_str()) {
        return Err(lease_conflict(id, &task));
    }

    let actor = agent.unwrap_or("human").to_string();
    tx.execute(UPDATE_TASK_RELEASE_SQL, (id,))?;
    let updated = query_task_conn(&tx, id)?;
    insert_event(
        &tx,
        &updated,
        "released",
        &actor,
        Some(task.status),
        Some(updated.status),
        json!({ "force": force, "previous_agent": holder }),
    )?;
    tx.commit()?;
    Ok(updated)
}

/// Complete a task. Human callers (`agent = None`) are allowed even when the
/// task is leased; agents are refused while another agent holds a valid lease.
pub fn complete_task(
    id: i32,
    project_scope: Option<i32>,
    agent: Option<&str>,
) -> anyhow::Result<Task> {
    let mut conn = db::get_db()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let task = load_scoped(&tx, id, project_scope)?;

    if task.status == TaskStatus::Done {
        return Err(conflict(format!("Task '{}' is already done.", task.title)));
    }
    let now = Utc::now().timestamp();
    if let Some(agent) = agent {
        if is_lease_valid(&task, now) && task.assigned_agent.as_deref() != Some(agent) {
            return Err(lease_conflict(id, &task));
        }
    }

    let actor = agent.unwrap_or("human").to_string();
    tx.execute(UPDATE_TASK_COMPLETE_SQL, (id,))?;
    let updated = query_task_conn(&tx, id)?;
    insert_event(
        &tx,
        &updated,
        "completed",
        &actor,
        Some(task.status),
        Some(TaskStatus::Done),
        json!({}),
    )?;
    tx.commit()?;
    Ok(updated)
}

pub fn next_task(
    project_id: i32,
    agent: Option<&str>,
    claim: bool,
    lease_seconds: i64,
) -> anyhow::Result<Option<Task>> {
    if claim && agent.is_none() {
        return Err(agent::to_anyhow(AgentError::missing_agent(
            "claiming the next task requires --agent (or TAM_AGENT)",
        )));
    }
    let conn = db::get_db()?;
    let now = Utc::now().timestamp();
    let candidate: Option<i32> = conn
        .query_row(SELECT_NEXT_TASK_SQL, params![project_id, now], |row| {
            row.get(0)
        })
        .optional()?;
    match candidate {
        None => Ok(None),
        Some(id) => {
            if claim {
                claim_task(
                    id,
                    Some(project_id),
                    agent.unwrap_or_default(),
                    lease_seconds,
                    false,
                )
                .map(Some)
            } else {
                query_task_conn(&conn, id).map(Some)
            }
        }
    }
}

pub fn task_events(id: i32, project_scope: Option<i32>) -> anyhow::Result<Vec<TaskEvent>> {
    let conn = db::get_db()?;
    let task = load_scoped(&conn, id, project_scope)?;
    let mut stmt = conn.prepare(SELECT_TASK_EVENTS_SQL)?;
    let events = stmt
        .query_map((task.id.unwrap_or(id),), |row| {
            let created: String = row.get(8)?;
            let created = parse_db_datetime(&created).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    8,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e.to_string(),
                    )),
                )
            })?;
            Ok(TaskEvent {
                id: row.get(0)?,
                task_id: row.get(1)?,
                project_id: row.get(2)?,
                event: row.get(3)?,
                actor: row.get(4)?,
                from_status: row.get(5)?,
                to_status: row.get(6)?,
                metadata: row.get(7)?,
                created,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(events)
}

#[derive(Debug, Clone)]
pub struct StatusSummary {
    pub todo: i64,
    pub in_progress: i64,
    pub done: i64,
    pub leased: i64,
    pub unassigned: i64,
    pub next: Vec<Task>,
}

pub fn status_summary(project_id: i32) -> anyhow::Result<StatusSummary> {
    let tasks = fetch_tasks(project_id)?;
    let now = Utc::now().timestamp();
    let mut summary = StatusSummary {
        todo: 0,
        in_progress: 0,
        done: 0,
        leased: 0,
        unassigned: 0,
        next: Vec::new(),
    };
    let mut candidates = Vec::new();
    for task in tasks {
        match task.status {
            TaskStatus::Todo => summary.todo += 1,
            TaskStatus::InProgress => summary.in_progress += 1,
            TaskStatus::Done => {
                summary.done += 1;
                continue;
            }
        }
        if is_lease_valid(&task, now) {
            summary.leased += 1;
        } else {
            summary.unassigned += 1;
            candidates.push(task);
        }
    }
    candidates.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.created.cmp(&b.created))
            .then_with(|| a.id.cmp(&b.id))
    });
    candidates.truncate(10);
    summary.next = candidates;
    Ok(summary)
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn sqlite_timestamp_parsed_as_utc() {
        let parsed = parse_db_datetime("2026-11-08 07:14:54").expect("valid SQLite timestamp");
        assert_eq!(
            parsed,
            Utc.with_ymd_and_hms(2026, 11, 8, 7, 14, 54).unwrap()
        );
        assert_eq!(parsed.to_rfc3339(), "2026-11-08T07:14:54+00:00");
    }

    #[test]
    fn task_title_normalization_trims_and_rejects_blank() {
        assert_eq!(normalize_task_title("  do it  ").unwrap(), "do it");
        assert!(normalize_task_title("").is_err());
        assert!(normalize_task_title("   ").is_err());
    }

    #[test]
    fn sqlite_timestamp_invalid_is_err() {
        assert!(parse_db_datetime("not-a-timestamp").is_err());
        assert!(parse_db_datetime("2026-11-08T07:14:54").is_err());
        assert!(parse_db_datetime("").is_err());
    }

    #[test]
    fn tags_are_normalized_and_deduplicated() {
        let tags = vec![
            " Bug ".to_string(),
            "backend".to_string(),
            "bug".to_string(),
            "".to_string(),
        ];
        assert_eq!(normalize_tags(&tags), "bug,backend");
        let task = Task {
            tags: "bug,backend".to_string(),
            ..Task::default()
        };
        assert_eq!(
            tags_of(&task),
            vec!["bug".to_string(), "backend".to_string()]
        );
    }

    #[test]
    fn lease_bounds_are_validated() {
        assert!(validate_lease(5).is_ok());
        assert!(validate_lease(900).is_ok());
        assert!(validate_lease(86_400).is_ok());
        assert!(validate_lease(4).is_err());
        assert!(validate_lease(86_401).is_err());
    }
}
