use anyhow::Result;

use crate::commands::provision;
use crate::config::GlobalConfig;
use crate::steps::notify;

/// Pre-provisions a machine without creating a project. Not a prerequisite
/// for `rproj new` (which runs the same selection inline) - this is just a
/// convenience for setting a fresh machine up ahead of time.
pub fn run() -> Result<()> {
    println!(
        "rproj setup\n\
         This installs and configures everything rproj knows about: system apps,\n\
         the Rojo/Wally toolchain, plugins, and editor extensions. Every choice\n\
         below shows what it does and whether it's actively maintained. Nothing\n\
         gets reinstalled if it's already present, and you can re-run this any\n\
         time to add or remove tools. You don't need to run this before\n\
         `rproj new` - it asks the same questions inline if you skip straight to it.\n"
    );

    let mut config = GlobalConfig::load()?;
    provision::run(&mut config)?;
    config.save()?;

    notify::summary("rproj setup complete", "Your machine is ready for Roblox development.");
    println!("\nDone. Run `rproj new <name>` to scaffold your first project.");
    Ok(())
}
