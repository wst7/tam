use clap::Parser;
use cli::{Cli, Commands};
use db::init_db;
use std::process;

use crate::{app::App, cli::{ProjectSubcommand, TaskSubcommand}};

mod app;
mod cli;
mod commands;
mod config;
mod db;
mod model;
mod services;
mod sql;
mod theme;
mod ui;
mod utils;
mod logs;

fn main() {
    logs::init_logging();
    match init_db() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("init database error: {}", e);
            std::process::exit(1)
        }
    }
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
    match App::new().run() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("run app error: {}", e);
            std::process::exit(1)
        }
    };
}

fn command_mode(command: Commands) {
    if let Err(err) = execute_command(command) {
        print_error!("{}", err.to_string());
        process::exit(1);
    }
}

fn execute_command(command: Commands) -> anyhow::Result<bool> {
    let result = match command {
        Commands::Project { command } => ProjectSubcommand::execute_command(command),
        Commands::Task { command } => TaskSubcommand::execute_command(command),
        Commands::Config => commands::config::config(),
    };
    result
}
