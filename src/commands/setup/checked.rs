//! Read-only plans and checked single-tool execution shared by terminal and worker.
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, ensure};
use rproj_core::project_tool_setup::{
    Input, Outcome, Plan, Step, StepResult, StepState, rokit_target,
};

use crate::{
    catalog::{tool_catalog, tool_usage},
    path_identity::identity,
    steps::{
        asphalt, blender, executable,
        execution::{MessageKind, Reporter},
        tungsten,
    },
};

const LIMIT: u64 = 2 * 1024 * 1024;

fn snapshot(project: &Path, relative: &str, directory: bool) -> Result<Input> {
    let path = project.join(relative);
    let original = identity(&path, directory)?;
    let digest = if !directory && original.is_some() {
        let mut file =
            fs::File::open(&path).with_context(|| format!("Cannot read {}", path.display()))?;
        let mut hash = blake3::Hasher::new();
        hash.update_reader(&mut file)?;
        Some(hash.finalize().to_hex().to_string())
    } else {
        None
    };
    ensure!(
        identity(&path, directory)? == original,
        "{} changed while reading; Refresh review",
        path.display()
    );
    Ok(Input {
        directory,
        identity: original,
        digest,
    })
}

fn capture(
    inputs: &mut BTreeMap<String, Input>,
    project: &Path,
    relative: &str,
    directory: bool,
) -> Result<()> {
    // Parents are captured in order so a linked directory is never traversed.
    let path = Path::new(relative);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        capture(
            inputs,
            project,
            &parent.to_string_lossy().replace('\\', "/"),
            true,
        )?;
    }
    if !inputs.contains_key(relative) {
        inputs.insert(relative.into(), snapshot(project, relative, directory)?);
    }
    Ok(())
}

fn read_text(path: &Path) -> Result<String> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= LIMIT,
        "{} exceeds the 2 MiB review limit",
        path.display()
    );
    String::from_utf8(bytes).with_context(|| format!("{} is not UTF-8", path.display()))
}

fn manifest(project: &Path) -> Result<toml::Table> {
    let path = project.join("rokit.toml");
    let parsed: toml::Table = toml::from_str(&read_text(&path)?)
        .context("Malformed rokit.toml; repair it and Refresh review")?;
    if let Some(tools) = parsed.get("tools") {
        ensure!(
            tools.as_table().is_some_and(|t| t
                .values()
                .all(|v| v.as_str().is_some_and(|s| !s.is_empty()))),
            "rokit.toml [tools] must contain nonempty string pins; repair it and Refresh review"
        );
    }
    Ok(parsed)
}

fn blender_executable() -> Option<PathBuf> {
    executable("blender").or_else(|| {
        let base = PathBuf::from(std::env::var_os("ProgramFiles")?).join("Blender Foundation");
        let mut candidates: Vec<_> = fs::read_dir(base)
            .ok()?
            .filter_map(Result::ok)
            .map(|e| e.path().join("blender.exe"))
            .filter(|p| p.is_file())
            .collect();
        candidates.sort();
        candidates.pop()
    })
}

