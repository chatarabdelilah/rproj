// `forbid`, not `deny`: an inner `#[allow(unsafe_code)]` cannot override
// it, so granting an exception has to happen here, in a diff a reviewer
// sees. The crate has never contained `unsafe` - this is what keeps that
// true. It caught its first case immediately: a test written to steer a
// path lookup reached for `std::env::set_var`, which is `unsafe` in
// edition 2024, and the lint forced the better design (pass the base
// directory in, rather than mutating process-global state).
#![forbid(unsafe_code)]

mod catalog;
mod catalog_view;
mod cli;
mod commands;
mod config;
mod diagnostics;
mod graph;
mod interrupt;
mod project_editor;
mod steps;
mod tui;
mod ui;

use std::io::IsTerminal;
use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Command};

/// Returns `ExitCode` rather than `anyhow::Result` so the error is printed
/// by `ui::error` instead of by Rust's default `Termination` impl, which
/// writes an uncoloured `Error: ...` with the chain in `Debug` form.
fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let code = error.exit_code() as u8;
            if error.use_stderr() {
                let log = diagnostics::start();
                diagnostics::event(
                    "cli.invalid",
                    "argument parsing failed; raw arguments omitted",
                );
                let _ = error.print();
                log.finish(code);
            } else {
                let _ = error.print();
            }
            return ExitCode::from(code);
        }
    };
    let log = diagnostics::start();
    log_command(&cli);
    ui::set_verbose(cli.verbose);

    let code = match dispatch(cli) {
        Ok(()) => 0,
        Err(err) => {
            ui::error(&err);
            err.downcast_ref::<commands::test::RunnerFailure>()
                .map(commands::test::RunnerFailure::exit_code)
                .unwrap_or(1)
        }
    };
    log.finish(code);
    ExitCode::from(code)
}

fn log_command(cli: &Cli) {
    let message = match &cli.command {
        None => "hub/welcome".into(),
        Some(Command::New {
            name,
            reconfigure,
            like,
            save_setup,
        }) => format!(
            "new name={name:?} reconfigure={reconfigure} like={like:?} save_setup={save_setup:?}"
        ),
        Some(Command::Setup { tool }) => format!("setup tool={tool:?}"),
        Some(Command::Configure { key }) => format!("configure key={key:?}"),
        Some(Command::Upgrade { yes }) => format!("upgrade yes={yes}"),
        Some(Command::Watch) => "watch".into(),
        Some(Command::Test { args }) => format!(
            "test; {} passthrough arguments (values omitted)",
            args.len()
        ),
        Some(Command::Copy) => "copy (source and clipboard contents omitted)".into(),
        Some(Command::Info { key }) => format!("info key={key:?}"),
    };
    diagnostics::event("command", message);
}

fn dispatch(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        None => {
            if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                commands::hub::run()
            } else {
                commands::welcome::run();
                Ok(())
            }
        }
        Some(Command::Setup { tool }) => commands::setup::run(tool.as_deref()),
        Some(Command::New {
            name,
            reconfigure,
            like,
            save_setup,
        }) => commands::new::run(&name, reconfigure, like.as_deref(), save_setup.as_deref()),
        Some(Command::Configure { key }) => commands::configure::run(key.as_deref()),
        Some(Command::Upgrade { yes }) => commands::upgrade::run(yes),
        Some(Command::Watch) => commands::watch::run(),
        Some(Command::Test { args }) => commands::test::run(&args),
        Some(Command::Copy) => commands::copy::run(),
        Some(Command::Info { key }) => commands::info::run(key.as_deref()),
    }
}

pub(crate) fn dispatch_hub(outcome: commands::hub::HubOutcome) -> anyhow::Result<()> {
    use commands::hub::HubOutcome;

    diagnostics::event("hub.dispatch", format!("{outcome:?}"));
    match outcome {
        HubOutcome::Quit => Ok(()),
        HubOutcome::New { .. } => unreachable!("creation questions run inside the hub"),
        HubOutcome::EditProjectTemplate => commands::project_template::run(),
        HubOutcome::SetupMachine => commands::setup::run(None),
        HubOutcome::Project { path, action } => action.run(&path),
    }
}
