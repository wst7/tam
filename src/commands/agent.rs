//! Agent command operations shared by the JSON CLI (`--json`) and the MCP
//! server. Human command handlers keep using the service layer directly.

use serde_json::{json, Value};

use crate::agent::{self, AgentError, Ctx, EventView, ProjectView, TaskView};
use crate::cli::{Commands, ProjectSubcommand, StatusArg, TaskSubcommand};
use crate::model::{Project, TaskStatus};
use crate::services::{project, task};

/// One command result: the JSON `data` body plus the project it resolved to
/// (used for the envelope `meta.project` field).
pub struct Outcome {
    pub data: Value,
    pub project: Option<String>,
}

impl Outcome {
    fn new(data: Value, project: Option<&Project>) -> Self {
        Self {
            data,
            project: project.map(|project| project.name.clone()),
        }
    }
}

pub fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::Project { command } => match command {
            None | Some(ProjectSubcommand::List) => "project.list",
            Some(ProjectSubcommand::Add { .. }) => "project.add",
            Some(ProjectSubcommand::Use { .. }) => "project.use",
            Some(ProjectSubcommand::Del { .. }) => "project.del",
            Some(ProjectSubcommand::Current) => "project.current",
            Some(ProjectSubcommand::Edit { .. }) => "project.edit",
        },
        Commands::Task { command } => match command {
            None | Some(TaskSubcommand::List { .. }) => "task.list",
            Some(TaskSubcommand::Add { .. }) => "task.add",
            Some(TaskSubcommand::Edit { .. }) => "task.edit",
            Some(TaskSubcommand::Delete { .. }) => "task.del",
            Some(TaskSubcommand::Start { .. }) => "task.start",
            Some(TaskSubcommand::Done { .. }) => "task.done",
            Some(TaskSubcommand::Show { .. }) => "task.show",
            Some(TaskSubcommand::Update { .. }) => "task.update",
            Some(TaskSubcommand::Claim { .. }) => "task.claim",
            Some(TaskSubcommand::Heartbeat { .. }) => "task.heartbeat",
            Some(TaskSubcommand::Release { .. }) => "task.release",
            Some(TaskSubcommand::Next { .. }) => "task.next",
            Some(TaskSubcommand::Events { .. }) => "task.events",
        },
        Commands::Status => "status",
        Commands::Mcp => "mcp",
        Commands::Config => "config",
    }
}

// ---------------------------------------------------------------------------
// Context resolution
// ---------------------------------------------------------------------------

pub fn resolve_project(ctx: &Ctx) -> Result<Project, AgentError> {
    match &ctx.project {
        Some(name) => project::query_project(name.clone()).map_err(agent::classify),
        None => project::get_current_project().map_err(|error| {
            if error.to_string().contains("No current project set") {
                AgentError::missing_project(error.to_string())
            } else {
                agent::classify(error)
            }
        }),
    }
}

/// Human commands also honour the global `--project` override.
pub fn resolve_project_for_human(ctx: &Ctx) -> anyhow::Result<Project> {
    match &ctx.project {
        Some(name) => project::query_project(name.clone()),
        None => project::get_current_project(),
    }
}

/// When `--project` is explicit, by-id operations must stay inside it.
fn project_scope(ctx: &Ctx) -> Result<Option<i32>, AgentError> {
    if ctx.project.is_some() {
        let project = resolve_project(ctx)?;
        Ok(project.id)
    } else {
        Ok(None)
    }
}

fn require_agent(ctx: &Ctx) -> Result<String, AgentError> {
    ctx.agent.clone().ok_or_else(|| {
        AgentError::missing_agent("this command requires an agent identity (--agent or TAM_AGENT)")
    })
}

fn view(task: &crate::model::Task, now: i64) -> Value {
    serde_json::to_value(TaskView::from_task(task, now)).unwrap_or(Value::Null)
}

/// Outcome for a by-id command: fill `meta.project` from the task itself.
fn task_outcome(task: &crate::model::Task, now: i64) -> Outcome {
    Outcome {
        data: json!({ "task": view(task, now) }),
        project: Some(task.project.clone()),
    }
}

