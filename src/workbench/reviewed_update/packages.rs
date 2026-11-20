use super::*;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Component,
};
pub(super) const DIRS: [&str; 3] = ["Packages", "ServerPackages", "DevPackages"];

fn mounts(value: &Value, dest: &mut BTreeSet<PathBuf>) -> Result<()> {
    if let Value::Object(fields) = value {
        if let Some(mount) = fields.get("$path") {
            let raw = mount
                .as_str()
                .or_else(|| mount.get("optional").and_then(Value::as_str))
                .context("Unsupported Rojo mount; review the project manually")?;
            let rel = Path::new(raw);
            ensure!(
                !rel.as_os_str().is_empty()
                    && rel.components().all(|c| matches!(c, Component::Normal(_))),
                "External or ambiguous Rojo mount: {raw}"
            );
            dest.insert(rel.to_owned());
        }
        for (key, child) in fields {
            if !key.starts_with('$') {
                mounts(child, dest)?;
            }
        }
    }
    Ok(())
}

pub(super) fn source_tree(project: &Path) -> Result<Tree> {
    let mut entries = Tree::new();
    let config = text(&project.join("default.project.json"))?;
    let json: Value = serde_json::from_str(&config)
        .context("Cannot safely stage this Rojo project description")?;
    let mut paths = BTreeSet::new();
    mounts(json.get("tree").context("Missing Rojo tree")?, &mut paths)?;
    for rel in paths {
        if DIRS.iter().any(|name| rel.starts_with(name)) {
            continue;
        }
        for ancestor in rel
            .ancestors()
            .skip(1)
            .filter(|p| !p.as_os_str().is_empty())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            identity(&project.join(ancestor), true)?;
        }
        for (p, e) in tree(project, &rel)? {
            ensure!(
                !p.to_string_lossy()
                    .to_ascii_lowercase()
                    .ends_with(".project.json"),
                "Nested Rojo project {} needs manual review",
                p.display()
            );
            entries.insert(p, e);
        }
    }
    entries.insert(
        "default.project.json".into(),
        file_state(&project.join("default.project.json"), false)?,
    );
    if identity(&project.join("rokit.toml"), false)?.is_some() {
        entries.insert(
            "rokit.toml".into(),
            file_state(&project.join("rokit.toml"), false)?,
        );
    }
    Ok(entries)
}

fn packages(project: &Path) -> Result<Tree> {
    let mut result = Tree::new();
    for name in DIRS {
        result.extend(tree(project, Path::new(name))?);
    }
    Ok(result)
}

#[derive(serde::Deserialize)]
struct Lock {
    package: Vec<Node>,
}
#[derive(serde::Deserialize)]
struct Node {
    name: String,
    version: String,
    #[serde(default)]
    dependencies: Vec<(String, String)>,
}

fn graph(lock: &str, root: &toml::Value) -> Result<(BTreeMap<String, String>, BTreeSet<String>)> {
    let package = root.get("package").context("Missing package table")?;
    let name = package
        .get("name")
        .and_then(toml::Value::as_str)
        .context("Missing package name")?;
    let version = package
        .get("version")
        .and_then(toml::Value::as_str)
        .context("Missing package version")?;
    let entries: Lock = toml::from_str(lock)?;
    let mut direct = None;
    let mut graph = BTreeSet::new();
    for node in entries.package {
        let id = format!("{}@{}", node.name, node.version);
        graph.insert(id.clone());
        for (alias, dependency) in &node.dependencies {
            graph.insert(format!("{id} / {alias} → {dependency}"));
        }
        if node.name == name && node.version == version {
            ensure!(direct.is_none(), "Ambiguous root lock entry");
            let mut edges = BTreeMap::new();
            for (alias, dep) in node.dependencies {
                ensure!(
                    edges.insert(alias, dep).is_none(),
                    "Ambiguous direct alias in lock"
                );
            }
            direct = Some(edges);
        }
    }
    Ok((
        direct.context("Missing project entry in wally.lock")?,
        graph,
    ))
}

