use std::collections::VecDeque;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

use super::metadata::Metadata;
use crate::steps::rojo;

pub const HISTORY_LIMIT: usize = 100;

const REQUIRED_NODES: &[&[&str]] = &[
    &["ReplicatedStorage"],
    &["ReplicatedStorage", "shared"],
    &["ServerScriptService"],
    &["ServerScriptService", "server"],
    &["StarterPlayer"],
    &["StarterPlayer", "StarterPlayerScripts"],
    &["StarterPlayer", "StarterPlayerScripts", "client"],
];

const LOCKED_MOUNTS: &[&[&str]] = &[
    &["ReplicatedStorage", "shared"],
    &["ServerScriptService", "server"],
    &["StarterPlayer", "StarterPlayerScripts", "client"],
];

const RESERVED_CHILDREN: &[(&[&str], &str)] = &[
    (&["ReplicatedStorage"], "packages"),
    (&["ReplicatedStorage"], "modules"),
    (&["ReplicatedStorage"], "test"),
    (&["ServerScriptService"], "serverPackages"),
    (&["ServerScriptService"], "test"),
    (&["StarterPlayer", "StarterPlayerScripts"], "test"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRow {
    pub path: Vec<String>,
    pub name: String,
    pub class_name: String,
    pub depth: usize,
    pub has_children: bool,
    pub locked: bool,
}

#[derive(Clone)]
pub struct EditorModel {
    original: Value,
    current: Value,
    undo: VecDeque<Value>,
    redo: Vec<Value>,
}

impl EditorModel {
    pub fn new(template: Value) -> Result<Self> {
        representable(&template)?;
        Ok(Self {
            original: template.clone(),
            current: template,
            undo: VecDeque::new(),
            redo: Vec::new(),
        })
    }

    pub fn value(&self) -> &Value {
        &self.current
    }

    pub fn is_dirty(&self) -> bool {
        self.current != self.original
    }

    pub fn mark_saved(&mut self) {
        self.original = self.current.clone();
    }

    pub fn matches_saved(&self, value: &Value) -> bool {
        &self.original == value
    }

    pub fn can_restructure(&self, path: &[String]) -> bool {
        !path.is_empty() && !is_required(path) && !is_mount_or_descendant(path)
    }

    pub fn can_add_children(&self, path: &[String]) -> bool {
        !is_mount_or_descendant(path)
    }

    pub fn replace_from_json(&mut self, value: Value) -> Result<()> {
        representable(&value)?;
        rojo::validate_template_structure(&value)?;
        self.mutate(|current| *current = value)
    }

    pub fn rows(&self) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        let tree = self.current["tree"]
            .as_object()
            .expect("model is representable");
        push_rows(tree, &mut Vec::new(), 0, &mut rows);
        rows
    }

    pub fn node(&self, path: &[String]) -> Option<&Map<String, Value>> {
        node_at(&self.current, path).and_then(Value::as_object)
    }

    pub fn class_name(&self, path: &[String]) -> String {
        if path.is_empty() {
            return "DataModel".into();
        }
        self.node(path)
            .and_then(|node| node.get("$className"))
            .and_then(Value::as_str)
            .unwrap_or_else(|| path.last().expect("non-empty path"))
            .to_string()
    }

    pub fn add(
        &mut self,
        parent: &[String],
        class_name: &str,
        service: bool,
    ) -> Result<Vec<String>> {
        if is_mount_or_descendant(parent) {
            bail!("rproj-managed source mounts cannot contain template children");
        }
        let base = class_name;
        let parent_node = self
            .node(parent)
            .context("selected parent no longer exists")?;
        if service
            && parent.is_empty()
            && parent_node.iter().any(|(name, node)| {
                node.as_object()
                    .is_some_and(|node| inferred_class(name, node) == class_name)
            })
        {
            bail!("the DataModel already contains the `{class_name}` service");
        }
        let name = unique_name(parent_node, base);
        ensure_child_allowed(parent, &name)?;
        let mut path = parent.to_vec();
        path.push(name.clone());
        self.mutate(|current| {
            let parent = node_at_mut(current, parent)
                .and_then(Value::as_object_mut)
                .expect("parent checked above");
            let node = if service && name == class_name {
                json!({})
            } else {
                json!({ "$className": class_name })
            };
            parent.insert(name, node);
        })?;
        Ok(path)
    }

    pub fn rename(&mut self, path: &[String], new_name: &str) -> Result<Vec<String>> {
        ensure_structurally_editable(path)?;
        let name = new_name.trim();
        if name.is_empty() || name.starts_with('$') {
            bail!("instance names cannot be empty or start with `$`");
        }
        let (parent_path, old_name) = split_path(path)?;
        ensure_child_allowed(parent_path, name)?;
        let parent = self.node(parent_path).context("parent no longer exists")?;
        if old_name != name && parent.contains_key(name) {
            bail!("an instance named `{name}` already exists here");
        }
        self.mutate(|current| {
            let parent = node_at_mut(current, parent_path)
                .and_then(Value::as_object_mut)
                .expect("parent checked above");
            let mut value = parent.remove(old_name).expect("node checked above");
            materialize_inferred_class(&mut value, old_name, name);
            parent.insert(name.to_string(), value);
        })?;
        let mut next = parent_path.to_vec();
        next.push(name.to_string());
        Ok(next)
    }

    pub fn change_class(
        &mut self,
        path: &[String],
        class_name: &str,
        service: bool,
    ) -> Result<Vec<String>> {
        ensure_structurally_editable(path)?;
        let (parent_path, old_name) = split_path(path)?;
        let new_name = if service && parent_path.is_empty() {
            class_name
        } else {
            old_name
        };
        let parent = self.node(parent_path).context("parent no longer exists")?;
        if old_name != new_name && parent.contains_key(new_name) {
            bail!("the DataModel already contains the `{new_name}` service");
        }
        if service
            && parent_path.is_empty()
            && parent.iter().any(|(name, node)| {
                name != old_name
                    && node
                        .as_object()
                        .is_some_and(|node| inferred_class(name, node) == class_name)
            })
        {
            bail!("the DataModel already contains the `{class_name}` service");
        }
        self.mutate(|current| {
            let parent = node_at_mut(current, parent_path)
                .and_then(Value::as_object_mut)
                .expect("parent checked before mutation");
            let mut value = parent
                .remove(old_name)
                .expect("node checked before mutation");
            let node = value.as_object_mut().expect("representable node");
            if service && new_name == class_name {
                node.remove("$className");
            } else {
                node.insert("$className".into(), json!(class_name));
            }
            parent.insert(new_name.to_string(), value);
        })?;
        let mut next = parent_path.to_vec();
        next.push(new_name.to_string());
        Ok(next)
    }

    pub fn duplicate(&mut self, path: &[String]) -> Result<Vec<String>> {
        ensure_structurally_editable(path)?;
        let (parent_path, old_name) = split_path(path)?;
        let parent = self.node(parent_path).context("parent no longer exists")?;
        let name = unique_name(parent, &format!("{old_name}Copy"));
        let mut value = parent
            .get(old_name)
            .context("node no longer exists")?
            .clone();
        materialize_inferred_class(&mut value, old_name, &name);
        self.mutate(|current| {
            node_at_mut(current, parent_path)
                .and_then(Value::as_object_mut)
                .expect("parent checked above")
                .insert(name.clone(), value);
        })?;
        let mut next = parent_path.to_vec();
        next.push(name);
        Ok(next)
    }

    pub fn delete(&mut self, path: &[String]) -> Result<()> {
        ensure_structurally_editable(path)?;
        let (parent_path, name) = split_path(path)?;
        self.mutate(|current| {
            node_at_mut(current, parent_path)
                .and_then(Value::as_object_mut)
                .expect("parent checked above")
                .remove(name);
        })
    }

    pub fn move_to(&mut self, path: &[String], destination: &[String]) -> Result<Vec<String>> {
        ensure_structurally_editable(path)?;
        if destination.starts_with(path) {
            bail!("an instance cannot be moved inside itself");
        }
        if is_mount_or_descendant(destination) {
            bail!("rproj-managed source mounts cannot contain template children");
        }
        self.node(destination)
            .context("move destination no longer exists")?;
        let (_, name) = split_path(path)?;
        ensure_child_allowed(destination, name)?;
        if self
            .node(destination)
            .is_some_and(|node| node.contains_key(name))
        {
            bail!("the destination already contains `{name}`");
        }
        let value = self.node(path).context("node no longer exists")?.clone();
        let value = Value::Object(value);
        let source_parent = &path[..path.len() - 1];
        self.mutate(|current| {
            node_at_mut(current, source_parent)
                .and_then(Value::as_object_mut)
                .expect("source parent checked")
                .remove(name);
            node_at_mut(current, destination)
                .and_then(Value::as_object_mut)
                .expect("destination checked")
                .insert(name.to_string(), value);
        })?;
        let mut next = destination.to_vec();
        next.push(name.to_string());
        Ok(next)
    }

    pub fn set_property(&mut self, path: &[String], name: &str, value: Value) -> Result<()> {
        if is_mount_or_descendant(path) {
            bail!("rproj-managed source mounts are read-only");
        }
        self.mutate(|current| {
            let node = node_at_mut(current, path)
                .and_then(Value::as_object_mut)
                .expect("node checked");
            let properties = node
                .entry("$properties")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .expect("representable properties");
            properties.insert(name.to_string(), value);
        })
    }

    pub fn remove_property(&mut self, path: &[String], name: &str) -> Result<()> {
        if is_mount_or_descendant(path) {
            bail!("rproj-managed source mounts are read-only");
        }
        self.mutate(|current| {
            let node = node_at_mut(current, path)
                .and_then(Value::as_object_mut)
                .expect("node checked");
            if let Some(properties) = node.get_mut("$properties").and_then(Value::as_object_mut) {
                properties.remove(name);
                if properties.is_empty() {
                    node.remove("$properties");
                }
            }
        })
    }

    pub fn remove_attribute(&mut self, path: &[String], name: &str) -> Result<()> {
        if is_mount_or_descendant(path) {
            bail!("rproj-managed source mounts are read-only");
        }
        self.mutate(|current| {
            let node = node_at_mut(current, path)
                .and_then(Value::as_object_mut)
                .expect("node checked");
            let Some(properties) = node.get_mut("$properties").and_then(Value::as_object_mut)
            else {
                return;
            };
            if let Some(attributes) = properties
                .get_mut("Attributes")
                .and_then(Value::as_object_mut)
            {
                attributes.remove(name);
                if attributes.is_empty() {
                    properties.remove("Attributes");
                }
            }
            if properties.is_empty() {
                node.remove("$properties");
            }
        })
    }

    pub fn set_ignore_unknown(&mut self, path: &[String], enabled: bool) -> Result<()> {
        if is_mount_or_descendant(path) {
            bail!("rproj-managed source mounts are read-only");
        }
        self.mutate(|current| {
            node_at_mut(current, path)
                .and_then(Value::as_object_mut)
                .expect("node checked")
                .insert("$ignoreUnknownInstances".into(), json!(enabled));
        })
    }

    pub fn set_setting(&mut self, key: &str, value: Option<Value>) -> Result<()> {
        if matches!(key, "name" | "tree") {
            bail!("`{key}` is managed by rproj");
        }
        self.mutate(|current| {
            let object = current.as_object_mut().expect("model is an object");
            match value {
                Some(value) => {
                    object.insert(key.to_string(), value);
                }
                None => {
                    object.remove(key);
                }
            }
        })
    }

    pub fn incompatible_properties(&self, path: &[String], metadata: &Metadata) -> Vec<String> {
        let class = self.class_name(path);
        let Some(properties) = self
            .node(path)
            .and_then(|node| node.get("$properties"))
            .and_then(Value::as_object)
        else {
            return Vec::new();
        };
        properties
            .iter()
            .filter(|(name, value)| {
                name.as_str() != "Attributes"
                    && metadata
                        .property(&class, name)
                        .is_none_or(|property| !property.kind.accepts(value))
            })
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop_back() else {
            return false;
        };
        self.redo
            .push(std::mem::replace(&mut self.current, previous));
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.push_undo();
        self.current = next;
        true
    }

    fn mutate(&mut self, update: impl FnOnce(&mut Value)) -> Result<()> {
        let previous = self.current.clone();
        update(&mut self.current);
        if let Err(error) = representable(&self.current) {
            self.current = previous;
            return Err(error);
        }
        self.undo.push_back(previous);
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.pop_front();
        }
        self.redo.clear();
        Ok(())
    }

    fn push_undo(&mut self) {
        self.undo.push_back(self.current.clone());
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.pop_front();
        }
    }
}

