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

/// Whether to draw the emoji markers.
///
/// On by default - rproj's own terminal output already relies on non-ASCII
/// (the multi-select help line's arrows), and every tool it drives renders
/// box-drawing characters, so a console that can't show these is a console
/// the whole workflow already looks wrong in. `RPROJ_NO_EMOJI` exists for
/// the exceptions: a log file, a CI transcript, a screen reader, or a
/// console still on a legacy code page, where a mojibake marker on every
/// line is worse than no marker at all.
fn emoji() -> bool {
    !matches!(std::env::var("RPROJ_NO_EMOJI").as_deref(), Ok("1" | "true"))
}

/// Whether callers laying out their own emoji should draw them.
pub fn emoji_enabled() -> bool {
    emoji()
}

/// The marker starting an outcome line, with its ASCII fallback.
///
/// Two spaces after `⚠️`: it is a base character plus VARIATION SELECTOR-16,
/// which consoles disagree about the width of, and the extra space is what
/// keeps the message text aligned with its neighbours. The others are
/// inherently double-width and need only one. Rokit does the same thing
/// with its `🛠️  Found tools:` header.
fn marker(icon: &str, pad: &str, fallback: &str) -> String {
    render_marker(emoji(), icon, pad, fallback)
}

/// The pure half of `marker`, so both branches are testable without
/// touching the environment (which every other test in the process shares).
fn render_marker(emoji: bool, icon: &str, pad: &str, fallback: &str) -> String {
    match emoji {
        true => format!("{icon}{pad}"),
        false => format!("{fallback} "),
    }
}

pub fn set_verbose(on: bool) {
    VERBOSE.store(on, Ordering::Relaxed);
}

pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

/// A top-level phase, e.g. "Machine setup" or "Scaffolding creamy".
pub fn section(title: &str) {
    match emoji() {
        true => println!("\n📦  {title}"),
        false => println!("\n{title}"),
    }
}

/// Something was done, or was already the case.
pub fn ok(msg: &str) {
    println!("  {}{msg}", marker("✅", " ", "+"));
}

/// Deliberately not done, with the reason.
pub fn skip(msg: &str) {
    println!("  {}{msg}", marker("➖", " ", "-"));
}

/// Something failed but the run continues.
pub fn warn(msg: &str) {
    println!("  {}{msg}", marker("⚠️", "  ", "!"));
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

    /// The lines this tally would print. Split out from `finish` so the
    /// summarising rules can be tested without capturing stdout.
    pub fn summary(&self, noun: &str) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.done.is_empty() {
            lines.push(format!("{noun}: installed {}", self.done.join(", ")));
        }
        if !self.already.is_empty() {
            let n = self.already.len();
            // Naming a handful is useful; naming twenty is a wall again.
            lines.push(if n <= 4 {
                format!("{noun}: {} already present", self.already.join(", "))
            } else {
                format!("{n} {noun} already present")
            });
        }
        lines
    }

    /// Prints the summary. `noun` is plural, e.g. "rokit tools".
    pub fn finish(self, noun: &str) {
        for line in self.summary(noun) {
            ok(&line);
        }
    }
}

/// Options in every picker are rendered as `key - description (badge)`, so
/// the key is the part before the first " - ". These helpers keep that
/// format in one place: it was previously encoded in seven separate
/// closures across three modules, where changing the separator would have
/// silently broken selection matching rather than failing to compile.
pub const OPTION_SEPARATOR: &str = " - ";

/// The key half of a rendered option line.
pub fn option_key(label: &str) -> &str {
    label.split(OPTION_SEPARATOR).next().unwrap_or(label)
}

/// Whether a rendered option line belongs to `key`. Matches on the full
/// `key - ` prefix rather than a bare `starts_with(key)`, so `react`
/// doesn't also match `reactRoblox`.
pub fn option_is(label: &str, key: &str) -> bool {
    label.starts_with(&format!("{key}{OPTION_SEPARATOR}"))
}

/// Help line for multi-selects. inquire supplies no default mentioning
/// enter-to-confirm, which people reliably get stuck on.
pub const MULTISELECT_HELP: &str =
    "↑↓ to move, space to select one, → to all, ← to none, enter to confirm, type to filter";

