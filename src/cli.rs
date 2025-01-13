use clap::{Parser, Subcommand};


#[derive(Parser, Debug)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    about = env!("CARGO_PKG_DESCRIPTION")
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// interactive mode
    #[arg(short, long)]
    pub interactive: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// add taskØ
    Add { title: String },
    /// update task
    Update { index: usize, title: String },
    /// remove task
    #[command(alias = "rm")]
    Remove { indexes: Vec<usize> },
    /// complete task
    Done { indexes: Vec<usize> },
    /// start task
    Start { indexes: Vec<usize> },
    /// list task
    #[command(alias = "ls")]
    List {
        #[command(subcommand)]
        command: Option<ListSubcommand>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ListSubcommand {
    All,
    Done,
    Todo,
    #[command(name = "in-progress")]
    InProgress,
}
