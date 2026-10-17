//! Read-only planning and one checked executor for both composition frontends.
use super::creation::ProjectEdit;
use crate::{
    catalog::wally_packages,
    config::PackageWorkflow,
    graph::TestRunner,
    steps::{
        asphalt, blender, figma, gitattributes, jest, rojo, testez, toolchain, tungsten, wally,
    },
    ui,
};
use anyhow::{Context, Result, bail, ensure};
use rproj_core::creation::{CompositionDraft, CompositionFile, CompositionInput, CompositionPlan};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    hash::{Hash, Hasher},
    io::{ErrorKind, Read},
    path::Path,
};

// Inputs to the pure generators, including absent seed targets. Never copy src/.
const INPUTS: &[&str] = &[
    "rproj.toml",
    "default.project.json",
    "rokit.toml",
    "wally.toml",
    "wally.lock",
    "selene.toml",
    "stylua.toml",
    ".gitattributes",
    ".gitignore",
    ".luaurc",
    ".vscode/settings.json",
    ".lute/check.luau",
    ".github/workflows/ci.yml",
    "testez.yml",
    "testez-companion.toml",
    "jest.project.json",
    "jest.config.json",
    "tests/.luaurc",
    "tests/shared/hello.spec.luau",
    "tests/server/hello.spec.luau",
    "tests/client/hello.spec.luau",
    "tests/shared/.gitkeep",
    "tests/server/.gitkeep",
    "tests/client/.gitkeep",
    "figma/README.md",
    "figma/exports/.gitkeep",
    "assets/.gitkeep",
    "asphalt.toml",
    "tungsten.toml",
];
const LIMIT: u64 = 2 * 1024 * 1024;

pub fn apply(project: &Path, edit: ProjectEdit) -> Result<()> {
    let plan = match &edit.reviewed {
        Some(plan) => plan.clone(),
        None => prepare(project, &edit)?,
    };
    println!(
        "\nComposition impact for {} (not an exact diff):",
        plan.project.display()
    );
    for file in &plan.files {
        println!(
            "  {} {} — {}",
            if file.creating { "create" } else { "refresh" },
            file.relative,
            file.reason
        );
    }
    for operation in &plan.external_operations {
        println!("  {operation}");
    }
    println!(
        "Existing pins and exclusions are preserved. Records may be reformatted. Failures/cancellation may leave partial changes; no rollback is promised."
    );
    execute(&plan, &mut || Ok(()))
}

pub(super) fn review(
    terminal: &mut crate::tui::TerminalSession,
    plan: &CompositionPlan,
) -> Result<bool> {
    use crossterm::event::{Event, KeyCode, KeyEventKind};
    use ratatui::{
        layout::{Constraint, Layout},
        widgets::{Block, Borders, Paragraph, Wrap},
    };
    let mut rows = vec![
        format!("Project: {}", plan.project.display()),
        "Impact review — estimated effects, not exact file diffs".into(),
    ];
    rows.extend(plan.files.iter().map(|file| {
        format!(
            "{} {} — {}",
            if file.creating { "Create" } else { "Refresh" },
            file.relative,
            file.reason
        )
    }));
    rows.extend(plan.external_operations.iter().cloned());
    rows.push("Existing declarations/pins and exclusions stay intact. Generated records may be reformatted. Failure/cancellation can leave partial changes. No rollback is promised.".into());
    let mut scroll = 0u16;
    let mut confirm = false;
    loop {
        let mut small = false;
        terminal.draw(|frame| {
            let area = frame.area();
            small = crate::tui::is_too_small(area);
            let panes = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(area);
            frame.render_widget(Paragraph::new(rows.join("\n\n")).wrap(Wrap { trim: false }).scroll((scroll, 0)).block(Block::default().borders(Borders::ALL).title(" Composition impact ")), panes[0]);
            frame.render_widget(Paragraph::new("↑/↓/PgUp/PgDn scroll   A Apply…   Esc Keep editing"), panes[1]);
            if confirm { crate::tui::render_confirm_default_no(frame, area, &crate::tui::ConfirmState::new(format!("Apply additions to {}? Partial file changes may remain if execution fails or is cancelled.", plan.project.display()))); }
        })?;
        if let Event::Key(key) = terminal.read_event()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            if confirm {
                match key.code {
                    KeyCode::Char('y' | 'Y') if !small => return Ok(true),
                    KeyCode::Enter | KeyCode::Esc | KeyCode::Char('n' | 'N') => confirm = false,
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Esc => return Ok(false),
                    KeyCode::Char('a' | 'A') if !small => confirm = true,
                    KeyCode::Down => scroll = scroll.saturating_add(1),
                    KeyCode::Up => scroll = scroll.saturating_sub(1),
                    KeyCode::PageDown => scroll = scroll.saturating_add(8),
                    KeyCode::PageUp => scroll = scroll.saturating_sub(8),
                    _ => {}
                }
            }
            crate::interrupt::check()?;
        }
    }
}

