#[cfg(test)]
use rproj_core::project_template::RESERVED_DYNAMIC_PATHS;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

use crate::config::PackageWorkflow;
use crate::steps::capture;
use crate::ui;

/// Writes `default.project.json` from scratch (rather than `rojo init` +
/// patching) with the conventional server/client/shared split, and creates
/// the matching `src/` folders. The packages folder mapped into
/// `ReplicatedStorage` depends on which package workflow this project uses:
/// Wally package mounts.
/// Starter files, one per source folder.
///
/// These are named `hello.*`, never `init.*`: Rojo turns a directory
/// containing an `init.luau` into a *script* named after the directory, so
/// `src/shared/init.luau` would make `ReplicatedStorage.shared` a
/// ModuleScript-with-children and `src/server/init.server.luau` would make
/// it a Script, instead of the plain Folders these are meant to be.
/// (Verified from the sourcemap's `className` both ways.)
///
/// They also do the job a `.gitkeep` was previously doing - git doesn't
/// track empty directories, so without *some* file a fresh clone would be
/// missing the very paths `default.project.json` maps - while actually
/// being useful, unlike an empty placeholder.
const STARTER_FILES: &[(&str, &str)] = &[
    (
        "src/shared/hello.luau",
        "--!strict\n\nreturn {\n\tgreeting = \"hello from shared\",\n}\n",
    ),
    (
        "src/server/hello.server.luau",
        "--!strict\n\nlocal ReplicatedStorage = game:GetService(\"ReplicatedStorage\")\n\nlocal hello = require(ReplicatedStorage.shared.hello)\n\nprint(hello.greeting, \"- server\")\n",
    ),
    (
        "src/client/hello.client.luau",
        "--!strict\n\nlocal ReplicatedStorage = game:GetService(\"ReplicatedStorage\")\n\nlocal hello = require(ReplicatedStorage.shared.hello)\n\nprint(hello.greeting, \"- client\")\n",
    ),
];

use rproj_core::project_template::ALWAYS_CREATED_TEMPLATE_PATHS;
pub use rproj_core::project_template::{builtin_project_template, validate_template_structure};

pub fn scaffold_project_json(
    project_dir: &Path,
    project_name: &str,
    package_workflow: PackageWorkflow,
    testez_selected: bool,
    has_server_packages: bool,
    template: Option<&Value>,
) -> Result<()> {
    let path = project_dir.join("default.project.json");
    if path.exists() {
        ui::ok("default.project.json already exists");
        return Ok(());
    }

    for (rel_path, contents) in STARTER_FILES {
        let file = project_dir.join(rel_path);
        fs::create_dir_all(file.parent().expect("starter paths have a parent"))?;
        if !file.exists() {
            fs::write(&file, contents)?;
        }
    }

    let project = project_document(
        project_name,
        package_workflow,
        testez_selected,
        has_server_packages,
        template,
    )?;
    fs::write(&path, serde_json::to_string_pretty(&project)?)?;
    ui::ok("wrote default.project.json");
    Ok(())
}

#[cfg(test)]
#[test]
fn template_generation_before_extraction_fixture() {
    let builtin = builtin_project_template();
    let mut variants = Map::new();
    for (name, workflow, tests, server) in [
        ("plain", PackageWorkflow::None, false, false),
        ("plain-tests", PackageWorkflow::None, true, false),
        ("wally", PackageWorkflow::Wally, false, false),
        ("wally-server", PackageWorkflow::Wally, false, true),
        ("wally-tests", PackageWorkflow::Wally, true, false),
        ("wally-tests-server", PackageWorkflow::Wally, true, true),
    ] {
        variants.insert(
            name.into(),
            project_document("Fixture", workflow, tests, server, Some(&builtin)).unwrap(),
        );
    }
    for (name, server) in [("jest", false), ("jest-server", true)] {
        let production = project_document(
            "Fixture",
            PackageWorkflow::Wally,
            false,
            server,
            Some(&builtin),
        )
        .unwrap();
        variants.insert(
            name.into(),
            crate::steps::jest::project_document(&production).unwrap(),
        );
    }
    let actual = format!(
        "{}\n",
        serde_json::to_string_pretty(&json!({"builtin": builtin, "variants": variants})).unwrap()
    );
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/project-template-generation.json");
    assert_eq!(actual, fs::read_to_string(path).unwrap());
}

