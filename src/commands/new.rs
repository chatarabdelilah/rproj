use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{bail, Context, Result};
use inquire::{MultiSelect, Select};

use crate::catalog::artifacts;
use crate::catalog::capabilities;
use crate::catalog::wally_packages::{self, companions_for, Category, PackageSpec};
use crate::commands::provision;
use crate::config::{GlobalConfig, PackageWorkflow, ProjectConfig, SavedSetup};
use crate::steps::{
    blender, figma, git, gitattributes, gitignore, modules, quality, rojo, tarmac, testez,
    toolchain, vscode, wally,
};
use crate::ui;

pub fn run(name: &str, reconfigure: bool, like: Option<&str>, save_setup: Option<&str>) -> Result<()> {
    let mut config = GlobalConfig::load()?;

    let project_dir = config.projects_root()?.join(name);
    if project_dir.exists() {
        bail!("{} already exists", project_dir.display());
    }

    // Resolve --like before doing any work, so a typo'd setup name fails
    // immediately rather than after provisioning and half a scaffold.
    let saved = match like {
        Some(setup) => Some(load_setup(setup)?),
        None => None,
    };

    // Machine setup is a once-per-PC concern, so it's only asked when
    // there's nothing recorded yet (a genuinely fresh machine) or when
    // explicitly requested. Re-asking four multi-selects about winget
    // packages on every new project was pure friction: the answers are
    // almost never different, and `rproj new` is a project command.
    if reconfigure || !config.machine_configured() {
        if !config.machine_configured() {
            println!("Setting your machine up first - this only happens once.\n");
        }
        provision::run(&mut config)?;
        config.save()?;
    } else {
        ui::ok(&format!("machine ready: {}", config.machine_summary()));
        ui::detail("rproj setup to re-check, or rproj new --reconfigure to change");
    }

    std::fs::create_dir_all(&project_dir)
        .with_context(|| format!("failed to create {}", project_dir.display()))?;

    ui::section(&format!("Scaffolding {name}"));
    ui::detail(&project_dir.display().to_string());

    // The order below is the whole redesign. Each answer narrows the next,
    // and no prompt asks about a consequence of a decision made after it:
    //
    //   strategy -> packages -> capabilities -> summary
    //
    // Strategy first because it decides which packages can even be
    // vendored. It used to come *after* packages, which meant picking React
    // silently overruled the user's architecture - the one decision that
    // should be theirs was the one rproj made for them.
    let (mode, packages, package_workflow) = match saved {
        Some((setup_name, setup)) => {
            let packages: BTreeSet<String> = setup.packages.into_iter().collect();
            ui::ok(&format!(
                "using saved setup `{setup_name}`: {}",
                if packages.is_empty() { "no packages".to_string() } else { packages.iter().cloned().collect::<Vec<_>>().join(", ") }
            ));
            (format!("like:{setup_name}"), packages, setup.package_workflow)
        }
        None => {
            let workflow = pick_strategy()?;
            let (mode, packages) = match workflow {
                // Nothing to compose. Asking "which packages?" right after
                // being told there is no package manager is the same class
                // of question this redesign exists to remove.
                PackageWorkflow::None => ("none", BTreeSet::new()),
                _ => pick_composition(workflow)?,
            };
            let (workflow, packages) = reconcile_strategy(workflow, packages)?;
            (mode.to_string(), packages, workflow)
        }
    };

    // Git submodules have no dependency resolution: the scaffold clones
    // exactly the list it's given. Wally does its own resolution, so its
    // manifest is left as the user picked it.
    let mut packages = match package_workflow {
        PackageWorkflow::Wally | PackageWorkflow::None => packages,
        PackageWorkflow::GitSubmodules => {
            let resolved = wally_packages::with_dependencies(&packages);
            let added: Vec<&str> = resolved
                .iter()
                .filter(|k| !packages.contains(*k))
                .map(String::as_str)
                .collect();
            if !added.is_empty() {
                ui::ok(&format!("added required dependencies: {}", added.join(", ")));
            }
            resolved
        }
    };

    // What this project should *do*. The one prompt that replaced two: it
    // used to be asked twice at the wrong level - once as "which tools do
    // you pin" and once as "which files do you generate" - so the user had
    // to translate one intent into tool names and then into filenames, and
    // any disagreement between those two answers became a contradiction.
    let chosen_capabilities = pick_capabilities()?;
    let derived = capabilities::derive(&chosen_capabilities);
    let capability_keys: Vec<String> =
        chosen_capabilities.iter().map(|(k, _)| k.clone()).collect();

    // A capability can pull in a package - `test` brings TestEZ - which is
    // why testing left the package picker. Added after the workflow
    // resolution above because the resolution is about what the *user*
    // chose; these are consequences, and TestEZ vendors fine either way.
    for key in &derived.packages {
        packages.insert(key.clone());
    }

    // Tools before the plan, because `rokit.toml` exists only to hold them:
    // with nothing to pin there is no manifest, and the count is what its
    // summary line reports.
    //
    // Capabilities derive most of the list and the dependency strategy
    // derives the rest - Wally is not a capability, it is how packages
    // arrive. Merged here so `rokit.toml`, the check script and `rproj.toml`
    // all read one list; deriving them separately is how the project's pins
    // and its gate came to disagree about which tools existed.
    let mut project_tools = derived.tools.clone();
    for tool in strategy_tools(package_workflow, &packages) {
        if !project_tools.iter().any(|t| t == tool) {
            project_tools.push(tool.to_string());
        }
    }

    let environment = artifacts::Environment {
        apps: &config.selected_system_apps,
        extensions: &config.selected_vscode_extensions,
        strategy: strategy_of(package_workflow),
    };
    let planned = artifacts::plan(
        &environment,
        &capability_keys,
        &derived.artifacts,
        &project_tools,
        &[],
    );

    // A loop, not a branch: customizing re-plans and shows the summary
    // again. Dropping files and then being scaffolded without seeing what
    // you actually get would make the summary a formality - and the summary
    // is the whole replacement for the two prompts this redesign deleted.
    // The undropped plan is kept so customize always offers the full list -
    // otherwise dropping a file would make it unrecoverable without
    // restarting the command.
    let full_plan = planned.clone();
    let mut planned = planned;
    let mut dropped: Vec<String> = Vec::new();
    loop {
        match confirm_plan(name, &packages, package_workflow, &chosen_capabilities, &planned)? {
            Outcome::Create => break,
            Outcome::Customize => {
                dropped = customize_plan(&full_plan, &dropped)?;
                // Re-planned rather than filtered, so dropping one thing
                // still drops whatever only existed to support it.
                planned = artifacts::plan(
                    &environment,
                    &capability_keys,
                    &derived.artifacts,
                    &project_tools,
                    &dropped,
                );
            }
            Outcome::Cancel => {
                // Nothing has been written yet beyond the directory, so
                // backing out here leaves no half-project behind.
                std::fs::remove_dir_all(&project_dir).ok();
                println!("\nNothing created.");
                return Ok(());
            }
        }
    }

    let chosen_artifacts: Vec<String> = planned.iter().map(|p| p.key.to_string()).collect();
    scaffold(
        &project_dir,
        name,
        &packages,
        package_workflow,
        &project_tools,
        &chosen_artifacts,
    )?;

    // Written here rather than in `scaffold`, because it records the mode
    // and the saved-setup name, which are `run`'s knowledge. Gated on the
    // same artifact key so declining it declines it - the cost being that
    // `rproj upgrade` then has nothing to read, which its own error says.
    let writes = |key: &str| chosen_artifacts.iter().any(|k| k == key);
    if writes("rproj.toml") {
        ProjectConfig {
            mode,
            package_workflow,
            packages: packages.iter().cloned().collect(),
            // What *this project* pins, not what the machine has selected.
            // `rproj upgrade` re-renders the check script from this field, so
            // recording the machine list gave a submodule project a script
            // with Wally steps in it - for tools the project never pinned.
            tools_at_creation: project_tools.clone(),
        }
        .save_to(&project_dir)?;
    } else {
        ui::skip("rproj.toml not written, so `rproj upgrade` won't know this project");
    }

    if let Some(setup_name) = save_setup {
        let setup = SavedSetup {
            packages: packages.iter().cloned().collect(),
            package_workflow,
        };
        setup.save(setup_name)?;
        ui::ok(&format!("saved setup `{setup_name}` - reuse with `rproj new <name> --like {setup_name}`"));
    }

    // A new project is exactly when someone doesn't yet know what to run,
    // so end with the next steps rather than just "done".
    // All plain double-width emoji: 🛡 and friends need a variation
    // selector to render as emoji at all, and land narrow without one.
    let (party, folder, eye, gate, book) = match ui::emoji_enabled() {
        true => ("🎉  ", "📁", "👀", "🧪", "📖"),
        false => ("", " ", " ", " ", " "),
    };
    println!(
        "\n{party}{name} is ready.\n\n\
         \x20 {folder}  cd {}\n\
         \x20 {eye}  rproj watch          start the dev loop (Rojo sourcemap watcher)\n\
         \x20 {gate}  lute run check       run the quality gate (types, lint, format)\n\
         \x20 {book}  rproj info <tool>    what a tool does, and the commands to use it\n",
        project_dir.display()
    );
    Ok(())
}

