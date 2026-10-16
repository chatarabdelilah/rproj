use crate::{
    commands::new,
    config::{GlobalConfig, Setups, project_template, setups},
};
use anyhow::{Context, Result, ensure};
use rproj_core::creation::{CreationDraft, CreationEnvironment, protocol::*};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(all(test, windows))]
#[path = "creation_tests.rs"]
mod worker_tests;

struct Storage {
    config: PathBuf,
    template: PathBuf,
    setups: PathBuf,
}

impl Storage {
    fn system() -> Result<Self> {
        Ok(Self {
            config: GlobalConfig::path()?,
            template: project_template::path()?,
            setups: Setups::dir()?,
        })
    }
}

fn snapshot(path: &Path) -> Result<SourceSnapshot> {
    match read_bounded(path) {
        Ok(bytes) => Ok(SourceSnapshot { bytes: Some(bytes) }),
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            Ok(SourceSnapshot { bytes: None })
        }
        Err(error) => Err(error),
    }
}

fn machine(source: &SourceSnapshot) -> Result<GlobalConfig> {
    source
        .bytes
        .as_ref()
        .map(|bytes| {
            toml::from_str(
                std::str::from_utf8(bytes).context("Machine configuration is not UTF-8")?,
            )
            .map_err(|error: toml::de::Error| {
                anyhow::anyhow!(
                    "Cannot parse machine configuration near byte {}; repair it with rproj setup",
                    error.span().map_or(0, |span| span.start)
                )
            })
        })
        .unwrap_or_else(|| Ok(GlobalConfig::default()))
}

fn saved(storage: &Storage) -> Result<Vec<SavedComposition>> {
    setups::setup_path(&storage.setups, "validation")?;
    let entries = match fs::read_dir(&storage.setups) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(error) => return Err(error).context("Cannot read saved setup storage"),
    };
    let mut result = Vec::new();
    let mut bytes_used = 0_u64;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "toml") || entry.file_type()?.is_dir() {
            continue;
        }
        let name = path
            .file_stem()
            .context("Missing setup name")?
            .to_str()
            .context("Setup name is not Unicode")?
            .to_owned();
        let loaded = (|| -> Result<(SourceSnapshot, crate::graph::ProjectGraph)> {
            let source = snapshot(&setups::setup_path(&storage.setups, &name)?)?;
            let bytes = source.bytes.as_ref().context("Saved setup disappeared")?;
            let value: toml::Value =
                toml::from_str(std::str::from_utf8(bytes)?).map_err(|error: toml::de::Error| {
                    anyhow::anyhow!(
                        "Malformed saved setup TOML near byte {}",
                        error.span().map_or(0, |span| span.start)
                    )
                })?;
            let table = value.as_table().context("Saved setup is not a table")?;
            ensure!(
                table.keys().all(|k| [
                    "mode",
                    "package_workflow",
                    "packages",
                    "dropped",
                    "capabilities"
                ]
                .contains(&k.as_str())),
                "Saved setup contains unsupported fields; inspect it with Saved Setups"
            );
            let graph: crate::graph::ProjectGraph = value
                .try_into()
                .map_err(|_| anyhow::anyhow!("Unsupported saved setup structure"))?;
            CreationDraft {
                name: "Validation".into(),
                graph: graph.clone(),
                save_setup: None,
            }
            .validate()?;
            Ok((source, graph))
        })();
        match loaded {
            Ok((source, graph)) => result.push(SavedComposition {
                name,
                graph: Some(graph),
                problem: None,
                source,
            }),
            Err(error) => result.push(SavedComposition {
                name,
                graph: None,
                problem: Some(format!("{error:#}")),
                source: SourceSnapshot { bytes: None },
            }),
        }
        bytes_used += serde_json::to_vec(result.last().expect("saved entry"))?.len() as u64;
        ensure!(
            bytes_used <= IPC_LIMIT,
            "Saved setup data exceeded the creation context limit; no setup list was truncated"
        );
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

fn context_at(storage: &Storage) -> Result<CreationContext> {
    let config_source = snapshot(&storage.config)?;
    let config = machine(&config_source)?;
    let projects_root = config.projects_root()?;
    ensure!(
        projects_root.is_absolute(),
        "The configured projects root must be absolute; review it with rproj setup"
    );
    let template_source = snapshot(&storage.template)?;
    let parsed = template_source
        .bytes
        .as_ref()
        .map(|bytes| serde_json::from_slice(bytes))
        .transpose();
    let (template, template_problem) = match parsed {
        Ok(template) => (template, None),
        Err(error) => (
            None,
            Some(format!(
                "Cannot parse the saved project template: {error}. Repair it with rproj configure project"
            )),
        ),
    };
    let (setups, setups_problem) = match saved(storage) {
        Ok(entries) => (entries, None),
        Err(error) => (vec![], Some(format!("{error:#}"))),
    };
    Ok(CreationContext {
        protocol: CREATION_PROTOCOL,
        environment: CreationEnvironment {
            projects_root,
            apps: config.selected_system_apps.clone(),
            extensions: config.selected_vscode_extensions.clone(),
        },
        machine_ready: config.machine_configured(),
        template,
        template_problem,
        config_source,
        template_source,
        setups,
        setups_problem,
    })
}

