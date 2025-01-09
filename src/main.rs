use clap::{Parser, Subcommand};
use std::process;

mod commands;
mod file;
mod task;
mod utils;

#[derive(Parser)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    about = env!("CARGO_PKG_DESCRIPTION")
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// add task
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

#[derive(Subcommand)]
enum ListSubcommand {
    All,
    Done,
    Todo,
    #[command(name = "in-progress")]
    InProgress,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Add { title } => commands::add(title),
        Commands::Update { index, title } => commands::update(index, title),
        Commands::Remove { indexes } => commands::remove(&indexes),
        Commands::Done { indexes } => commands::done(&indexes),
        Commands::Start { indexes } => commands::start(&indexes),
        Commands::List { command } => match command.unwrap_or(ListSubcommand::All) {
            ListSubcommand::Done => commands::list_done(),
            ListSubcommand::InProgress => commands::list_in_progress(),
            ListSubcommand::Todo => commands::list_todo(),
            ListSubcommand::All => commands::list_all(),
        },
    };
    match result {
        Ok(_) => (),
        Err(err) => {
            print_error!("{}", err.to_string());
            process::exit(1);
        }
    }
}
