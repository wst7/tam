use clap::{value_parser, Parser, Subcommand, ValueEnum};
use colored::Colorize;

use crate::agent::Ctx;
use crate::commands::{self, task};
use crate::model::TaskStatus;

#[derive(Parser, Debug)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    about = format!(
        "{}\nVersion: {}",
        env!("CARGO_PKG_DESCRIPTION"),
        env!("CARGO_PKG_VERSION").green()
    )
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Interactive mode
    #[arg(short, long)]
    pub interactive: bool,

    /// Machine readable JSON output (schema tam.agent/v1)
    #[arg(long, global = true)]
    pub json: bool,

    /// Project context override (default: TAM_PROJECT, then config)
    #[arg(long, global = true, value_name = "PROJECT")]
    pub project: Option<String>,

    /// Agent identity for leases and audit events (default: TAM_AGENT)
    #[arg(long, global = true, value_name = "AGENT")]
    pub agent: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Manage projects, use `tam project --help` to see more options
    #[command(name = "project", visible_alias = "p")]
    Project {
        #[command(subcommand)]
        command: Option<ProjectSubcommand>,
    },
    /// Manage tasks, use `tam task --help` to see more options
    #[command(name = "task", visible_alias = "t")]
    Task {
        #[command(subcommand)]
        command: Option<TaskSubcommand>,
    },
    /// Show a compact project status summary
    Status,
    /// Run the MCP stdio server (JSON-RPC 2.0, newline delimited)
    Mcp,
    /// Show tam configuration
    Config,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ProjectSubcommand {
    /// Add a new project
    #[command(name = "add", visible_alias = "a")]
    Add { name: String },
    /// List all projects
    #[command(name = "list", visible_alias = "l")]
    List,
    /// Use a project
    #[command(name = "use", visible_alias = "u")]
    Use { name: String },
    /// Delete a project
    #[command(name = "del", visible_alias = "d")]
    Del { name: String },
    /// Show current project
    #[command(name = "current", visible_alias = "c")]
    Current,
    /// edit a project
    #[command(name = "edit", visible_alias = "e")]
    Edit { name: String, new_name: String },
}

impl ProjectSubcommand {
    pub fn execute_command(command: Option<Self>, _ctx: &Ctx) -> anyhow::Result<bool> {
        match command.unwrap_or(Self::List) {
            ProjectSubcommand::Add { name } => commands::project::add(name),
            ProjectSubcommand::List => commands::project::list(),
            ProjectSubcommand::Use { name } => commands::project::use_project(name),
            ProjectSubcommand::Del { name } => commands::project::delete(name),
            ProjectSubcommand::Current => commands::project::get_current_project(),
            ProjectSubcommand::Edit { name, new_name } => commands::project::edit(name, new_name),
        }
    }
}