pub(super) struct PackageStage {
    owner: String,
    folder: tempfile::TempDir,
    source: Tree,
    installed: Tree,
    files: BTreeMap<String, FileState>,
}
impl PackageStage {
    pub(super) fn prepare(s: &Selection, pending: &mut Plan, programs: &Programs) -> Result<Self> {
        let project = &s.project;
        let Local::Package {
            realm,
            alias,
            package: Some(package),
            declaration,
            ..
        } = &s.local
        else {
            anyhow::bail!("Expected a resolved package");
        };
        let files = ["wally.toml", "wally.lock", "sourcemap.json"]
            .into_iter()
            .map(|n| Ok((n.to_string(), file_state(&project.join(n), false)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        let source = source_tree(project)?;
        let installed = packages(project)?;
        let manifest = text(&project.join("wally.toml"))?;
        let config: toml::Value = toml::from_str(&manifest)?;
        let lock = text(&project.join("wally.lock"))?;
        let folder = tempfile::tempdir()?;
        let exact = edit(
            &manifest,
            realm,
            alias,
            declaration,
            &format!("{package}@={}", s.version),
        )?;
        let reviewed = edit(&manifest, realm, alias, declaration, &s.declaration()?)?;
        let install = |name: &str, decl: &str, lock: &str| -> Result<PathBuf> {
            let dir = folder.path().join(name);
            fs::create_dir(&dir)?;
            copy_tree(project, &dir, &source)?;
            fs::write(dir.join("wally.toml"), decl)?;
            fs::write(dir.join("wally.lock"), lock)?;
            programs.run("wally", &dir, &["install"])?;
            programs.run(
                "rojo",
                &dir,
                &[
                    "sourcemap",
                    "default.project.json",
                    "--output",
                    "sourcemap.json",
                ],
            )?;
            let mut args = vec!["-s", "sourcemap.json"];
            args.extend(DIRS.into_iter().filter(|d| dir.join(d).is_dir()));
            programs.run("wally-package-types", &dir, &args)?;
            Ok(dir)
        };
        let baseline = install("old", &manifest, &lock)?;
        ensure!(
            graph(&text(&baseline.join("wally.lock"))?, &config)? == graph(&lock, &config)?,
            "Current Wally lock cannot be reproduced"
        );
        let old = packages(&baseline)?;
        for p in old.keys() {
            if let Some(base) = p.components().next()
                && installed.contains_key(Path::new(base.as_os_str()))
                && !installed.contains_key(p)
            {
                pending.blockers.push(format!(
                    "Missing path in installed directory: {}",
                    p.display()
                ));
            }
        }
        for (p, e) in &installed {
            if old.get(p).is_none_or(|other| other.content != e.content) {
                pending
                    .blockers
                    .push(format!("Preserve local path {}", p.display()));
            }
        }
        if files["sourcemap.json"].id.is_some()
            && file_state(&baseline.join("sourcemap.json"), false)?.content
                != files["sourcemap.json"].content
        {
            pending.blockers.push("sourcemap.json differs from the generated baseline; preserve or resolve it before Apply".into());
        }
        let candidate = install("selected", &exact, &lock)?;
        let new_lock = text(&candidate.join("wally.lock"))?;
        let replay = install("reviewed", &reviewed, &new_lock)?;
        let new = packages(&candidate)?;
        let replayed = packages(&replay)?;
        ensure!(
            text(&replay.join("wally.lock"))? == new_lock && contents(&new) == contents(&replayed),
            "Cannot reproduce the selected lock and generated package files"
        );
        let (before, old_graph) = graph(&lock, &config)?;
        let (after, new_graph) = graph(&new_lock, &config)?;
        let mut realms = BTreeMap::new();
        for section in ["dependencies", "server-dependencies", "dev-dependencies"] {
            if let Some(table) = config.get(section) {
                for key in table
                    .as_table()
                    .context("Expected dependency table")?
                    .keys()
                {
                    ensure!(
                        realms.insert(key.clone(), section).is_none(),
                        "Ambiguous direct alias {key} across realms"
                    );
                }
            }
        }
        ensure!(
            realms.keys().eq(before.keys()) && realms.keys().eq(after.keys()),
            "Direct declarations do not match the lockfile root"
        );
        ensure!(
            after.get(alias) == Some(&format!("{package}@{}", s.version)),
            "Resolver did not install the selected version"
        );
        for key in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
            if key != alias && before.get(key) != after.get(key) {
                pending.blockers.push(format!(
                    "Other direct alias {}.{key}: {:?} → {:?}",
                    realms[key],
                    before.get(key),
                    after.get(key)
                ));
            }
        }
        pending.changes.push(format!(
            "wally.toml [{realm}] {alias}: {declaration} → {}",
            s.declaration()?
        ));
        pending.changes.extend(
            old_graph
                .difference(&new_graph)
                .map(|v| format!("wally.lock − {v}")),
        );
        pending.changes.extend(
            new_graph
                .difference(&old_graph)
                .map(|v| format!("wally.lock + {v}")),
        );
        pending.changes.push("Rebuild installed package directories, typed links and sourcemap.json; preparation downloads public packages.".into());
        pending.inputs = digest(&(&pending.inputs, &source, &installed, &files))?;
        pending.outputs = digest(&(
            reviewed,
            new_lock,
            contents(&replayed),
            file_state(&replay.join("sourcemap.json"), false)?.content,
        ))?;
        Ok(Self {
            owner: identity(project, true)?.context("Missing project")?,
            folder,
            source,
            installed,
            files,
        })
    }
    pub(super) fn apply(&self, s: &Selection) -> Result<()> {
        let project = &s.project;
        let replay = self.folder.path().join("reviewed");
        ensure!(
            identity(project, true)?.as_ref() == Some(&self.owner),
            "Project directory changed; prepare again"
        );
        ensure!(
            source_tree(project)? == self.source && packages(project)? == self.installed,
            "Local inputs changed; check for updates again"
        );
        for (name, state) in &self.files {
            ensure!(
                file_state(&project.join(name), false)? == *state,
                "{name} changed; check for updates again"
            );
        }
        let prepared = packages(&replay)?;
        let staging = tempfile::Builder::new()
            .prefix(".rproj-update-")
            .tempdir_in(project)?;
        copy_tree(&replay, staging.path(), &prepared)?;
        for name in DIRS {
            let path = project.join(name);
            let original = tree(project, Path::new(name))?;
            let expected: Tree = self
                .installed
                .iter()
                .filter(|(p, _)| p.starts_with(name))
                .map(|(p, s)| (p.clone(), s.clone()))
                .collect();
            ensure!(
                original == expected,
                "{name} changed; inspect partial effects"
            );
            if !original.is_empty() {
                let backup = staging.path().join(format!("previous-{name}"));
                fs::rename(&path, &backup)?;
                // Check the moved directory before it can be discarded. Keep it on any mismatch.
                let verified = tree(&backup, Path::new("")).and_then(|items| {
                    let moved: Tree = items
                        .into_iter()
                        .map(|(p, s)| {
                            (
                                if p.as_os_str().is_empty() {
                                    PathBuf::from(name)
                                } else {
                                    Path::new(name).join(p)
                                },
                                s,
                            )
                        })
                        .collect();
                    ensure!(moved == original, "Directory changed during replacement");
                    Ok(())
                });
                if verified.is_err() {
                    let kept = staging.keep();
                    anyhow::bail!(
                        "Preserved changed files in {}. Inspect the project before retrying",
                        kept.display()
                    );
                }
            }
            if staging.path().join(name).is_dir() {
                fs::rename(staging.path().join(name), &path)?;
            }
        }
        for name in ["wally.toml", "sourcemap.json", "wally.lock"] {
            let bytes = fs::read(replay.join(name))?;
            if self.files[name].content != Some(blake3::hash(&bytes).to_string()) {
                replace(&project.join(name), &bytes, &self.files[name])?;
            }
            ensure!(
                fs::read(project.join(name))? == bytes,
                "{name} did not match the review"
            );
        }
        ensure!(
            contents(&packages(project)?) == contents(&prepared),
            "Installed package tree differs from review"
        );
        Ok(())
    }
}
