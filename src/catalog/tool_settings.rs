//! CLI configuration writers; the shared catalog lives in rproj-core.
use anyhow::{Context, Result, ensure};
pub use rproj_core::catalog::tool_settings::*;
use serde_json::{Value, json};

/// Renders `(setting, value)` pairs as TOML.
///
/// Top-level keys must come before any `[table]` header: in TOML every key
/// after a header belongs to that table, so emitting them in declaration
/// order would silently nest a top-level key under whatever section
/// preceded it.
pub fn render_toml(answers: &[(&SettingSpec, Value)]) -> String {
    let mut out = String::new();
    for (setting, value) in answers.iter().filter(|(s, _)| s.section.is_none()) {
        out.push_str(&format!("{} = {}\n", setting.key, toml_value(value)));
    }

    let mut sections: Vec<&str> = answers.iter().filter_map(|(s, _)| s.section).collect();
    sections.dedup();
    for section in sections {
        out.push_str(&format!("\n[{section}]\n"));
        for (setting, value) in answers.iter().filter(|(s, _)| s.section == Some(section)) {
            out.push_str(&format!("{} = {}\n", setting.key, toml_value(value)));
        }
    }
    out
}

fn toml_value(value: &Value) -> String {
    match value {
        Value::String(s) => format!("\"{}\"", escape_toml_string(s)),
        other => other.to_string(),
    }
}

/// Escapes a TOML basic string.
///
/// Every value written here currently comes from a fixed vocabulary the
/// user picks from a menu, so nothing needs escaping today - which made
/// the unescaped `format!("\"{s}\"")` safe by circumstance rather than by
/// construction. A single quote or backslash in a value produced a file
/// that did not parse.
fn escape_toml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // The remaining control characters have no short escape.
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// The `# comment` at the end of a `key = value` line, if there is one.
///
/// Scans past the `=` tracking whether it is inside a quoted value, so a
/// `#` within a string is not mistaken for the start of a comment.
fn trailing_comment(line: &str) -> Option<&str> {
    let after_eq = line.find('=')? + 1;
    let mut in_string = false;
    let mut escaped = false;
    for (i, c) in line.char_indices().skip_while(|(i, _)| *i < after_eq) {
        match c {
            _ if escaped => escaped = false,
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            '#' if !in_string => return Some(&line[i..]),
            _ => {}
        }
    }
    None
}

/// Applies `answers` to an existing config file, rewriting only the lines
/// that hold a setting the catalog describes and leaving everything else -
/// comments, blank lines, keys and whole tables rproj knows nothing about -
/// byte for byte as it found them.
///
/// Re-rendering the file from the catalog instead was silently destructive.
/// A scaffolded `selene.toml` carries an `exclude` key that no `SettingSpec`
/// describes, and `rproj configure selene` deleted it - putting the project
/// straight back to the 2335-findings lint run that `exclude` exists to
/// prevent. Verified by running configure against a scaffolded project:
/// `exclude` was gone from the file afterwards.
///
/// Line-based rather than parse-and-reserialise so comments survive, which
/// also makes accepting every default a byte-exact no-op.
pub fn merge_toml(existing: &str, answers: &[(&SettingSpec, Value)]) -> String {
    if existing.trim().is_empty() {
        return render_toml(answers);
    }

    let mut out = String::new();
    let mut section: Option<String> = None;
    let mut replaced = vec![false; answers.len()];

    for line in existing.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            section = Some(name.trim().to_string());
            out.push_str(line);
            out.push('\n');
            continue;
        }

        let key = match trimmed.split_once('=') {
            Some((key, _)) if !trimmed.starts_with('#') => key.trim(),
            _ => {
                out.push_str(line);
                out.push('\n');
                continue;
            }
        };
        match answers
            .iter()
            .position(|(spec, _)| spec.key == key && spec.section == section.as_deref())
        {
            Some(i) => {
                replaced[i] = true;
                // The value is rewritten; anything the user wrote after it
                // is theirs. Without this a trailing `# why` was dropped
                // even when the value did not change, so "accepting every
                // default is a byte-exact no-op" held for comments on
                // their own lines and quietly failed for these.
                let comment = trailing_comment(line)
                    .map(|c| format!("   {c}"))
                    .unwrap_or_default();
                out.push_str(&format!("{key} = {}{comment}\n", toml_value(&answers[i].1)));
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }

    // A setting the file didn't carry yet - a new entry in the catalog, or a
    // hand-written config that only set some of them.
    for (i, (setting, value)) in answers.iter().enumerate() {
        if replaced[i] {
            continue;
        }
        let line = format!("{} = {}", setting.key, toml_value(value));
        out = match setting.section {
            None => insert_top_level(&out, &line),
            Some(section) => insert_into_section(&out, section, &line),
        };
    }
    out
}