// ---------------------------------------------------------------------------
// Read operations
// ---------------------------------------------------------------------------

pub fn op_project_list() -> Result<Outcome, AgentError> {
    let projects = project::list().map_err(agent::classify)?;
    let data = Value::Array(
        projects
            .iter()
            .map(|project| serde_json::to_value(ProjectView::from_project(project)).unwrap())
            .collect(),
    );
    Ok(Outcome {
        data,
        project: None,
    })
}

pub fn op_project_current() -> Result<Outcome, AgentError> {
    match project::get_current_project() {
        Ok(project) => Ok(Outcome::new(
            json!({ "project": serde_json::to_value(ProjectView::from_project(&project)).unwrap_or(Value::Null) }),
            Some(&project),
        )),
        Err(error) => {
            if error.to_string().contains("No current project set") {
                Ok(Outcome {
                    data: json!({ "project": null }),
                    project: None,
                })
            } else {
                Err(agent::classify(error))
            }
        }
    }
}

pub fn op_project_add(name: String) -> Result<Outcome, AgentError> {
    let (project, restored) = project::add(name).map_err(agent::classify)?;
    // `project::add` returns a stub without the row id; reload for the contract.
    let project = project::query_project(project.name).map_err(agent::classify)?;
    let data = json!({
        "project": serde_json::to_value(ProjectView::from_project(&project)).unwrap_or(Value::Null),
        "restored": restored,
    });
    Ok(Outcome::new(data, Some(&project)))
}

pub fn op_status(ctx: &Ctx) -> Result<Outcome, AgentError> {
    let project = resolve_project(ctx)?;
    let project_id = project.id.unwrap_or_default();
    let summary = task::status_summary(project_id).map_err(agent::classify)?;
    let now = chrono::Utc::now().timestamp();
    let data = agent::status_body(
        &project,
        (summary.todo, summary.in_progress, summary.done),
        summary.leased,
        summary.unassigned,
        &summary.next,
        now,
    );
    Ok(Outcome::new(data, Some(&project)))
}

#[derive(Debug, Clone, Default)]
pub struct ListFilters {
    pub status: Option<StatusArg>,
    pub assigned: Option<String>,
    pub priority: Option<i32>,
    pub tags: Vec<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
}

fn build_filter(ctx: &Ctx, filters: &ListFilters) -> Result<task::TaskFilter, AgentError> {
    let assigned = match filters.assigned.as_deref() {
        None => None,
        Some("unassigned") => Some(task::AssignedFilter::Unassigned),
        Some("me") => Some(task::AssignedFilter::Agent(require_agent(ctx)?)),
        Some(agent) => Some(task::AssignedFilter::Agent(agent.to_string())),
    };
    Ok(task::TaskFilter {
        status: filters.status.map(TaskStatus::from),
        assigned,
        priority: filters.priority,
        tags: filters.tags.clone(),
        search: filters.search.clone(),
        limit: filters.limit,
    })
}

pub fn op_task_list(ctx: &Ctx, filters: &ListFilters) -> Result<Outcome, AgentError> {
    let project = resolve_project(ctx)?;
    let filter = build_filter(ctx, filters)?;
    let (tasks, truncated) =
        task::list_filtered(project.id.unwrap_or_default(), &filter).map_err(agent::classify)?;
    let now = chrono::Utc::now().timestamp();
    let data = json!({
        "tasks": tasks.iter().map(|task| view(task, now)).collect::<Vec<_>>(),
        "count": tasks.len(),
        "limit": filters.limit.unwrap_or(100).clamp(1, 1000),
        "truncated": truncated,
    });
    Ok(Outcome::new(data, Some(&project)))
}

pub fn op_task_show(ctx: &Ctx, id: i32) -> Result<Outcome, AgentError> {
    let scope = project_scope(ctx)?;
    let task = task::get_task(id, scope).map_err(agent::classify)?;
    let events = task::task_events(id, scope).map_err(agent::classify)?;
    let now = chrono::Utc::now().timestamp();
    let data = json!({
        "task": view(&task, now),
        "events": events.iter().map(|event| serde_json::to_value(EventView::from_event(event)).unwrap()).collect::<Vec<_>>(),
    });
    Ok(Outcome {
        data,
        project: Some(task.project),
    })
}