/// How this project's dependencies arrive.
///
/// **Asked first, and deliberately still asked at all.** Hiding it behind a
/// flag and defaulting to Wally is technically correct - Wally is the answer
/// for anyone who does not already know otherwise - and it was rejected: a
/// summary line reading `via Wally` is a receipt, not an explanation, and
/// this is the only place a newcomer meets the concept. A default does not
/// make a question fake.
///
/// It comes before packages because it decides which packages can be
/// vendored at all. Asked after, as it used to be, selecting React silently
/// overruled the answer.
fn pick_strategy() -> Result<PackageWorkflow> {
    let choice = Select::new(
        "How should this project get its dependencies?",
        owned(&[
            "wally - Roblox's package manager. Recommended for almost every project.",
            "git-submodules - vendor each package's own repo into the project.",
            "none - no dependency manager. A tutorial project, or one you'll wire up yourself.",
        ]),
    )
    .with_formatter(&ui::compact_select_answer)
    .prompt()?;

    Ok(match ui::option_key(&choice) {
        "git-submodules" => PackageWorkflow::GitSubmodules,
        "none" => PackageWorkflow::None,
        _ => PackageWorkflow::Wally,
    })
}

/// Owns a fixed option list.
///
/// inquire types its formatter on the option type, and every picker here
/// shares one formatter (`ui::compact_select_answer`, which echoes just the
/// key). Owning the strings keeps that single formatter usable rather than
/// needing a `&str` twin of it.
fn owned(options: &[&str]) -> Vec<String> {
    options.iter().map(|s| (*s).to_string()).collect()
}

fn strategy_of(workflow: PackageWorkflow) -> artifacts::Strategy {
    match workflow {
        PackageWorkflow::Wally => artifacts::Strategy::Wally,
        PackageWorkflow::GitSubmodules => artifacts::Strategy::GitSubmodules,
        PackageWorkflow::None => artifacts::Strategy::None,
    }
}

