use crate::commands::{creation::ProjectEdit, project_composition};
use anyhow::{Context, Result, ensure};
use rproj_core::creation::{CompositionDraft, composition_protocol::*};
use std::path::Path;

#[cfg(all(test, windows))]
#[path = "composition_tests.rs"]
mod tests;

pub(super) fn context(project: &Path) -> Result<CompositionContext> {
    let project = project
        .canonicalize()
        .context("Cannot open the captured project")?;
    ensure!(
        project.is_dir(),
        "Existing-project editing requires a directory"
    );
    let bytes = match rproj_core::creation::protocol::read_bounded(&project.join("rproj.toml")) {
        Ok(bytes) => Some(bytes),
        Err(error) if !project.join("rproj.toml").exists() => {
            let _ = error;
            None
        }
        Err(error) => return Err(error),
    };
    let source = bytes
        .map(String::from_utf8)
        .transpose()
        .context("rproj.toml must be UTF-8")?
        .unwrap_or_default();
    let graph = toml::from_str::<rproj_core::creation::ProjectGraph>(&source);
    let (graph, problem) = if !project.join("rproj.toml").is_file() {
        (None, Some("Missing rproj.toml; editing needs a recorded composition. Project adoption is unavailable.".into()))
    } else {
        match graph {
            Ok(graph) => {
                let problem = CompositionDraft::new(graph.clone())
                    .validate()
                    .err()
                    .map(|e| e.to_string());
                (Some(graph), problem)
            }
            Err(error) => (
                None,
                Some(format!(
                    "Malformed rproj.toml near byte {}; repair the record, then explicitly reload",
                    error.span().map_or(0, |span| span.start)
                )),
            ),
        }
    };
    Ok(CompositionContext {
        protocol: COMPOSITION_PROTOCOL,
        name: project
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        inputs: project_composition::capture_inputs(&project)?,
        project,
        source,
        graph,
        problem,
    })
}

pub(super) fn job(directory: &Path) -> Result<()> {
    ensure!(
        directory.is_absolute() && directory.is_dir(),
        "Composition requires an existing absolute IPC directory"
    );
    let request: CompositionRequest = read_json(&directory.join("request.json"))?;
    ensure!(
        request.protocol == COMPOSITION_PROTOCOL
            && !request.id.is_empty()
            && request.id.len() <= 256
            && request.project.is_absolute(),
        "Unsupported composition request"
    );
    // Dedicated worker process: successful tool output is Activity output too,
    // rather than the terminal helper's normal quiet-success presentation.
    crate::ui::set_verbose(true);
    let path = directory.join("report.json");
    let mut report = CompositionReport::initial(&request);
    write_atomic(&path, &report)?;
    let result = perform(&request, &path, &mut report);
    report.status = Some(match &result {
        Ok(()) => CompositionStatus::Succeeded,
        Err(error) if crate::interrupt::is_cancelled(error) => CompositionStatus::Cancelled,
        Err(_) => CompositionStatus::Failed,
    });
    report.message = result
        .as_ref()
        .err()
        .map(|e| format!("{e:#}").chars().take(8192).collect());
    report.check(&request)?;
    write_atomic(&path, &report)?;
    result
}

fn perform(
    request: &CompositionRequest,
    report_path: &Path,
    report: &mut CompositionReport,
) -> Result<()> {
    crate::interrupt::check()?;
    match &request.action {
        CompositionAction::Prepare(prepare) => {
            let current = context(&request.project)?;
            ensure!(
                current == prepare.context,
                "Project source files changed since editing began; explicitly reload the draft before Review"
            );
            ensure!(
                current.problem.is_none(),
                "{}",
                current
                    .problem
                    .as_deref()
                    .unwrap_or("Unsupported composition")
            );
            ensure!(
                current.graph.as_ref() == Some(&prepare.draft.original),
                "Draft has a different original composition"
            );
            prepare.draft.validate()?;
            let plan = project_composition::prepare(
                &request.project,
                &ProjectEdit {
                    source: current.source,
                    original: prepare.draft.original.clone(),
                    updated: prepare.draft.graph.clone(),
                    reviewed: None,
                },
            )?;
            project_composition::check_prerequisites(&plan)?;
            ensure!(
                project_composition::capture_inputs(&request.project)? == plan.inputs,
                "Project files changed during preparation; explicitly reload before Review"
            );
            report.prepared = Some(plan);
        }
        CompositionAction::Apply(plan) => {
            ensure!(
                plan.project == request.project,
                "Reviewed plan belongs to another project"
            );
            project_composition::execute(plan, &mut || {
                report.writes = WriteState::MayHaveBegun;
                write_atomic(report_path, report)
            })?;
            report.writes = if plan.draft.graph == plan.draft.original {
                WriteState::NotStarted
            } else {
                WriteState::Completed
            };
        }
    }
    Ok(())
}
