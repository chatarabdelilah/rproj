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
        "--!strict\n\nlocal ReplicatedStorage = game:GetService(\"ReplicatedStorage\")\nlocal JestGlobals = require(ReplicatedStorage.DevPackages.JestGlobals)\nlocal describe = JestGlobals.describe\nlocal it = JestGlobals.it\nlocal expect = JestGlobals.expect\n\ndescribe(\"{area}\", function()\n\tit(\"runs\", function()\n\t\texpect(1 + 1).toBe(2)\n\tend)\nend)\n"
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
    let tree = project
        .get_mut("tree")
        .and_then(Value::as_object_mut)
        .context("default.project.json `tree` must be an object")?;

    insert_path(
        child_object_mut(tree, "ReplicatedStorage")?,
        "DevPackages",
        "DevPackages",
        "tree.ReplicatedStorage.DevPackages",
    )?;
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
    let production: Value = serde_json::from_str(
        &fs::read_to_string(&source)
            .with_context(|| format!("failed to read {}", source.display()))?,
    )
    .with_context(|| format!("failed to parse {}", source.display()))?;
    atomic_write(
        &project_dir.join(PROJECT_FILE),
        project_contents(&production)?.as_bytes(),
    )?;
    ui::ok("wrote jest.project.json");
    Ok(())
}

pub fn merged_config(project_dir: &Path) -> Result<String> {
    let path = project_dir.join(CONFIG_FILE);
    let mut root = if path.exists() {
        let value: Value = serde_json::from_str(&fs::read_to_string(&path)?)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        value
            .as_object()
            .cloned()
            .with_context(|| format!("{} must contain a JSON object", path.display()))?
    } else {
        Map::new()
    };
    root.insert("backend".into(), json!("studio-cli"));
    root.insert("rojoProject".into(), json!(PROJECT_FILE));
    root.insert(
        "jestPath".into(),
        json!("ReplicatedStorage/DevPackages/Jest"),
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

pub fn ensure_config(project_dir: &Path) -> Result<()> {
    atomic_write(
        &project_dir.join(CONFIG_FILE),
        merged_config(project_dir)?.as_bytes(),
    )?;
    ui::ok("wrote jest.config.json");
    Ok(())
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path.parent().context("output path has no parent")?;
    let mut pending = tempfile::NamedTempFile::new_in(parent)?;
    use std::io::Write;
    pending.write_all(contents)?;
    pending.flush()?;
    if !path.exists() {
        return pending
            .persist(path)
            .map(|_| ())
            .map_err(|error| error.error.into());
    }

    let backup = path.with_extension("rproj-old");
    let _ = fs::remove_file(&backup);
    fs::rename(path, &backup)
        .with_context(|| format!("failed to prepare replacement for {}", path.display()))?;
    match pending.persist(path) {
        Ok(_) => {
            let _ = fs::remove_file(backup);
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(&backup, path);
            Err(error.error.into())
        }
    }
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
        assert_eq!(
            test["tree"]["ReplicatedStorage"]["DevPackages"]["$path"],
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
        source["tree"]["ReplicatedStorage"]["DevPackages"] = json!({});
        assert!(
            project_document(&source)
                .unwrap_err()
                .to_string()
                .contains("already exists")
        );
    }

    #[test]
    fn config_merge_preserves_unknown_fields() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(CONFIG_FILE),
            r#"{"timeout":123,"test":{"verbose":true}}"#,
        )
        .unwrap();
        let merged: Value = serde_json::from_str(&merged_config(dir.path()).unwrap()).unwrap();
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
        let config: Value = serde_json::from_str(&merged_config(dir.path()).unwrap()).unwrap();
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
        ensure_config(dir.path()).unwrap();
        wally::sync_for_project(dir.path(), PROJECT_FILE).unwrap();
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
