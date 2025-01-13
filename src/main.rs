use clap::{Parser, Subcommand};
use rustyline::{error::ReadlineError, DefaultEditor};
use std::process;

mod commands;
mod file;
mod task;
mod utils;

#[derive(Parser, Debug)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    about = env!("CARGO_PKG_DESCRIPTION")
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// interactive mode
    #[arg(short, long)]
    interactive: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
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
enum ListSubcommand {
    All,
    Done,
    Todo,
    #[command(name = "in-progress")]
    InProgress,
}

fn main() {
    let cli = Cli::parse();
    if cli.interactive {
        interactive_mode()
    } else if let Some(command) = cli.command {
        command_mode(command);
    }
}

fn interactive_mode() {
    let mut rl = DefaultEditor::new().unwrap();
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());

                let input = line.trim();
                if input == "exit" {
                    println!("exit.");
                    break;
                }
                let args = shlex::split(&format!("tam {input}")).unwrap_or_else(|| vec![]);

                match Cli::try_parse_from(args) {
                    Ok(cli) => {
                        println!("{:?}", cli);
                        if let Some(command) = cli.command {
                            if let Err(err) = execute_command(command) {
                                print_error!("{}", err);
                            }
                        }
                    }
                    Err(err) => {
                        print_error!("Error: {}", err);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                break;
            },
            Err(err) => {
                print_error!("Error: {}", err);
                continue;
            }
        }
    }
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
