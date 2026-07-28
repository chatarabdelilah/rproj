mod catalog;
mod cli;
mod commands;
mod config;
mod steps;
mod ui;

use clap::Parser;

use cli::{Cli, Command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    ui::set_verbose(cli.verbose);

    match cli.command {
        None => {
            commands::welcome::run();
            Ok(())
        }
        Some(Command::Setup) => commands::setup::run(),
        Some(Command::New { name, reconfigure, like, save_setup }) => {
            commands::new::run(&name, reconfigure, like.as_deref(), save_setup.as_deref())
        }
        Some(Command::Configure { key }) => commands::configure::run(key.as_deref()),
        Some(Command::Upgrade { yes }) => commands::upgrade::run(yes),
        Some(Command::Watch) => commands::watch::run(),
        Some(Command::Copy) => commands::copy::run(),
        Some(Command::Info { key }) => commands::info::run(key.as_deref()),
    }
}