pub(crate) fn prepare(project: &Path, edit: &ProjectEdit) -> Result<CompositionPlan> {
    let project = project
        .canonicalize()
        .context("Cannot open the captured project")?;
    let draft = CompositionDraft {
        original: edit.original.clone(),
        graph: edit.updated.clone(),
    };
    draft.validate()?;
    let record = draft.record(&edit.source)?;
    ensure!(
        fs::read_to_string(project.join("rproj.toml"))? == edit.source,
        "rproj.toml changed while the project editor was open; reopen Edit Packages & Capabilities"
    );
    let mut inputs = vec![];
    let scratch = tempfile::tempdir()?;
    let mut total = 0;
    for relative in INPUTS {
        let path = project.join(relative);
        inputs.push(snapshot(&project, relative)?);
        if path.is_file() {
            total += fs::metadata(&path)?.len();
            ensure!(
                total <= LIMIT,
                "Composition review inputs exceed 2 MiB; reduce configuration size before editing"
            );
            let target = scratch.path().join(relative);
            fs::create_dir_all(target.parent().unwrap())?;
            fs::copy(path, target)?;
        }
    }
    for relative in [
        ".",
        "figma/exports",
        "blender/scene.blend",
        "sourcemap.json",
    ] {
        inputs.push(snapshot(&project, relative)?);
    }
    if project.join("figma/exports").is_dir() {
        fs::create_dir_all(scratch.path().join("figma/exports"))?;
    }
    if draft.graph == draft.original {
        return Ok(CompositionPlan {
            project,
            draft,
            inputs,
            files: vec![],
            tools_to_add: vec![],
            external_operations: vec![],
            create_blender: false,
        });
    }
    ensure!(
        scratch.path().join("default.project.json").is_file(),
        "default.project.json is missing; restore it before editing"
    );
    let graph = &draft.graph;
    let (apps, extensions) = rproj_core::catalog::artifacts::every_machine_requirement();
    let before: BTreeSet<_> = draft
        .original
        .full_plan(&apps, &extensions)
        .into_iter()
        .map(|p| p.key)
        .collect();
    let wanted: BTreeSet<_> = graph
        .maintenance_plan()
        .into_iter()
        .map(|p| p.key)
        .collect();
    for artifact in graph.full_plan(&apps, &extensions) {
        if artifact.key != "test-examples"
            && graph.dropped.iter().any(|k| k == artifact.key)
            && !before.contains(artifact.key)
        {
            ensure!(
                project.join(artifact_path(artifact.key)).exists(),
                "{} is excluded and missing; the addition requires this file. Existing exclusions stay unchanged.",
                artifact.key
            );
        }
    }
    let wants = |key: &str| wanted.contains(key);
    let newly = |key: &str| wants(key) && !before.contains(key);
    let rojo_path = scratch.path().join("default.project.json");
    let old_rojo: serde_json::Value = serde_json::from_str(&fs::read_to_string(&rojo_path)?)?;
    rojo::ensure_project_mounts(
        scratch.path(),
        graph.package_workflow,
        graph.test_runner() == Some(TestRunner::TestEz),
        graph.package_workflow == PackageWorkflow::Wally
            && wally_packages::has_server_realm(&graph.package_set()),
    )?;
    preserve_rojo_values(
        &old_rojo,
        &serde_json::from_str(&fs::read_to_string(rojo_path)?)?,
        "default.project.json",
    )?;
    if graph.package_workflow == PackageWorkflow::Wally {
        ensure!(
            scratch.path().join("wally.toml").is_file(),
            "Wally project is missing wally.toml; restore it before editing"
        );
        if wants("wally.toml") {
            wally::merge_selected(scratch.path(), &graph.packages)?;
        } else {
            ensure!(
                draft.additions()?.packages.is_empty(),
                "wally.toml is excluded; new dependencies cannot be declared"
            );
        }
    }
    if newly("stylua.toml") {
        toolchain::ensure_stylua_config(scratch.path())?;
    }
    if newly(".gitattributes") {
        gitattributes::ensure_gitattributes(scratch.path())?;
    }
    if newly("figma") {
        figma::scaffold_design_folder(scratch.path())?;
    }
    if newly("asphalt.toml") {
        asphalt::ensure_config(scratch.path())?;
    }
    if newly("tungsten.toml") {
        tungsten::ensure_config(scratch.path())?;
    }
    if newly("tests") {
        match graph.test_runner() {
            Some(TestRunner::TestEz) => {
                testez::ensure_test_tree(scratch.path(), wants("test-examples"))?
            }
            Some(TestRunner::JestRoblox) => {
                jest::ensure_test_tree(scratch.path(), wants("test-examples"))?
            }
            None => bail!("Selected testing has no supported runner"),
        }
    }
    let mut reasons = BTreeMap::new();
    for file in super::upgrade::composition_files(scratch.path(), graph)? {
        if ["testez.yml", "testez-companion.toml"].contains(&file.relative.as_str())
            && project.join(&file.relative).exists()
        {
            continue;
        }
        let path = scratch.path().join(&file.relative);
        fs::create_dir_all(path.parent().unwrap())?;
        reasons.insert(file.relative, file.reason);
        fs::write(path, file.contents)?;
    }
    fs::write(scratch.path().join("rproj.toml"), record)?;
    let mut files = vec![];
    for entry in walkdir::WalkDir::new(scratch.path()).sort_by_file_name() {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(scratch.path())?
            .to_string_lossy()
            .replace('\\', "/");
        ensure!(
            INPUTS.contains(&relative.as_str()),
            "Unreviewed generator output: {relative}"
        );
        let contents = fs::read_to_string(entry.path())?;
        let old = read_text(&project.join(&relative))?;
        if old.as_deref() != Some(contents.as_str()) {
            let reason = reasons.remove(&relative).unwrap_or_else(|| match relative.as_str() {
                "rproj.toml" => "record additions last; preserve unrelated metadata; formatting/comments may change".into(),
                "default.project.json" => "add required package/test mounts without replacing custom nodes".into(),
                "wally.toml" => "add missing selected declarations; retain existing aliases, comments, pins and forks".into(),
                "stylua.toml" | ".gitattributes" => "new formatting capability; seed missing defaults only".into(),
                value if value.starts_with("figma/") => "new asset pipeline; seed export folder and documentation".into(),
                value if value.starts_with("tests/") => "new testing capability; seed missing runner examples/roots only".into(),
                "asphalt.toml" | "tungsten.toml" => "new asset pipeline; seed config using the planned export directory".into(),
                _ => "selected capability; generated configuration".into(),
            });
            files.push(CompositionFile {
                relative,
                creating: old.is_none(),
                contents,
                reason,
            });
        }
    }
    let manifest: toml::Value =
        toml::from_str(&read_text(&project.join("rokit.toml"))?.unwrap_or_default())
            .context("Invalid rokit.toml; repair it and reload")?;
    let tools_to_add: Vec<_> = graph
        .tools()
        .into_iter()
        .filter(|k| manifest.get("tools").and_then(|v| v.get(k)).is_none())
        .collect();
    ensure!(
        tools_to_add.is_empty() || wants("rokit.toml"),
        "rokit.toml is excluded; required tools must already be pinned before editing"
    );
    let create_blender = newly("blender") && !project.join("blender/scene.blend").is_file();
    if create_blender {
        blender::preflight_starter_scene(&project)?;
    }
    let mut external_operations = vec![];
    if !tools_to_add.is_empty() {
        external_operations.push(format!("Rokit: add missing pins ({}) with --force and trust/download their catalog sources; existing pins stay unchanged", tools_to_add.join(", ")));
    }
    if !graph.tools().is_empty() && wants("rokit.toml") {
        external_operations.push("Rokit: install tools from project pins".into());
    }
    if graph.package_workflow == PackageWorkflow::Wally {
        external_operations.push("Wally: install dependencies and refresh package types; resolutions and generated files may change".into());
    }
    if wants("sourcemap.json") || graph.package_workflow == PackageWorkflow::Wally {
        if !graph.dropped.iter().any(|key| key == "sourcemap.json") {
            external_operations.push("Rojo: refresh sourcemap.json for type generation".into());
        } else {
            ensure!(
                project.join("sourcemap.json").is_file(),
                "sourcemap.json is excluded and missing; restore it before adding Wally dependencies"
            );
        }
    }
    if create_blender {
        external_operations.push("Blender: create blender/scene.blend at Roblox stud scale".into());
    }
    if newly("asphalt.toml") || newly("tungsten.toml") {
        external_operations.push("Seed asset configuration only; configure creator/API credentials separately before uploads".into());
    }
    Ok(CompositionPlan {
        project,
        draft,
        inputs,
        files,
        tools_to_add,
        external_operations,
        create_blender,
    })
}

