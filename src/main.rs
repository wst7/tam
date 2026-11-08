use clap::Parser;
use cli::{Cli, Commands};
use db::init_db;
use std::process;

use crate::{
    agent::{AgentError, Ctx},
    app::App,
    cli::{ProjectSubcommand, TaskSubcommand},
};

mod agent;
mod app;
mod cli;
mod commands;
mod config;
mod db;
mod logs;
mod mcp;
mod model;
mod services;
mod sql;
mod theme;
mod ui;
mod utils;

fn main() {
    // Parse first: clap handles `--help` / `-V` / usage errors before any
    // filesystem or database side effect. Machine mode is then known in time to
    // report initialization errors as JSON.
    let args = Cli::parse();
    let ctx = Ctx::new(args.json, args.project.clone(), args.agent.clone());

    // `tam mcp` owns stdout with JSON-RPC: dispatch before any human output.
    if matches!(args.command, Some(Commands::Mcp)) {
        run_mcp();
        return;
    }

    // JSON mode: a broken config must still produce an envelope instead of the
    // human `Failed to load config` message from the Lazy initializer.
    if ctx.json {
        if let Err(e) = config::preflight_load() {
            let error = AgentError::internal(format!("failed to load config: {}", e));
            agent::emit_error("startup", &error);
            process::exit(error.exit_code());
        }
    }

    logs::init_logging();

    if args.interactive {
        init_db_or_exit(&ctx, "interactive");
        interactive_mode();
        return;
    }

    match args.command {
        Some(command) => {
            init_db_or_exit(&ctx, commands::agent::command_name(&command));
            if ctx.json {
                run_json(command, &ctx);
            } else {
                command_mode(command, &ctx);
            }
        }
        None => {
            // print help (clap exits with code 0)
            Cli::parse_from(["tam", "--help"]);
        }
    }
}

fn run_mcp() {
    logs::init_logging();
    if let Err(e) = init_db() {
        eprintln!("init database error: {}", e);
        process::exit(1);
    }
    match mcp::run() {
        Ok(()) => process::exit(0),
        Err(e) => {
            eprintln!("mcp error: {}", e);
            process::exit(1);
        }
    }
}

fn init_db_or_exit(ctx: &Ctx, command: &str) {
    if let Err(e) = init_db() {
        if ctx.json {
            let error = AgentError::internal(format!("database initialization failed: {}", e));
            agent::emit_error(command, &error);
            process::exit(error.exit_code());
        }
        eprintln!("init database error: {}", e);
        process::exit(1);
    }
}

fn run_json(command: Commands, ctx: &Ctx) {
    let name = commands::agent::command_name(&command);
    match commands::agent::dispatch_json(&command, ctx) {
        Ok(outcome) => agent::emit_success(name, outcome.data, ctx, outcome.project.as_deref()),
        Err(error) => {
            agent::emit_error(name, &error);
            process::exit(error.exit_code());
        }
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

fn command_mode(command: Commands, ctx: &Ctx) {
    if let Err(err) = execute_command(command, ctx) {
        print_error!("{}", err.to_string());
        process::exit(1);
    }
}

fn execute_command(command: Commands, ctx: &Ctx) -> anyhow::Result<bool> {
    match command {
        Commands::Project { command } => ProjectSubcommand::execute_command(command, ctx),
        Commands::Task { command } => TaskSubcommand::execute_command(command, ctx),
        Commands::Config => commands::config::config(),
        Commands::Status => commands::task::status(ctx),
        Commands::Mcp => unreachable!("mcp is dispatched before command_mode"),
    }
}
