//! Read-only preparation and the execution boundary shared by terminal and worker.
use anyhow::{Context, Result, ensure};
use rproj_core::machine_setup::{
    Detection, PathIdentity, Persistence, SetupContext, SetupResult, SetupReview,
};
use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    model::{Selection, SelectionConfig},
    worker::{self, Event, Item, Outcome},
};
use crate::{config::GlobalConfig, path_identity::identity, steps::execution::Reporter};

#[cfg(test)]
mod tests;

fn absolute(path: &Path) -> Result<PathBuf> {
    ensure!(
        !path.components().any(|c| matches!(c, Component::ParentDir)),
        "Machine paths containing '..' must be normalized before setup"
    );
    Ok(std::path::absolute(path)?)
}

fn capture_path(path: &Path, directory: bool) -> Result<Vec<PathIdentity>> {
    let mut paths = path.ancestors().collect::<Vec<_>>();
    paths.reverse();
    paths
        .into_iter()
        .map(|current| {
            let directory = current != path || directory;
            Ok(PathIdentity {
                path: current.into(),
                directory,
                identity: identity(current, directory)?,
            })
        })
        .collect()
}

fn text(path: &Path) -> Result<Option<String>> {
    if identity(path, false)?.is_none() {
        return Ok(None);
    }
    ensure!(
        fs::metadata(path)?.len() <= 2 * 1024 * 1024,
        "Machine configuration exceeds the 2 MiB limit"
    );
    let mut text = String::new();
    fs::File::open(path)?
        .take(2 * 1024 * 1024 + 1)
        .read_to_string(&mut text)
        .context("Cannot read machine configuration; repair it and reload")?;
    ensure!(
        text.len() <= 2 * 1024 * 1024,
        "Machine configuration exceeds the 2 MiB limit"
    );
    Ok(Some(text))
}

pub(crate) fn capture() -> Result<SetupContext> {
    capture_at(
        &GlobalConfig::path()?,
        &GlobalConfig::default().projects_root()?,
    )
}

fn capture_at(config_path: &Path, default_root: &Path) -> Result<SetupContext> {
    let config_path = absolute(config_path)?;
    let mut identities = capture_path(&config_path, false)?;
    let original = text(&config_path)?;
    let config: GlobalConfig = match &original {
        Some(text) => {
            toml::from_str(text).context("Malformed machine configuration; repair it and reload")?
        }
        None => GlobalConfig::default(),
    };
    let mut saved = Selection::load(&GlobalConfig {
        roblox_projects_root: Some(
            config
                .roblox_projects_root
                .clone()
                .unwrap_or_else(|| default_root.into()),
        ),
        ..config
    })?;
    saved.root = absolute(&saved.root)?;
    identities.extend(capture_path(&saved.root, true)?);
    let context = SetupContext {
        config_path,
        original,
        saved,
        identities,
        detected: crate::catalog::tool_catalog::all_setup_entries().map(|entry| (entry.key.into(), Detection::NotVerified("Presence has not been confirmed; saved selection is not installation evidence".into()))).collect(),
    };
    verify(&context)?;
    Ok(context)
}

fn verify_identities(paths: &[PathIdentity]) -> Result<()> {
    for path in paths {
        ensure!(
            identity(&path.path, path.directory)? == path.identity,
            "{} changed since review; reload Machine Setup",
            path.path.display()
        );
    }
    Ok(())
}

fn verify_config(context: &SetupContext) -> Result<()> {
    let paths = context
        .identities
        .iter()
        .filter(|p| context.config_path.starts_with(&p.path))
        .cloned()
        .collect::<Vec<_>>();
    verify_identities(&paths)?;
    ensure!(
        text(&context.config_path)? == context.original,
        "Machine configuration changed; reload Machine Setup before applying"
    );
    Ok(())
}

fn verify(context: &SetupContext) -> Result<()> {
    verify_identities(&context.identities)?;
    verify_config(context)
}

pub(crate) fn prepare(context: &SetupContext, selection: &Selection) -> Result<SetupReview> {
    verify(context)?;
    SetupReview::new(context.clone(), selection.clone())
}

fn authoritative(context: &SetupContext) -> Result<()> {
    let fresh = capture_at(
        &GlobalConfig::path()?,
        &GlobalConfig::default().projects_root()?,
    )?;
    ensure!(
        context.config_path == fresh.config_path
            && context.original == fresh.original
            && context.saved == fresh.saved
            && context.identities == fresh.identities,
        "Machine configuration or captured setup identity changed; reload Machine Setup"
    );
    Ok(())
}

/// A process lock is held only during execution, not while reviewing selections.
struct SetupLock(fs::File);
impl SetupLock {
    fn acquire(path: &Path) -> Result<Self> {
        ensure!(
            identity(path, false)?.is_some() || !path.exists(),
            "Unsafe setup lock"
        );
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;
        file.try_lock()
            .context("Another Machine Setup is running; wait for it to finish")?;
        ensure!(identity(path, false)?.is_some(), "Setup lock disappeared");
        Ok(Self(file))
    }
}
impl Drop for SetupLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

fn lock_path() -> Result<PathBuf> {
    // Config's parent may not exist yet. The per-user runtime directory already
    // exists; locking there keeps preparation and the captured config untouched.
    Ok(std::env::temp_dir().join(format!(
        "rproj-machine-setup-{}.lock",
        lock_key(&GlobalConfig::path()?)
    )))
}

fn lock_key(path: &Path) -> String {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hash);
    format!("{:016x}", hash.finish())
}