pub fn project_document(
    project_name: &str,
    package_workflow: PackageWorkflow,
    testez_selected: bool,
    has_server_packages: bool,
    template: Option<&Value>,
) -> Result<Value> {
    let mut project = template.cloned().unwrap_or_else(builtin_project_template);
    validate_template_structure(&project)?;
    project["name"] = json!(project_name);

    apply_dynamic_mounts(
        &mut project,
        package_workflow,
        testez_selected,
        has_server_packages,
    )?;
    Ok(project)
}

pub fn ensure_project_mounts(
    project_dir: &Path,
    package_workflow: PackageWorkflow,
    testez_selected: bool,
    has_server_packages: bool,
) -> Result<()> {
    let path = project_dir.join("default.project.json");
    let mut project: Value = serde_json::from_str(&fs::read_to_string(&path)?)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let before = project.clone();
    apply_dynamic_mounts(
        &mut project,
        package_workflow,
        testez_selected,
        has_server_packages,
    )?;
    if project == before {
        ui::ok("default.project.json already has selected mounts");
    } else {
        fs::write(
            &path,
            format!("{}\n", serde_json::to_string_pretty(&project)?),
        )?;
        ui::ok("updated default.project.json mounts");
    }
    Ok(())
}

fn apply_dynamic_mounts(
    project: &mut Value,
    package_workflow: PackageWorkflow,
    testez_selected: bool,
    has_server_packages: bool,
) -> Result<()> {
    let tree = project["tree"]
        .as_object_mut()
        .context("project template `tree` must be an object")?;
    let replicated_storage = child_object_mut(tree, "ReplicatedStorage")?;
    match package_workflow {
        PackageWorkflow::Wally => {
            replicated_storage.insert("packages".into(), json!({ "$path": "Packages" }));
        }

        PackageWorkflow::None => {}
    }
    if testez_selected {
        replicated_storage.insert("test".into(), json!({ "$path": "tests/shared" }));
    }

    let server_scripts = child_object_mut(tree, "ServerScriptService")?;
    if has_server_packages {
        server_scripts.insert(
            "serverPackages".into(),
            json!({ "$path": "ServerPackages" }),
        );
    }
    if testez_selected {
        server_scripts.insert("test".into(), json!({ "$path": "tests/server" }));
    }

    let starter_player = child_object_mut(tree, "StarterPlayer")?;
    starter_player.insert("$className".into(), json!("StarterPlayer"));
    let client_scripts = child_object_mut(starter_player, "StarterPlayerScripts")?;
    client_scripts.insert("$className".into(), json!("StarterPlayerScripts"));
    if testez_selected {
        client_scripts.insert("test".into(), json!({ "$path": "tests/client" }));
    }
    Ok(())
}

fn child_object_mut<'a>(
    parent: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>> {
    parent
        .get_mut(key)
        .and_then(Value::as_object_mut)
        .with_context(|| format!("project template `{key}` must be an object"))
}

pub fn validate_template_with_rojo(template: &Value) -> Result<()> {
    validate_template_structure(template)?;
    ensure_rojo_available()?;
    validate_template_variants(template, validate_rojo_project)
}

