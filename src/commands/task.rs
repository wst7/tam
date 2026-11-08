use crate::{
    agent::Ctx,
    cli::StatusArg,
    commands::agent::resolve_project_for_human,
    model::{Task, TaskStatus},
    print_info, print_success,
    services::task::{self, AssignedFilter, CreateTaskInput, TaskFilter, TaskUpdateInput},
};

fn human_scope(ctx: &Ctx) -> anyhow::Result<Option<i32>> {
    if ctx.project.is_some() {
        Ok(resolve_project_for_human(ctx)?.id)
    } else {
        Ok(None)
    }
}

fn resolve_actor(ctx: &Ctx) -> String {
    ctx.agent.clone().unwrap_or_else(|| "human".to_string())
}

fn parse_assigned(ctx: &Ctx, assigned: Option<String>) -> anyhow::Result<Option<AssignedFilter>> {
    match assigned.as_deref() {
        None => Ok(None),
        Some("unassigned") => Ok(Some(AssignedFilter::Unassigned)),
        Some("me") => match &ctx.agent {
            Some(agent) => Ok(Some(AssignedFilter::Agent(agent.clone()))),
            None => anyhow::bail!("--assigned me requires --agent (or TAM_AGENT)"),
        },
        Some(agent) => Ok(Some(AssignedFilter::Agent(agent.to_string()))),
    }
}

