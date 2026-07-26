//! Terminal output.
//!
//! The rule: **one line per thing that happened.** Sub-processes rproj
//! shells out to (winget, rokit, git, blender, rojo) are chatty in ways
//! that are noise to someone running rproj - `rokit add --global` prints a
//! five-line ERROR block for a tool that's simply already installed, `git
//! submodule add` prints clone progress for every package, `rokit init`
//! prints a banner. Multiply that by nine tools and six packages and the
//! actual outcome is invisible.
//!
//! So sub-process output is captured, not inherited, and only surfaced
//! when something actually went wrong (or when `--verbose` asks for all of
//! it). Callers report outcomes through `ok`/`skip`/`warn` instead, and
//! batch repetitive ones through `Tally`.

use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn set_verbose(on: bool) {
    VERBOSE.store(on, Ordering::Relaxed);
}

pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

/// A top-level phase, e.g. "Machine setup" or "Scaffolding creamy".
pub fn section(title: &str) {
    println!("\n{title}");
}

/// Something was done, or was already the case.
pub fn ok(msg: &str) {
    println!("  + {msg}");
}

/// Deliberately not done, with the reason.
pub fn skip(msg: &str) {
    println!("  - {msg}");
}

/// Something failed but the run continues.
pub fn warn(msg: &str) {
    println!("  ! {msg}");
}

/// Detail under the most recent line: indented, and only worth printing
/// when it tells the user something they'd otherwise have to go find.
pub fn detail(msg: &str) {
    for line in msg.lines() {
        println!("      {line}");
    }
}

/// The command about to run. Only shown under `--verbose` - the outcome
/// line is what matters otherwise.
pub fn command(program: &str, args: &[&str]) {
    if is_verbose() {
        println!("  $ {program} {}", args.join(" "));
    }
}

/// Raw captured sub-process output, shown when a step failed (so the user
/// can see why) or when running verbose.
pub fn passthrough(stdout: &str, stderr: &str) {
    for stream in [stdout, stderr] {
        for line in stream.lines().filter(|l| !l.trim().is_empty()) {
            println!("      {line}");
        }
    }
}

/// Collapses a run of same-kind outcomes into one line.
///
/// Nine "already added globally" lines say exactly as much as
/// "9 tools already installed", and the latter doesn't bury the one tool
/// that actually needed attention.
#[derive(Default)]
pub struct Tally {
    done: Vec<String>,
    already: Vec<String>,
}

impl Tally {
    pub fn new() -> Self {
        Self::default()
    }

    /// Newly done this run.
    pub fn did(&mut self, name: &str) {
        self.done.push(name.to_string());
    }

    /// Already in place; nothing to do.
    pub fn already(&mut self, name: &str) {
        self.already.push(name.to_string());
    }

    /// Prints the summary. `noun` is plural, e.g. "rokit tools".
    pub fn finish(self, noun: &str) {
        if !self.done.is_empty() {
            ok(&format!("{noun}: installed {}", self.done.join(", ")));
        }
        if !self.already.is_empty() {
            let n = self.already.len();
            // Naming a handful is useful; naming twenty is a wall again.
            if n <= 4 {
                ok(&format!("{noun}: {} already present", self.already.join(", ")));
            } else {
                ok(&format!("{n} {noun} already present"));
            }
        }
    }
}
