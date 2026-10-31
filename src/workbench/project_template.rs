//! Machine-wide Project Template worker. The matching CLI supplies all destinations.
mod global_rojo;
use crate::{
    config::{project_template, template_checked},
    steps::rojo,
};
use anyhow::{Context, Result, ensure};
use rproj_core::project_template::{
    protocol::*,
    review::{TemplateAction, TemplateSource},
};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

pub(super) fn job(directory: &Path) -> Result<()> {
    job_at(directory, &project_template::path()?, &|| {
        global_rojo::MachineTools::system()?.resolve()
    })
}

fn job_at(
    directory: &Path,
    destination: &Path,
    resolve: &impl Fn() -> Result<GlobalRojo>,
) -> Result<()> {
    ensure!(
        directory.is_absolute() && directory.is_dir(),
        "Project Template requires an absolute IPC directory"
    );
    let request: TemplateRequest = read_json(&directory.join("request.json"))?;
    let path = directory.join("report.json");
    let mut report = TemplateReport::initial(&request);
    write_atomic(&path, &report)?;
    let result = perform(&request, destination, resolve, &path, &mut report).and_then(|()| {
        report.status = Some(TemplateStatus::Succeeded);
        report.check(&request)?;
        ensure!(
            serde_json::to_vec(&report)?.len() as u64 <= IPC_LIMIT,
            "Project Template report exceeds the 2 MiB worker limit; use the terminal editor"
        );
        Ok(())
    });
    if let Err(error) = result {
        report.loaded = None;
        report.prepared = None;
        report.saved = None;
        report.status = Some(if report.mutation == TemplateMutation::NotStarted {
            TemplateStatus::Failed
        } else {
            report.mutation = TemplateMutation::MayHaveBegun;
            TemplateStatus::Uncertain
        });
        report.message = Some(format!("{error:#}").chars().take(8192).collect());
    }
    write_atomic(&path, &report)?;
    ensure!(
        report.status == Some(TemplateStatus::Succeeded),
        "{}",
        report
            .message
            .as_deref()
            .unwrap_or("Template worker did not complete")
    );
    Ok(())
}

fn perform(
    request: &TemplateRequest,
    destination: &Path,
    resolve: &impl Fn() -> Result<GlobalRojo>,
    report_path: &Path,
    report: &mut TemplateReport,
) -> Result<()> {
    request.validate()?;
    let destination = std::path::absolute(destination)?;
    if let Some(source) = request.action.source() {
        ensure!(
            source.path == destination,
            "This review belongs to a different machine template. Reload from file."
        );
    }
    match &request.action {
        TemplateJob::Context => {
            let source = template_checked::capture(&destination, Some(IPC_LIMIT))?;
            report.path = Some(source.path.clone());
            report.loaded = Some(LoadedTemplate::new(source));
        }
        TemplateJob::Prepare { source, action } => {
            let prepared = prepare(source, action, resolve, None)?;
            let submission = TemplateRequest {
                protocol: TEMPLATE_PROTOCOL,
                id: request.id.clone(),
                action: TemplateJob::Execute(Box::new(prepared.clone())),
            };
            ensure!(
                serde_json::to_vec(&submission)?.len() as u64 <= IPC_LIMIT,
                "Template Save request exceeds the 2 MiB worker limit; use the terminal editor"
            );
            report.prepared = Some(prepared);
        }
        TemplateJob::Execute(submitted) => {
            let checked = prepare(
                &submitted.review.source,
                &submitted.review.action,
                resolve,
                Some(submitted),
            )?;
            ensure!(
                checked == **submitted,
                "Template review changed; prepare again"
            );
            let saved = template_checked::execute_with_limit(
                &checked.review,
                &mut || {
                    report.mutation = TemplateMutation::MayHaveBegun;
                    write_atomic(report_path, report)
                },
                Some(IPC_LIMIT),
            )?;
            if checked.review.changes() {
                report.mutation = TemplateMutation::Completed;
            }
            report.saved = Some(saved);
        }
    }
    Ok(())
}

fn prepare(
    source: &TemplateSource,
    action: &TemplateAction,
    resolve: &impl Fn() -> Result<GlobalRojo>,
    expected: Option<&PreparedTemplate>,
) -> Result<PreparedTemplate> {
    let mut rojo = None;
    let review = template_checked::prepare_with_limit(
        source,
        action.clone(),
        |template| {
            let machine_tool = resolve()?;
            if let Some(prior) = expected {
                ensure!(
                    prior.rojo.as_ref() == Some(&machine_tool),
                    "Global Rojo or its configuration changed; prepare a new review"
                );
            }
            rojo::validate_template_variants(template, |variant, project, scratch| {
                for (command, output) in [
                    ("sourcemap", "sourcemap.json"),
                    ("build", "validation.rbxl"),
                ] {
                    let _ = writeln!(std::io::stdout(), "Validating {variant}: Rojo {command}");
                    let mut run = Command::new(&machine_tool.executable);
                    run.arg(command)
                        .arg(project)
                        .arg("-o")
                        .arg(scratch.join(output))
                        .current_dir(scratch)
                        .stdin(Stdio::null());
                    #[cfg(windows)]
                    {
                        use std::os::windows::process::CommandExt;
                        run.creation_flags(0x08000000);
                    }
                    let status = run
                        .status()
                        .context("Could not launch global Rojo for template validation")?;
                    ensure!(
                        status.success(),
                        "Global Rojo rejected the {variant} template during {command} ({status}). See Activity output."
                    );
                }
                Ok(())
            })?;
            ensure!(
                resolve()? == machine_tool,
                "Global Rojo changed during validation; prepare again"
            );
            rojo = Some(machine_tool);
            Ok(())
        },
        Some(IPC_LIMIT),
    )?;
    Ok(PreparedTemplate { review, rojo })
}

#[cfg(all(test, windows))]
mod tests;
