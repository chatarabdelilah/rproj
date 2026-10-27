use crate::config::{
    GlobalConfig, Setups,
    setups::{self, checked},
};
use anyhow::{Context, Result, ensure};
use rproj_core::{
    creation::CreationEnvironment,
    saved_setups::{protocol::*, *},
};
use std::{fs, path::Path};

pub(super) fn job(directory: &Path) -> Result<()> {
    job_at(directory, &Setups::dir()?, &GlobalConfig::path()?)
}

fn listing(root: &Path, config: &Path) -> Result<SetupListing> {
    // Check ancestors even when storage does not yet exist. Browsing creates nothing.
    setups::setup_path(root, "Validation")?;
    let mut listing = SetupListing {
        root: root.to_path_buf(),
        entries: vec![],
        environment: None,
        environment_problem: None,
    };
    let machine = (|| -> Result<CreationEnvironment> {
        setups::setup_path(
            config.parent().context("Missing config parent")?,
            "Validation",
        )?;
        crate::path_identity::identity(config, false)?
            .context("Machine configuration is missing; file suggestions use composition only")?;
        let bytes = rproj_core::creation::protocol::read_bounded(config)?;
        let config: GlobalConfig = toml::from_str(std::str::from_utf8(&bytes)?).map_err(|_| {
            anyhow::anyhow!(
                "Machine configuration is malformed; file suggestions use composition only"
            )
        })?;
        Ok(CreationEnvironment {
            projects_root: config.projects_root()?,
            apps: config.selected_system_apps,
            extensions: config.selected_vscode_extensions,
        })
    })();
    match machine {
        Ok(env) => listing.environment = Some(env),
        Err(e) => listing.environment_problem = Some(format!("{e:#}")),
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(listing),
        Err(e) => return Err(e).context("Cannot browse saved setup storage"),
    };
    listing.root = root.canonicalize()?;
    let storage_identity = crate::path_identity::identity(&listing.root, true)?;
    let mut size = 0usize;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "toml") || entry.file_type()?.is_dir() {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let (summary, warning) =
            match checked::capture_with_limit(&listing.root, name, Some(IPC_LIMIT))
                .map(LoadedSetup::new)
            {
                Ok(loaded) => (
                    loaded.graph.as_ref().map_or_else(
                        || "Unreadable composition".into(),
                        |g| {
                            format!(
                                "{:?} · {} packages · {} capabilities",
                                g.package_workflow,
                                g.packages.len(),
                                g.capabilities.len()
                            )
                        },
                    ),
                    loaded
                        .problem
                        .or(loaded.edit_problem)
                        .or(loaded.reuse_problem),
                ),
                Err(e) => ("Unavailable document".into(), Some(format!("{e:#}"))),
            };
        let entry = SetupEntry {
            name: name.into(),
            path: listing.root.join(format!("{name}.toml")),
            summary,
            warning,
        };
        size += serde_json::to_vec(&entry)?.len();
        ensure!(
            size <= IPC_LIMIT as usize,
            "Saved setup listing exceeds the 2 MiB worker limit; use terminal Saved Setups"
        );
        listing.entries.push(entry);
    }
    ensure!(
        crate::path_identity::identity(&listing.root, true)? == storage_identity,
        "Setup storage changed during listing; refresh"
    );
    listing.entries.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.name.cmp(&b.name))
    });
    Ok(listing)
}

fn verify_root(root: &Path, source: &CapturedSetup) -> Result<()> {
    setups::setup_path(root, &source.name)?;
    ensure!(
        root.canonicalize()? == source.root,
        "Saved setup belongs to another storage directory; reload"
    );
    ensure!(
        checked::capture_with_limit(root, &source.name, Some(IPC_LIMIT))? == *source,
        "Saved setup or storage changed; Reload from file"
    );
    Ok(())
}

fn job_at(directory: &Path, root: &Path, config: &Path) -> Result<()> {
    ensure!(
        directory.is_absolute() && directory.is_dir(),
        "Saved Setups requires an absolute IPC directory"
    );
    let request: SavedRequest = read_json(&directory.join("request.json"))?;
    request.validate()?;
    let path = directory.join("report.json");
    let mut report = SavedReport::initial(&request);
    write_atomic(&path, &report)?;
    let result = perform(&request, root, config, &path, &mut report);
    if let Err(error) = result {
        report.status = Some(if report.mutation == MutationState::NotStarted {
            ManagementStatus::Failed
        } else {
            ManagementStatus::Uncertain
        });
        report.listing = None;
        report.loaded = None;
        report.prepared = None;
        report.reused = None;
        report.outcome = None;
        report.message = Some(format!("{error:#}").chars().take(8192).collect());
    }
    report.check(&request)?;
    write_atomic(&path, &report)?;
    ensure!(
        report.status == Some(ManagementStatus::Succeeded),
        "{}",
        report
            .message
            .as_deref()
            .unwrap_or("Saved setup operation did not complete; reload before retry")
    );
    Ok(())
}

fn perform(
    request: &SavedRequest,
    root: &Path,
    config: &Path,
    path: &Path,
    report: &mut SavedReport,
) -> Result<()> {
    if let Some(source) = request.action.source() {
        verify_root(root, source)?;
    }
    match &request.action {
        SavedAction::Browse => report.listing = Some(listing(root, config)?),
        SavedAction::Load(name) => {
            report.loaded = Some(LoadedSetup::new(checked::capture_with_limit(
                root,
                name,
                Some(IPC_LIMIT),
            )?))
        }
        SavedAction::Prepare { source, action } => {
            report.prepared = Some(PreparedSetup::new(checked::prepare_with_limit(
                source,
                action.clone(),
                Some(IPC_LIMIT),
            )?)?)
        }
        SavedAction::Reuse(source) => {
            let graph = reusable_graph(&source.bytes)?;
            verify_root(root, source)?;
            report.reused = Some(ReusedSetup {
                source: source.clone(),
                graph,
            });
        }
        SavedAction::Execute(review) => {
            println!("Updating saved setup {}", review.source.name);
            let outcome = checked::execute_with_limit(
                review,
                &mut |mutation| {
                    report.mutation = mutation;
                    write_atomic(path, report)
                },
                Some(IPC_LIMIT),
            );
            report.status = Some(outcome.status);
            report.mutation = outcome.mutation;
            report.message.clone_from(&outcome.message);
            report.outcome = Some(outcome);
        }
    }
    if report.status.is_none() {
        report.status = Some(ManagementStatus::Succeeded);
    }
    ensure!(
        serde_json::to_vec(report)?.len() as u64 <= IPC_LIMIT,
        "Saved Setups response exceeds the 2 MiB worker limit; use terminal management. Reload if a mutation was started."
    );
    Ok(())
}

#[cfg(test)]
mod tests;