pub(crate) fn prepare(project: &Path, tool: &str) -> Result<Plan> {
    let entry = tool_catalog::find(tool)
        .with_context(|| format!("no tool called `{tool}`; rproj info lists known tools"))?;
    ensure!(
        identity(project, true)?.is_some(),
        "Project folder is missing; select an existing Rojo project"
    );
    let project = project.canonicalize()?;
    let mut inputs = BTreeMap::new();
    capture(&mut inputs, &project, "", true)?;
    capture(&mut inputs, &project, "default.project.json", false)?;
    ensure!(
        inputs["default.project.json"].identity.is_some(),
        "Missing default.project.json; select a Rojo project"
    );
    // Setup neither edits nor builds this file. Keep Rojo's JSONC/parser
    // boundary rather than rejecting valid project syntax with serde_json.
    read_text(&project.join("default.project.json"))?;
    let mut steps = Vec::new();
    let mut prerequisites = Vec::new();
    if let Some((source, pin)) = rokit_target(tool)? {
        capture(&mut inputs, &project, "rokit.toml", false)?;
        let table = if inputs["rokit.toml"].identity.is_some() {
            manifest(&project)?
        } else {
            steps.push(Step::InitializeRokit);
            toml::Table::new()
        };
        if let Some(value) = table
            .get("tools")
            .and_then(|t| t.get(pin))
            .and_then(toml::Value::as_str)
        {
            steps.push(Step::PreservePin {
                pin: pin.into(),
                value: value.into(),
            });
        } else {
            steps.push(Step::AddTool {
                source: source.into(),
                pin: pin.into(),
            });
            if executable("rokit").is_none() {
                prerequisites.push("Rokit is required to add a missing pin. Use Machine Setup or install Rokit, then Refresh review.".into());
            }
        }
    } else if tool != "blender" {
        steps.push(Step::Guidance { message: format!("{} is a {}; use Machine Setup for machine-wide provisioning. No project installation is performed.", entry.key, entry.kind.label()) });
    }
    if matches!(tool, "asphalt" | "tungsten") {
        let config = format!("{tool}.toml");
        capture(&mut inputs, &project, &config, false)?;
        if inputs[&config].identity.is_some() {
            steps.push(Step::PreserveFile { relative: config });
        } else {
            capture(&mut inputs, &project, "figma/exports", true)?;
            let (glob, folder) = asphalt::asset_source(&project);
            capture(&mut inputs, &project, folder, true)?;
            let keep = format!("{folder}/.gitkeep");
            capture(&mut inputs, &project, &keep, false)?;
            steps.push(Step::CreateFile {
                relative: config,
                contents: if tool == "asphalt" {
                    asphalt::render(glob)
                } else {
                    tungsten::render(glob)
                },
            });
            if inputs[folder].identity.is_none() {
                steps.push(Step::CreateDirectory {
                    relative: folder.into(),
                });
            }
            if inputs[&keep].identity.is_none() {
                steps.push(Step::CreateFile {
                    relative: keep,
                    contents: String::new(),
                });
            }
        }
    }
    if tool == "blender" {
        capture(&mut inputs, &project, "blender/scene.blend", false)?;
        if inputs["blender/scene.blend"].identity.is_some() {
            steps.push(Step::PreserveFile {
                relative: "blender/scene.blend".into(),
            });
        } else {
            if blender_executable().is_none() {
                prerequisites.push("Blender is required to create a starter scene. Install Blender, then Refresh review.".into());
            }
            if inputs["blender"].identity.is_none() {
                steps.push(Step::CreateDirectory {
                    relative: "blender".into(),
                });
            }
            steps.push(Step::CreateScene);
        }
    }
    let mut notes: Vec<String> = tool_usage::find(tool)
        .map(|u| u.notes.iter().map(|s| (*s).into()).collect())
        .unwrap_or_default();
    notes.push("Project file presence/readability is checked; validating its contents remains Rojo's responsibility.".into());
    verify(&project, &inputs)?;
    let plan = Plan {
        project,
        tool: tool.into(),
        inputs,
        steps,
        prerequisites,
        notes,
    };
    ensure!(
        serde_json::to_vec(&plan)?.len() as u64 <= LIMIT,
        "Tool setup review exceeds 2 MiB; no changes were made"
    );
    Ok(plan)
}

fn verify(project: &Path, inputs: &BTreeMap<String, Input>) -> Result<()> {
    for (relative, expected) in inputs {
        ensure!(
            snapshot(project, relative, expected.directory)? == *expected,
            "{} changed since Review; Refresh review before setup",
            project.join(relative).display()
        );
    }
    Ok(())
}

pub(crate) fn execute(
    plan: &Plan,
    reporter: &Reporter,
    before_effect: &mut impl FnMut() -> Result<()>,
) -> Result<Outcome> {
    execute_with(plan, reporter, before_effect, |command| {
        reporter.run(command)
    })
}

fn execute_with(
    plan: &Plan,
    reporter: &Reporter,
    before_effect: &mut impl FnMut() -> Result<()>,
    mut run: impl FnMut(&mut Command) -> Result<()>,
) -> Result<Outcome> {
    let _guard = crate::background_watch::mutation_guard(Some(&plan.project), false)?;
    ensure!(
        prepare(&plan.project, &plan.tool)? == *plan,
        "Tool setup inputs or effects changed; Refresh review"
    );
    ensure!(
        plan.prerequisites.is_empty(),
        "{}",
        plan.prerequisites.join("\n")
    );
    let mut expected = plan.inputs.clone();
    let mut outcome = Outcome {
        project: plan.project.clone(),
        tool: plan.tool.clone(),
        effects_may_have_begun: false,
        steps: Vec::new(),
    };
    let mut stop = false;
    for step in &plan.steps {
        let label = step.label();
        if stop {
            outcome.steps.push(StepResult {
                label,
                state: StepState::Skipped,
                detail: "An earlier step prevented safe continuation; Refresh review".into(),
            });
            continue;
        }
        verify(&plan.project, &expected)?;
        if step.mutates() {
            crate::interrupt::check()?;
            before_effect()?;
            outcome.effects_may_have_begun = true;
            verify(&plan.project, &expected)?;
        }
        reporter.message(MessageKind::Detail, &label);
        let result = execute_step(step, &plan.project, &mut expected, reporter, &mut run);
        let (state, detail) = match result {
            Ok(()) => (
                match step {
                    Step::PreservePin { .. } | Step::PreserveFile { .. } => {
                        StepState::AlreadyPresent
                    }
                    Step::Guidance { .. } => StepState::ManualAction,
                    _ => StepState::Completed,
                },
                String::new(),
            ),
            Err(error) => {
                // File seeding is independent of tool installation, but an unsafe
                // or unparseable external mutation must stop further project writes.
                stop = !matches!(step, Step::AddTool { .. })
                    || verify(&plan.project, &expected).is_err();
                reporter.message(MessageKind::Warning, &format!("{error:#}"));
                (
                    StepState::Failed,
                    format!("{error:#}. Partial effects may remain; no rollback is promised."),
                )
            }
        };
        outcome.steps.push(StepResult {
            label,
            state,
            detail,
        });
    }
    Ok(outcome)
}