pub fn representable(value: &Value) -> Result<()> {
    let object = value
        .as_object()
        .context("project template must be a JSON object")?;
    let tree = object
        .get("tree")
        .and_then(Value::as_object)
        .context("project template `tree` must be an object")?;
    validate_node(tree, "tree")
}

fn validate_node(node: &Map<String, Value>, location: &str) -> Result<()> {
    if let Some(properties) = node.get("$properties")
        && !properties.is_object()
    {
        bail!("`{location}.$properties` must be an object");
    }
    for (name, child) in node.iter().filter(|(name, _)| !name.starts_with('$')) {
        let child = child
            .as_object()
            .with_context(|| format!("`{location}.{name}` must be an instance object"))?;
        validate_node(child, &format!("{location}.{name}"))?;
    }
    Ok(())
}

fn push_rows(
    node: &Map<String, Value>,
    path: &mut Vec<String>,
    depth: usize,
    rows: &mut Vec<TreeRow>,
) {
    let name = path.last().cloned().unwrap_or_else(|| "ProjectName".into());
    let class_name = node
        .get("$className")
        .and_then(Value::as_str)
        .unwrap_or_else(|| if path.is_empty() { "DataModel" } else { &name })
        .to_string();
    rows.push(TreeRow {
        path: path.clone(),
        name,
        class_name,
        depth,
        has_children: node
            .iter()
            .any(|(name, value)| !name.starts_with('$') && value.is_object()),
        locked: path.is_empty() || is_required(path) || is_mount_or_descendant(path),
    });
    for (name, child) in node
        .iter()
        .filter(|(name, value)| !name.starts_with('$') && value.is_object())
    {
        path.push(name.clone());
        push_rows(
            child.as_object().expect("filtered above"),
            path,
            depth + 1,
            rows,
        );
        path.pop();
    }
}

