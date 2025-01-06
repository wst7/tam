use std::process;
use clap::{Parser, Subcommand};

mod commands;
mod file;
mod task;
mod utils;

#[derive(Parser)]
#[command(name = "tam", version = "1.0", about = "A tasks manager cli tool")]
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
    Remove { index: usize },
    /// complete task
    Done { index: usize },
    /// start task
    Start { index: usize },
    /// list task
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
        Commands::Remove { index } => commands::remove(index),
        Commands::Done { index } => commands::done(index),
        Commands::Start { index } => commands::start(index),
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
