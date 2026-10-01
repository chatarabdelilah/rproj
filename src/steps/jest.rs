use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

use crate::ui;

pub const PROJECT_FILE: &str = "jest.project.json";
pub const CONFIG_FILE: &str = "jest.config.json";

const STARTER_SPECS: &[(&str, &str)] = &[
    ("tests/shared/hello.spec.luau", "shared"),
    ("tests/server/hello.spec.luau", "server"),
    ("tests/client/hello.spec.luau", "client"),
];

/// Jest Roblox 0.3.x treats a string in `test.projects` as the path to a
/// project config file. Inline entries are therefore required for the three
/// Luau test trees that rproj scaffolds.
const PROJECTS: &[(&str, &str)] = &[
    ("shared", "tests/shared/**/*.spec.luau"),
    ("server", "tests/server/**/*.spec.luau"),
    ("client", "tests/client/**/*.spec.luau"),
];

fn starter_spec(area: &str) -> String {
    format!(
        "--!strict\n\nlocal ReplicatedStorage = game:GetService(\"ReplicatedStorage\")\nlocal JestGlobals = require(ReplicatedStorage.devPackages.JestGlobals)\nlocal describe = JestGlobals.describe\nlocal it = JestGlobals.it\nlocal expect = JestGlobals.expect\n\ndescribe(\"{area}\", function()\n\tit(\"runs\", function()\n\t\texpect(1 + 1).toBe(2)\n\tend)\nend)\n"
    )
}

pub fn ensure_test_tree(project_dir: &Path, examples: bool) -> Result<()> {
    for (relative, area) in STARTER_SPECS {
        let file = project_dir.join(relative);
        let parent = file.parent().expect("test path has a parent");
        fs::create_dir_all(parent)?;
        if examples && !file.exists() {
            fs::write(&file, starter_spec(area))
                .with_context(|| format!("failed to write {}", file.display()))?;
        } else if !examples {
            let keep = parent.join(".gitkeep");
            if !keep.exists() {
                fs::write(keep, "")?;
            }
        }
    }
    ui::ok("prepared Jest test tree");
    Ok(())
}

pub fn project_document(production: &Value) -> Result<Value> {
    let mut project = production.clone();
    ensure_dev_mount(&mut project)?;
    let tree = project
        .get_mut("tree")
        .and_then(Value::as_object_mut)
        .context("default.project.json `tree` must be an object")?;

    tree.remove("Lighting");
    let server = child_object_mut(tree, "ServerScriptService")?;
    let properties = server.entry("$properties").or_insert_with(|| json!({}));
    properties
        .as_object_mut()
        .context("ServerScriptService properties must be an object")?
        .insert("LoadStringEnabled".into(), json!(true));
    insert_path(
        child_object_mut(tree, "ReplicatedStorage")?,
        "test",
        "tests/shared",
        "tree.ReplicatedStorage.test",
    )?;
    insert_path(
        child_object_mut(tree, "ServerScriptService")?,
        "test",
        "tests/server",
        "tree.ServerScriptService.test",
    )?;
    let starter_player = child_object_mut(tree, "StarterPlayer")?;
    insert_path(
        child_object_mut(starter_player, "StarterPlayerScripts")?,
        "test",
        "tests/client",
        "tree.StarterPlayer.StarterPlayerScripts.test",
    )?;
    Ok(project)
}

