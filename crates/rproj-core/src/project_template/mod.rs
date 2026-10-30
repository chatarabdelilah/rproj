use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
pub mod metadata;
pub mod model;
mod place_template;

pub const TEMPLATE_PROJECT_NAME: &str = "ProjectName";

pub const ALWAYS_CREATED_TEMPLATE_PATHS: &[&str] = &["src/shared", "src/server", "src/client"];

pub const RESERVED_DYNAMIC_PATHS: &[&[&str]] = &[
    &["tree", "ReplicatedStorage", "packages"],
    &["tree", "ReplicatedStorage", "test"],
    &["tree", "ReplicatedStorage", "DevPackages"],
    &["tree", "ReplicatedStorage", "devPackages"],
    &["tree", "ServerScriptService", "serverPackages"],
    &["tree", "ServerScriptService", "test"],
    &["tree", "StarterPlayer", "StarterPlayerScripts", "test"],
];

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
        json!({
            "$className": "StarterPlayer",
            "StarterPlayerScripts": {
                "$className": "StarterPlayerScripts",
                "client": { "$path": "src/client" }
            }
        }),
    );
    for (name, node) in place_template::render() {
        tree.insert(name, node);
    }
    json!({ "name": TEMPLATE_PROJECT_NAME, "tree": tree })
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
    validate_instance_names(tree, "tree")?;
    validate_paths(template, "")
}

fn validate_instance_names(node: &Map<String, Value>, location: &str) -> Result<()> {
    if node
        .get("$properties")
        .and_then(Value::as_object)
        .is_some_and(|properties| properties.contains_key("Name"))
    {
        bail!(
            "`{location}.$properties.Name` cannot be set manually; rename the `{location}` tree key instead"
        );
    }
    for (name, child) in node.iter().filter(|(name, _)| !name.starts_with('$')) {
        if let Some(child) = child.as_object() {
            validate_instance_names(child, &format!("{location}.{name}"))?;
        }
    }
    Ok(())
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

pub mod review;
pub mod values;
