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
/// Every icon here is a single scalar value with no VARIATION SELECTOR, so
/// all three are unambiguously double-width and take one space of padding.
///
/// `warn` used to be `⚠️` with *two* spaces, on the theory that a base
/// character plus VS-16 renders one column narrower. Measured with
/// `unicode-width`: VS-16 requests emoji presentation, which is wide, so
/// the standard makes it two columns and the extra space put warn one
/// column right of its neighbours. The legacy console ignores VS-16 and
/// draws it narrow, where the extra space was right - and nothing the
/// program can query tells the two apart.
///
/// `❗` (U+2757) removes the question: one char, two columns, no selector,
/// same as `✅` and `➖`. Alignment is now a property of the glyphs rather
/// than a guess about the terminal.
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
    println!("  {}{msg}", marker("❗", " ", "!"));
}

/// A failure that ends the run, with its full cause chain.
///
/// Printed here rather than by Rust's default `Termination` impl for
/// `Result`, which writes an uncoloured `Error: ` followed by the chain in
/// `Debug` form. Red, because the one line the user must not miss should
/// not look like the twenty `✅` lines above it.
///
/// Colour goes through `anstream`, so it is stripped automatically when
/// stderr is redirected to a file or a CI log rather than emitting escape
/// codes into it. `NO_COLOR` and `CLICOLOR=0` are honoured for free.
pub fn error(err: &anyhow::Error) {
    use anstyle::{AnsiColor, Color, Style};
    let red = Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::Red)))
        .bold();
    let reset = red.render_reset();
    let red = red.render();

    anstream::eprintln!("\n{red}{}{}{reset}", marker("❗", " ", "!"), err);
    // Each `with_context` layer, innermost last - the OS or library error
    // that actually stopped things is the deepest one.
    for cause in err.chain().skip(1) {
        anstream::eprintln!("      {cause}");
    }
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