pub(crate) fn validate_template_variants(
    template: &Value,
    validate: impl Fn(&str, &Path, &Path) -> Result<()>,
) -> Result<()> {
    let workspace = tempfile::Builder::new()
        .prefix("rproj-template-validation-")
        .tempdir()?;
    for (label, workflow, tests, server_packages) in [
        ("plain", PackageWorkflow::None, false, false),
        ("plain-tests", PackageWorkflow::None, true, false),
        ("wally", PackageWorkflow::Wally, false, false),
        ("wally-server", PackageWorkflow::Wally, false, true),
        ("wally-tests", PackageWorkflow::Wally, true, false),
        ("wally-tests-server", PackageWorkflow::Wally, true, true),
    ] {
        let dir = workspace.path().join(label);
        materialize_validation_project(&dir, workflow, tests, server_packages)?;
        let document = project_document(
            "TemplateValidation",
            workflow,
            tests,
            server_packages,
            Some(template),
        )?;
        let project_file = dir.join("default.project.json");
        fs::write(&project_file, serde_json::to_string_pretty(&document)?)?;
        validate(label, &project_file, &dir)?;
    }
    for (label, server_packages) in [("jest", false), ("jest-server", true)] {
        let dir = workspace.path().join(label);
        materialize_validation_project(&dir, PackageWorkflow::Wally, true, server_packages)?;
        materialize_validation_path(&dir, "DevPackages")?;
        let production = project_document(
            "TemplateValidation",
            PackageWorkflow::Wally,
            false,
            server_packages,
            Some(template),
        )?;
        let document = crate::steps::jest::project_document(&production)?;
        let project_file = dir.join("jest.project.json");
        fs::write(&project_file, serde_json::to_string_pretty(&document)?)?;
        validate(label, &project_file, &dir)?;
    }
    Ok(())
}

fn validate_rojo_project(label: &str, project_file: &Path, dir: &Path) -> Result<()> {
    let project_file = project_file.to_string_lossy().into_owned();
    for (check, command, output_file) in [
        ("sourcemap", "sourcemap", dir.join("sourcemap.json")),
        ("build", "build", dir.join("validation.rbxl")),
    ] {
        let output_file = output_file.to_string_lossy().into_owned();
        let output = capture(
            "rojo",
            &[command, project_file.as_str(), "-o", output_file.as_str()],
            None,
        )
        .context(
            "Rojo is required to validate templates; run `rokit add --global rojo` and try again",
        )?;
        if !output.success {
            bail!(
                "Rojo rejected the {label} template during {check} validation:\n{}",
                output.combined().trim()
            );
        }
    }
    Ok(())
}

fn ensure_rojo_available() -> Result<()> {
    let output = capture("rojo", &["--version"], None).context(
        "Rojo is required to validate templates; run `rokit add --global rojo` and try again",
    )?;
    if output.success {
        return Ok(());
    }
    bail!("{}", rojo_preflight_error(output.combined().trim()))
}

fn rojo_preflight_error(details: &str) -> String {
    if details.contains("Failed to find tool 'rojo'") {
        return "Rojo is not configured in this project or Rokit's global manifest. Run `rokit add --global rojo`, then save the template again.".into();
    }
    format!("Rojo could not start, so the template was not checked:\n{details}")
}

fn materialize_validation_project(
    dir: &Path,
    workflow: PackageWorkflow,
    tests: bool,
    server_packages: bool,
) -> Result<()> {
    for path in ALWAYS_CREATED_TEMPLATE_PATHS {
        materialize_validation_path(dir, path)?;
    }
    match workflow {
        PackageWorkflow::Wally => materialize_validation_path(dir, "Packages")?,
        PackageWorkflow::None => {}
    }
    if tests {
        for path in ["tests/shared", "tests/server", "tests/client"] {
            materialize_validation_path(dir, path)?;
        }
    }
    if server_packages {
        materialize_validation_path(dir, "ServerPackages")?;
    }
    Ok(())
}

fn materialize_validation_path(dir: &Path, path: &str) -> Result<()> {
    let path = dir.join(path);
    fs::create_dir_all(&path)?;
    fs::write(path.join("template.luau"), "return nil\n")?;
    Ok(())
}

/// Generates `sourcemap.json` once.
///
/// Deliberately *not* `--watch`: the scaffold only needs one sourcemap,
/// and the watch-then-kill approach it replaced was the source of two
/// bugs. It polled for the file with a 10s timeout, so any rojo failure
/// surfaced as "timed out waiting for sourcemap.json" with rojo's actual
/// explanation captured and thrown away - and on that timeout path it
/// bailed without killing the child, leaving a `rojo sourcemap --watch`
/// running forever. Running it once and reading the exit status reports
/// whatever rojo actually said, and leaves nothing behind.
pub fn generate_sourcemap(project_dir: &Path) -> Result<()> {
    generate_sourcemap_from(project_dir, "default.project.json")
}

