mod catalog;
mod cli;
mod commands;
mod config;
mod steps;

use clap::Parser;

use cli::{Cli, Command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            commands::welcome::run();
            Ok(())
        }
        Some(Command::Setup) => commands::setup::run(),
        Some(Command::New { name }) => commands::new::run(&name),
        Some(Command::Configure { key }) => commands::configure::run(key.as_deref()),
        Some(Command::Watch) => commands::watch::run(),
        Some(Command::Copy) => commands::copy::run(),
        Some(Command::Info { key }) => commands::info::run(key.as_deref()),
    }
}
