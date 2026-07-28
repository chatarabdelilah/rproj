//! The screen `rproj` shows with no arguments.
//!
//! Deliberately not clap's `--help` dump. Someone typing `rproj` on a fresh
//! PC has not decided to use this yet - they need to know what it is, what
//! it will do to their machine, and which single command to type next.
//! `--help` answers none of those; it lists flags.
//!
//! Laid out after `rokit list`, which is the closest thing this toolchain
//! has to a house style: an emoji-led header, then aligned rows with the
//! name on the left and the detail on the right.

use crate::ui;

/// Command rows. Kept as data so the column stays aligned by construction -
/// hand-padded columns in a string literal drift the moment one row
/// changes, and nothing fails when they do.
const COMMANDS: &[(&str, &str, &[&str])] = &[
    (
        "🚀",
        "rproj new <name>",
        &[
            "Scaffold a project. Sets your machine up first if it",
            "hasn't been, then asks only about this project's packages.",
            "--like <setup> reuses a saved selection, --save-setup saves one",
        ],
    ),
    (
        "🔧",
        "rproj setup",
        &["Install or change the machine-wide tools: system apps, CLI", "tools, Studio plugins, editor extensions"],
    ),
    (
        // Every icon here is a plain double-width emoji with no variation
        // selector. A selector-carrying one (⚙️, 🛠️) is two `char`s wide but
        // one or two *columns* depending on the console, so no padding
        // arithmetic can keep the column straight for it.
        "🔩",
        "rproj configure [tool]",
        &[
            "Walk through a tool's settings one at a time - StyLua,",
            "Selene, luau-lsp - explaining each, then write them out",
        ],
    ),
    (
        "🔄",
        "rproj upgrade",
        &[
            "Re-apply rproj's generated config to a project you already made,",
            "so it picks up fixes shipped since it was scaffolded",
        ],
    ),
    ("👀", "rproj watch", &["Resume the dev loop: install what's missing, watch the sourcemap"]),
    ("📋", "rproj copy", &["Copy every file under src/ to the clipboard, with path headers"]),
    ("📖", "rproj info [key]", &["Look up what a tool or package does, and how to use it"]),
];

pub fn run() {
    let version = env!("CARGO_PKG_VERSION");
    let (rocket, book, sparkle) = match ui::emoji_enabled() {
        true => ("🎮  ", "📚  ", "✨  "),
        false => ("", "", ""),
    };

    println!("\n{rocket}rproj {version} - guided bootstrap-to-game-dev CLI for Roblox\n");
    println!("    Takes a fresh PC all the way to a working Roblox dev setup, then");
    println!("    scaffolds projects on top of it. Explains every tool and package");
    println!("    as it goes, so you end up knowing why your setup looks like it does.\n");

    println!("{book}Commands\n");
    let width = COMMANDS.iter().map(|(_, name, _)| name.len()).max().unwrap_or(0);
    // Terminal *columns* the icon occupies, not `char`s: every icon above is
    // one char and two columns wide, plus two spaces of gutter.
    let icon_columns = if ui::emoji_enabled() { 4 } else { 0 };
    for (icon, name, lines) in COMMANDS {
        for (i, line) in lines.iter().enumerate() {
            let lead = match (i, ui::emoji_enabled()) {
                (0, true) => format!("{icon}  "),
                // Continuation rows leave the icon and name columns blank,
                // so a multi-line description reads as one paragraph rather
                // than as several commands.
                _ => " ".repeat(icon_columns),
            };
            let name = if i == 0 { *name } else { "" };
            println!("  {lead}{name:<width$}  {line}");
        }
        println!();
    }

    println!("{sparkle}New here?  rproj new my-first-game  sets your machine up and");
    println!("    scaffolds your first project in one go.\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The icon column is padded by a fixed number of terminal columns, on
    /// the assumption that every icon is one `char` and two columns wide.
    /// A variation-selector emoji (⚙️, 🛠️) is two `char`s and an
    /// unpredictable number of columns, and adding one silently bends the
    /// description column out of line on every continuation row.
    #[test]
    fn every_icon_is_a_single_char() {
        for (icon, name, _) in COMMANDS {
            assert_eq!(icon.chars().count(), 1, "{name}'s icon {icon} is not a single char");
        }
    }

    /// Every command clap accepts should be findable here - the welcome
    /// screen is the only place someone who typed `rproj` learns they exist.
    #[test]
    fn every_command_is_listed() {
        for command in ["new", "setup", "configure", "upgrade", "watch", "copy", "info"] {
            assert!(
                COMMANDS.iter().any(|(_, name, _)| name.starts_with(&format!("rproj {command}"))),
                "`{command}` is missing from the welcome screen"
            );
        }
    }

    /// An empty description row would print a name with nothing after it.
    #[test]
    fn every_command_explains_itself() {
        for (_, name, lines) in COMMANDS {
            assert!(!lines.is_empty(), "{name} has no description");
            assert!(lines.iter().all(|l| !l.trim().is_empty()), "{name} has a blank line");
        }
    }
}