/// What this project should *do* - and, in the badge slot, what does it.
///
/// One prompt where there used to be two ("Tools to pin" and "Files to
/// generate"), because those asked the same decision at two levels below the
/// one the user thinks in. The implementation is always named: a capability
/// that hides its tool teaches nothing about the ecosystem, and someone who
/// later asks "how do I configure this?" needs to have seen the word Selene.
///
/// Returns `(capability, implementation)` pairs. The implementation is
/// `None` wherever there is only one, which is every capability but `test`
/// once jest-lua lands - see `capabilities::needs_an_implementation_prompt`.
fn pick_capabilities() -> Result<Vec<(String, Option<String>)>> {
    let offerable: Vec<&'static capabilities::Capability> =
        capabilities::CAPABILITIES.iter().collect();

    let options: Vec<String> = offerable
        .iter()
        .map(|c| ui::option_line(c.key, c.outcome, c.default_implementation().display))
        .collect();
    let defaults: Vec<usize> = offerable
        .iter()
        .enumerate()
        .filter(|(_, c)| c.default_selected)
        .map(|(i, _)| i)
        .collect();

    let picked = MultiSelect::new("What should this project do?", options)
        .with_default(&defaults)
        .with_help_message(ui::MULTISELECT_HELP)
        .with_formatter(&ui::compact_multi_answer)
        .prompt()?;

    let mut chosen: Vec<(String, Option<String>)> = Vec::new();
    for capability in &offerable {
        if !picked.iter().any(|p| ui::option_is(p, capability.key)) {
            continue;
        }
        // A capability whose requirement was not chosen contributes nothing
        // (see `capabilities::derive`), so say so rather than letting the
        // user believe they enabled it.
        let keys: Vec<String> = chosen.iter().map(|(k, _)| k.clone()).collect();
        if !capability.requires.iter().all(|r| keys.iter().any(|k| k == r)) {
            ui::skip(&format!(
                "{} needs {} - skipping it",
                capability.key,
                capability.requires.join(", ")
            ));
            continue;
        }

        let implementation = if capability.needs_an_implementation_prompt() {
            let options: Vec<String> = capability
                .implementations
                .iter()
                .map(|i| ui::option_line(i.key, i.display, capability.key))
                .collect();
            let picked = Select::new(&format!("{}:", capability.key), options)
                .with_formatter(&ui::compact_select_answer)
                .prompt()?;
            Some(ui::option_key(&picked).to_string())
        } else {
            None
        };
        chosen.push((capability.key.to_string(), implementation));
    }
    Ok(chosen)
}

fn pick_composition(workflow: PackageWorkflow) -> Result<(&'static str, BTreeSet<String>)> {
    // Said *before* the picker rather than after the selection. Under git
    // submodules the react family cannot be vendored at all, and silently
    // omitting them is how a user ends up wondering where React went.
    let unavailable = unvendorable_keys();
    if workflow == PackageWorkflow::GitSubmodules && !unavailable.is_empty() {
        ui::detail(&format!(
            "Not listed: {}.\nUpstream ships these only through an npm install step, which git\nsubmodules can't reproduce.",
            unavailable.join(", ")
        ));
    }

    let mode = Select::new(
        "How do you want to pick packages?",
        owned(&[
            "guided - one question per category, with explanations (recommended)",
            "expert - one flat list of everything, no hand-holding",
        ]),
    )
    .with_formatter(&ui::compact_select_answer)
    .prompt()?;

    if ui::option_key(&mode) == "guided" {
        Ok(("guided", pick_guided(workflow)?))
    } else {
        Ok(("expert", pick_expert(workflow)?))
    }
}

/// Packages a capability brings in on its own, so no picker offers them.
///
/// TestEZ is the case: *do I want tests* is a capability question, and it
/// used to be asked in the package step **and** consequenced four prompts
/// later in the files step - one intent, two prompts. Derived from the
/// capability catalog rather than flagged on the package, so the two cannot
/// drift apart.
fn capability_owned(key: &str) -> bool {
    capabilities::CAPABILITIES
        .iter()
        .any(|c| c.implementations.iter().any(|i| i.packages.contains(&key)))
}

/// Keys with no vendorable source, for the note above the package picker.
fn unvendorable_keys() -> Vec<&'static str> {
    wally_packages::PACKAGES
        .iter()
        .filter(|p| p.submodule.is_none() && !capability_owned(p.key))
        .map(|p| p.key)
        .collect()
}

/// Whether a package may be offered at all, given the strategy already
/// chosen. Offering one that cannot be installed is offering a broken
/// project.
fn offerable_package(spec: &PackageSpec, workflow: PackageWorkflow) -> bool {
    !capability_owned(spec.key)
        && (workflow != PackageWorkflow::GitSubmodules || spec.submodule.is_some())
}

/// The summary, and the last chance to back out.
///
/// Not a picker. Every line here is already determined by an earlier answer,
/// so asking about them again would be the defect this redesign removed -
/// and every line carries **why**, because files are the one thing a
/// beginner will actually open and edit, and a list with no reasons is a
/// receipt rather than an explanation.
fn confirm_plan(
    name: &str,
    packages: &BTreeSet<String>,
    workflow: PackageWorkflow,
    chosen: &[(String, Option<String>)],
    planned: &[artifacts::Planned],
) -> Result<Outcome> {
    println!();
    for line in summary_lines(name, packages, workflow, chosen, planned) {
        println!("{line}");
    }
    println!();

    let choice = Select::new(
        "Create it?",
        owned(&[
            "create - go ahead and scaffold this",
            "customize - drop individual files first",
            "cancel - change nothing and exit",
        ]),
    )
    .with_formatter(&ui::compact_select_answer)
    .prompt()?;
    match ui::option_key(&choice) {
        "create" => Ok(Outcome::Create),
        "customize" => Ok(Outcome::Customize),
        _ => Ok(Outcome::Cancel),
    }
}

/// What the user chose at the summary.
#[derive(PartialEq, Eq)]
enum Outcome {
    Create,
    Customize,
    Cancel,
}

