use anyhow::Result;

use crate::commands::provision;
use crate::config::GlobalConfig;
use crate::steps::notify;
use crate::ui;

/// Pre-provisions a machine without creating a project. Not a prerequisite
/// for `rproj new` (which runs the same selection inline) - this is just a
/// convenience for setting a fresh machine up ahead of time.
pub fn run() -> Result<()> {
    // One line, not the seven-line explanation this used to print.
    //
    // Everything that paragraph said is either already known (the user
    // typed `setup`), visible in the pickers that follow (each option
    // carries its own description and badge), or belongs on the welcome
    // screen, which already describes what this command is for. Seven
    // lines of preamble ahead of the first question is a wall between the
    // user and the thing they asked for.
    ui::detail("Nothing already installed is reinstalled. Safe to re-run any time.");

    let mut config = GlobalConfig::load()?;
    provision::run(&mut config)?;
    config.save()?;

    notify::summary("rproj setup complete", "Your machine is ready for Roblox development.");
    println!("\nDone. Run `rproj new <name>` to scaffold your first project.");
    Ok(())
}
