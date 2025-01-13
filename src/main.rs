use clap::Parser;
use cli::{Cli, Commands, ListSubcommand};
use cursive::Cursive;
use rustyline::{error::ReadlineError, DefaultEditor};
use std::process;

mod cli;
mod commands;
mod file;
mod task;
mod utils;
mod ui;


fn main() {
    let args = Cli::parse();
    if args.interactive {
        interactive_mode()
    } else if let Some(command) = args.command {
        command_mode(command);
    }
}

fn interactive_mode() {
    let siv: Cursive = Cursive::default();
    ui::render(siv)
}

fn command_mode(command: Commands) {
    if let Err(err) = execute_command(command) {
        print_error!("{}", err.to_string());
        process::exit(1);
    }
}

fn execute_command(command: Commands) -> anyhow::Result<bool> {
    let result = match command {
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
    result
}
