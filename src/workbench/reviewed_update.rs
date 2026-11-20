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
    ensure!(cfg!(windows), "Reviewed updates currently require Windows");
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
    if serde_json::to_vec(&report)?.len() as u64 > rproj_core::creation::protocol::IPC_LIMIT {
        report.state = State::Failed {
            possible_writes: false,
            reason: "Prepared review exceeds the 2 MiB limit. No changes applied.".into(),
        };
    }
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

fn version_probe(exe: &Path, dir: &Path) -> Result<String> {
    use std::{
        io::Read,
        time::{Duration, Instant},
    };
    let mut child = Command::new(exe)
        .arg("--version")
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;
    let pipe = child
        .stdout
        .take()
        .context("Missing version probe output")?;
    let (send, recv) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = pipe.take(4097).read_to_end(&mut bytes).map(|_| bytes);
        let _ = send.send(result);
    });
    let mut response = None;
    let started = Instant::now();
    loop {
        if response.is_none() {
            response = recv.try_recv().ok().transpose()?;
        }
        if response.as_ref().is_some_and(|b| b.len() > 4096)
            || started.elapsed() > Duration::from_secs(5)
        {
            let _ = child.kill();
            child.wait()?;
            anyhow::bail!(
                "Version probe exceeded time or output limit at {}",
                exe.display()
            );
        }
        if response.is_some()
            && let Some(status) = child.try_wait()?
        {
            ensure!(
                status.success(),
                "Version probe failed at {}",
                exe.display()
            );
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let text = String::from_utf8(response.context("Missing version response")?)?;
    ensure!(
        text.len() <= 4096 && !text.trim().is_empty(),
        "Invalid version output at {}",
        exe.display()
    );
    Ok(text.trim().to_owned())
}
struct Programs(
    BTreeMap<String, (PathBuf, FileState, String)>,
    #[allow(dead_code)] Vec<fs::File>,
);
impl Programs {
    fn capture(s: &Selection) -> Result<Self> {
        let mut result = BTreeMap::new();
        let mut handles = Vec::new();
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
            let mut options = fs::OpenOptions::new();
            options.read(true);
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                options.share_mode(1);
            }
            handles.push(options.open(&path).with_context(|| {
                format!("Cannot hold {name} stable during the reviewed operation")
            })?);
            let version = version_probe(&path, &s.project)?;
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
        Ok(Self(result, handles))
    }
    fn verify(&self, dir: &Path) -> Result<()> {
        for (name, (exe, state, version)) in &self.0 {
            ensure!(
                file_state(exe, false)? == *state && version_probe(exe, dir)? == *version,
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
        // A successful Rokit add may reuse an existing cache entry. Compare it
        // to a fresh copy of the same release without executing either file.
        let isolated = dir.join("verification-storage");
        let check_dir = dir.join("verification-project");
        fs::create_dir(&isolated)?;
        fs::create_dir(&check_dir)?;
        fs::write(check_dir.join("rokit.toml"), "[tools]\n")?;
        for args in [vec!["trust", repo], vec!["add", &pin, alias]] {
            let status = Command::new(&programs.0["rokit"].0)
                .env("ROKIT_ROOT", &isolated)
                .current_dir(&check_dir)
                .stdin(Stdio::null())
                .args(args)
                .status()?;
            ensure!(
                status.success(),
                "Could not verify {pin} using isolated tool storage"
            );
        }
        let fresh = isolated.join(storage.strip_prefix(&self.root)?).join(
            artifacts[0]
                .file_name()
                .context("Missing artifact filename")?,
        );
        let observed = file_state(&artifacts[0], false)?.content;
        ensure!(
            observed.is_some() && observed == file_state(&fresh, false)?.content,
            "Cached executable differs from the downloaded {pin} release; inspect the shared cache before retrying"
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
            plan.changes.push("Install the exact release and alias with Rokit. This changes the shared cache and shim. Verify the executable against a fresh download in disposable storage. Existing trust is required.".into());
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