fn node_at<'a>(project: &'a Value, path: &[String]) -> Option<&'a Value> {
    path.iter()
        .try_fold(project.get("tree")?, |node, name| node.get(name))
}

fn node_at_mut<'a>(project: &'a mut Value, path: &[String]) -> Option<&'a mut Value> {
    path.iter()
        .fold(project.get_mut("tree"), |node, name| node?.get_mut(name))
}

fn split_path(path: &[String]) -> Result<(&[String], &str)> {
    let (name, parent) = path
        .split_last()
        .context("the DataModel root cannot be changed")?;
    Ok((parent, name))
}

fn is_required(path: &[String]) -> bool {
    REQUIRED_NODES
        .iter()
        .any(|required| path.iter().map(String::as_str).eq(required.iter().copied()))
}

fn is_mount_or_descendant(path: &[String]) -> bool {
    LOCKED_MOUNTS.iter().any(|mount| {
        path.len() >= mount.len()
            && path
                .iter()
                .map(String::as_str)
                .zip(mount.iter().copied())
                .all(|(a, b)| a == b)
    })
}

fn ensure_structurally_editable(path: &[String]) -> Result<()> {
    if path.is_empty() || is_required(path) || is_mount_or_descendant(path) {
        bail!(
            "this instance is required by rproj and cannot be renamed, moved, deleted, or reclassed"
        );
    }
    Ok(())
}

