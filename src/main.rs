// `forbid`, not `deny`: an inner `#[allow(unsafe_code)]` cannot override
// it, so granting an exception has to happen here, in a diff a reviewer
// sees. The crate has never contained `unsafe` - this is what keeps that
// true. It caught its first case immediately: a test written to steer a
// path lookup reached for `std::env::set_var`, which is `unsafe` in
// edition 2024, and the lint forced the better design (pass the base
// directory in, rather than mutating process-global state).
#![forbid(unsafe_code)]

mod catalog;
mod cli;
mod commands;
mod config;
mod steps;
mod ui;

use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Command};

/// Returns `ExitCode` rather than `anyhow::Result` so the error is printed
/// by `ui::error` instead of by Rust's default `Termination` impl, which
/// writes an uncoloured `Error: ...` with the chain in `Debug` form.
fn main() -> ExitCode {
    let cli = Cli::parse();
    ui::set_verbose(cli.verbose);

    match dispatch(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            ui::error(&err);
            ExitCode::FAILURE
        }
    }
}

fn dispatch(cli: Cli) -> anyhow::Result<()> {
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
