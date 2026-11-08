//! Agent-facing contract primitives: JSON envelope, typed errors, execution
//! context (project/agent resolution) and serializable views.
//!
//! The command implementations live in [`crate::commands::agent`]; this module
//! deliberately contains no database access so it can be used by both the CLI
//! and the MCP server.

use chrono::{DateTime, SecondsFormat, TimeZone, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use std::fmt;

use crate::model::{Project, Task, TaskEvent};

pub const ENVELOPE_SCHEMA: &str = "tam.agent/v1";

/// Exit codes from docs/agent-protocol.md §3.2.
pub const EXIT_INVALID: i32 = 1;
pub const EXIT_NOT_FOUND: i32 = 3;
pub const EXIT_CONFLICT: i32 = 4;

/// A business error carrying a stable machine-readable code plus the process
/// exit code mandated by the protocol.
#[derive(Debug, Clone)]
pub struct AgentError {
    pub code: &'static str,
    pub message: String,
    pub details: Option<Value>,
}

impl AgentError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid", message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new("conflict", message)
    }

    pub fn missing_project(message: impl Into<String>) -> Self {
        Self::new("missing_project", message)
    }

    pub fn missing_agent(message: impl Into<String>) -> Self {
        Self::new("missing_agent", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }

    pub fn exit_code(&self) -> i32 {
        match self.code {
            "not_found" => EXIT_NOT_FOUND,
            "conflict" => EXIT_CONFLICT,
            _ => EXIT_INVALID,
        }
    }

    pub fn to_json(&self) -> Value {
        let mut error = json!({ "code": self.code, "message": self.message });
        if let Some(details) = &self.details {
            error["details"] = details.clone();
        }
        error
    }
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AgentError {}

/// Convert an `anyhow::Error` into a protocol error, preserving a typed
/// [`AgentError`] when the error chain carries one.
pub fn classify(err: anyhow::Error) -> AgentError {
    match err.downcast::<AgentError>() {
        Ok(agent_error) => agent_error,
        Err(other) => {
            let message = other.to_string();
            // Safety net for legacy `anyhow` errors from the human service path.
            if message.contains("No current project set") {
                AgentError::missing_project(message)
            } else if message.contains("not found") {
                AgentError::not_found(message)
            } else {
                AgentError::internal(message)
            }
        }
    }
}

/// Wrap a typed error into `anyhow` so service layers can still return
/// `anyhow::Result` and the CLI can classify it by downcasting.
pub fn to_anyhow(err: AgentError) -> anyhow::Error {
    anyhow::Error::new(err)
}

/// Execution context: how to talk to the world for this invocation.
#[derive(Debug, Clone, Default)]
pub struct Ctx {
    pub json: bool,
    /// `--project`, else `TAM_PROJECT`.
    pub project: Option<String>,
    /// `--agent`, else `TAM_AGENT`. `None` means a human caller.
    pub agent: Option<String>,
}

impl Ctx {
    pub fn new(json: bool, project: Option<String>, agent: Option<String>) -> Self {
        Self {
            json,
            project: normalize_optional(project).or_else(|| normalize_optional(tam_project_env())),
            agent: normalize_optional(agent).or_else(|| normalize_optional(tam_agent_env())),
        }
    }
}

fn tam_project_env() -> Option<String> {
    std::env::var("TAM_PROJECT").ok()
}

fn tam_agent_env() -> Option<String> {
    std::env::var("TAM_AGENT").ok()
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

// ---------------------------------------------------------------------------
// Views (docs/agent-protocol.md §3.3)
// ---------------------------------------------------------------------------

pub fn timestamp(value: &DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn timestamp_opt(seconds: Option<i64>) -> Option<String> {
    seconds.and_then(|seconds| {
        Utc.timestamp_opt(seconds, 0)
            .single()
            .map(|value| value.to_rfc3339_opts(SecondsFormat::Secs, true))
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskView {
    pub id: i32,
    pub project_id: i32,
    pub project: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: i32,
    pub tags: Vec<String>,
    pub assigned_agent: Option<String>,
    pub lease_expires_at: Option<String>,
    pub lease_expired: bool,
    pub revision: i32,
    pub created: String,
    pub updated: String,
}

impl TaskView {
    pub fn from_task(task: &Task, now: i64) -> Self {
        let tags = if task.tags.is_empty() {
            Vec::new()
        } else {
            task.tags.split(',').map(|tag| tag.to_string()).collect()
        };
        // Only a task that actually has an assigned holder can have an expired
        // lease; unassigned tasks report `false`.
        let lease_expired = match (task.assigned_agent.as_ref(), task.lease_expires_at) {
            (Some(_), Some(expires)) => expires <= now,
            _ => false,
        };
        Self {
            id: task.id.unwrap_or_default(),
            project_id: task.project_id,
            project: task.project.clone(),
            title: task.title.clone(),
            description: task.content.clone(),
            status: task.status.as_str().to_string(),
            priority: task.priority,
            tags,
            assigned_agent: task.assigned_agent.clone(),
            lease_expires_at: timestamp_opt(task.lease_expires_at),
            lease_expired,
            revision: task.revision,
            created: timestamp(&task.created),
            updated: timestamp(&task.updated),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EventView {
    pub id: i32,
    pub task_id: i32,
    pub project_id: i32,
    pub event: String,
    pub actor: String,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub metadata: Value,
    pub created: String,
}

impl EventView {
    pub fn from_event(event: &TaskEvent) -> Self {
        Self {
            id: event.id,
            task_id: event.task_id,
            project_id: event.project_id,
            event: event.event.clone(),
            actor: event.actor.clone(),
            from_status: event.from_status.map(|status| status.as_str().to_string()),
            to_status: event.to_status.map(|status| status.as_str().to_string()),
            metadata: serde_json::from_str(&event.metadata).unwrap_or_else(|_| json!({})),
            created: timestamp(&event.created),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectView {
    pub id: i32,
    pub name: String,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl ProjectView {
    pub fn from_project(project: &Project) -> Self {
        Self {
            id: project.id.unwrap_or_default(),
            name: project.name.clone(),
            created: project.created.as_ref().map(timestamp),
            updated: project.updated.as_ref().map(timestamp),
        }
    }
}

/// Minimal `{id,name}` project reference used by `project current`.
pub fn project_ref(project: &Project) -> Value {
    json!({ "id": project.id.unwrap_or_default(), "name": project.name })
}

/// `{todo,in_progress,done,leased,unassigned,next}` body for `tam status`.
pub fn status_body(
    project: &Project,
    counts: (i64, i64, i64),
    leased: i64,
    unassigned: i64,
    next: &[Task],
    now: i64,
) -> Value {
    json!({
        "project": project_ref(project),
        "counts": {
            "todo": counts.0,
            "in_progress": counts.1,
            "done": counts.2,
        },
        "leased": leased,
        "unassigned": unassigned,
        "next": next.iter().map(|task| TaskView::from_task(task, now)).collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------------
// Envelope output
// ---------------------------------------------------------------------------

pub fn success_envelope(command: &str, data: Value, ctx: &Ctx, project: Option<&str>) -> Value {
    json!({
        "ok": true,
        "schema": ENVELOPE_SCHEMA,
        "command": command,
        "data": data,
        "meta": meta(ctx, project),
    })
}

pub fn error_envelope(command: &str, error: &AgentError) -> Value {
    json!({
        "ok": false,
        "schema": ENVELOPE_SCHEMA,
        "command": command,
        "error": error.to_json(),
    })
}

fn meta(ctx: &Ctx, project: Option<&str>) -> Value {
    json!({
        "project": project.or(ctx.project.as_deref()),
        "agent": ctx.agent.as_deref(),
    })
}

/// Emit one success envelope on stdout.
pub fn emit_success(command: &str, data: Value, ctx: &Ctx, project: Option<&str>) {
    let envelope = success_envelope(command, data, ctx, project);
    println!("{}", envelope);
}

/// Emit one failure envelope on stderr.
pub fn emit_error(command: &str, error: &AgentError) {
    eprintln!("{}", error_envelope(command, error));
}