pub fn add(
    ctx: &Ctx,
    title: String,
    description: Option<String>,
    priority: Option<i32>,
    tag: Vec<String>,
    idempotency_key: Option<String>,
) -> anyhow::Result<bool> {
    let project = resolve_project_for_human(ctx)?;
    let project_id = project
        .id
        .ok_or_else(|| anyhow::anyhow!("project '{}' has no id", project.name))?;
    let (task, deduplicated) = task::create_task(
        project_id,
        CreateTaskInput {
            title,
            description,
            priority,
            tags: tag,
            idempotency_key,
        },
        &resolve_actor(ctx),
    )?;
    print_success!("Task '{}' added successfully.", task.title);
    if deduplicated {
        print_info!(
            "Task {} already existed for this idempotency key.",
            task.id.unwrap_or_default()
        );
    }
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
pub fn list(
    ctx: &Ctx,
    status: Option<StatusArg>,
    assigned: Option<String>,
    priority: Option<i32>,
    tag: Vec<String>,
    search: Option<String>,
    limit: Option<usize>,
) -> anyhow::Result<bool> {
    let project = resolve_project_for_human(ctx)?;
    let project_id = project
        .id
        .ok_or_else(|| anyhow::anyhow!("project '{}' has no id", project.name))?;
    let filter = TaskFilter {
        status: status.map(TaskStatus::from),
        assigned: parse_assigned(ctx, assigned)?,
        priority,
        tags: tag,
        search,
        limit,
    };
    let (tasks, _) = task::list_filtered(project_id, &filter)?;
    if tasks.is_empty() {
        print_info!("No tasks found. Use `tam task add` to add a task.");
    } else {
        print_info!("Current project: {}", project.name);
        Task::print(&tasks);
    }
    Ok(true)
}

pub fn delete(id: i32) -> anyhow::Result<bool> {
    task::delete(id)?;
    print_success!("Task '{}' deleted successfully.", id);
    Ok(true)
}

pub fn start(id: i32) -> anyhow::Result<bool> {
    task::start(id)?;
    print_success!("Task '{}' started successfully.", id);
    Ok(true)
}

pub fn done(ctx: &Ctx, id: i32) -> anyhow::Result<bool> {
    task::complete_task(id, human_scope(ctx)?, ctx.agent.as_deref())?;
    print_success!("Task '{}' marked as done successfully.", id);
    Ok(true)
}

pub fn edit(id: i32, title: String) -> anyhow::Result<bool> {
    task::edit(id, title)?;
    print_success!("Task '{}' updated successfully.", id);
    Ok(true)
}

pub fn show(ctx: &Ctx, id: i32) -> anyhow::Result<bool> {
    let task = task::get_task(id, human_scope(ctx)?)?;
    println!("ID: {}", task.id.unwrap_or_default());
    println!("Project: {} (#{})", task.project, task.project_id);
    println!("Title: {}", task.title);
    if !task.content.is_empty() {
        println!("Description: {}", task.content);
    }
    println!("Status: {}", task.status.as_str());
    println!("Priority: {}", task.priority);
    println!("Tags: {}", task.tags);
    println!(
        "Assigned agent: {}",
        task.assigned_agent.as_deref().unwrap_or("-")
    );
    println!(
        "Lease expires: {}",
        task.lease_expires_at
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    println!("Revision: {}", task.revision);
    Ok(true)
}

pub fn update(
    ctx: &Ctx,
    id: i32,
    title: Option<String>,
    description: Option<String>,
    status: Option<StatusArg>,
    priority: Option<i32>,
    tags: Option<String>,
) -> anyhow::Result<bool> {
    let updated = task::update_task(
        id,
        human_scope(ctx)?,
        TaskUpdateInput {
            title,
            description,
            status: status.map(TaskStatus::from),
            priority,
            tags: tags.map(|tags| tags.split(',').map(|tag| tag.to_string()).collect()),
        },
        &resolve_actor(ctx),
    )?;
    print_success!("Task '{}' updated successfully.", updated.id.unwrap_or(id));
    Ok(true)
}

pub fn claim(ctx: &Ctx, id: i32, lease: i64, force: bool) -> anyhow::Result<bool> {
    let agent = ctx
        .agent
        .clone()
        .ok_or_else(|| anyhow::anyhow!("this command requires --agent (or TAM_AGENT)"))?;
    let claimed = task::claim_task(id, human_scope(ctx)?, &agent, lease, force)?;
    print_success!(
        "Task '{}' claimed by '{}' until {}.",
        claimed.id.unwrap_or(id),
        agent,
        claimed
            .lease_expires_at
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    Ok(true)
}

pub fn heartbeat(ctx: &Ctx, id: i32, lease: i64) -> anyhow::Result<bool> {
    let agent = ctx
        .agent
        .clone()
        .ok_or_else(|| anyhow::anyhow!("this command requires --agent (or TAM_AGENT)"))?;
    task::heartbeat_task(id, human_scope(ctx)?, &agent, lease)?;
    print_success!("Task '{}' lease renewed.", id);
    Ok(true)
}

pub fn release(ctx: &Ctx, id: i32, force: bool) -> anyhow::Result<bool> {
    let released = task::release_task(id, human_scope(ctx)?, ctx.agent.as_deref(), force)?;
    print_success!(
        "Task '{}' released (status: {}).",
        released.id.unwrap_or(id),
        released.status.as_str()
    );
    Ok(true)
}

pub fn next(ctx: &Ctx, claim: bool, lease: i64) -> anyhow::Result<bool> {
    let project = resolve_project_for_human(ctx)?;
    let project_id = project
        .id
        .ok_or_else(|| anyhow::anyhow!("project '{}' has no id", project.name))?;
    if claim && ctx.agent.is_none() {
        anyhow::bail!("claiming the next task requires --agent (or TAM_AGENT)");
    }
    match task::next_task(project_id, ctx.agent.as_deref(), claim, lease)? {
        Some(task) => {
            print_info!(
                "Next task: #{} {} [{}] priority {}",
                task.id.unwrap_or_default(),
                task.title,
                task.status.as_str(),
                task.priority
            );
        }
        None => print_info!("No claimable tasks."),
    }
    Ok(true)
}

pub fn events(ctx: &Ctx, id: i32) -> anyhow::Result<bool> {
    let events = task::task_events(id, human_scope(ctx)?)?;
    if events.is_empty() {
        print_info!("No events for task {}.", id);
    } else {
        for event in events {
            println!(
                "#{} {} actor={} from={} to={} at={}",
                event.id,
                event.event,
                event.actor,
                event
                    .from_status
                    .map(|status| status.as_str())
                    .unwrap_or("-"),
                event.to_status.map(|status| status.as_str()).unwrap_or("-"),
                event.created.format("%Y-%m-%d %H:%M:%S")
            );
        }
    }
    Ok(true)
}

pub fn status(ctx: &Ctx) -> anyhow::Result<bool> {
    let project = resolve_project_for_human(ctx)?;
    let project_id = project
        .id
        .ok_or_else(|| anyhow::anyhow!("project '{}' has no id", project.name))?;
    let summary = task::status_summary(project_id)?;
    print_info!("Project: {}", project.name);
    println!(
        "todo={} in-progress={} done={} leased={} unassigned={}",
        summary.todo, summary.in_progress, summary.done, summary.leased, summary.unassigned
    );
    for task in &summary.next {
        println!(
            "next: #{} {} [priority {}] status={}",
            task.id.unwrap_or_default(),
            task.title,
            task.priority,
            task.status.as_str()
        );
    }
    Ok(true)
}