/// The escape hatch, and the reason the summary can be a summary.
///
/// Every line on it is already determined, so re-asking about them would be
/// the defect this redesign removed. But "determined" is not "immutable":
/// the housekeeping entries in particular (`rproj.toml`, `.gitignore`) are
/// written for every project on the grounds that nine users in ten want
/// them, and the tenth needs a way out. **Without this the truly bare
/// project - `src/` and `default.project.json`, nothing else - stops being
/// reachable**, which is the property the whole artifact model exists for.
///
/// Not a prompt everyone answers: one keystroke past it for the nine, one
/// extra screen for the tenth.
fn customize_plan(
    full_plan: &[artifacts::Planned],
    dropped: &[String],
) -> Result<Vec<String>> {
    let droppable: Vec<&artifacts::Planned> = full_plan
        .iter()
        .filter(|p| artifacts::find(p.key).is_some_and(|a| !a.mandatory))
        .collect();
    if droppable.is_empty() {
        return Ok(Vec::new());
    }

    let options: Vec<String> = droppable
        .iter()
        .map(|p| ui::option_line(p.key, &p.reason.describe(), "keep"))
        .collect();
    // Pre-checked to the current state rather than to "all", so re-entering
    // shows what you already decided instead of silently undoing it.
    let keep_now: Vec<usize> = droppable
        .iter()
        .enumerate()
        .filter(|(_, p)| !dropped.iter().any(|d| d == p.key))
        .map(|(i, _)| i)
        .collect();

    let kept = MultiSelect::new("Files to keep", options)
        .with_default(&keep_now)
        .with_help_message(ui::MULTISELECT_HELP)
        .with_formatter(&ui::compact_multi_answer)
        .prompt()?;

    Ok(droppable
        .iter()
        .filter(|p| !kept.iter().any(|k| ui::option_is(k, p.key)))
        .map(|p| p.key.to_string())
        .collect())
}

/// The summary as text. Pure, so the wording is a test rather than something
/// only visible by running the whole flow.
fn summary_lines(
    name: &str,
    packages: &BTreeSet<String>,
    workflow: PackageWorkflow,
    chosen: &[(String, Option<String>)],
    planned: &[artifacts::Planned],
) -> Vec<String> {
    let mut lines = vec![format!("  {name}"), String::new()];

    let strategy = match workflow {
        PackageWorkflow::Wally => "Wally",
        PackageWorkflow::GitSubmodules => "git submodules",
        PackageWorkflow::None => "none",
    };
    lines.push(format!("  {:<14}{strategy}", "Dependencies"));
    lines.push(format!(
        "  {:<14}{}",
        "Packages",
        if packages.is_empty() {
            "none".to_string()
        } else {
            packages.iter().cloned().collect::<Vec<_>>().join(", ")
        }
    ));

    // Capability, then the thing that provides it - the user should never
    // have to guess what "linting" actually installed.
    let does: Vec<String> = chosen
        .iter()
        .filter_map(|(key, implementation)| {
            let capability = capabilities::find(key)?;
            let implementation = implementation
                .as_deref()
                .and_then(|i| capability.implementation(i))
                .unwrap_or_else(|| capability.default_implementation());
            Some(format!("{key} ({})", implementation.display))
        })
        .collect();
    lines.push(format!(
        "  {:<14}{}",
        "Does",
        if does.is_empty() { "nothing extra".to_string() } else { does.join(", ") }
    ));

    lines.push(String::new());
    lines.push("  Creates".to_string());
    let width = planned.iter().map(|p| p.key.len()).max().unwrap_or(0);
    for entry in planned {
        lines.push(format!(
            "    {:<width$}  {}",
            entry.key,
            entry.reason.describe()
        ));
    }
    lines
}

/// The strategy the project ends up with, after checking the chosen packages
/// against it.
///
/// Some packages (the react-lua family) only ship a working module through an
/// npm/pnpm install step upstream, so raw git submodules cannot vendor them
/// at all. Checked over the **transitive closure**, not just what was picked:
/// `reactReflex` is vendorable itself and reaches for React, which is not, and
/// before this it scaffolded happily and failed at runtime in Studio with no
/// build error anywhere.
///
/// This used to run *before* the strategy was chosen and silently force
/// Wally. Now the user has already said what they want, so the conflict is
/// put to them as a **forward correction** - the same fact, offered as a
/// revision of a decision they made knowingly rather than an override
/// announced after the fact.
fn reconcile_strategy(
    workflow: PackageWorkflow,
    packages: BTreeSet<String>,
) -> Result<(PackageWorkflow, BTreeSet<String>)> {
    if workflow != PackageWorkflow::GitSubmodules || packages.is_empty() {
        return Ok((workflow, packages));
    }
    let blocked = wally_packages::unvendorable_in_closure(&packages);
    if blocked.is_empty() {
        return Ok((workflow, packages));
    }

    for (key, pulled_in_by) in &blocked {
        match pulled_in_by {
            // Names the dependent, not just the blocker: someone who never
            // picked react has no way to connect the two otherwise.
            Some(dependent) => ui::warn(&format!(
                "{dependent} requires {key}, which upstream only ships through an npm install \
                 step - git submodules can't reproduce that"
            )),
            None => ui::warn(&format!(
                "{key} only ships through an npm install step upstream, which git submodules \
                 can't reproduce"
            )),
        }
    }

    let choice = Select::new(
        "So this selection can't be vendored. What now?",
        owned(&[
            "wally - switch this project to Wally, which handles every catalog package",
            "drop - keep git submodules and remove the packages that can't be vendored",
        ]),
    )
    .with_formatter(&ui::compact_select_answer)
    .prompt()?;

    if ui::option_key(&choice) != "drop" {
        return Ok((PackageWorkflow::Wally, packages));
    }
    // Keep submodules, and actually remove what cannot be vendored - both
    // the blockers and anything that reaches them, or the project scaffolds
    // around a package whose dependency is missing.
    let removed: Vec<String> = blocked
        .iter()
        .flat_map(|(key, via)| [Some((*key).to_string()), via.map(|d| d.to_string())])
        .flatten()
        .collect();
    let kept: BTreeSet<String> = packages.iter().filter(|k| !removed.contains(k)).cloned().collect();
    ui::skip(&format!("removed: {}", {
        let mut names: Vec<&str> = removed.iter().map(String::as_str).collect();
        names.sort_unstable();
        names.dedup();
        names.join(", ")
    }));
    Ok((PackageWorkflow::GitSubmodules, kept))
}