fn artifact_path(key: &str) -> &str {
    match key {
        "blender" => "blender/scene.blend",
        "figma" => "figma/exports",
        "test-examples" => "tests/shared/hello.spec.luau",
        other => other,
    }
}

fn preserve_rojo_values(
    before: &serde_json::Value,
    after: &serde_json::Value,
    path: &str,
) -> Result<()> {
    if let Some(object) = before.as_object() {
        for (key, value) in object {
            preserve_rojo_values(value, &after[key], &format!("{path}.{key}"))?;
        }
    } else {
        ensure!(
            before == after,
            "Cannot safely add a mount: {path} has custom data; resolve the conflict before editing"
        );
    }
    Ok(())
}

fn read_text(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("Cannot read {}", path.display())),
    }
}

fn snapshot(project: &Path, relative: &str) -> Result<CompositionInput> {
    let path = project.join(relative);
    let mut cursor = path.as_path();
    while cursor != project {
        match fs::symlink_metadata(cursor) {
            Ok(meta) => {
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::MetadataExt;
                    meta.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let linked = meta.file_type().is_symlink();
                ensure!(
                    !linked,
                    "{} is linked; use regular project files before editing",
                    cursor.display()
                );
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => {
                return Err(e).with_context(|| format!("Cannot inspect {}", cursor.display()));
            }
        }
        cursor = cursor.parent().context("Project input escapes its root")?;
    }
    let fingerprint = match fs::metadata(&path) {
        Ok(meta) if meta.is_dir() => {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            "directory".hash(&mut hash);
            meta.created().ok().hash(&mut hash);
            Some(hash.finish())
        }
        Ok(_) => {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            let mut file = fs::File::open(&path)?;
            let mut buffer = [0; 16384];
            loop {
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                hash.write(&buffer[..count]);
            }
            "file".hash(&mut hash);
            Some(hash.finish())
        }
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    Ok(CompositionInput {
        relative: relative.into(),
        fingerprint,
    })
}

pub(crate) fn capture_inputs(project: &Path) -> Result<Vec<CompositionInput>> {
    INPUTS
        .iter()
        .copied()
        .chain([
            ".",
            "figma/exports",
            "blender/scene.blend",
            "sourcemap.json",
        ])
        .map(|relative| snapshot(project, relative))
        .collect()
}

pub(crate) fn execute(
    plan: &CompositionPlan,
    before_write: &mut impl FnMut() -> Result<()>,
) -> Result<()> {
    let _guard = crate::background_watch::mutation_guard(Some(&plan.project), false)?;
    let source = fs::read_to_string(plan.project.join("rproj.toml"))?;
    let current = prepare(
        &plan.project,
        &ProjectEdit {
            source,
            original: plan.draft.original.clone(),
            updated: plan.draft.graph.clone(),
            reviewed: None,
        },
    )?;
    ensure!(
        &current == plan,
        "Project sources or planned effects changed since Review; explicitly reload before applying"
    );
    if plan.draft.graph == plan.draft.original {
        ui::ok("project composition is unchanged");
        return Ok(());
    }
    check_prerequisites(plan)?;
    crate::interrupt::check()?;
    before_write()?;
    execute_changes(plan).with_context(|| format!("Composition was not fully applied at {}. Files may contain partial changes; rproj.toml commits only after required steps succeed. Inspect git status/diff before retrying.", plan.project.display()))
}

pub(crate) fn check_prerequisites(plan: &CompositionPlan) -> Result<()> {
    if plan.draft.graph == plan.draft.original {
        return Ok(());
    }
    let graph = &plan.draft.graph;
    let pins_tools = graph
        .maintenance_plan()
        .iter()
        .any(|p| p.key == "rokit.toml");
    let mut required = BTreeSet::new();
    if pins_tools && !graph.tools().is_empty() {
        required.insert("rokit");
    }
    if graph.package_workflow == PackageWorkflow::Wally {
        for key in ["wally", "wally-package-types", "rojo"] {
            if !pins_tools || !graph.tools().iter().any(|tool| tool == key) {
                required.insert(key);
            }
        }
    } else if graph
        .maintenance_plan()
        .iter()
        .any(|p| p.key == "sourcemap.json")
        && !pins_tools
    {
        required.insert("rojo");
    }
    for program in required {
        let available = std::env::var_os("PATH").is_some_and(|path| {
            std::env::split_paths(&path).any(|dir| {
                if cfg!(windows) {
                    dir.join(format!("{program}.exe")).is_file()
                } else {
                    dir.join(program).is_file()
                }
            })
        });
        ensure!(
            available,
            "Missing prerequisite: {program}. Run `rproj setup` or install the tool, then Recheck; no composition files have been changed"
        );
    }
    if plan.create_blender {
        blender::preflight_starter_scene(&plan.project)?;
    }
    Ok(())
}

fn execute_changes(plan: &CompositionPlan) -> Result<()> {
    execute_changes_with(plan, execute_external)
}

fn execute_changes_with(
    plan: &CompositionPlan,
    external: impl FnOnce(&CompositionPlan) -> Result<()>,
) -> Result<()> {
    let project = &plan.project;
    for input in &plan.inputs {
        ensure!(
            snapshot(project, &input.relative)? == *input,
            "{} changed before staging",
            input.relative
        );
    }
    let staged = plan
        .files
        .iter()
        .filter(|f| f.relative != "rproj.toml")
        .map(|file| {
            Ok((
                file,
                crate::file_replace::stage(
                    &project.join(&file.relative),
                    file.contents.as_bytes(),
                    !file.creating,
                )?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    for input in &plan.inputs {
        if input.relative == "figma/exports"
            && input.fingerprint.is_none()
            && plan
                .files
                .iter()
                .any(|file| file.relative.starts_with("figma/exports/"))
        {
            continue;
        }
        ensure!(
            snapshot(project, &input.relative)? == *input,
            "{} changed before writes; reload the composition",
            input.relative
        );
    }
    for (file, pending) in staged {
        crate::interrupt::check()?;
        let original = plan
            .inputs
            .iter()
            .find(|input| input.relative == file.relative)
            .context("Missing file identity")?;
        ensure!(
            snapshot(project, &file.relative)? == *original,
            "{} changed before replacement; refusing to overwrite it",
            file.relative
        );
        if file.creating {
            pending
                .persist_noclobber(project.join(&file.relative))
                .map_err(|e| e.error)?;
        } else {
            pending
                .persist(project.join(&file.relative))
                .map_err(|e| e.error)?;
        }
        ui::ok(&format!("wrote {}", file.relative));
    }
    external(plan)?;
    commit_record(plan)
}

fn execute_external(plan: &CompositionPlan) -> Result<()> {
    let project = &plan.project;
    let graph = &plan.draft.graph;
    let wants = |key: &str| graph.maintenance_plan().iter().any(|p| p.key == key);
    if !plan.tools_to_add.is_empty() {
        toolchain::ensure_rokit_init(project)?;
        ensure!(
            project.join("rokit.toml").is_file(),
            "Rokit did not create the project manifest; refusing global changes"
        );
        for key in &plan.tools_to_add {
            use crate::catalog::tool_catalog::{ROKIT_TOOLS, ToolKind};
            let entry = ROKIT_TOOLS
                .iter()
                .find(|entry| entry.key == key)
                .context("Unknown required tool")?;
            let ToolKind::RokitTool { rokit_source } = entry.kind else {
                bail!("Unsupported project tool: {key}")
            };
            crate::steps::run_in("rokit", &["add", "--force", rokit_source], Some(project))?;
        }
    }
    if !graph.tools().is_empty() && wants("rokit.toml") {
        toolchain::sync_installed_tools(project)?;
    }
    if graph.package_workflow == PackageWorkflow::Wally {
        wally::wally_install(project)?;
        let project_file = if graph.test_runner() == Some(TestRunner::JestRoblox) {
            jest::PROJECT_FILE
        } else {
            "default.project.json"
        };
        if !graph.dropped.iter().any(|key| key == "sourcemap.json") {
            rojo::generate_sourcemap_from(project, project_file)?;
        }
        wally::wally_package_types(project)?;
    } else if wants("sourcemap.json") {
        rojo::generate_sourcemap(project)?;
    }
    if plan.create_blender {
        blender::scaffold_starter_scene(project)?;
    }
    Ok(())
}

fn commit_record(plan: &CompositionPlan) -> Result<()> {
    let project = &plan.project;
    crate::interrupt::check()?;
    let record = plan
        .files
        .iter()
        .find(|f| f.relative == "rproj.toml")
        .context("Missing final record change")?;
    let original = plan
        .inputs
        .iter()
        .find(|f| f.relative == "rproj.toml")
        .unwrap();
    ensure!(
        snapshot(project, "rproj.toml")? == *original,
        "rproj.toml changed during execution; refusing replacement"
    );
    let pending = crate::file_replace::stage(
        &project.join("rproj.toml"),
        record.contents.as_bytes(),
        true,
    )?;
    ensure!(
        snapshot(project, "rproj.toml")? == *original,
        "rproj.toml changed before commit"
    );
    pending
        .persist(project.join("rproj.toml"))
        .map_err(|e| e.error)?;
    ui::ok("committed rproj.toml");
    Ok(())
}

#[cfg(test)]
#[path = "project_composition_tests.rs"]
mod tests;