pub(crate) fn checked_toml_merge(
    existing: &str,
    answers: &[(&SettingSpec, Value)],
) -> Result<String> {
    let mut expected = toml::from_str::<toml::Table>(existing)
        .context("could not parse current TOML; edit the file manually and try again")?;
    for (setting, value) in answers {
        let table = match setting.section {
            Some(section) => expected
                .entry(section)
                .or_insert_with(|| toml::Value::Table(toml::Table::new()))
                .as_table_mut()
                .with_context(|| format!("{section} is not a TOML table"))?,
            None => &mut expected,
        };
        table.insert(setting.key.into(), toml::Value::try_from(value)?);
    }
    let merged = merge_toml(existing, answers);
    let actual = toml::from_str::<toml::Table>(&merged)
        .context("cannot safely edit this TOML layout; edit the file manually")?;
    // The line writer must not change content inside multiline values or other tables.
    ensure!(
        actual == expected,
        "cannot safely edit this TOML layout without changing other values; edit the file manually"
    );
    Ok(merged)
}

/// Adds `line` directly under `[section]`, creating the table at the end of
/// the file if it isn't there. Appending to the file blindly would put the
/// key in whichever table happened to come last.
fn insert_into_section(toml: &str, section: &str, line: &str) -> String {
    let header = format!("[{section}]");
    match toml.lines().position(|l| l.trim() == header) {
        Some(i) => {
            let mut lines: Vec<&str> = toml.lines().collect();
            lines.insert(i + 1, line);
            format!("{}\n", lines.join("\n"))
        }
        None => format!("{toml}\n{header}\n{line}\n"),
    }
}

/// What each of `tool`'s settings is currently set to in `existing`.
///
/// `rproj configure` offers these as the prompt defaults. Offering the
/// catalog defaults instead meant pressing enter through the walkthrough
/// reverted every choice already made - including `std = "roblox+testez"`,
/// which a TestEZ project needs and which reverting breaks.
pub fn current_toml_values(tool: &ConfigurableTool, existing: &str) -> Vec<Option<Value>> {
    // A whole document, not a single value: `str::parse::<toml::Value>`
    // parses one value and rejects the rest of the file.
    let Ok(doc) = toml::from_str::<toml::Table>(existing) else {
        return tool.settings.iter().map(|_| None).collect();
    };
    tool.settings
        .iter()
        .map(|setting| {
            let value = match setting.section {
                Some(section) => doc.get(section)?.as_table()?.get(setting.key)?,
                None => doc.get(setting.key)?,
            };
            toml_to_json(value)
        })
        .collect()
}

fn toml_to_json(value: &toml::Value) -> Option<Value> {
    // Unsupported shapes still count as present, so configure can keep them without rewriting.
    serde_json::to_value(value).ok()
}

/// The config file a tool gets at scaffold time: every setting at its
/// documented default, with `overrides` applied by key.
///
/// This exists so the scaffolded file and `rproj configure` can't disagree.
/// They used to: the scaffold hardcoded `indent_type = "Spaces"` and no
/// `syntax`, while configure's defaults were `Tabs` and `Luau` - so
/// running configure and accepting every default silently reformatted the
/// whole project. One table now feeds both.
pub fn default_toml(tool_key: &str, overrides: &[(&str, &str)]) -> Option<String> {
    let tool = find(tool_key)?;
    if !matches!(tool.target, ConfigTarget::ProjectToml { .. }) {
        return None;
    }
    let answers: Vec<(&SettingSpec, Value)> = tool
        .settings
        .iter()
        .map(|setting| {
            let value = overrides
                .iter()
                .find(|(key, _)| *key == setting.key)
                .map(|(_, v)| json!(v))
                .unwrap_or_else(|| setting.kind.default_value());
            (setting, value)
        })
        .collect();
    Some(render_toml(&answers))
}