pub fn op_task_events(ctx: &Ctx, id: i32) -> Result<Outcome, AgentError> {
    let scope = project_scope(ctx)?;
    let task_row = task::get_task(id, scope).map_err(agent::classify)?;
    let events = task::task_events(id, scope).map_err(agent::classify)?;
    let data = json!({
        "events": events.iter().map(|event| serde_json::to_value(EventView::from_event(event)).unwrap()).collect::<Vec<_>>(),
    });
    Ok(Outcome {
        data,
        project: Some(task_row.project),
    })
}

/// `task del --json`: soft delete + `deleted` event.
pub fn op_task_delete(ctx: &Ctx, id: i32) -> Result<Outcome, AgentError> {
    let scope = project_scope(ctx)?;
    let actor = ctx.agent.clone().unwrap_or_else(|| "human".to_string());
    let task = task::delete_task_scoped(id, scope, &actor).map_err(agent::classify)?;
    let now = chrono::Utc::now().timestamp();
    Ok(task_outcome(&task, now))
}

// ---------------------------------------------------------------------------
// Write operations
// ---------------------------------------------------------------------------

pub fn op_task_add(
    ctx: &Ctx,
    title: String,
    description: Option<String>,
    priority: Option<i32>,
    tags: Vec<String>,
    idempotency_key: Option<String>,
) -> Result<Outcome, AgentError> {
    let project = resolve_project(ctx)?;
    let actor = ctx.agent.clone().unwrap_or_else(|| "human".to_string());
    let (task, deduplicated) = task::create_task(
        project.id.unwrap_or_default(),
        task::CreateTaskInput {
            title,
            description,
            priority,
            tags,
            idempotency_key,
        },
        &actor,
    )
    .map_err(agent::classify)?;
    let now = chrono::Utc::now().timestamp();
    let data = json!({ "task": view(&task, now), "deduplicated": deduplicated });
    Ok(Outcome::new(data, Some(&project)))
}

pub fn op_task_update(
    ctx: &Ctx,
    id: i32,
    input: task::TaskUpdateInput,
) -> Result<Outcome, AgentError> {
    let scope = project_scope(ctx)?;
    let actor = ctx.agent.clone().unwrap_or_else(|| "human".to_string());
    let task = task::update_task(id, scope, input, &actor).map_err(agent::classify)?;
    let now = chrono::Utc::now().timestamp();
    Ok(task_outcome(&task, now))
}

pub fn op_task_claim(ctx: &Ctx, id: i32, lease: i64, force: bool) -> Result<Outcome, AgentError> {
    let agent = require_agent(ctx)?;
    let scope = project_scope(ctx)?;
    let task = task::claim_task(id, scope, &agent, lease, force).map_err(agent::classify)?;
    let now = chrono::Utc::now().timestamp();
    Ok(task_outcome(&task, now))
}

pub fn op_task_heartbeat(ctx: &Ctx, id: i32, lease: i64) -> Result<Outcome, AgentError> {
    let agent = require_agent(ctx)?;
    let scope = project_scope(ctx)?;
    let task = task::heartbeat_task(id, scope, &agent, lease).map_err(agent::classify)?;
    let now = chrono::Utc::now().timestamp();
    Ok(task_outcome(&task, now))
}

pub fn op_task_release(ctx: &Ctx, id: i32, force: bool) -> Result<Outcome, AgentError> {
    let scope = project_scope(ctx)?;
    let agent = ctx.agent.as_deref();
    let task = task::release_task(id, scope, agent, force).map_err(agent::classify)?;
    let now = chrono::Utc::now().timestamp();
    Ok(task_outcome(&task, now))
}

pub fn op_task_done(ctx: &Ctx, id: i32) -> Result<Outcome, AgentError> {
    let scope = project_scope(ctx)?;
    let agent = ctx.agent.as_deref();
    let task = task::complete_task(id, scope, agent).map_err(agent::classify)?;
    let now = chrono::Utc::now().timestamp();
    Ok(task_outcome(&task, now))
}

