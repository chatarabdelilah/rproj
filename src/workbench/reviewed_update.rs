//! Rebuild reviewed effects from the captured selection before every write.
mod files;
mod packages;
#[cfg(test)]
mod tests;

use crate::path_identity::identity;
use anyhow::{Context, Result, ensure};
use files::*;
use rproj_core::{
    creation::protocol::{read_json, write_atomic},
    updates::{Local, review::*},
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub(super) fn job(dir: &Path) -> Result<()> {
    ensure!(
        dir.is_absolute() && identity(dir, true)?.is_some(),
        "Invalid operation directory"
    );
    let request: Request = read_json(&dir.join("request.json"))?;
    let selection = request.action.selection();
    ensure!(
        request.protocol == PROTOCOL
            && dir.file_name().and_then(|s| s.to_str()) == Some(request.id.as_str()),
        "Invalid reviewed update request"
    );
    let path = dir.join("report.json");
    let mut report = Report {
        protocol: PROTOCOL,
        id: request.id.clone(),
        selection: selection.clone(),
        state: State::Preparing,
    };
    write_atomic(&path, &report)?;
    let result = (|| -> Result<State> {
        ensure!(
            identity(&selection.project, true)?.is_some()
                && selection.project.canonicalize()? == selection.project,
            "Select an existing canonical project"
        );
        let _guard = if matches!(&request.action, Action::Apply(_)) {
            Some(crate::background_watch::mutation_guard(
                Some(&selection.project),
                false,
            )?)
        } else {
            None
        };
        let (plan, stage, programs) = prepare(selection)?;
        match &request.action {
            Action::Prepare(_) => Ok(State::Prepared(Box::new(plan))),
            Action::Apply(review) => {
                ensure!(
                    &plan == review,
                    "Inputs or resolution changed; check for updates and prepare again"
                );
                ensure!(
                    plan.blockers.is_empty(),
                    "Resolve the blocked changes before Apply"
                );
                programs.verify(&selection.project)?;
                report.state = State::Writing;
                write_atomic(&path, &report)?;
                match stage {
                    Stage::Wally(stage) => stage.apply(selection)?,
                    Stage::Tool(stage) => stage.apply(selection, &programs)?,
                }
                Ok(State::Applied)
            }
        }
    })();
    report.state = match result {
        Ok(s) => s,
        Err(ref e) => State::Failed {
            possible_writes: matches!(report.state, State::Writing),
            reason: format!("{e:#}").chars().take(8192).collect(),
        },
    };
    let failed = matches!(report.state, State::Failed { .. });
    report.verify(&request, !failed)?;
    write_atomic(&path, &report)?;
    if let State::Failed { reason, .. } = report.state {
        anyhow::bail!("{reason}");
    }
    Ok(())
}

type Fields = BTreeMap<String, toml::Spanned<String>>;
#[derive(Default, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct Declarations {
    tools: Fields,
    dependencies: Fields,
    server_dependencies: Fields,
    dev_dependencies: Fields,
}

fn edit(text: &str, section: &str, alias: &str, before: &str, after: &str) -> Result<String> {
    let model: Declarations = toml::from_str(text)?;
    let fields = match section {
        "tools" => model.tools,
        "dependencies" => model.dependencies,
        "server-dependencies" => model.server_dependencies,
        "dev-dependencies" => model.dev_dependencies,
        _ => anyhow::bail!("Unexpected declaration section"),
    };
    let field = fields.get(alias).context("Missing selected alias")?;
    ensure!(field.get_ref() == before, "Selected declaration changed");
    if before == after {
        return Ok(text.to_owned());
    }
    let mut result = text.to_owned();
    result.replace_range(
        field.span(),
        &toml::Value::String(after.to_owned()).to_string(),
    );
    Ok(result)
}

fn tool_root() -> Result<PathBuf> {
    let root = std::env::var_os("ROKIT_ROOT")
        .map(PathBuf::from)
        .or_else(|| Some(directories::BaseDirs::new()?.home_dir().join(".rokit")))
        .context("Missing Rokit storage")?;
    ensure!(root.is_absolute(), "ROKIT_ROOT must be absolute");
    Ok(root)
}

struct Programs(BTreeMap<String, (PathBuf, FileState, String)>);
impl Programs {
    fn capture(s: &Selection) -> Result<Self> {
        let mut result = BTreeMap::new();
        let search = std::env::var_os("PATH").context("Missing PATH")?;
        let wanted: &[&str] = if s.local.is_package() {
            &["wally", "rojo", "wally-package-types"]
        } else {
            &["rokit"]
        };
        for &name in wanted {
            let exe = format!("{name}{}", std::env::consts::EXE_SUFFIX);
            let path = std::env::split_paths(&search)
                .filter(|p| p.is_absolute())
                .map(|p| p.join(&exe))
                .find(|p| p.is_file())
                .with_context(|| {
                    format!("Missing {name}; install the prerequisite and check again")
                })?
                .canonicalize()?;
            let output = Command::new(&path)
                .arg("--version")
                .current_dir(&s.project)
                .stdin(Stdio::null())
                .output()?;
            ensure!(output.status.success(), "Cannot verify {name} version");
            let version = String::from_utf8(output.stdout)?.trim().to_owned();
            if name == "wally" {
                ensure!(
                    version == "wally 0.3.2",
                    "This update requires released Wally 0.3.2"
                );
            }
            if name == "rokit" {
                ensure!(
                    version == "rokit 1.2.0",
                    "This update requires released Rokit 1.2.0"
                );
            }
            result.insert(
                name.into(),
                (path.clone(), file_state(&path, false)?, version),
            );
        }
        Ok(Self(result))
    }
    fn verify(&self, dir: &Path) -> Result<()> {
        for (name, (exe, state, version)) in &self.0 {
            let check = Command::new(exe)
                .arg("--version")
                .current_dir(dir)
                .stdin(Stdio::null())
                .output()?;
            ensure!(
                file_state(exe, false)? == *state
                    && check.status.success()
                    && String::from_utf8_lossy(&check.stdout).trim() == version,
                "{name} changed; prepare again"
            );
        }
        Ok(())
    }
    fn run(&self, name: &str, dir: &Path, args: &[&str]) -> Result<()> {
        let status = Command::new(&self.0[name].0)
            .current_dir(dir)
            .args(args)
            .stdin(Stdio::null())
            .status()?;
        ensure!(
            status.success(),
            "{name} exited with {status}; check output and prerequisites"
        );
        Ok(())
    }
}