pub fn ensure_dev_mount(project: &mut Value) -> Result<()> {
    let tree = project
        .get_mut("tree")
        .and_then(Value::as_object_mut)
        .context("default.project.json `tree` must be an object")?;
    let storage = child_object_mut(tree, "ReplicatedStorage")?;
    if let Some(existing) = storage.get("devPackages") {
        let compatible = existing.get("$path").and_then(Value::as_str) == Some("DevPackages")
            && existing
                .get("$className")
                .is_none_or(|class| class.as_str() == Some("Folder"));
        if !compatible {
            bail!("cannot create Jest mount `devPackages` because node already exists");
        }
    } else {
        storage.insert("devPackages".into(), json!({ "$path": "DevPackages" }));
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
        .with_context(|| format!("default.project.json `{key}` must be an object"))
}

fn insert_path(
    parent: &mut Map<String, Value>,
    key: &str,
    path: &str,
    location: &str,
) -> Result<()> {
    if parent.contains_key(key) {
        bail!("cannot create Jest mount `{location}` because that node already exists");
    }
    parent.insert(key.to_string(), json!({ "$path": path }));
    Ok(())
}

pub fn project_contents(production: &Value) -> Result<String> {
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&project_document(production)?)?
    ))
}

pub fn refresh_project(project_dir: &Path) -> Result<()> {
    let source = project_dir.join("default.project.json");
    let mut production: Value = serde_json::from_str(
        &fs::read_to_string(&source)
            .with_context(|| format!("failed to read {}", source.display()))?,
    )
    .with_context(|| format!("failed to parse {}", source.display()))?;
    let original = production.clone();
    ensure_dev_mount(&mut production)?;
    let contents = project_contents(&production)?;
    if production != original {
        atomic_write(
            &source,
            format!("{}\n", serde_json::to_string_pretty(&production)?).as_bytes(),
        )?;
    }
    atomic_write(&project_dir.join(PROJECT_FILE), contents.as_bytes())?;
    ui::ok("wrote jest.project.json");
    Ok(())
}

pub fn merged_config(project_dir: &Path, backend: crate::graph::JestBackend) -> Result<String> {
    let path = project_dir.join(CONFIG_FILE);
    let text = if path.exists() {
        Some(fs::read_to_string(&path)?)
    } else {
        None
    };
    merged_config_text(&path, text.as_deref(), backend)
}

pub(crate) fn merged_config_text(
    path: &Path,
    text: Option<&str>,
    backend: crate::graph::JestBackend,
) -> Result<String> {
    let mut root = if let Some(text) = text {
        let value: Value = serde_json::from_str(text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        value
            .as_object()
            .cloned()
            .with_context(|| format!("{} must contain a JSON object", path.display()))?
    } else {
        Map::new()
    };
    root.insert("backend".into(), json!(backend.key()));
    root.insert("rojoProject".into(), json!(PROJECT_FILE));
    root.insert(
        "jestPath".into(),
        json!("ReplicatedStorage/devPackages/Jest"),
    );
    let test = root.entry("test").or_insert_with(|| json!({}));
    let test = test
        .as_object_mut()
        .context("jest.config.json `test` must be an object")?;
    let projects: Vec<Value> = PROJECTS
        .iter()
        .map(|(display_name, include)| {
            json!({
                "test": {
                    "displayName": display_name,
                    "include": [include],
                }
            })
        })
        .collect();
    test.insert("projects".into(), Value::Array(projects));
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&Value::Object(root))?
    ))
}