fn pick_guided(workflow: PackageWorkflow) -> Result<BTreeSet<String>> {
    let mut packages = BTreeSet::new();

    for category in Category::ALL {
        let choices: Vec<&PackageSpec> = wally_packages::in_category(category)
            .filter(|p| p.primary_choice && offerable_package(p, workflow))
            .collect();
        // Testing empties out here, because a test runner is the
        // implementation of a capability rather than a package the user
        // picks. A category with nothing offerable is skipped rather than
        // shown as a list containing only "none".
        if choices.is_empty() {
            continue;
        }

        let options: Vec<String> = choices
            .iter()
            .map(|p| ui::option_line(p.key, p.description, p.maintenance.short_badge()))
            .collect();
        // Just the category name. The question is implied by a picker, and
        // the answer line reads as `State management: reflex` rather than
        // `State management: which do you want? reflex`.
        let prompt = format!("{}:", category.label());

        if category.allows_multiple() {
            let selected = MultiSelect::new(&prompt, options)
                .with_help_message(ui::MULTISELECT_HELP)
                .with_formatter(&ui::compact_multi_answer)
                .prompt()?;
            for spec in &choices {
                if selected.iter().any(|s| ui::option_is(s, spec.key)) {
                    packages.insert(spec.key.to_string());
                }
            }
        } else {
            // `none` **first**, because `Select` highlights index 0 and the
            // safe answer must be the resting position. It used to be
            // appended last, so pressing enter through four categories
            // handed a beginner four packages they never chose.
            let mut options = options;
            options.insert(0, "none".to_string());
            let picked = Select::new(&prompt, options).with_formatter(&ui::compact_select_answer).prompt()?;
            if picked != "none"
                // Matched on the full "key - " prefix, not a bare
                // starts_with: two keys where one prefixes the other
                // (react/reactRoblox, charm/charmSync) would otherwise
                // resolve to whichever the catalog happened to list first.
                && let Some(spec) = choices.iter().find(|p| ui::option_is(&picked, p.key))
            {
                packages.insert(spec.key.to_string());
            }
        }
    }

    add_companions(&mut packages);
    Ok(packages)
}

fn pick_expert(workflow: PackageWorkflow) -> Result<BTreeSet<String>> {
    // Same filter as the guided path. Without it the flat list would still
    // offer TestEZ, letting someone take the package with none of the test
    // layout that makes it useful - the double-ask, reintroduced through the
    // back door.
    let offerable: Vec<&PackageSpec> = wally_packages::PACKAGES
        .iter()
        .filter(|p| offerable_package(p, workflow))
        .collect();

    let options: Vec<String> = offerable
        .iter()
        .map(|p| {
            format!(
                "{} - {} [{}] ({})",
                p.key,
                p.description,
                p.category.label(),
                p.maintenance.short_badge()
            )
        })
        .collect();

    let selected = MultiSelect::new("Pick every package this project needs", options)
        .with_help_message(ui::MULTISELECT_HELP)
        .with_formatter(&ui::compact_multi_answer)
        .prompt()?;

    Ok(offerable
        .iter()
        .filter(|p| selected.iter().any(|s| ui::option_is(s, p.key)))
        .map(|p| p.key.to_string())
        .collect())
}


fn add_companions(packages: &mut BTreeSet<String>) {
    let primaries: Vec<String> = packages.iter().cloned().collect();
    let snapshot = packages.clone();
    for key in primaries {
        for companion in companions_for(&key, |k| snapshot.contains(k)) {
            packages.insert(companion.to_string());
        }
    }
}

