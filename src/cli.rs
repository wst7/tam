use clap::{Parser, Subcommand};
use colored::Colorize;

use crate::commands::{self, task};

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
}

#[derive(Subcommand, Debug)]
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
    /// Show tam configuration
    Config,
}

#[derive(Subcommand, Debug)]
pub enum ProjectSubcommand {
    /// Add a new project
    #[command(name = "add",  visible_alias = "a")]
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
    pub fn execute_command(command: Option<Self>) -> anyhow::Result<bool> {
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

#[derive(Subcommand, Debug)]
pub enum TaskSubcommand {
    /// Add a new task
    #[command(name = "add", visible_alias = "a")]
    Add { title: String },
    /// Edit an existing task
    #[command(name = "edit", visible_alias = "e")]
    Edit { id: String, title: String },
    /// List all tasks
    #[command(name = "list", visible_alias = "l")]
    List,
    /// Delete a task
    #[command(name = "del", visible_alias = "d")]
    Delete { id: String },
    /// Start a task
    #[command(name = "start")]
    Start { id: String },
    /// Mark a task as done
    #[command(name = "done")]
    Done { id: String },
}

impl TaskSubcommand {
    pub fn execute_command(command: Option<Self>) -> anyhow::Result<bool> {
        match command.unwrap_or(Self::List) {
            TaskSubcommand::Add{title} => task::add(title),
            TaskSubcommand::List => task::list(),
            TaskSubcommand::Edit { id, title } => task::edit(id, title),
            TaskSubcommand::Delete { id } => task::delete(id),
            TaskSubcommand::Start { id } => task::start(id),
            TaskSubcommand::Done { id } => task::done(id),
        }
    }
}