struct ToolStage {
    owner: String,
    source: FileState,
    reviewed: String,
    root: PathBuf,
}
impl ToolStage {
    fn apply(&self, s: &Selection, programs: &Programs) -> Result<()> {
        let Local::Tool { alias, .. } = &s.local else {
            anyhow::bail!("Expected a tool pin");
        };
        let scratch = tempfile::tempdir()?;
        let dir = scratch.path();
        fs::write(dir.join("rokit.toml"), "[tools]\n")?;
        let pin = s.declaration()?;
        programs.run("rokit", dir, &["add", &pin, alias]).context(
            "Rokit requires normal trust; check the repository before trusting it manually",
        )?;
        let manifest: Declarations = toml::from_str(&text(&dir.join("rokit.toml"))?)?;
        let actual = manifest
            .tools
            .get(alias)
            .context("Missing installed alias")?
            .get_ref();
        ensure!(
            manifest.tools.len() == 1 && actual.eq_ignore_ascii_case(&pin),
            "Unexpected installed pin or repository"
        );
        let (repo, ver) = pin.split_once('@').context("Invalid pin")?;
        let storage = self
            .root
            .join("tool-storage")
            .join(repo.to_ascii_lowercase())
            .join(ver);
        ensure!(
            identity(&storage, true)?.is_some(),
            "Exact installed version directory missing"
        );
        let mut artifacts = Vec::new();
        for entry in fs::read_dir(&storage)? {
            let p = entry?.path();
            if p.extension().is_some_and(|v| v.eq_ignore_ascii_case("exe")) {
                ensure!(
                    identity(&p, false)?.is_some() && fs::metadata(&p)?.len() > 0,
                    "Empty installed executable"
                );
                artifacts.push(p);
            }
        }
        ensure!(
            artifacts.len() == 1,
            "Could not identify a unique executable for {pin} in {}",
            storage.display()
        );
        println!("Verified installed artifact {}", artifacts[0].display());
        let path = s.project.join("rokit.toml");
        ensure!(
            identity(&s.project, true)?.as_ref() == Some(&self.owner),
            "Project directory changed during installation; inspect the shared tool cache"
        );
        replace(&path, self.reviewed.as_bytes(), &self.source)?;
        ensure!(
            text(&path)? == self.reviewed,
            "Final tool manifest differs from the reviewed file"
        );
        Ok(())
    }
}

enum Stage {
    Wally(packages::PackageStage),
    Tool(ToolStage),
}
fn prepare(s: &Selection) -> Result<(Plan, Stage, Programs)> {
    let decl = s.declaration()?;
    let alias = s.local.alias();
    ensure!(
        !alias.is_empty()
            && !alias.starts_with('-')
            && alias
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-')),
        "Unsupported alias for reviewed update"
    );
    let entry = crate::commands::updates::collect_selected(&s.project, &s.local)?;
    ensure!(
        choices(&s.project, &entry).contains(s),
        "Discovery is stale or unavailable; check for updates again"
    );
    let programs = Programs::capture(s)?;
    let owner = identity(&s.project, true)?.context("Missing project")?;
    let mut plan = Plan {
        selection: s.clone(),
        inputs: digest(&(&owner, &programs.0))?,
        outputs: String::new(),
        changes: vec![],
        blockers: vec![],
    };
    let stage = match &s.local {
        Local::Tool { declaration, .. } => {
            let path = s.project.join("rokit.toml");
            let source = file_state(&path, false)?;
            let root = tool_root()?;
            let reviewed = edit(&text(&path)?, "tools", alias, declaration, &decl)?;
            plan.inputs = digest(&(&plan.inputs, &source, &root))?;
            plan.outputs = digest(&reviewed)?;
            plan.changes.push(format!(
                "rokit.toml [tools] {alias}: {declaration} → {decl}"
            ));
            plan.changes.push("Install the exact release and alias with Rokit. This may download the executable and change the shared cache and shim. Existing trust is required.".into());
            Stage::Tool(ToolStage {
                owner: owner.clone(),
                source,
                reviewed,
                root,
            })
        }
        Local::Package { .. } => {
            Stage::Wally(packages::PackageStage::prepare(s, &mut plan, &programs)?)
        }
    };
    ensure!(
        identity(&s.project, true)?.as_ref() == Some(&owner),
        "Project directory changed"
    );
    Ok((plan, stage, programs))
}
