use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

use crate::catalog::place_template;
use crate::config::PackageWorkflow;
use crate::steps::{capture, run};
use crate::ui;

/// Writes `default.project.json` from scratch (rather than `rojo init` +
/// patching) with the conventional server/client/shared split, and creates
/// the matching `src/` folders. The packages folder mapped into
/// `ReplicatedStorage` depends on which package workflow this project uses:
/// Wally's `packages/` or git submodules' `modules/`.
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

pub const TEMPLATE_PROJECT_NAME: &str = "ProjectName";

const ALWAYS_CREATED_TEMPLATE_PATHS: &[&str] = &["src/shared", "src/server", "src/client"];

const RESERVED_DYNAMIC_PATHS: &[&[&str]] = &[
    &["tree", "ReplicatedStorage", "packages"],
    &["tree", "ReplicatedStorage", "modules"],
    &["tree", "ReplicatedStorage", "test"],
    &["tree", "ServerScriptService", "serverPackages"],
    &["tree", "ServerScriptService", "test"],
    &["tree", "StarterPlayer", "StarterPlayerScripts", "test"],
];

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

pub fn builtin_project_template() -> Value {
    let mut tree = Map::new();
    tree.insert("$className".to_string(), json!("DataModel"));
    tree.insert(
        "ReplicatedStorage".to_string(),
        json!({ "shared": { "$path": "src/shared" } }),
    );
    tree.insert(
        "ServerScriptService".to_string(),
        json!({ "server": { "$path": "src/server" } }),
    );
    tree.insert(
        "StarterPlayer".to_string(),
        json!({ "StarterPlayerScripts": { "client": { "$path": "src/client" } } }),
    );
    for (name, node) in place_template::render() {
        tree.insert(name, node);
    }
    json!({ "name": TEMPLATE_PROJECT_NAME, "tree": tree })
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

    let tree = project["tree"]
        .as_object_mut()
        .context("project template `tree` must be an object")?;
    let replicated_storage = child_object_mut(tree, "ReplicatedStorage")?;
    match package_workflow {
        PackageWorkflow::Wally => {
            replicated_storage.insert("packages".into(), json!({ "$path": "Packages" }));
        }
        PackageWorkflow::GitSubmodules => {
            replicated_storage.insert("modules".into(), json!({ "$path": "modules" }));
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
    let client_scripts = child_object_mut(starter_player, "StarterPlayerScripts")?;
    if testez_selected {
        client_scripts.insert("test".into(), json!({ "$path": "tests/client" }));
    }
    Ok(project)
}

pub fn validate_template_structure(template: &Value) -> Result<()> {
    let object = template
        .as_object()
        .context("project template must be a JSON object")?;
    if object.get("name") != Some(&json!(TEMPLATE_PROJECT_NAME)) {
        bail!("`name` is managed by rproj and must remain `{TEMPLATE_PROJECT_NAME}`");
    }
    let tree = object
        .get("tree")
        .and_then(Value::as_object)
        .context("project template `tree` must be an object")?;
    if tree.get("$className") != Some(&json!("DataModel")) {
        bail!("`tree.$className` is managed by rproj and must remain `DataModel`");
    }

    for (path, expected) in [
        (
            &["tree", "ReplicatedStorage", "shared", "$path"][..],
            "src/shared",
        ),
        (
            &["tree", "ServerScriptService", "server", "$path"][..],
            "src/server",
        ),
        (
            &[
                "tree",
                "StarterPlayer",
                "StarterPlayerScripts",
                "client",
                "$path",
            ][..],
            "src/client",
        ),
    ] {
        if value_at(template, path) != Some(&json!(expected)) {
            bail!(
                "`{}` is managed by rproj and must remain `{expected}`",
                path.join(".")
            );
        }
    }

    for path in RESERVED_DYNAMIC_PATHS {
        if value_at(template, path).is_some() {
            bail!(
                "`{}` is reserved for dependency or testing mounts and must be removed",
                path.join(".")
            );
        }
    }
    validate_paths(template, "")
}

fn validate_paths(value: &Value, location: &str) -> Result<()> {
    match value {
        Value::Object(object) => {
            if let Some(path) = object.get("$path") {
                let path = match path {
                    Value::String(path) => path.as_str(),
                    Value::Object(optional) if optional.len() == 1 => optional
                        .get("optional")
                        .and_then(Value::as_str)
                        .context("optional `$path` must contain a string")?,
                    _ => bail!("`{location}.$path` must be a string or an optional path"),
                };
                if !ALWAYS_CREATED_TEMPLATE_PATHS.contains(&path) {
                    bail!(
                        "`{location}.$path` points to `{path}`; custom paths may only use the always-created `src/shared`, `src/server`, or `src/client` directories"
                    );
                }
            }
            for (key, child) in object {
                let next = if location.is_empty() {
                    key.clone()
                } else {
                    format!("{location}.{key}")
                };
                validate_paths(child, &next)?;
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                validate_paths(child, &format!("{location}[{index}]"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |current, key| current.get(key))
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
    let workspace = ValidationWorkspace::new()?;
    for (label, workflow, tests, server_packages) in [
        ("plain", PackageWorkflow::None, false, false),
        ("plain-tests", PackageWorkflow::None, true, false),
        ("wally", PackageWorkflow::Wally, false, false),
        ("wally-server", PackageWorkflow::Wally, false, true),
        ("wally-tests", PackageWorkflow::Wally, true, false),
        ("wally-tests-server", PackageWorkflow::Wally, true, true),
        ("submodules", PackageWorkflow::GitSubmodules, false, false),
        (
            "submodules-tests",
            PackageWorkflow::GitSubmodules,
            true,
            false,
        ),
    ] {
        let dir = workspace.path.join(label);
        materialize_validation_project(&dir, workflow, tests, server_packages)?;
        let document = project_document(
            "TemplateValidation",
            workflow,
            tests,
            server_packages,
            Some(template),
        )?;
        fs::write(
            dir.join("default.project.json"),
            serde_json::to_string_pretty(&document)?,
        )?;
        for (check, args) in [
            (
                "sourcemap",
                ["sourcemap", "default.project.json", "-o", "sourcemap.json"],
            ),
            (
                "build",
                ["build", "default.project.json", "-o", "validation.rbxl"],
            ),
        ] {
            let output = capture("rojo", &args, Some(&dir)).context(
                "Rojo is required to validate templates; run `rproj setup` and try again",
            )?;
            if !output.success {
                bail!(
                    "Rojo rejected the {label} template during {check} validation (run `rproj setup` if Rojo is not installed):\n{}",
                    output.combined().trim()
                );
            }
        }
    }
    Ok(())
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
        PackageWorkflow::GitSubmodules => materialize_validation_path(dir, "modules")?,
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

struct ValidationWorkspace {
    path: PathBuf,
}

impl ValidationWorkspace {
    fn new() -> Result<Self> {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "rproj-template-validation-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for ValidationWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Installs/updates the Rojo Studio plugin via Rojo's own CLI command -
/// no generic file-copy logic needed for this one. This targets Studio's
/// plugin folder directly, not any particular project, so it only needs
/// to run once from `rproj setup`, not per-project.
pub fn install_studio_plugin() -> Result<()> {
    run("rojo", &["plugin", "install"])
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
    let output = capture(
        "rojo",
        &["sourcemap", "default.project.json", "-o", "sourcemap.json"],
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
    let args = [
        "sourcemap",
        "--watch",
        "default.project.json",
        "-o",
        "sourcemap.json",
    ];
    ui::command("rojo", &args);
    let status = Command::new("rojo")
        .args(args)
        .current_dir(project_dir)
        .status()
        .context("failed to start `rojo sourcemap --watch`")?;
    // Ctrl+C reaches the child too and is the normal way to stop watching,
    // so a non-zero exit here is expected rather than a failure worth
    // reporting as one.
    let _ = status;
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
            "modules",
            "tests/shared",
            "tests/server",
            "tests/client",
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
        let wally = project_document("WallyGame", PackageWorkflow::Wally, false, false, None)
            .expect("wally project");
        let modules = project_document(
            "ModuleGame",
            PackageWorkflow::GitSubmodules,
            false,
            false,
            None,
        )
        .expect("submodule project");

        assert!(wally["tree"]["ReplicatedStorage"].get("packages").is_some());
        assert!(wally["tree"]["ReplicatedStorage"].get("modules").is_none());
        assert!(
            modules["tree"]["ReplicatedStorage"]
                .get("modules")
                .is_some()
        );
        assert!(
            modules["tree"]["ReplicatedStorage"]
                .get("packages")
                .is_none()
        );
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
}
