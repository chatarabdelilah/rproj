use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rproj", version, about = "Guided bootstrap-to-game-dev CLI for Roblox projects")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Show every command rproj runs and all of its output. Without this,
    /// sub-process output is only shown when something fails.
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Install and configure every tool rproj knows about (Git, VS Code, Roblox Studio,
    /// Blender, Rojo, Wally, Selene, StyLua, Studio plugins, editor extensions...)
    Setup,
    /// Scaffold a new Roblox project under your RobloxProjects folder
    New {
        /// Project name / folder name under RobloxProjects
        name: String,
        /// Re-ask the machine setup questions (system apps, CLI tools,
        /// plugins, extensions). Normally these are skipped once your
        /// machine is set up.
        #[arg(long)]
        reconfigure: bool,
        /// Reuse a saved package setup instead of picking packages
        /// (see --save-setup). `rproj info` lists what's saved.
        #[arg(long, value_name = "SETUP")]
        like: Option<String>,
        /// Save this project's package selection and workflow under a
        /// name, so a later `rproj new ... --like <name>` reuses it.
        #[arg(long, value_name = "NAME")]
        save_setup: Option<String>,
    },
    /// Walk through a tool's settings one at a time, explaining each one,
    /// and write them to its config file in the current project
    Configure {
        /// Tool key to configure (stylua, selene, luau-lsp...). Omit to pick from a list.
        key: Option<String>,
    },
    /// Resume the dev loop in the current project: install anything missing,
    /// then start the Rojo sourcemap watcher
    Watch,
    /// Copy every file under src/ (with relative-path headers) to the clipboard
    Copy,
    /// Show what a catalog entry does, or list the whole catalog
    Info {
        /// Package/tool key to look up. Omit to list everything.
        key: Option<String>,
    },
}