pub fn op_task_next(ctx: &Ctx, claim: bool, lease: i64) -> Result<Outcome, AgentError> {
    let project = resolve_project(ctx)?;
    let agent = ctx.agent.as_deref();
    if claim && agent.is_none() {
        return Err(AgentError::missing_agent(
            "claiming the next task requires --agent (or TAM_AGENT)",
        ));
    }
    let next = task::next_task(project.id.unwrap_or_default(), agent, claim, lease)
        .map_err(agent::classify)?;
    let now = chrono::Utc::now().timestamp();
    let data = match next {
        Some(task) => json!({ "task": view(&task, now) }),
        None => json!({ "task": null }),
    };
    Ok(Outcome::new(data, Some(&project)))
}

// ---------------------------------------------------------------------------
// JSON CLI dispatch
// ---------------------------------------------------------------------------

pub fn dispatch_json(command: &Commands, ctx: &Ctx) -> Result<Outcome, AgentError> {
    match command {
        Commands::Project { command } => match command {
            None | Some(ProjectSubcommand::List) => op_project_list(),
            Some(ProjectSubcommand::Add { name }) => op_project_add(name.clone()),
            Some(ProjectSubcommand::Current) => op_project_current(),
            Some(other) => Err(AgentError::invalid(format!(
                "JSON output is not supported for command '{}'",
                command_name(&Commands::Project {
                    command: Some(other.clone())
                })
            ))),
        },
        Commands::Task { command } => match command {
            None => op_task_list(ctx, &ListFilters::default()),
            Some(TaskSubcommand::Add {
                title,
                description,
                priority,
                tag,
                idempotency_key,
            }) => op_task_add(
                ctx,
                title.clone(),
                description.clone(),
                *priority,
                tag.clone(),
                idempotency_key.clone(),
            ),
            Some(TaskSubcommand::List {
                status,
                assigned,
                priority,
                tag,
                search,
                limit,
            }) => op_task_list(
                ctx,
                &ListFilters {
                    status: *status,
                    assigned: assigned.clone(),
                    priority: *priority,
                    tags: tag.clone(),
                    search: search.clone(),
                    limit: *limit,
                },
            ),
            Some(TaskSubcommand::Show { id }) => op_task_show(ctx, *id),
            Some(TaskSubcommand::Events { id }) => op_task_events(ctx, *id),
            Some(TaskSubcommand::Update {
                id,
                title,
                description,
                status,
                priority,
                tags,
            }) => op_task_update(
                ctx,
                *id,
                task::TaskUpdateInput {
                    title: title.clone(),
                    description: description.clone(),
                    status: status.map(TaskStatus::from),
                    priority: *priority,
                    tags: tags
                        .clone()
                        .map(|tags| tags.split(',').map(|tag| tag.to_string()).collect()),
                },
            ),
            Some(TaskSubcommand::Claim { id, lease, force }) => {
                op_task_claim(ctx, *id, *lease, *force)
            }
            Some(TaskSubcommand::Heartbeat { id, lease }) => op_task_heartbeat(ctx, *id, *lease),
            Some(TaskSubcommand::Release { id, force }) => op_task_release(ctx, *id, *force),
            Some(TaskSubcommand::Done { id }) => op_task_done(ctx, *id),
            Some(TaskSubcommand::Delete { id }) => op_task_delete(ctx, *id),
            Some(TaskSubcommand::Next { claim, lease }) => op_task_next(ctx, *claim, *lease),
            Some(other) => Err(AgentError::invalid(format!(
                "JSON output is not supported for command '{}'",
                command_name(&Commands::Task {
                    command: Some(other.clone())
                })
            ))),
        },
        Commands::Status => op_status(ctx),
        Commands::Mcp => Err(AgentError::invalid("the mcp server does not use --json")),
        Commands::Config => Err(AgentError::invalid(
            "JSON output is not supported for command 'config'",
        )),
    }
}
