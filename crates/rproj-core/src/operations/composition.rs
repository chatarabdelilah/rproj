use super::*;
use crate::creation::{CompositionPlan, composition_protocol::*};

impl WorkbenchClient {
    pub fn composition_context(&self, project: &Path) -> Result<CompositionContext> {
        let project = project
            .canonicalize()
            .context("Cannot open the captured project")?;
        let text = query(&self.worker, &[WORKER_FLAG, "composition-context"], Some(&project)).map_err(|error| {
            let reason = format!("{error:#}");
            if reason.contains("unknown workbench operation") {
                anyhow::anyhow!("Composition editing is unavailable; use the matching desktop and CLI from one portable distribution: {reason}")
            } else {
                anyhow::anyhow!("Cannot read project composition: {reason}")
            }
        })?;
        let context: CompositionContext =
            serde_json::from_str(&text).context("Invalid composition context")?;
        ensure!(
            context.protocol == COMPOSITION_PROTOCOL && context.project == project,
            "Unsupported composition worker or mismatched project; use a matching portable distribution"
        );
        Ok(context)
    }

    pub fn prepare_composition(
        &self,
        prepare: PrepareComposition,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<CompositionPreparation> {
        let project = prepare.context.project.clone();
        let report = match self.composition_operation(
            project,
            CompositionAction::Prepare(prepare),
            cancellation,
            sink,
        ) {
            Ok(report) => report,
            Err(error) if error.is::<OwnedTeardownUnconfirmed>() => return Err(error),
            Err(error) => {
                return Ok(CompositionPreparation::Failed(format!(
                    "Preparation could not start; no project changes were made: {error:#}"
                )));
            }
        };
        if report.status == Some(CompositionStatus::Cancelled) {
            return Ok(CompositionPreparation::Cancelled);
        }
        if report.status != Some(CompositionStatus::Succeeded) {
            return Ok(CompositionPreparation::Failed(
                report.message.unwrap_or_else(|| {
                    "Composition preparation failed; preparation made no project changes".into()
                }),
            ));
        }
        Ok(CompositionPreparation::Prepared(Box::new(
            report
                .prepared
                .context("Composition preparation returned no plan")?,
        )))
    }

    pub fn apply_composition(
        &self,
        plan: CompositionPlan,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<CompositionOutcome> {
        let project = plan.project.clone();
        let report = match self.composition_operation(
            project.clone(),
            CompositionAction::Apply(plan),
            cancellation,
            sink,
        ) {
            Ok(report) => report,
            Err(error) if error.is::<OwnedTeardownUnconfirmed>() => return Err(error),
            Err(error) => {
                return Ok(CompositionOutcome {
                    project,
                    status: CompositionStatus::Failed,
                    writes: WriteState::NotStarted,
                    message: Some(format!(
                        "Apply could not start; no composition writes were made: {error:#}"
                    )),
                });
            }
        };
        Ok(CompositionOutcome {
            project,
            status: report.status.unwrap_or(CompositionStatus::Uncertain),
            writes: report.writes,
            message: report.message,
        })
    }

    fn composition_operation(
        &self,
        project: PathBuf,
        action: CompositionAction,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<CompositionReport> {
        let _guard = self
            .busy
            .try_lock()
            .map_err(|_| anyhow::anyhow!("Another operation is running"))?;
        ensure!(
            project.is_absolute(),
            "Composition requires an absolute captured project"
        );
        let directory = tempfile::Builder::new()
            .prefix("rproj-composition-")
            .tempdir()?;
        let request = CompositionRequest {
            protocol: COMPOSITION_PROTOCOL,
            id: directory
                .path()
                .file_name()
                .context("Missing operation identity")?
                .to_string_lossy()
                .into_owned(),
            project,
            action,
        };
        write_atomic(&directory.path().join("request.json"), &request)?;
        if cancellation.is_cancelled() {
            let mut report = CompositionReport::initial(&request);
            report.status = Some(CompositionStatus::Cancelled);
            return Ok(report);
        }
        sink(OperationEvent::Started);
        let sink: Arc<dyn Fn(OperationEvent) + Send + Sync> = Arc::new(sink);
        let outcome =
            run_owned_composition(&self.worker, directory.path(), cancellation, sink.clone())?;
        // Owned process teardown and both readers have completed. A terminal
        // report is still necessary to establish what happened to project files.
        let checked = read_json::<CompositionReport>(&directory.path().join("report.json"))
            .and_then(|report| {
                report.check(&request)?;
                ensure!(
                    outcome == OperationOutcome::Cancelled || report.status.is_some(),
                    "Composition worker exited without a terminal report"
                );
                ensure!(
                    outcome != OperationOutcome::Succeeded
                        || report.status == Some(CompositionStatus::Succeeded),
                    "Composition worker exit and report disagree"
                );
                ensure!(
                    !matches!(outcome, OperationOutcome::Failed { .. })
                        || report.status != Some(CompositionStatus::Succeeded),
                    "Composition reported success after an unsuccessful exit"
                );
                Ok(report)
            });
        let mut report = match checked {
            Ok(report) => report,
            Err(error) => {
                let mut report = CompositionReport::initial(&request);
                let preparing = matches!(&request.action, CompositionAction::Prepare(_));
                report.writes = if preparing {
                    WriteState::NotStarted
                } else {
                    WriteState::Unknown
                };
                report.status = Some(if outcome == OperationOutcome::Cancelled {
                    CompositionStatus::Cancelled
                } else {
                    CompositionStatus::Uncertain
                });
                report.message = Some(if preparing {
                    format!(
                        "Preparation returned no trustworthy report; no composition writes were requested: {error:#}"
                    )
                } else {
                    format!(
                        "No trustworthy composition report. Inspect {} for possible partial changes before retrying: {error:#}",
                        request.project.display()
                    )
                });
                report
            }
        };
        if report.status.is_none() {
            // Cancellation can interrupt the report write. Preserve its known
            // mutation boundary, but never turn an incomplete report into success.
            report.status = Some(CompositionStatus::Cancelled);
            report.message = Some("Cancellation teardown completed. Inspect the captured project for possible partial changes before retrying.".into());
        }
        sink(OperationEvent::Finished { outcome });
        Ok(report)
    }
}

#[cfg(windows)]
fn run_owned_composition(
    worker: &Path,
    directory: &Path,
    cancellation: &Cancellation,
    sink: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    let args = [
        std::ffi::OsString::from(WORKER_FLAG),
        "composition-job".into(),
        directory.as_os_str().to_owned(),
    ];
    execute_owned_os(worker, &args, directory, cancellation, sink)
}

#[cfg(not(windows))]
fn run_owned_composition(
    _: &Path,
    _: &Path,
    _: &Cancellation,
    _: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    bail!(
        "Desktop composition is currently supported on Windows; terminal editing remains available"
    )
}