fn ensure_child_allowed(parent: &[String], name: &str) -> Result<()> {
    if RESERVED_CHILDREN
        .iter()
        .any(|(reserved_parent, reserved_name)| {
            parent
                .iter()
                .map(String::as_str)
                .eq(reserved_parent.iter().copied())
                && name == *reserved_name
        })
    {
        bail!("`{name}` is reserved for dependency or testing mounts");
    }
    Ok(())
}

fn unique_name(parent: &Map<String, Value>, base: &str) -> String {
    if !parent.contains_key(base) {
        return base.to_string();
    }
    (2..)
        .map(|number| format!("{base}{number}"))
        .find(|name| !parent.contains_key(name))
        .expect("unbounded names")
}

fn materialize_inferred_class(value: &mut Value, old_name: &str, new_name: &str) {
    if old_name == new_name {
        return;
    }
    let Some(node) = value.as_object_mut() else {
        return;
    };
    if !node.contains_key("$className") && !node.contains_key("$path") {
        node.insert("$className".into(), json!(old_name));
    }
}

fn inferred_class<'a>(name: &'a str, node: &'a Map<String, Value>) -> &'a str {
    node.get("$className")
        .and_then(Value::as_str)
        .unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> EditorModel {
        EditorModel::new(rojo::builtin_project_template()).unwrap()
    }

    #[test]
    fn tree_operations_preserve_unrelated_unknown_fields() {
        let mut model = model();
        model
            .set_setting("futureRojoField", Some(json!({"kept": true})))
            .unwrap();
        let folder = model.add(&[], "Folder", false).unwrap();
        let part = model.add(&[], "Part", false).unwrap();
        let renamed = model.rename(&part, "Spawn").unwrap();
        let copy = model.duplicate(&renamed).unwrap();
        model.move_to(&copy, &folder).unwrap();
        model.delete(&renamed).unwrap();
        assert_eq!(model.value()["futureRojoField"]["kept"], true);
    }

    #[test]
    fn required_and_reserved_nodes_are_blocked_before_save() {
        let mut model = model();
        assert!(model.delete(&["ReplicatedStorage".into()]).is_err());
        assert!(
            model
                .rename(&["ReplicatedStorage".into(), "shared".into()], "other")
                .is_err()
        );
        assert!(
            model
                .add(&["ReplicatedStorage".into()], "packages", false)
                .is_err()
        );
    }

    #[test]
    fn undo_and_redo_restore_whole_operations() {
        let mut model = model();
        model.add(&[], "Part", false).unwrap();
        assert!(model.value()["tree"].get("Part").is_some());
        assert!(model.undo());
        assert!(model.value()["tree"].get("Part").is_none());
        assert!(model.redo());
        assert!(model.value()["tree"].get("Part").is_some());
    }

    #[test]
    fn class_changes_keep_properties_for_explicit_repair() {
        let metadata = Metadata::bundled();
        let mut model = model();
        let part = model.add(&[], "Part", false).unwrap();
        model.set_property(&part, "Anchored", json!(true)).unwrap();
        model.change_class(&part, "Folder", false).unwrap();
        assert_eq!(
            model.value()["tree"]["Part"]["$properties"]["Anchored"],
            true
        );
        assert_eq!(
            model.incompatible_properties(&part, &metadata),
            ["Anchored"]
        );
    }

    #[test]
    fn root_services_are_canonical_and_unique() {
        let mut model = model();
        assert!(model.add(&[], "ReplicatedStorage", true).is_err());

        let folder = model.add(&[], "Folder", false).unwrap();
        let service = model.change_class(&folder, "Workspace", true).unwrap();
        assert_eq!(service, ["Workspace"]);
        assert!(
            model.value()["tree"]["Workspace"]
                .get("$className")
                .is_none()
        );

        let another = model.add(&[], "Folder", false).unwrap();
        assert!(model.change_class(&another, "Workspace", true).is_err());
    }

    #[test]
    fn malformed_common_property_values_are_flagged_before_rojo_runs() {
        let metadata = Metadata::bundled();
        let mut model = model();
        let part = model.add(&[], "Part", false).unwrap();
        model
            .set_property(&part, "Anchored", json!("not a boolean"))
            .unwrap();
        assert_eq!(
            model.incompatible_properties(&part, &metadata),
            ["Anchored"]
        );
    }

    #[test]
    fn renamed_and_duplicated_inferred_services_keep_their_class() {
        let mut model = model();
        let service = model.add(&[], "Workspace", true).unwrap();
        let copy = model.duplicate(&service).unwrap();
        assert_eq!(model.node(&copy).unwrap()["$className"], "Workspace");
        let renamed = model.rename(&service, "World").unwrap();
        assert_eq!(model.node(&renamed).unwrap()["$className"], "Workspace");
    }
}