/// Post-answer summary for a multi-select. inquire's default echoes every
/// selected option's full `key - description (badge)` text joined together,
/// which is an unreadable wall once more than a couple are selected.
pub fn compact_multi_answer(opts: &[inquire::list_option::ListOption<&String>]) -> String {
    if opts.is_empty() {
        return "none".to_string();
    }
    let keys: Vec<&str> = opts.iter().map(|o| option_key(o.value)).collect();
    format!("{} selected: {}", keys.len(), keys.join(", "))
}

/// Post-answer summary for a single select.
pub fn compact_select_answer(opt: inquire::list_option::ListOption<&String>) -> String {
    option_key(opt.value).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `starts_with(key)` would make `react` match `reactRoblox`, quietly
    /// selecting a package the user didn't pick.
    #[test]
    fn option_matching_does_not_confuse_prefixes() {
        let label = "reactRoblox - React's Roblox renderer (active)";
        assert!(option_is(label, "reactRoblox"));
        assert!(!option_is(label, "react"), "prefix must not match a longer key");
        assert_eq!(option_key(label), "reactRoblox");
    }

    #[test]
    fn option_key_handles_labels_without_a_separator() {
        assert_eq!(option_key("none"), "none");
    }

    /// Turning emoji off has to give back a marker of the *same* width, or
    /// every outcome line shifts and the output stops lining up.
    #[test]
    fn both_marker_styles_are_the_same_width() {
        assert_eq!(render_marker(true, "✅", " ", "+"), "✅ ");
        assert_eq!(render_marker(false, "✅", " ", "+"), "+ ");
        // Two `char`s either way: one emoji plus one space, or one ASCII
        // character plus one space.
        for (icon, pad, fallback) in [("✅", " ", "+"), ("➖", " ", "-")] {
            let on = render_marker(true, icon, pad, fallback);
            let off = render_marker(false, icon, pad, fallback);
            assert_eq!(on.chars().count(), off.chars().count(), "{icon}");
        }
    }

    /// A marker built from a variation-selector emoji is one `char` wider
    /// than it looks, which is why `warn` pads by two - see `marker`.
    #[test]
    fn the_warning_marker_compensates_for_its_variation_selector() {
        assert_eq!("⚠️".chars().count(), 2, "⚠️ is a base char plus VS-16");
        assert_eq!("✅".chars().count(), 1, "✅ needs no selector");
        assert_eq!(render_marker(true, "⚠️", "  ", "!"), "⚠️  ");
    }

    /// The help text is the only place enter-to-confirm is mentioned.
    #[test]
    fn multiselect_help_mentions_how_to_confirm() {
        assert!(MULTISELECT_HELP.contains("enter to confirm"));
    }

    /// The whole point of a tally: what happened is one line, and what
    /// didn't need doing is another - never one line each.
    #[test]
    fn a_tally_reports_each_kind_on_a_single_line() {
        let mut tally = Tally::new();
        tally.did("rojo");
        tally.did("wally");
        tally.already("stylua");
        assert_eq!(
            tally.summary("rokit tools"),
            vec![
                "rokit tools: installed rojo, wally".to_string(),
                "rokit tools: stylua already present".to_string(),
            ]
        );
    }

    /// Naming a handful is useful; naming twenty is the wall of output the
    /// tally exists to prevent, so past four it becomes a count.
    #[test]
    fn a_long_already_present_list_collapses_to_a_count() {
        let mut tally = Tally::new();
        for name in ["a", "b", "c", "d"] {
            tally.already(name);
        }
        assert_eq!(tally.summary("tools"), vec!["tools: a, b, c, d already present"]);

        tally.already("e");
        assert_eq!(tally.summary("tools"), vec!["5 tools already present"]);
    }

    /// A tally with nothing in it should print nothing at all, not an empty
    /// "installed" line.
    #[test]
    fn an_empty_tally_says_nothing() {
        assert!(Tally::new().summary("tools").is_empty());
    }

    /// inquire's own summary echoes every selected option's full
    /// `key - description (badge)` line, which is unreadable past two.
    #[test]
    fn a_multi_select_summary_lists_keys_only() {
        let labels = ["react - Roact-style declarative UI library (active)".to_string(),
                      "reflex - Redux-inspired state container (active)".to_string()];
        let opts: Vec<inquire::list_option::ListOption<&String>> = labels
            .iter()
            .enumerate()
            .map(|(i, label)| inquire::list_option::ListOption::new(i, label))
            .collect();
        assert_eq!(compact_multi_answer(&opts), "2 selected: react, reflex");
        assert_eq!(compact_multi_answer(&[]), "none");
    }
}