/// Collapses a run of same-kind outcomes into one wrapped list.
///
/// Nine separate `already added globally` lines bury the one tool that
/// actually needed attention. One line naming all nine does not - and it
/// still answers *which*, which a bare count does not (see
/// `summary_within`).
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
        self.summary_within(noun, option_budget())
    }

    /// The pure half, taking the line budget instead of reading the
    /// terminal - so the wrapping rule is testable at a fixed width.
    ///
    /// **Names every item, wrapping rather than collapsing to a count.**
    /// This used to print `5 system apps already present` past four items,
    /// which answers "how many" when the question is "which". The wall of
    /// output the tally exists to prevent was one line *per item* - nine
    /// `✅ ... already added globally` lines - and a list wrapped across two
    /// lines is not that.
    pub fn summary_within(&self, noun: &str, budget: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.done.is_empty() {
            lines.extend(wrap_list(
                &format!("{noun}: installed "),
                &self.done,
                " ",
                budget,
            ));
        }
        if !self.already.is_empty() {
            lines.extend(wrap_list(
                &format!("{noun}: "),
                &self.already,
                " already present",
                budget,
            ));
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

/// `prefix` then a comma-separated list then `suffix`, wrapped to `budget`
/// columns with a hanging indent on continuation lines.
///
/// Returns one string per output line. Never breaks an item across lines:
/// an item longer than the budget gets its own line and overflows, because
/// a truncated tool name is worse than a long line.
fn wrap_list(prefix: &str, items: &[String], suffix: &str, budget: usize) -> Vec<String> {
    const HANGING_INDENT: &str = "  ";
    let mut lines = Vec::new();
    let mut current = prefix.to_string();
    let mut count_on_line = 0;

    for (i, item) in items.iter().enumerate() {
        let last = i + 1 == items.len();
        let piece = if last {
            format!("{item}{suffix}")
        } else {
            format!("{item}, ")
        };
        // Start a new line when this item would overflow - unless nothing is
        // on the line yet, in which case overflowing is the only option.
        if count_on_line > 0 && display_width(&current) + display_width(&piece) > budget {
            lines.push(current.trim_end().to_string());
            current = HANGING_INDENT.to_string();
            count_on_line = 0;
        }
        current.push_str(&piece);
        count_on_line += 1;
    }
    if !current.trim().is_empty() {
        lines.push(current.trim_end().to_string());
    }
    lines
}

/// Options in every picker are rendered as `key - description (badge)`, so
/// the key is the part before the first " - ". These helpers keep that
/// format in one place: it was previously encoded in seven separate
/// closures across three modules, where changing the separator would have
/// silently broken selection matching rather than failing to compile.
pub const OPTION_SEPARATOR: &str = " - ";

/// Renders one picker option as `key - description (badge)`, truncated so
/// it cannot wrap in the current terminal.
///
/// **Truncation is not cosmetic - it is what stops the picker corrupting
/// itself.** inquire redraws its list by moving the cursor up by the number
/// of options it rendered, one row per option. A line longer than the
/// terminal is wrapped by the terminal into two or more rows, so the cursor
/// moves up too few rows and each redraw overwrites the wrong region: after
/// a few arrow keys the list is unreadable garbage. Reproduced in a 66-column
/// terminal, where descriptions of 71-216 characters wrap to 2-4 rows each.
///
/// The key is never truncated. `option_is` matches on the `key - ` prefix,
/// so shortening it would silently break selection matching rather than
/// merely look wrong.
pub fn option_line(key: &str, description: &str, badge: &str) -> String {
    let full = format!("{key}{OPTION_SEPARATOR}{description} ({badge})");
    let budget = option_budget();
    if display_width(&full) <= budget {
        return full;
    }

    // The description is what gets shortened. The key must survive intact
    // (selection matches on it) and so must the badge - a maintenance
    // status is guidance the user is choosing on, not decoration, so
    // dropping it to save nine columns defeats the point of the picker.
    let prefix = format!("{key}{OPTION_SEPARATOR}");
    let tail = format!(" ({badge})");
    let room = budget
        .saturating_sub(display_width(&prefix))
        .saturating_sub(display_width(&tail))
        .saturating_sub(1); // the ellipsis

    if room < 8 {
        // Not enough width for a useful description. Key and badge still
        // carry the two things a decision needs.
        return format!("{key}{tail}");
    }
    format!("{prefix}{}…{tail}", truncate_to(description, room))
}

/// Columns an option line may occupy.
///
/// inquire prints its own `> [ ] ` marker ahead of the label, and a line
/// that exactly fills the terminal still wraps on some consoles, so this
/// leaves room for both.
fn option_budget() -> usize {
    const MARKER_AND_MARGIN: usize = 8;
    const FALLBACK: usize = 100;
    let columns = crossterm::terminal::size()
        .map(|(cols, _)| cols as usize)
        // Not a terminal (piped, or a CI log): nothing wraps, so a
        // generous fixed width keeps descriptions intact.
        .unwrap_or(FALLBACK);
    columns.saturating_sub(MARKER_AND_MARGIN).max(24)
}

/// How many options a picker may draw at once.
///
/// The vertical twin of `option_line`'s problem, and the same mechanism.
/// inquire redraws by moving the cursor up one row per rendered option; if
/// the block is taller than the terminal, the top rows scroll off, the cursor
/// arithmetic addresses rows that no longer exist, and the list corrupts.
/// inquire's own default of 7 is safe on any terminal but wastes a tall one,
/// which matters for `rproj info`, where the whole point is browsing a
/// catalog of eighty entries.
///
/// Capped at 15 regardless of height: a list longer than that is faster to
/// filter by typing than to scroll.
pub fn page_size() -> usize {
    // Prompt line, help line, the answer line inquire leaves behind, margin.
    const CHROME: usize = 5;
    const FALLBACK: usize = 10;
    crossterm::terminal::size()
        .map(|(_, rows)| (rows as usize).saturating_sub(CHROME))
        .unwrap_or(FALLBACK)
        .clamp(5, 15)
}

fn display_width(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}

/// Truncates to at most `width` display columns, never mid-character.
fn truncate_to(s: &str, width: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for c in s.chars() {
        let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if used + w > width {
            break;
        }
        out.push(c);
        used += w;
    }
    out.trim_end().to_string()
}

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

    /// A truncated option line must still select correctly.
    ///
    /// This is the assertion that makes truncation safe: `option_is`
    /// matches the `key - ` prefix, so shortening the line from the end
    /// leaves matching intact. Truncating from the front, or shortening
    /// the key to fit, would break selection silently - the user picks
    /// one package and gets another, or none.
    #[test]
    fn a_truncated_option_line_still_matches_its_key() {
        let long = "Immutable data utility library for tables/arrays (Llama-style helpers) -                     no longer actively maintained upstream, but stable and widely used";
        let line = option_line("sift", long, "stable");

        assert!(option_is(&line, "sift"), "{line}");
        assert_eq!(option_key(&line), "sift", "{line}");
        assert!(!option_is(&line, "sif"), "a shorter key must not match");
        assert!(
            line.ends_with("(stable)"),
            "the maintenance badge survives truncation - it is what the user              is choosing on: {line}"
        );
    }

    /// Even with no room for a description, the two things a decision needs
    /// are still there.
    #[test]
    fn a_very_narrow_budget_keeps_the_key_and_the_badge() {
        let line = option_line("wally-package-types", &"x".repeat(200), "active");
        assert!(option_is(&line, "wally-package-types") || line.starts_with("wally-package-types"));
        assert!(line.contains("(active)"), "{line}");
    }

    /// The property the picker depends on: no option line can wrap.
    #[test]
    fn every_option_line_fits_the_budget() {
        let budget = option_budget();
        for (key, desc) in [
            ("t", "Runtime type checker - validates values (e.g. RemoteEvent payloads) against type definitions"),
            ("testez", &"very long description ".repeat(12)),
            ("sift", "Immutable data utility library for tables/arrays (Llama-style helpers) - no longer actively maintained upstream, but stable and widely used"),
            ("janitor", "Cleanup"),
        ] {
            let line = option_line(key, desc, "stable");
            assert!(
                display_width(&line) <= budget,
                "{key} renders {} columns, budget {budget}: {line}",
                display_width(&line)
            );
        }
    }

    /// A short description is left exactly as it was - truncation must not
    /// touch lines that already fit.
    #[test]
    fn a_short_option_line_is_untouched() {
        let line = option_line("janitor", "Cleanup utility", "active");
        assert_eq!(line, "janitor - Cleanup utility (active)");
    }

    /// Truncation happens on character boundaries and accounts for
    /// double-width characters, so a description containing them cannot
    /// overshoot the budget by a column.
    #[test]
    fn truncation_respects_display_width_not_byte_length() {
        // Each of these is one char and two columns.
        assert_eq!(truncate_to("ok", 4), "ok");
        assert!(display_width(&truncate_to("aaaaaa", 5)) <= 5);
        assert_eq!(truncate_to("abcdef", 3), "abc");
        assert_eq!(truncate_to("ab cdef", 3), "ab");
    }

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
        // character plus one space. All three markers, not just two - see
        // `no_marker_icon_carries_a_variation_selector`.
        for (icon, pad, fallback) in [("✅", " ", "+"), ("➖", " ", "-"), ("❗", " ", "!")] {
            let on = render_marker(true, icon, pad, fallback);
            let off = render_marker(false, icon, pad, fallback);
            assert_eq!(on.chars().count(), off.chars().count(), "{icon}");
        }
    }

    /// The alignment guarantee, and it is a property of the glyphs.
    ///
    /// A base character plus VARIATION SELECTOR-16 is two `char`s whose
    /// rendered width consoles disagree about, so a marker built from one
    /// cannot be aligned for every terminal. Every icon here is a single
    /// scalar with emoji presentation of its own.
    #[test]
    fn no_marker_icon_carries_a_variation_selector() {
        for icon in ["✅", "➖", "❗", "📦"] {
            assert_eq!(
                icon.chars().count(),
                1,
                "{icon} must be one scalar, not a base char plus a selector"
            );
            assert!(
                !icon.contains('\u{FE0F}'),
                "{icon} must not carry VARIATION SELECTOR-16"
            );
        }
    }

    /// The help text is the only place enter-to-confirm is mentioned.
    #[test]
    fn multiselect_help_mentions_how_to_confirm() {
        assert!(MULTISELECT_HELP.contains("enter to confirm"));
    }

    /// Every item is named, so a user can see *which* - the complaint the
    /// old count-collapse produced ("5 system apps already present" when
    /// the question was which five).
    #[test]
    fn a_tally_names_every_item() {
        let mut tally = Tally::new();
        for name in ["git", "vscode", "studio", "roblox", "blender"] {
            tally.already(name);
        }
        assert_eq!(
            tally.summary_within("system apps", 200),
            vec!["system apps: git, vscode, studio, roblox, blender already present"]
        );
    }

    /// What happened and what did not need doing stay on separate lines -
    /// never one line each per item.
    #[test]
    fn a_tally_reports_each_kind_separately() {
        let mut tally = Tally::new();
        tally.did("rojo");
        tally.did("wally");
        tally.already("stylua");
        assert_eq!(
            tally.summary_within("rokit tools", 200),
            vec![
                "rokit tools: installed rojo, wally".to_string(),
                "rokit tools: stylua already present".to_string(),
            ]
        );
    }

    /// A long list wraps with a hanging indent instead of collapsing to a
    /// count, and no line exceeds the budget.
    #[test]
    fn a_long_list_wraps_rather_than_collapsing() {
        let mut tally = Tally::new();
        for name in [
            "rojo", "wally", "wally-package-types", "selene", "stylua", "lute",
            "luau-lsp-cli", "tarmac", "mantle",
        ] {
            tally.already(name);
        }
        let lines = tally.summary_within("rokit tools", 60);

        assert!(lines.len() > 1, "should wrap: {lines:?}");
        for line in &lines {
            assert!(display_width(line) <= 60, "line too wide: {line:?}");
        }
        // Every tool is still named somewhere.
        let joined = lines.join(" ");
        for name in ["rojo", "wally-package-types", "mantle"] {
            assert!(joined.contains(name), "{name} missing from {lines:?}");
        }
        assert!(lines[1].starts_with("  "), "hanging indent: {lines:?}");
    }

    /// An item longer than the whole budget gets its own line and overflows
    /// rather than being truncated - a cut-off tool name is worse than a
    /// long line.
    #[test]
    fn an_item_wider_than_the_budget_is_not_truncated() {
        let mut tally = Tally::new();
        tally.already("a-very-long-tool-name-that-exceeds-any-sensible-budget");
        let lines = tally.summary_within("tools", 20);
        assert!(lines.join(" ").contains("a-very-long-tool-name-that-exceeds-any-sensible-budget"));
    }

    /// A tally with nothing in it should print nothing at all, not an empty
    /// "installed" line.
    #[test]
    fn an_empty_tally_says_nothing() {
        assert!(Tally::new().summary_within("tools", 80).is_empty());
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
