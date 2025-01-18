use clap::{Parser, Subcommand};
use colored::Colorize;

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
    /// Add a new task with a given title.
    Add { title: String },

    /// Update the title of an existing task by its index.
    #[command(aliases = ["et", "edit", "up"])]
    Update { index: usize, title: String },

    /// Remove one or more tasks by their indexes.
    #[command(aliases = ["rm", "dl", "delete"])]
    Remove { indexes: Vec<usize> },

    /// Mark one or more tasks as completed by their indexes.
    Done { indexes: Vec<usize> },

    /// Mark one or more tasks as started by their indexes.
    Start { indexes: Vec<usize> },

    /// List tasks, optionally with a subcommand to filter or sort the list.
    #[command(alias = "ls")]
    List {
        #[command(subcommand)]
        command: Option<ListSubcommand>,
    },
    /// Show tam configuration
    Config
}

#[derive(Subcommand, Debug)]
pub enum ListSubcommand {
    All,
    Done,
    Todo,
    #[command(name = "in-progress")]
    InProgress,
}