fn scaffold(
    project_dir: &Path,
    name: &str,
    packages: &BTreeSet<String>,
    package_workflow: PackageWorkflow,
    project_tools: &[String],
    chosen_artifacts: &[String],
) -> Result<()> {
    // Which files this project gets, resolved from the selections rather
    // than decided by the order of the calls below. Everything after this
    // asks `writes(...)` instead of inventing its own condition - which is
    // how six artifacts came to be written whatever the user answered.
    // Already resolved by `pick_artifacts`, so this is a membership test
    // rather than a second resolution that could disagree with the first.
    let writes = |key: &str| chosen_artifacts.iter().any(|k| k == key);

    git::ensure_repo_init(project_dir)?;

    // Both calls sit behind the same gate. `rokit add` writes rokit.toml
    // itself, so pinning while the file was declined would have re-created it
    // anyway - the answer could not have been honoured, which is why the
    // artifact is entailed by having any tool to pin.
    //
    // Ungated, this was worse than untidy: on a machine with nine tools
    // selected and a project whose rokit.toml had been declined, `rokit add`
    // ran nine times against a directory with no manifest, rokit walked up to
    // the global one, and the project pinned **nothing** - silently, because
    // each failure is warned and continued. Verified on a real scaffold:
    // `t1/` contained no rokit.toml at all.
    if writes("rokit.toml") {
        toolchain::ensure_rokit_init(project_dir)?;
        toolchain::add_selected_tools(project_dir, project_tools)?;
    }
    if writes("selene.toml") {
        toolchain::ensure_selene_config(
            project_dir,
            packages.contains("testez"),
            package_workflow,
            wally_packages::allows_mixed_tables(packages),
        )?;
    }
    if writes("stylua.toml") {
        toolchain::ensure_stylua_config(project_dir)?;
    }

    let testez_selected = packages.contains("testez");
    // Only the Wally workflow has realms at all - a submodule checkout is
    // just files, mounted wholesale under modules/ regardless.
    let has_server_packages =
        package_workflow == PackageWorkflow::Wally && wally_packages::has_server_realm(packages);
    rojo::scaffold_project_json(
        project_dir,
        name,
        package_workflow,
        testez_selected,
        has_server_packages,
    )?;
    if writes("tests") {
        testez::ensure_test_folders(project_dir)?;
        // Part of the folder, not a separate decision: it declares TestEZ's
        // globals so the specs written a line above don't light up red in
        // the editor. It used to be its own catalog entry, i.e. a checkbox
        // asking whether you wanted the files you just asked for to work.
        testez::ensure_tests_luaurc(project_dir)?;
    }

    // default.project.json maps a $path (packages/ or modules/) that has to
    // exist before rojo will touch it at all - generating a sourcemap while
    // that folder is missing fails outright ("could not be turned into a
    // Roblox Instance"), not just incompletely. So the package install has
    // to happen, and the folder has to exist, before sourcemap generation -
    // which is why each workflow generates its own sourcemap at the end of
    // its own branch rather than sharing one call afterwards.
    match package_workflow {
        // The `writes` checks here look redundant against the `match` -
        // the artifacts require these exact workflows. They are what makes
        // the requirement live in one place rather than two, and what lets
        // a test assert every offered artifact has a gate.
        PackageWorkflow::Wally if writes("wally.toml") => {
            let package_name = format!("rproj/{}", slugify(name));
            let package_list: Vec<String> = packages.iter().cloned().collect();
            wally::ensure_wally_init(project_dir)?;
            wally::write_wally_toml(project_dir, &package_name, &package_list)?;
            // Installs, generates the sourcemap, and re-adds the exported
            // types a plain `wally install` leaves off. See `wally::sync`.
            wally::sync(project_dir)?;
        }
        PackageWorkflow::GitSubmodules if writes("modules") => {
            // Dedupe by target directory, not by package: monorepos like
            // littensy/charm back several catalog entries (charm,
            // charmSync, videCharm) from one clone.
            let mut cloned = BTreeSet::new();
            for spec in modules::vendorable(packages) {
                let sub = spec.submodule.expect("vendorable() filtered to Some");
                if cloned.insert(sub.dir) {
                    git::add_submodule(project_dir, spec.git_repo, sub.dir)?;
                }
            }
            // Both of these have to exist before the sourcemap runs below:
            // the nested project file is what stops Rojo from walking into
            // the vendored repos' own project files, and the link files are
            // what project code actually requires.
            modules::write_submodules_project(project_dir, packages)?;
            modules::write_link_files(project_dir, packages)?;
            // Now that modules/ exists, an initial sourcemap.json for
            // luau-lsp. (The Wally branch got its own inside `wally::sync`,
            // which needs it for wally-package-types.)
            if writes("sourcemap.json") {
                rojo::generate_sourcemap(project_dir)?;
            }
        }
        // Declined the dependency manifest, so there is nothing to install.
        // A project can legitimately want the tree and manage packages by
        // hand.
        _ => {
            if writes("sourcemap.json") {
                rojo::generate_sourcemap(project_dir)?;
            }
        }
    }

    // Quality gate. The check script is generated from the tools this
    // project actually selected, so it never invokes something that was
    // never installed; CI only lands if there's a script for it to run.
    // selene.toml says std = "roblox+testez", and without testez.yml to
    // resolve it selene prints "Could not find all standard library files"
    // and lints nothing at all - src/ included. Hence entailed by selene
    // being pinned rather than offered.
    if writes("testez.yml") {
        testez::ensure_selene_std(project_dir)?;
    }
    if writes("testez-companion.toml") {
        testez::ensure_companion_config(project_dir)?;
    }

    // Tells the editor which folders are vendored third-party code. Only
    // needed for the submodule workflow - luau-lsp already ignores Wally's
    // `_Index` by default, which is why that workflow never showed this.
    if writes(".vscode/settings.json") {
        vscode::ensure_project_settings(project_dir, package_workflow)?;
    }

    if writes(".luaurc") {
        quality::ensure_luaurc(project_dir)?;
    }
    // The same list the project's rokit.toml got: a check script must only
    // invoke tools this project actually pins, or CI fails on a command that
    // isn't installed.
    if writes(".lute/check.luau")
        && quality::ensure_check_script(project_dir, project_tools, testez_selected)?
    {
        // The workflow only exists to run the script, so the artifact model
        // requires one on the other - unticking the script drops the CI
        // file rather than leaving a workflow whose first command is
        // missing.
        if writes(".github/workflows/ci.yml") {
            quality::ensure_ci_workflow(project_dir, package_workflow, has_server_packages)?;
        }
        quality::lute_setup(project_dir)?;
    }

    if writes(".gitignore") {
        gitignore::ensure_entries(project_dir)?;
    }
    if writes(".gitattributes") {
        gitattributes::ensure_gitattributes(project_dir)?;
    }

    // Was `config.blender_enabled()`, which meant that selecting Blender once at
    // setup silently added a folder to every project afterwards. Now it is
    // an artifact requiring the app AND defaulting to off, so it is opt-in
    // per project.
    if writes("blender") {
        blender::scaffold_starter_scene(project_dir)?;
    }
    if writes("figma") {
        figma::scaffold_design_folder(project_dir)?;
    }
    // After figma/, so `asset_source` can see the exports folder and point
    // Tarmac at it rather than at a generic assets/ directory.
    if writes("tarmac.toml") {
        tarmac::ensure_config(project_dir, &slugify(name))?;
    }

    Ok(())
}