pub fn ensure_config(project_dir: &Path, backend: crate::graph::JestBackend) -> Result<()> {
    atomic_write(
        &project_dir.join(CONFIG_FILE),
        merged_config(project_dir, backend)?.as_bytes(),
    )?;
    ui::ok("wrote jest.config.json");
    Ok(())
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let existing = path
        .try_exists()
        .with_context(|| format!("failed to inspect {}", path.display()))?;
    let pending = crate::file_replace::stage(path, contents, existing)
        .with_context(|| format!("failed to prepare {}", path.display()))?;
    pending
        .persist(path)
        .map(|_| ())
        .map_err(|error| error.error)
        .with_context(|| format!("failed to save {}; fix the error and retry", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::steps::{run_in, wally};
    use tempfile::TempDir;

    fn production() -> Value {
        crate::steps::rojo::project_document(
            "Demo",
            crate::config::PackageWorkflow::Wally,
            false,
            false,
            None,
        )
        .unwrap()
    }

    #[test]
    fn test_project_inherits_production_and_adds_only_test_mounts() {
        let mut source = production();
        source["tree"]["Workspace"] = json!({ "$properties": { "StreamingEnabled": true } });
        let test = project_document(&source).unwrap();
        assert_eq!(test["tree"]["Workspace"], source["tree"]["Workspace"]);
        assert!(test["tree"].get("Lighting").is_none());
        assert_eq!(
            test["tree"]["ServerScriptService"]["$properties"]["LoadStringEnabled"],
            true
        );
        assert_ne!(
            source["tree"]["ServerScriptService"]["$properties"]["LoadStringEnabled"],
            true
        );
        assert_eq!(
            test["tree"]["ReplicatedStorage"]["devPackages"]["$path"],
            "DevPackages"
        );
        assert!(
            source["tree"]["ReplicatedStorage"]
                .get("DevPackages")
                .is_none()
        );
    }

    #[test]
    fn collisions_are_rejected() {
        let mut source = production();
        source["tree"]["ReplicatedStorage"]["devPackages"] = json!({});
        assert!(
            project_document(&source)
                .unwrap_err()
                .to_string()
                .contains("already exists")
        );
    }

    #[test]
    fn refresh_and_config_saves_preserve_existing_backups() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("default.project.json");
        fs::write(&source, serde_json::to_vec(&production()).unwrap()).unwrap();
        fs::write(dir.path().join(PROJECT_FILE), "old project").unwrap();
        fs::write(dir.path().join(CONFIG_FILE), r#"{"custom":true}"#).unwrap();
        for name in ["default.project.json", PROJECT_FILE, CONFIG_FILE] {
            fs::write(dir.path().join(name).with_extension("rproj-old"), name).unwrap();
        }

        refresh_project(dir.path()).unwrap();
        ensure_config(dir.path(), crate::graph::JestBackend::Studio).unwrap();

        for name in ["default.project.json", PROJECT_FILE, CONFIG_FILE] {
            assert_eq!(
                fs::read_to_string(dir.path().join(name).with_extension("rproj-old")).unwrap(),
                name
            );
        }
        let config: Value =
            serde_json::from_slice(&fs::read(dir.path().join(CONFIG_FILE)).unwrap()).unwrap();
        assert_eq!(config["custom"], true);
        assert_eq!(config["backend"], "studio-cli");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 6);
    }

    fn save_fixture(dir: &Path) {
        fs::write(
            dir.join("default.project.json"),
            serde_json::to_vec(&production()).unwrap(),
        )
        .unwrap();
        refresh_project(dir).unwrap();
        fs::write(dir.join(CONFIG_FILE), r#"{"custom":true}"#).unwrap();
    }

    fn save_target(dir: &Path, name: &str) -> Result<()> {
        if name == CONFIG_FILE {
            ensure_config(dir, crate::graph::JestBackend::Studio)
        } else {
            refresh_project(dir)
        }
    }

    #[test]
    fn read_only_targets_keep_contents_and_backups_and_allow_retry() {
        for name in ["default.project.json", PROJECT_FILE, CONFIG_FILE] {
            let dir = TempDir::new().unwrap();
            save_fixture(dir.path());
            let target = dir.path().join(name);
            if name == "default.project.json" {
                // Force the production mount repair to write this target.
                fs::write(&target, serde_json::to_vec(&production()).unwrap()).unwrap();
            }
            let original = fs::read(&target).unwrap();
            let backup = target.with_extension("rproj-old");
            fs::write(&backup, "user backup").unwrap();
            let permissions = fs::metadata(&target).unwrap().permissions();
            let mut read_only = permissions.clone();
            read_only.set_readonly(true);
            fs::set_permissions(&target, read_only).unwrap();

            let result = save_target(dir.path(), name);
            fs::set_permissions(&target, permissions).unwrap();
            let error = format!("{:#}", result.unwrap_err());
            assert!(error.contains("read-only"), "{error}");
            assert_eq!(fs::read(&target).unwrap(), original);
            assert_eq!(fs::read_to_string(&backup).unwrap(), "user backup");
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 4);

            save_target(dir.path(), name).unwrap();
            assert_eq!(fs::read_to_string(&backup).unwrap(), "user backup");
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 4);
        }
    }

    #[test]
    #[cfg(windows)]
    fn locked_targets_keep_contents_and_backups_and_allow_retry() {
        use std::os::windows::fs::OpenOptionsExt;

        for name in ["default.project.json", PROJECT_FILE, CONFIG_FILE] {
            let dir = TempDir::new().unwrap();
            save_fixture(dir.path());
            let target = dir.path().join(name);
            if name == "default.project.json" {
                fs::write(&target, serde_json::to_vec(&production()).unwrap()).unwrap();
            }
            let original = fs::read(&target).unwrap();
            let backup = target.with_extension("rproj-old");
            fs::write(&backup, "user backup").unwrap();
            // Share reads/writes while denying replacement or removal.
            let lock = fs::OpenOptions::new()
                .read(true)
                .share_mode(3)
                .open(&target)
                .unwrap();

            let error = format!("{:#}", save_target(dir.path(), name).unwrap_err());
            assert!(error.contains("failed to save"), "{error}");
            assert!(error.contains("retry"), "{error}");
            assert_eq!(fs::read(&target).unwrap(), original);
            assert_eq!(fs::read_to_string(&backup).unwrap(), "user backup");
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 4);

            drop(lock);
            save_target(dir.path(), name).unwrap();
            assert_eq!(fs::read_to_string(&backup).unwrap(), "user backup");
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 4);
        }
    }

    #[test]
    fn refresh_adds_analysis_mount_and_is_repeatable() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("default.project.json");
        fs::write(&source, serde_json::to_string(&production()).unwrap()).unwrap();
        refresh_project(dir.path()).unwrap();
        let first = fs::read(&source).unwrap();
        let production: Value = serde_json::from_slice(&first).unwrap();
        assert_eq!(
            production["tree"]["ReplicatedStorage"]["devPackages"]["$path"],
            "DevPackages"
        );
        refresh_project(dir.path()).unwrap();
        assert_eq!(fs::read(&source).unwrap(), first);
        assert!(starter_spec("shared").contains("ReplicatedStorage.devPackages.JestGlobals"));
    }

    #[test]
    fn refresh_preserves_existing_folder_mount_with_metadata() {
        let dir = TempDir::new().unwrap();
        let mut original = production();
        original["tree"]["ReplicatedStorage"]["devPackages"] = json!({
            "$path": "DevPackages",
            "$className": "Folder",
            "$attributes": { "Custom": true }
        });
        let source = dir.path().join("default.project.json");
        let bytes = serde_json::to_vec(&original).unwrap();
        fs::write(&source, &bytes).unwrap();
        for _ in 0..2 {
            refresh_project(dir.path()).unwrap();
            assert_eq!(fs::read(&source).unwrap(), bytes);
            let test: Value =
                serde_json::from_slice(&fs::read(dir.path().join(PROJECT_FILE)).unwrap()).unwrap();
            assert_eq!(
                test["tree"]["ReplicatedStorage"]["devPackages"],
                original["tree"]["ReplicatedStorage"]["devPackages"]
            );
        }
    }

    #[test]
    fn incompatible_dev_mounts_are_rejected_without_changes() {
        for mount in [
            json!({ "$path": "OtherPackages" }),
            json!({ "$path": "DevPackages", "$className": "ModuleScript" }),
            json!({ "$path": "DevPackages", "$className": null }),
            json!("DevPackages"),
        ] {
            let mut source = production();
            source["tree"]["ReplicatedStorage"]["devPackages"] = mount;
            let original = source.clone();
            assert!(ensure_dev_mount(&mut source).is_err());
            assert_eq!(source, original);
        }
    }

    #[test]
    fn config_merge_preserves_unknown_fields() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(CONFIG_FILE),
            r#"{"timeout":123,"test":{"verbose":true}}"#,
        )
        .unwrap();
        let merged: Value = serde_json::from_str(
            &merged_config(dir.path(), crate::graph::JestBackend::Studio).unwrap(),
        )
        .unwrap();
        assert_eq!(merged["timeout"], 123);
        assert_eq!(merged["test"]["verbose"], true);
        assert_eq!(merged["backend"], "studio-cli");
    }

    #[test]
    fn starter_specs_use_explicit_jest_globals() {
        let spec = starter_spec("shared");
        assert!(spec.contains("JestGlobals.describe"));
        assert!(spec.contains("expect(1 + 1).toBe(2)"));
        assert!(!spec.contains("return function()"));
    }

    #[test]
    fn generated_config_uses_inline_projects_for_luau_test_trees() {
        let dir = TempDir::new().unwrap();
        let config: Value = serde_json::from_str(
            &merged_config(dir.path(), crate::graph::JestBackend::Studio).unwrap(),
        )
        .unwrap();
        let projects = config["test"]["projects"].as_array().unwrap();

        assert_eq!(projects.len(), PROJECTS.len());
        for (project, (display_name, include)) in projects.iter().zip(PROJECTS) {
            assert_eq!(project["test"]["displayName"], json!(display_name));
            assert_eq!(project["test"]["include"], json!([include]));
        }
    }

    #[test]
    #[ignore = "requires Wally, Rojo, wally-package-types, jest-roblox, Roblox Studio, and its runner plugin"]
    fn real_jest_stack_installs_validates_retypes_and_executes() {
        use crate::catalog::package_usage;
        let examples = [
            "charm", "reflex", "matter", "sift", "t", "greentea", "janitor", "ripple",
        ];
        let dir = TempDir::new().unwrap();
        for path in ["src/shared", "src/server", "src/client"] {
            fs::create_dir_all(dir.path().join(path)).unwrap();
        }
        let production = production();
        fs::write(
            dir.path().join("default.project.json"),
            format!("{}\n", serde_json::to_string_pretty(&production).unwrap()),
        )
        .unwrap();
        fs::write(
            dir.path().join("wally.toml"),
            wally::render_wally_toml(
                "rproj/jest-validation",
                &["jest", "jest-globals"]
                    .into_iter()
                    .chain(examples)
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
        )
        .unwrap();
        ensure_test_tree(dir.path(), false).unwrap();
        let mut spec = package_usage::import(
            crate::catalog::wally_packages::find("jest-globals").unwrap(),
            false,
        );
        for key in examples {
            let guide = package_usage::find(key).unwrap();
            spec.push_str(&format!(
                "\nJestGlobals.it(\"catalog {key}\", function()\n{}\nend)\n",
                package_usage::example(&guide)
            ));
        }
        fs::write(dir.path().join("tests/shared/catalog.spec.luau"), spec).unwrap();
        refresh_project(dir.path()).unwrap();
        ensure_config(dir.path(), crate::graph::JestBackend::Studio).unwrap();
        wally::sync_for_project(dir.path(), PROJECT_FILE).unwrap();
        // The ordinary quality gate replaces the Jest sourcemap with this one.
        wally::sync(dir.path()).unwrap();
        let sourcemap = fs::read_to_string(dir.path().join("sourcemap.json")).unwrap();
        assert!(sourcemap.contains("devPackages"));
        assert!(sourcemap.contains("JestGlobals"));
        run_in(
            "jest-roblox-cli",
            &[
                "--backend",
                "studio-cli",
                "--no-color",
                "--formatters",
                "json",
                "--outputFile",
                "report.json",
            ],
            Some(dir.path()),
        )
        .unwrap();
        let report: Value =
            serde_json::from_slice(&fs::read(dir.path().join("report.json")).unwrap()).unwrap();
        assert_eq!(report["numPassedTests"], examples.len(), "{report}");
        assert_eq!(report["numFailedTests"], 0, "{report}");
    }
}