pub(super) fn context() -> Result<CreationContext> {
    context_at(&Storage::system()?)
}

fn prepare_at(
    storage: &Storage,
    request: &PrepareCreation,
    validate_template: impl FnOnce(&serde_json::Value) -> Result<()>,
) -> Result<PreparedCreation> {
    let context = context_at(storage)?;
    ensure!(
        context.machine_ready,
        "Run rproj setup, then Recheck; New Project does not install machine applications"
    );
    ensure!(
        context.template_problem.is_none(),
        "{}",
        context.template_problem.as_deref().unwrap_or_default()
    );
    ensure!(
        context.setups_problem.is_none(),
        "{}",
        context.setups_problem.as_deref().unwrap_or_default()
    );
    let setup = request
        .sources
        .selected_setup
        .as_ref()
        .map(|(name, _)| name.as_str());
    ensure!(
        context.sources(setup)? == request.sources,
        "Creation sources changed; refresh and review again before creating"
    );
    let plan = request.draft.plan(&context.environment)?;
    match fs::symlink_metadata(&plan.destination) {
        Ok(_) => anyhow::bail!(
            "Destination already exists; choose a new name. Nothing will be overwritten"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("Cannot inspect the creation destination"),
    }
    if let Some(name) = &plan.draft.save_setup {
        setups::setup_path(&storage.setups, name)?;
        ensure!(
            !context
                .setups
                .iter()
                .any(|s| s.name.to_lowercase() == name.to_lowercase()),
            "Saved setup name already exists; choose a new name"
        );
    }
    if let Some(template) = &context.template {
        validate_template(template)?;
    }
    Ok(PreparedCreation {
        plan,
        sources: request.sources.clone(),
        template: context.template,
    })
}

fn perform(
    storage: &Storage,
    request: &CreationRequest,
    report_path: &Path,
    report: &mut CreationReport,
) -> Result<()> {
    let validate = crate::steps::rojo::validate_template_with_rojo;
    match &request.action {
        CreationAction::Prepare(selection) => {
            report.prepared = Some(prepare_at(storage, selection, validate)?);
        }
        CreationAction::Execute(prepared) => {
            let selection = PrepareCreation {
                draft: prepared.plan.draft.clone(),
                sources: prepared.sources.clone(),
            };
            let checked = prepare_at(storage, &selection, validate)?;
            ensure!(
                &checked == prepared,
                "The reviewed creation plan changed; review again before creating"
            );
            let mut config = machine(&checked.sources.config)?;
            let graph = &checked.plan.draft.graph;
            let planned = graph.plan(
                &config.selected_system_apps,
                &config.selected_vscode_extensions,
            );
            let _guard = crate::background_watch::creation_guard(&checked.plan.destination)?;
            let save_path = checked
                .plan
                .draft
                .save_setup
                .as_deref()
                .map(|name| setups::setup_path(&storage.setups, name))
                .transpose()?;
            new::execute_reviewed(
                new::ConfirmedCreation {
                    name: &checked.plan.draft.name,
                    project_dir: &checked.plan.destination,
                    graph,
                    planned: &planned,
                    project_template: checked.template.as_ref(),
                    save_setup: checked
                        .plan
                        .draft
                        .save_setup
                        .as_deref()
                        .zip(save_path.as_deref())
                        .map(|(name, path)| new::SetupSave::NewAt { name, path }),
                    config_path: Some(&storage.config),
                },
                &mut config,
                &mut |claim| {
                    report.claim = claim;
                    write_atomic(report_path, report)
                },
            )?;
        }
    }
    Ok(())
}

pub(super) fn job(directory: &Path) -> Result<()> {
    job_at(directory, &Storage::system()?)
}

fn job_at(directory: &Path, storage: &Storage) -> Result<()> {
    ensure!(
        directory.is_absolute() && directory.is_dir(),
        "Creation requires an existing absolute IPC directory"
    );
    let request: CreationRequest = read_json(&directory.join("request.json"))?;
    ensure!(
        request.protocol == CREATION_PROTOCOL && !request.id.is_empty() && request.id.len() <= 256,
        "Unsupported creation request"
    );
    let path = directory.join("report.json");
    let mut report = CreationReport::initial(request.id.clone());
    write_atomic(&path, &report)?;
    let result = perform(storage, &request, &path, &mut report);
    report.status = Some(match &result {
        Ok(()) => CreationStatus::Succeeded,
        Err(error) if crate::interrupt::is_cancelled(error) => CreationStatus::Cancelled,
        Err(_) => CreationStatus::Failed,
    });
    report.error = result
        .as_ref()
        .err()
        .map(|error| format!("{error:#}").chars().take(8192).collect());
    write_atomic(&path, &report)?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Storage) {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let storage = Storage {
            config: root.join("config.toml"),
            template: root.join("template.json"),
            setups: root.join("setups"),
        };
        let config = GlobalConfig {
            roblox_projects_root: Some(root.join("projects")),
            last_checked: Some("fixture".into()),
            ..Default::default()
        };
        fs::write(&storage.config, toml::to_string(&config).unwrap()).unwrap();
        (directory, storage)
    }
    fn selection(storage: &Storage) -> PrepareCreation {
        let mut draft = CreationDraft::new("日本語 with spaces".into());
        draft.set_workflow(rproj_core::creation::PackageWorkflow::None);
        PrepareCreation {
            draft,
            sources: context_at(storage).unwrap().sources(None).unwrap(),
        }
    }
    #[test]
    fn context_and_preparation_do_not_create_destinations_or_change_sources() {
        let (_directory, storage) = fixture();
        let before = fs::read(&storage.config).unwrap();
        let request = selection(&storage);
        let plan = prepare_at(&storage, &request, |_| panic!("no custom template")).unwrap();
        assert!(!plan.plan.destination.exists());
        assert!(!plan.plan.destination.parent().unwrap().exists());
        assert_eq!(fs::read(&storage.config).unwrap(), before);
        assert!(!storage.setups.exists());
    }
    #[test]
    fn stale_sources_destinations_and_saved_names_are_refused_without_writes() {
        let (_directory, storage) = fixture();
        let mut request = selection(&storage);
        fs::write(&storage.template, b"{}").unwrap();
        assert!(
            prepare_at(&storage, &request, |_| Ok(()))
                .unwrap_err()
                .to_string()
                .contains("changed")
        );
        request.sources = context_at(&storage).unwrap().sources(None).unwrap();
        let destination = context_at(&storage)
            .unwrap()
            .environment
            .projects_root
            .join(&request.draft.name);
        fs::create_dir_all(&destination).unwrap();
        assert!(prepare_at(&storage, &request, |_| Ok(())).is_err());
        assert_eq!(fs::read_dir(destination).unwrap().count(), 0);
        request.draft.name = "Different".into();
        fs::create_dir_all(&storage.setups).unwrap();
        fs::write(storage.setups.join("Existing.toml"), "mode='expert'\n").unwrap();
        request.draft.save_setup = Some("existing".into());
        assert!(
            prepare_at(&storage, &request, |_| Ok(()))
                .unwrap_err()
                .to_string()
                .contains("already exists")
        );
    }
    #[test]
    fn malformed_or_future_saved_compositions_remain_visible_and_unchanged() {
        let (_directory, storage) = fixture();
        fs::create_dir_all(&storage.setups).unwrap();
        for (name, bytes) in [
            ("malformed", "not [toml"),
            ("future", "future = true\n"),
            ("unknown", "packages=['future-package']\n"),
        ] {
            let path = storage.setups.join(format!("{name}.toml"));
            fs::write(&path, bytes).unwrap();
            let context = context_at(&storage).unwrap();
            let entry = context.setups.iter().find(|s| s.name == name).unwrap();
            assert!(entry.problem.is_some() && entry.graph.is_none());
            assert!(context.sources(Some(name)).is_err());
            assert_eq!(fs::read_to_string(path).unwrap(), bytes);
        }
    }

    #[test]
    fn selected_setup_source_and_machine_readiness_are_revalidated() {
        let (_directory, storage) = fixture();
        fs::create_dir_all(&storage.setups).unwrap();
        let path = storage.setups.join("Selected.toml");
        fs::write(&path, "mode='expert'\npackage_workflow='none'\n").unwrap();
        let mut request = selection(&storage);
        request.sources = context_at(&storage)
            .unwrap()
            .sources(Some("Selected"))
            .unwrap();
        prepare_at(&storage, &request, |_| Ok(())).unwrap();
        fs::write(
            &path,
            "# external edit\nmode='expert'\npackage_workflow='none'\n",
        )
        .unwrap();
        assert!(
            prepare_at(&storage, &request, |_| Ok(()))
                .unwrap_err()
                .to_string()
                .contains("changed")
        );
        let mut config = machine(&snapshot(&storage.config).unwrap()).unwrap();
        config.last_checked = None;
        fs::write(&storage.config, toml::to_string(&config).unwrap()).unwrap();
        assert!(
            prepare_at(&storage, &request, |_| Ok(()))
                .unwrap_err()
                .to_string()
                .contains("rproj setup")
        );
        assert!(!config.projects_root().unwrap().exists());
    }

    #[test]
    fn native_parse_diagnostics_do_not_echo_unrelated_values() {
        let source = SourceSnapshot {
            bytes: Some(b"selected_system_apps='private-value'\n".to_vec()),
        };
        assert!(!format!("{:#}", machine(&source).unwrap_err()).contains("private-value"));
        let (_directory, storage) = fixture();
        fs::create_dir_all(&storage.setups).unwrap();
        fs::write(
            storage.setups.join("Malformed.toml"),
            "private-value='unterminated",
        )
        .unwrap();
        let context = context_at(&storage).unwrap();
        assert!(
            !context.setups[0]
                .problem
                .as_ref()
                .unwrap()
                .contains("private-value")
        );
    }
}