fn slugify(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect()
}

/// The tools the **dependency strategy** pins, as opposed to the ones
/// capabilities pin.
///
/// Wally is not a capability - nobody wants "Wally", they want their
/// packages installed - so it is derived here, from the strategy that
/// actually needs it. `wally-package-types` travels with it because a plain
/// `wally install` writes link files with no `export type` lines, so without
/// it every package silently degrades to `any`.
///
/// Empty with no packages: a manifest with nothing in it needs no installer.
/// Empty under submodules: there is no `wally.toml` to install from and no
/// thunks to retype, so pinning either would be noise in `rokit.toml`.
fn strategy_tools(workflow: PackageWorkflow, packages: &BTreeSet<String>) -> &'static [&'static str] {
    match workflow {
        PackageWorkflow::Wally if !packages.is_empty() => &["wally", "wally-package-types"],
        _ => &[],
    }
}

/// Resolves `--like`, failing with the list of what *is* available rather
/// than a bare "not found" - the names are user-chosen, so a typo is the
/// likely cause and the correction is right there.
///
/// Also re-applies the workflow guard that `pick_package_workflow` would
/// have applied interactively. A saved setup records an answer given at
/// some earlier point; a package can stop being vendorable since (or the
/// file can be edited by hand), and scaffolding a submodule project around
/// a package that has no vendorable source produces a broken tree.
fn load_setup(name: &str) -> Result<(String, SavedSetup)> {
    if let Some(mut setup) = SavedSetup::load(name)? {
        // Same transitive check as the interactive path: a saved setup can
        // name only vendorable packages and still be unbuildable because one
        // of them requires something that isn't.
        let selected: BTreeSet<String> = setup.packages.iter().cloned().collect();
        let blocked = wally_packages::unvendorable_in_closure(&selected);
        if setup.package_workflow == PackageWorkflow::GitSubmodules && !blocked.is_empty() {
            let reasons: Vec<String> = blocked
                .iter()
                .map(|(key, via)| match via {
                    Some(dependent) => format!("{key} (required by {dependent})"),
                    None => (*key).to_string(),
                })
                .collect();
            ui::warn(&format!(
                "setup `{name}` asks for git submodules, but {} can't be vendored that way - using Wally",
                reasons.join(", ")
            ));
            setup.package_workflow = PackageWorkflow::Wally;
        }

        let unknown: Vec<&str> = setup
            .packages
            .iter()
            .filter(|k| wally_packages::find(k).is_none())
            .map(String::as_str)
            .collect();
        if !unknown.is_empty() {
            ui::warn(&format!(
                "setup `{name}` names packages no longer in the catalog, skipping them: {}",
                unknown.join(", ")
            ));
            setup.packages.retain(|k| wally_packages::find(k).is_some());
        }

        return Ok((name.to_string(), setup));
    }
    let available = SavedSetup::list();
    if available.is_empty() {
        bail!(
            "no saved setup called `{name}` - none have been saved yet. \
             Add `--save-setup <name>` to a `rproj new` run to create one."
        );
    }
    bail!("no saved setup called `{name}`. Available: {}", available.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wally is derived from the strategy, not chosen as a capability - and
    /// `wally-package-types` travels with it, because a plain `wally install`
    /// writes link files with no `export type` lines and every package
    /// silently degrades to `any`.
    #[test]
    fn a_wally_project_with_packages_pins_wally_and_the_retyper() {
        let packages: BTreeSet<String> = ["reflex".to_string()].into_iter().collect();
        assert_eq!(
            strategy_tools(PackageWorkflow::Wally, &packages),
            ["wally", "wally-package-types"]
        );
    }

    /// A manifest with nothing in it needs no installer, so an empty
    /// selection pins neither - which is what lets a package-free project
    /// have no `rokit.toml` at all.
    #[test]
    fn a_wally_project_with_no_packages_pins_nothing() {
        assert!(strategy_tools(PackageWorkflow::Wally, &BTreeSet::new()).is_empty());
    }

    /// **Every package the picker offers is one the strategy can install.**
    /// Under submodules that excludes the react family, which upstream ships
    /// only through an npm step - offering them would be offering a project
    /// that breaks at runtime in Studio with no build error anywhere.
    #[test]
    fn the_package_picker_never_offers_what_the_strategy_cannot_install() {
        for spec in wally_packages::PACKAGES {
            if offerable_package(spec, PackageWorkflow::GitSubmodules) {
                assert!(
                    spec.submodule.is_some(),
                    "{} has no vendorable source but is offered under submodules",
                    spec.key
                );
            }
            // Wally installs everything, so only capability-owned entries
            // are held back there.
            assert_eq!(
                offerable_package(spec, PackageWorkflow::Wally),
                !capability_owned(spec.key),
                "{}",
                spec.key
            );
        }
    }

    /// TestEZ left the package picker: *do I want tests* is a capability
    /// question, and asking it again as a package was one intent asked
    /// twice, four prompts apart.
    #[test]
    fn the_test_runner_is_not_offered_as_a_package() {
        assert!(capability_owned("testez"), "the test capability must own it");
        let testez = wally_packages::find("testez").expect("in the catalog");
        for workflow in [PackageWorkflow::Wally, PackageWorkflow::GitSubmodules] {
            assert!(!offerable_package(testez, workflow));
        }
    }

    /// Choosing git submodules and then having Wally pinned into the
    /// project anyway is the strategy leaking across its own boundary.
    #[test]
    fn submodule_projects_do_not_pin_wally_tools() {
        let packages: BTreeSet<String> = ["charm".to_string()].into_iter().collect();
        assert!(strategy_tools(PackageWorkflow::GitSubmodules, &packages).is_empty());
        assert!(strategy_tools(PackageWorkflow::None, &packages).is_empty());
    }

    /// A machine that has never been provisioned must still get the
    /// questions; one that has must not be asked again.
    #[test]
    fn machine_is_only_configured_once() {
        let fresh = GlobalConfig::default();
        assert!(!fresh.machine_configured(), "a fresh config needs provisioning");

        let provisioned = GlobalConfig { last_checked: Some("123".into()), ..Default::default() };
        assert!(provisioned.machine_configured(), "already provisioned, don't re-ask");
    }

    /// The skip path prints this instead of the pickers, so it has to say
    /// something rather than being blank on a real config.
    #[test]
    fn machine_summary_describes_what_is_set_up() {
        let config = GlobalConfig {
            selected_system_apps: vec!["git".into(), "vscode".into()],
            selected_rokit_tools: vec!["rojo".into()],
            ..Default::default()
        };
        let summary = config.machine_summary();
        assert!(summary.contains("2 apps"), "{summary}");
        assert!(summary.contains("1 tools"), "{summary}");
        assert!(!summary.contains("plugins"), "empty groups shouldn't be listed: {summary}");
        assert_eq!(GlobalConfig::default().machine_summary(), "nothing selected");
    }

    fn chosen(keys: &[&str]) -> Vec<(String, Option<String>)> {
        keys.iter().map(|k| (k.to_string(), None)).collect()
    }

    fn plan_for(
        capability_keys: &[&str],
        workflow: PackageWorkflow,
    ) -> Vec<artifacts::Planned> {
        let selected = chosen(capability_keys);
        let derived = capabilities::derive(&selected);
        let (apps, extensions) = (Vec::new(), Vec::new());
        let environment = artifacts::Environment {
            apps: &apps,
            extensions: &extensions,
            strategy: strategy_of(workflow),
        };
        let keys: Vec<String> = capability_keys.iter().map(|k| k.to_string()).collect();
        artifacts::plan(&environment, &keys, &derived.artifacts, &derived.tools, &[])
    }

    /// **"Just the Rojo basics."** No dependency manager, nothing chosen -
    /// and the result is the source tree, the project file, and the two
    /// housekeeping entries. Nothing about the machine can change this,
    /// which is the property the old tools prompt existed to protect and
    /// this model gets for free.
    #[test]
    fn choosing_nothing_yields_only_the_basics() {
        let planned = plan_for(&[], PackageWorkflow::None);
        let keys: Vec<&str> = planned.iter().map(|p| p.key).collect();
        assert_eq!(keys, ["src", "default.project.json", "rproj.toml", ".gitignore"]);
    }

    /// Choosing a capability derives its tool *and* its file, in one answer.
    /// This is the pair the user used to have to state twice.
    #[test]
    fn one_capability_derives_both_the_tool_and_the_file() {
        let derived = capabilities::derive(&chosen(&["lint"]));
        assert_eq!(derived.tools, ["selene"]);

        let keys: Vec<&str> = plan_for(&["lint"], PackageWorkflow::None)
            .iter()
            .map(|p| p.key)
            .collect();
        assert!(keys.contains(&"selene.toml"), "{keys:?}");
    }

    /// The summary is the user-visible half, so its exact text is a test.
    /// Every file names the answer that caused it - that is what makes it an
    /// explanation rather than a receipt.
    #[test]
    fn the_summary_names_every_file_and_why_it_is_there() {
        let packages: BTreeSet<String> = ["reflex".to_string()].into_iter().collect();
        let selected = chosen(&["lint", "format"]);
        let planned = plan_for(&["lint", "format"], PackageWorkflow::Wally);
        let lines = summary_lines("MyGame", &packages, PackageWorkflow::Wally, &selected, &planned);
        let text = lines.join("\n");

        assert!(text.contains("  MyGame"), "{text}");
        assert!(text.contains("Dependencies  Wally"), "{text}");
        assert!(text.contains("Packages      reflex"), "{text}");
        // The tool is named beside the capability, never hidden behind it.
        assert!(text.contains("lint (Selene)"), "{text}");
        assert!(text.contains("format (StyLua)"), "{text}");
        assert!(text.contains("selene.toml"), "{text}");
        assert!(text.contains("you chose lint"), "{text}");
        assert!(text.contains("wally.toml"), "{text}");
        assert!(text.contains("this project uses Wally"), "{text}");
    }

    /// A project with no capabilities still reads sensibly rather than
    /// showing an empty column.
    #[test]
    fn the_summary_says_so_when_nothing_extra_was_chosen() {
        let planned = plan_for(&[], PackageWorkflow::None);
        let lines = summary_lines("bare", &BTreeSet::new(), PackageWorkflow::None, &[], &planned);
        let text = lines.join("\n");
        assert!(text.contains("Dependencies  none"), "{text}");
        assert!(text.contains("Packages      none"), "{text}");
        assert!(text.contains("Does          nothing extra"), "{text}");
    }

    /// **The gap that opened when tools stopped being asked about.**
    ///
    /// Capabilities derive `selene` and friends, but nothing derives the
    /// package manager - so a Wally project briefly pinned no wally at all,
    /// and a teammate cloning it got no pinned version of the one tool the
    /// project cannot build without. The strategy pins its own.
    #[test]
    fn a_wally_project_with_packages_pins_the_package_manager() {
        let packages: BTreeSet<String> = ["reflex".to_string()].into_iter().collect();
        assert_eq!(
            strategy_tools(PackageWorkflow::Wally, &packages),
            ["wally", "wally-package-types"]
        );
    }

    /// An empty manifest needs no installer, so a project that chose Wally
    /// and then no packages pins nothing - which is what keeps the bare
    /// project bare.
    #[test]
    fn wally_with_no_packages_pins_nothing() {
        assert!(strategy_tools(PackageWorkflow::Wally, &BTreeSet::new()).is_empty());
    }
}