fn execute_step(
    step: &Step,
    project: &Path,
    expected: &mut BTreeMap<String, Input>,
    reporter: &Reporter,
    run: &mut impl FnMut(&mut Command) -> Result<()>,
) -> Result<()> {
    match step {
        Step::InitializeRokit => {
            let rokit = executable("rokit").context("Rokit disappeared; Refresh review")?;
            run(Command::new(rokit).arg("init").current_dir(project))?;
            identity(&project.join("rokit.toml"), false)?
                .context("Rokit init did not create rokit.toml")?;
            let table = manifest(project)?;
            ensure!(
                table
                    .get("tools")
                    .is_none_or(|t| t.as_table().is_some_and(|t| t.is_empty())),
                "Rokit init produced unreviewed tool pins; inspect and Refresh review"
            );
            expected.insert("rokit.toml".into(), snapshot(project, "rokit.toml", false)?);
        }
        Step::AddTool { source, pin } => {
            let before = manifest(project)?;
            let rokit = executable("rokit").context("Rokit disappeared; Refresh review")?;
            // Trust separately: --force would let Rokit overwrite a pin added
            // by another writer between our verification and its own preflight.
            run(Command::new(&rokit)
                .args(["trust", source])
                .current_dir(project))?;
            verify(project, expected)?;
            let command_result = run(Command::new(rokit)
                .args(["add", source, pin])
                .current_dir(project));
            identity(&project.join("rokit.toml"), false)?
                .context("Rokit removed the project manifest")?;
            let after = manifest(project)?;
            let mut preserved = after.clone();
            if let Some(tools) = preserved
                .get_mut("tools")
                .and_then(toml::Value::as_table_mut)
            {
                tools.remove(pin);
            }
            let mut original = before;
            original
                .entry("tools")
                .or_insert_with(|| toml::Value::Table(toml::Table::new()));
            preserved
                .entry("tools")
                .or_insert_with(|| toml::Value::Table(toml::Table::new()));
            ensure!(
                preserved == original,
                "Rokit changed existing pins or settings; inspect rokit.toml and Refresh review"
            );
            expected.insert("rokit.toml".into(), snapshot(project, "rokit.toml", false)?);
            command_result?;
            let value = after
                .get("tools")
                .and_then(|t| t.get(pin))
                .and_then(toml::Value::as_str)
                .context("Rokit exited successfully without the expected tool pin")?;
            let (actual_source, version) = value
                .rsplit_once('@')
                .context("Rokit produced an unresolved pin")?;
            ensure!(
                !version.trim().is_empty()
                    && (!source.contains('/') || actual_source.trim().eq_ignore_ascii_case(source)),
                "Rokit produced an unexpected source or unresolved version; Refresh review"
            );
        }
        Step::CreateDirectory { relative } => {
            fs::create_dir(project.join(relative))?;
            expected.insert(relative.clone(), snapshot(project, relative, true)?);
        }
        Step::CreateFile { relative, contents } => {
            let pending =
                crate::file_replace::stage(&project.join(relative), contents.as_bytes(), false)?;
            verify(project, expected)?;
            pending
                .persist_noclobber(project.join(relative))
                .map_err(|e| e.error)?;
            expected.insert(relative.clone(), snapshot(project, relative, false)?);
        }
        Step::CreateScene => {
            let exe = blender_executable().context("Blender disappeared; Refresh review")?;
            blender::scaffold_starter_scene_checked(project, &exe, reporter, || {
                verify(project, expected)
            })?;
            expected.insert(
                "blender/scene.blend".into(),
                snapshot(project, "blender/scene.blend", false)?,
            );
        }
        _ => {}
    }
    verify(project, expected)
}

#[cfg(test)]
mod tests;