pub fn generate_sourcemap_from(project_dir: &Path, project_file: &str) -> Result<()> {
    let output = capture(
        "rojo",
        &["sourcemap", project_file, "-o", "sourcemap.json"],
        Some(project_dir),
    )?;
    if !output.success || ui::is_verbose() {
        ui::passthrough(&output.stdout, &output.stderr);
    }
    if !output.success {
        bail!("rojo could not generate a sourcemap (see above)");
    }
    ui::ok("generated sourcemap.json");
    Ok(())
}

/// Runs `rojo sourcemap --watch` in the foreground until interrupted, for
/// `rproj watch`. Unlike the one-shot `generate_sourcemap`, this genuinely
/// wants a long-lived process, so stdio is inherited: the user is watching
/// this run and rojo's own "Created sourcemap at ..." on each rebuild is
/// the feedback that it's working.
pub fn watch_sourcemap(project_dir: &Path) -> Result<()> {
    watch_sourcemap_from(project_dir, "default.project.json")
}

pub fn watch_sourcemap_from(project_dir: &Path, project_file: &str) -> Result<()> {
    crate::interrupt::check()?;
    let args = ["sourcemap", "--watch", project_file, "-o", "sourcemap.json"];
    ui::command("rojo", &args);
    let status = Command::new("rojo")
        .args(args)
        .current_dir(project_dir)
        .status()
        .context("failed to start `rojo sourcemap --watch`")?;
    // Windows reports console interruption as STATUS_CONTROL_C_EXIT.
    crate::diagnostics::tool_exit("rojo", status);
    if !status.success()
        && !crate::interrupt::requested()
        && status.code() != Some(0xC000013Au32 as i32)
    {
        bail!("Rojo watcher failed: {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_template_matches_the_existing_project_shape() {
        let project = project_document("MyGame", PackageWorkflow::None, false, false, None)
            .expect("render project");

        assert_eq!(project["name"], "MyGame");
        assert_eq!(project["tree"]["$className"], "DataModel");
        assert_eq!(
            project["tree"]["StarterPlayer"]["$className"],
            "StarterPlayer"
        );
        assert_eq!(
            project["tree"]["StarterPlayer"]["StarterPlayerScripts"]["$className"],
            "StarterPlayerScripts"
        );
        assert_eq!(
            project["tree"]["ReplicatedStorage"]["shared"]["$path"],
            "src/shared"
        );
        assert_eq!(
            project["tree"]["Lighting"]["$properties"]["Brightness"],
            2.5
        );
    }

    #[test]
    fn custom_nodes_and_top_level_settings_survive_generation() {
        let mut template = builtin_project_template();
        template["servePort"] = json!(40000);
        template["tree"]["ReplicatedStorage"]["modules"] = json!({
            "$className": "Folder",
            "$properties": { "Archivable": false }
        });
        template["tree"]["Workspace"] = json!({
            "$className": "Workspace",
            "$properties": { "Gravity": 100 }
        });

        let project = project_document(
            "CustomGame",
            PackageWorkflow::Wally,
            true,
            true,
            Some(&template),
        )
        .expect("render custom project");

        assert_eq!(project["name"], "CustomGame");
        assert_eq!(project["servePort"], 40000);
        assert_eq!(
            project["tree"]["ReplicatedStorage"]["modules"],
            template["tree"]["ReplicatedStorage"]["modules"]
        );
        assert_eq!(project["tree"]["Workspace"]["$properties"]["Gravity"], 100);
        assert_eq!(
            project["tree"]["ReplicatedStorage"]["packages"]["$path"],
            "Packages"
        );
        assert_eq!(
            project["tree"]["ServerScriptService"]["serverPackages"]["$path"],
            "ServerPackages"
        );
        assert_eq!(
            project["tree"]["StarterPlayer"]["StarterPlayerScripts"]["test"]["$path"],
            "tests/client"
        );
    }

    #[test]
    fn changing_a_core_mount_is_rejected() {
        let mut template = builtin_project_template();
        template["tree"]["ReplicatedStorage"]["shared"]["$path"] = json!("src/other");

        let error = validate_template_structure(&template)
            .unwrap_err()
            .to_string();
        assert!(error.contains("ReplicatedStorage.shared.$path"), "{error}");
        assert!(error.contains("src/shared"), "{error}");
    }

    #[test]
    fn dependency_and_test_mount_names_are_reserved() {
        for path in RESERVED_DYNAMIC_PATHS {
            let mut template = builtin_project_template();
            let mut current = &mut template;
            for key in &path[..path.len() - 1] {
                current = current.get_mut(*key).expect("reserved parent exists");
            }
            current[path[path.len() - 1]] = json!({ "$className": "Folder" });

            let error = validate_template_structure(&template)
                .unwrap_err()
                .to_string();
            assert!(error.contains("reserved"), "{}: {error}", path.join("."));
        }
    }

    #[test]
    fn paths_outside_the_generated_tree_are_rejected() {
        let mut template = builtin_project_template();
        template["tree"]["Workspace"] = json!({ "$path": "assets/map.rbxm" });

        let error = validate_template_structure(&template)
            .unwrap_err()
            .to_string();
        assert!(error.contains("always-created"), "{error}");
    }

    #[test]
    fn conditional_paths_are_reserved_for_generated_mounts() {
        for path in [
            "Packages",
            "ServerPackages",
            "tests/shared",
            "tests/server",
            "tests/client",
            "DevPackages",
        ] {
            let mut template = builtin_project_template();
            template["tree"]["Workspace"] = json!({ "$path": path });

            let error = validate_template_structure(&template)
                .unwrap_err()
                .to_string();
            assert!(error.contains("always-created"), "{path}: {error}");
        }
    }

    #[test]
    fn each_dependency_workflow_gets_only_its_own_mount() {
        for workflow in PackageWorkflow::ALL {
            let project = project_document("Game", *workflow, false, false, None).unwrap();
            let storage = &project["tree"]["ReplicatedStorage"];
            assert_eq!(
                storage.get("packages").is_some(),
                *workflow == PackageWorkflow::Wally
            );
            assert!(storage.get("modules").is_none());
        }
    }

    #[test]
    #[ignore = "requires a real Rojo binary on PATH"]
    fn built_in_template_passes_every_real_rojo_validation_variant() {
        validate_template_with_rojo(&builtin_project_template())
            .expect("built-in template should pass Rojo validation");
    }

    #[test]
    #[ignore = "requires a real Rojo binary on PATH"]
    fn real_rojo_rejects_an_invalid_property_value() {
        let mut template = builtin_project_template();
        template["tree"]["Broken"] = json!({
            "$className": "Part",
            "$properties": { "Anchored": "not-a-boolean" }
        });

        let error = validate_template_with_rojo(&template)
            .unwrap_err()
            .to_string();
        assert!(error.contains("Rojo rejected"), "{error}");
    }

    #[test]
    fn missing_rokit_tool_is_not_reported_as_template_rejection() {
        let error =
            rojo_preflight_error("ERROR Failed to find tool 'rojo' in any project manifest file.");
        assert!(error.contains("rokit add --global rojo"), "{error}");
        assert!(!error.contains("Rojo rejected"), "{error}");
    }

    #[test]
    fn instance_name_property_is_rejected_before_rojo_ignores_it() {
        let mut template = builtin_project_template();
        template["tree"]["Part"] = json!({
            "$className": "Part",
            "$properties": { "Name": "IgnoredName" }
        });

        let error = validate_template_structure(&template)
            .unwrap_err()
            .to_string();
        assert!(error.contains("tree.Part.$properties.Name"), "{error}");
        assert!(error.contains("rename the `tree.Part` tree key"), "{error}");
    }
}