pub(crate) fn execute(
    review: &SetupReview,
    stop: &(dyn Fn() -> bool + Sync),
    emit: Arc<dyn Fn(Event) + Send + Sync>,
    mutation: &mut dyn FnMut() -> Result<()>,
) -> Result<SetupResult> {
    authoritative(&review.context)?;
    guarded(&lock_path()?, || {
        execute_with(review, stop, emit, mutation, worker::execute, prerequisites)
    })
}

fn guarded<T>(lock: &Path, run: impl FnOnce() -> Result<T>) -> Result<T> {
    let _lock = SetupLock::acquire(lock)?;
    let _watch = crate::background_watch::mutation_guard(None, false)?;
    run()
}

fn execute_with(
    review: &SetupReview,
    stop: &(dyn Fn() -> bool + Sync),
    emit: Arc<dyn Fn(Event) + Send + Sync>,
    mutation: &mut dyn FnMut() -> Result<()>,
    external: impl Fn(&Item, &Selection, &Reporter) -> Result<()>,
    prerequisites: impl Fn(&Selection) -> Result<()>,
) -> Result<SetupResult> {
    ensure!(
        prepare(&review.context, &review.selection)? == *review,
        "Setup review differs from the authoritative plan; review again"
    );
    prerequisites(&review.selection)?;
    verify(&review.context)?;
    if stop() {
        return Ok(SetupResult {
            outcome: Outcome::Cancelled,
            statuses: vec![worker::Status::Skipped; review.items.len()],
            changes_may_have_begun: false,
            persistence: Persistence::NotSaved,
        });
    }
    mutation()?;
    let mut result = worker::run_items(
        &review.selection,
        stop,
        emit.clone(),
        |item, selection, reporter| {
            if *item == Item::ProjectsFolder {
                let paths = review
                    .context
                    .identities
                    .iter()
                    .filter(|p| selection.root.starts_with(&p.path))
                    .cloned()
                    .collect::<Vec<_>>();
                verify_identities(&paths)?;
            }
            crate::interrupt::finish_active_item(|| external(item, selection, reporter))
        },
    );
    result.changes_may_have_begun = true;
    if !matches!(result.outcome, Outcome::Completed | Outcome::Warnings) {
        return Ok(result);
    }
    // Stop is cooperative and checked once more before committing selections.
    if stop() {
        result.outcome = Outcome::Cancelled;
        return Ok(result);
    }
    result.persistence = match persist(review, stop) {
        Ok(Some(contents)) => Persistence::Saved { contents },
        Ok(None) => {
            result.outcome = Outcome::Cancelled;
            Persistence::NotSaved
        }
        Err(error) => {
            let message =
                format!("Installations finished, but configuration could not be saved: {error:#}");
            emit(Event::Message(
                crate::steps::execution::MessageKind::Warning,
                message.clone(),
            ));
            Persistence::Failed { message }
        }
    };
    // No Stop check follows confirmed persistence: a late request cannot undo it.
    Ok(result)
}

fn persist(review: &SetupReview, stop: &dyn Fn() -> bool) -> Result<Option<String>> {
    let context = &review.context;
    verify_config(context)?;
    let mut parents = capture_path(
        context
            .config_path
            .parent()
            .context("Configuration has no parent")?,
        true,
    )?;
    // Create missing parents one at a time, retaining identities for the final
    // check. Staging must not silently adopt a replaced existing directory.
    for index in 0..parents.len() {
        verify_identities(&parents[..=index])?;
        if parents[index].identity.is_none() {
            fs::create_dir(&parents[index].path)?;
            parents[index].identity = identity(&parents[index].path, true)?;
        }
    }
    let checked_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .to_string();
    let contents = toml::to_string_pretty(&review.selection.to_config(checked_at))?;
    let pending = crate::file_replace::stage(
        &context.config_path,
        contents.as_bytes(),
        context.original.is_some(),
    )?;
    verify_identities(&parents)?;
    let file = context
        .identities
        .iter()
        .find(|p| p.path == context.config_path)
        .context("Missing configuration identity")?;
    verify_identities(std::slice::from_ref(file))?;
    ensure!(
        text(&context.config_path)? == context.original,
        "Machine configuration changed before replacement; reload Machine Setup"
    );
    if stop() {
        return Ok(None);
    }
    pending.persist(&context.config_path).map_err(|e| e.error)?;
    Ok(Some(contents))
}

fn command_present(name: &str, argument: &str) -> bool {
    let mut command = std::process::Command::new(name);
    command
        .arg(argument)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.status().is_ok_and(|status| status.success())
}

fn prerequisites(selection: &Selection) -> Result<()> {
    ensure!(
        command_present("rokit", "--version") || command_present("cargo", "--version"),
        "Rokit bootstrap needs Cargo; install Rust or Rokit, then review again"
    );
    let missing_app = crate::catalog::tool_catalog::SYSTEM_APPS
        .iter()
        .any(|entry| {
            selection.apps.contains(entry.key) && !crate::steps::bootstrap::is_installed(entry)
        });
    ensure!(
        !missing_app || command_present("winget", "--version"),
        "Selected system apps need WinGet; make it available and review again"
    );
    if selection.plugins.contains("rojo-plugin") {
        ensure!(
            selection.apps.contains("studio")
                || crate::catalog::tool_catalog::find("studio")
                    .is_some_and(crate::steps::bootstrap::is_installed),
            "Rojo plugin needs Studio; select Studio or install it before applying"
        );
        ensure!(
            selection.tools.contains("rojo") || command_present("rojo", "--version"),
            "Rojo plugin needs the Rojo CLI; select Rojo or install it before applying"
        );
    }
    Ok(())
}