/// Inserts a raw top-level `key = value` line before the first `[section]`
/// header.
///
/// Appending would be wrong: in TOML every key after a header belongs to
/// that table, so an `exclude` tacked onto the end of a file with a
/// `[rules]` section becomes `rules.exclude` and is silently ignored.
pub fn insert_top_level(toml: &str, line: &str) -> String {
    // A file whose very first line is a header has no `\n[` to find, and
    // appending would drop the key into whichever table ends the file.
    if toml.starts_with('[') {
        return format!("{line}\n\n{toml}");
    }
    match toml.find("\n[") {
        // `i` is the newline immediately before the header. Slicing
        // through it keeps the blank line that already separated the
        // sections, and the trailing `\n\n` restores one after the
        // inserted key so the header isn't left crowded against it.
        Some(i) => format!("{}{line}\n\n{}", &toml[..=i], &toml[i + 1..]),
        None => format!("{toml}{line}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A setting used only by the tests below, so they do not depend on
    /// which real settings happen to exist.
    const A_TOP_LEVEL: SettingSpec = SettingSpec {
        key: "indent_type",
        description: "for the tests below",
        section: None,
        kind: SettingKind::Choice {
            default: "Tabs",
            options: &[],
        },
    };

    /// Accepting the value a file already holds must not disturb the
    /// line - including a comment the user wrote after it.
    ///
    /// This failed before: a matched line was replaced with a canonical
    /// `key = value`, so `indent_type = "Tabs"   # agreed with the team`
    /// came back without the reason.
    #[test]
    fn a_trailing_comment_on_a_setting_line_survives() {
        let existing = "indent_type = \"Tabs\"   # agreed with the team\n";
        let merged = merge_toml(existing, &[(&A_TOP_LEVEL, json!("Tabs"))]);
        assert_eq!(merged, existing, "same value in, same bytes out");
    }

    #[test]
    fn a_trailing_comment_survives_a_changed_value() {
        let merged = merge_toml(
            "indent_type = \"Tabs\"   # agreed with the team\n",
            &[(&A_TOP_LEVEL, json!("Spaces"))],
        );
        assert_eq!(
            merged,
            "indent_type = \"Spaces\"   # agreed with the team\n"
        );
    }

    /// A `#` inside the value is part of the value, not a comment.
    #[test]
    fn a_hash_inside_a_string_value_is_not_a_comment() {
        assert_eq!(trailing_comment("key = \"a # b\""), None);
        assert_eq!(trailing_comment("key = \"a\" # b"), Some("# b"));
        assert_eq!(trailing_comment("key = \"a \\\" # b\""), None);
        assert_eq!(trailing_comment("key = 1"), None);
    }

    /// A quote or backslash in a value used to produce a file that did
    /// not parse. Every value is written through the escaper now.
    #[test]
    fn a_quote_in_a_value_produces_parseable_toml() {
        let merged = merge_toml("", &[(&A_TOP_LEVEL, json!("a\"b\\c"))]);
        let parsed: toml::Table = toml::from_str(&merged).expect("valid TOML");
        assert_eq!(parsed["indent_type"].as_str(), Some("a\"b\\c"));
    }

    #[test]
    fn control_characters_are_escaped_too() {
        assert_eq!(escape_toml_string("a\nb\tc"), "a\\nb\\tc");
        assert_eq!(escape_toml_string("a\u{1}b"), "a\\u0001b");
        assert_eq!(escape_toml_string("plain"), "plain");
    }

    /// The scaffolded file and `rproj configure` accepting every default
    /// must produce the same thing, or configure silently reformats the
    /// project the first time it's run.
    #[test]
    fn scaffolded_defaults_match_configure_defaults() {
        for tool in CONFIGURABLE_TOOLS {
            let ConfigTarget::ProjectToml { .. } = tool.target else {
                continue;
            };
            let scaffolded = default_toml(tool.key, &[]).expect("project-toml tool renders");
            let answers: Vec<(&SettingSpec, Value)> = tool
                .settings
                .iter()
                .map(|s| (s, s.kind.default_value()))
                .collect();
            assert_eq!(scaffolded, render_toml(&answers), "{} diverged", tool.key);
        }
    }

    /// Luau type annotations are a parse error under StyLua's default
    /// `syntax = "All"`, so the scaffolded config has to set it.
    #[test]
    fn stylua_defaults_select_luau_syntax() {
        let toml = default_toml("stylua", &[]).unwrap();
        assert!(toml.contains(r#"syntax = "Luau""#), "{toml}");
    }

    /// Selene reports every Roblox global as undefined without the right
    /// std, and TestEZ projects need its globals too.
    #[test]
    fn selene_std_can_be_overridden_for_testez() {
        let plain = default_toml("selene", &[]).unwrap();
        assert!(plain.contains(r#"std = "roblox""#), "{plain}");

        let testez = default_toml("selene", &[("std", "roblox+testez")]).unwrap();
        assert!(testez.contains(r#"std = "roblox+testez""#), "{testez}");
    }

    /// A top-level key emitted after a [table] header silently becomes
    /// part of that table.
    #[test]
    fn top_level_keys_precede_any_section_header() {
        let toml = default_toml("selene", &[]).unwrap();
        let first_header = toml.find('[').expect("selene has a [rules] section");
        assert!(
            toml[..first_header].contains("std ="),
            "std must appear before the first header:\n{toml}"
        );
    }

    /// An inserted key landing after a header silently becomes part of
    /// that table - `exclude` would turn into `rules.exclude` and do
    /// nothing at all.
    #[test]
    fn inserted_keys_land_above_the_first_section() {
        let base = default_toml("selene", &[]).unwrap();
        let with_exclude = insert_top_level(&base, r#"exclude = ["vendor/**"]"#);
        let first_header = with_exclude
            .find(
                "
[",
            )
            .expect("selene has a [rules] section");
        assert!(
            with_exclude[..first_header].contains("exclude ="),
            "{with_exclude}"
        );
        assert!(
            with_exclude.contains("[rules]"),
            "sections must survive:
{with_exclude}"
        );
        assert!(
            with_exclude.contains(
                "

[rules]"
            ),
            "the header should keep a blank line above it:
{with_exclude}"
        );
    }

    /// A file with no sections at all still has to get the key.
    #[test]
    fn inserted_keys_work_without_any_section() {
        let out = insert_top_level(
            "std = \"roblox\"
",
            "exclude = []",
        );
        assert!(out.contains("exclude = []"), "{out}");
    }

    /// Answers built from what the file already says, i.e. what `rproj
    /// configure` sends to the writer when you press enter through it.
    fn enter_through(
        tool: &'static ConfigurableTool,
        existing: &str,
    ) -> Vec<(&'static SettingSpec, Value)> {
        let current = current_toml_values(tool, existing);
        tool.settings
            .iter()
            .zip(current)
            .map(|(spec, current)| (spec, current.unwrap_or_else(|| spec.kind.default_value())))
            .collect()
    }

    /// The scaffolded `exclude` is what keeps selene out of `Packages/`.
    /// Configure used to re-render the file from the catalog, which deleted
    /// it and put the project back to thousands of vendored-code findings.
    #[test]
    fn merging_keeps_keys_the_catalog_does_not_describe() {
        let tool = find("selene").unwrap();
        let existing = default_toml("selene", &[]).unwrap();
        let existing = insert_top_level(&existing, r#"exclude = ["Packages/**"]"#);

        let merged = merge_toml(&existing, &enter_through(tool, &existing));
        assert!(merged.contains(r#"exclude = ["Packages/**"]"#), "{merged}");
    }

    /// Accepting every prompt has to be a no-op. Anything else means the
    /// walkthrough rewrites your project just for being opened.
    #[test]
    fn enter_through_configure_changes_nothing() {
        for tool in CONFIGURABLE_TOOLS {
            let ConfigTarget::ProjectToml { .. } = tool.target else {
                continue;
            };
            // A file carrying every kind of thing configure has to preserve:
            // a comment, an unmanaged top-level key, a non-default managed
            // value, an unmanaged key inside a managed table, and an
            // unmanaged table.
            let mut existing = default_toml(tool.key, &[]).unwrap();
            existing = insert_top_level(&existing, "# hand-written\nunmanaged_top = 7");
            existing.push_str("\n[unmanaged_table]\nkey = \"value\"\n");

            let merged = merge_toml(&existing, &enter_through(tool, &existing));
            assert_eq!(merged, existing, "{} rewrote its config", tool.key);
        }
    }

    /// A changed answer has to actually land, and only on its own line.
    #[test]
    fn merging_rewrites_only_the_answered_line() {
        let tool = find("selene").unwrap();
        let existing = "# keep me\nstd = \"roblox\"\nexclude = [\"Packages/**\"]\n\n[rules]\nshadowing = \"warn\"\n";
        let mut answers = enter_through(tool, existing);
        for (spec, value) in answers.iter_mut() {
            if spec.key == "shadowing" {
                *value = json!("allow");
            }
        }
        let merged = merge_toml(existing, &answers);
        assert!(merged.contains("# keep me"), "{merged}");
        assert!(merged.contains(r#"exclude = ["Packages/**"]"#), "{merged}");
        assert!(merged.contains(r#"shadowing = "allow""#), "{merged}");
        assert!(!merged.contains(r#"shadowing = "warn""#), "{merged}");
    }

    /// A setting the file predates has to be added under its own table,
    /// not appended wherever the file happens to end.
    #[test]
    fn a_missing_sectioned_setting_lands_under_its_own_header() {
        let tool = find("selene").unwrap();
        let existing = "std = \"roblox\"\n";
        let merged = merge_toml(existing, &enter_through(tool, existing));
        let parsed = toml::from_str::<toml::Table>(&merged).expect("still valid TOML");
        assert_eq!(
            parsed["rules"]["shadowing"].as_str(),
            Some("warn"),
            "{merged}"
        );
        assert_eq!(parsed["std"].as_str(), Some("roblox"), "{merged}");
    }

    /// An empty or absent file has nothing to merge into.
    #[test]
    fn merging_into_nothing_renders_the_defaults() {
        let tool = find("stylua").unwrap();
        let answers = enter_through(tool, "");
        assert_eq!(
            merge_toml("", &answers),
            default_toml("stylua", &[]).unwrap()
        );
    }

    /// Pressing enter through `rproj configure` proposes these, so reading
    /// them back wrong is how a project's own settings get reverted.
    #[test]
    fn current_values_come_from_the_file_not_the_catalog() {
        let tool = find("selene").unwrap();
        let existing = "std = \"roblox+testez\"\n\nexclude = [\"Packages/**\"]\n\n[rules]\nunused_variable = \"allow\"\n";
        let current = current_toml_values(tool, existing);
        assert_eq!(current.len(), tool.settings.len());

        let by_key = |key: &str, section: Option<&str>| {
            let i = tool
                .settings
                .iter()
                .position(|s| s.key == key && s.section == section)
                .expect("setting exists");
            current[i].clone()
        };
        assert_eq!(by_key("std", None), Some(json!("roblox+testez")));
        assert_eq!(
            by_key("unused_variable", Some("rules")),
            Some(json!("allow"))
        );
        // Not in the file: the catalog default has to fill in.
        assert_eq!(by_key("shadowing", Some("rules")), None);
    }

    /// Every tool's key must be resolvable, or `rproj configure <key>` and
    /// the `rproj info` cross-link point at nothing.
    #[test]
    fn every_configurable_tool_is_findable_by_key() {
        for tool in CONFIGURABLE_TOOLS {
            assert!(find(tool.key).is_some(), "{} not findable", tool.key);
        }
    }

    #[test]
    fn every_configuration_requirement_names_a_real_artifact() {
        for tool in CONFIGURABLE_TOOLS {
            assert!(
                !tool.requires_artifacts.is_empty(),
                "{} applies everywhere",
                tool.key
            );
            for key in tool.requires_artifacts {
                assert!(
                    crate::catalog::artifacts::find(key).is_some(),
                    "{} requires unknown artifact {key}",
                    tool.key
                );
            }
        }
    }
}