#[derive(Subcommand, Debug, Clone)]
pub enum TaskSubcommand {
    /// Add a new task
    #[command(name = "add", visible_alias = "a")]
    Add {
        title: String,
        /// Task description / context (stored in the existing `content` column)
        #[arg(long)]
        description: Option<String>,
        /// Priority 1..5 (1 is highest, default 3)
        #[arg(long)]
        priority: Option<i32>,
        /// Tag (repeatable; normalized lowercase, comma separated)
        #[arg(long = "tag", action = clap::ArgAction::Append)]
        tag: Vec<String>,
        /// Idempotency key (project scoped; retries return the existing task)
        #[arg(long)]
        idempotency_key: Option<String>,
    },
    /// Edit an existing task
    #[command(name = "edit", visible_alias = "e")]
    Edit {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
        title: String,
    },
    /// List all tasks
    #[command(name = "list", visible_alias = "l")]
    List {
        /// Only show tasks with this status
        #[arg(long, value_enum)]
        status: Option<StatusArg>,
        /// Only show tasks assigned to this agent (`me` / `unassigned`)
        #[arg(long)]
        assigned: Option<String>,
        /// Only show tasks with this priority (1..5)
        #[arg(long)]
        priority: Option<i32>,
        /// Only show tasks carrying this tag (repeatable, all must match)
        #[arg(long = "tag", action = clap::ArgAction::Append)]
        tag: Vec<String>,
        /// Case-insensitive title/description substring
        #[arg(long)]
        search: Option<String>,
        /// Maximum number of tasks (default 100, max 1000)
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Delete a task
    #[command(name = "del", visible_alias = "d")]
    Delete {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
    },
    /// Start a task
    #[command(name = "start")]
    Start {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
    },
    /// Mark a task as done
    #[command(name = "done")]
    Done {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
    },
    /// Show one task with its event log
    #[command(name = "show")]
    Show {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
    },
    /// Partially update a task
    #[command(name = "update")]
    Update {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, value_enum)]
        status: Option<StatusArg>,
        #[arg(long)]
        priority: Option<i32>,
        /// Comma separated tags (empty string clears the tag list)
        #[arg(long)]
        tags: Option<String>,
    },
    /// Atomically claim a task lease
    #[command(name = "claim")]
    Claim {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
        /// Lease duration in seconds (5..86400)
        #[arg(long, default_value_t = 900)]
        lease: i64,
        /// Preempt another agent's valid lease
        #[arg(long)]
        force: bool,
    },
    /// Renew a task lease (holder only)
    #[command(name = "heartbeat")]
    Heartbeat {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
        /// Lease duration in seconds (5..86400)
        #[arg(long, default_value_t = 900)]
        lease: i64,
    },
    /// Release a task lease
    #[command(name = "release")]
    Release {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
        /// Release even when another agent holds the lease
        #[arg(long)]
        force: bool,
    },
    /// Show the next claimable task
    #[command(name = "next")]
    Next {
        /// Atomically claim the returned task
        #[arg(long)]
        claim: bool,
        /// Lease duration in seconds (5..86400)
        #[arg(long, default_value_t = 900)]
        lease: i64,
    },
    /// Show a task's event log
    #[command(name = "events")]
    Events {
        #[arg(value_parser = value_parser!(i32).range(1..))]
        id: i32,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum StatusArg {
    Todo,
    InProgress,
    Done,
}

impl From<StatusArg> for TaskStatus {
    fn from(value: StatusArg) -> Self {
        match value {
            StatusArg::Todo => TaskStatus::Todo,
            StatusArg::InProgress => TaskStatus::InProgress,
            StatusArg::Done => TaskStatus::Done,
        }
    }
}

impl TaskSubcommand {
    pub fn execute_command(command: Option<Self>, ctx: &Ctx) -> anyhow::Result<bool> {
        match command.unwrap_or(Self::List {
            status: None,
            assigned: None,
            priority: None,
            tag: Vec::new(),
            search: None,
            limit: None,
        }) {
            TaskSubcommand::Add {
                title,
                description,
                priority,
                tag,
                idempotency_key,
            } => task::add(ctx, title, description, priority, tag, idempotency_key),
            TaskSubcommand::List {
                status,
                assigned,
                priority,
                tag,
                search,
                limit,
            } => task::list(ctx, status, assigned, priority, tag, search, limit),
            TaskSubcommand::Edit { id, title } => task::edit(id, title),
            TaskSubcommand::Delete { id } => task::delete(id),
            TaskSubcommand::Start { id } => task::start(id),
            TaskSubcommand::Done { id } => task::done(ctx, id),
            TaskSubcommand::Show { id } => task::show(ctx, id),
            TaskSubcommand::Update {
                id,
                title,
                description,
                status,
                priority,
                tags,
            } => task::update(ctx, id, title, description, status, priority, tags),
            TaskSubcommand::Claim { id, lease, force } => task::claim(ctx, id, lease, force),
            TaskSubcommand::Heartbeat { id, lease } => task::heartbeat(ctx, id, lease),
            TaskSubcommand::Release { id, force } => task::release(ctx, id, force),
            TaskSubcommand::Next { claim, lease } => task::next(ctx, claim, lease),
            TaskSubcommand::Events { id } => task::events(ctx, id),
        }
    }
}
