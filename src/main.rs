use clap::Parser;
use cli::{Cli, Commands, ListSubcommand};
use cursive::Cursive;
use std::process;

mod cli;
mod commands;
mod config;
mod query;
mod task;
mod theme;
mod ui;
mod utils;

fn main() {
    let _ = config::init();

    let args = Cli::parse();
    if args.interactive {
        interactive_mode()
    } else if let Some(command) = args.command {
        command_mode(command);
    } else {
        // print help
        Cli::parse_from(&["tam", "--help"]);
    }
}

fn interactive_mode() {
    let mut siv: Cursive = Cursive::default();
    siv.load_toml(include_str!("../theme.toml")).unwrap();
    ui::start(siv)
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
        Commands::Config => commands::config(),
    };
    result
}
