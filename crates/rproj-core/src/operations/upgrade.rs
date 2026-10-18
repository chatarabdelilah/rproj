use super::*;
use crate::upgrade::{UpgradeReview, protocol::*};

impl WorkbenchClient {
    pub fn upgrade_support(&self) -> Result<()> {
        let text = query(&self.worker, &[WORKER_FLAG, "upgrade-support"], None)
            .context("Upgrade review is unavailable; use a matching desktop/CLI distribution")?;
        ensure!(
            serde_json::from_str::<u32>(&text)? == UPGRADE_PROTOCOL,
            "Unsupported Upgrade worker; use a matching distribution"
        );
        Ok(())
    }
    pub fn upgrade_context(&self, project: &Path) -> Result<UpgradeContext> {
        let project = project
            .canonicalize()
            .context("Cannot open the captured project")?;
        let text = query(&self.worker, &[WORKER_FLAG, "upgrade-context"], Some(&project)).map_err(|error| {
            let reason = format!("{error:#}");
            if reason.contains("unknown workbench operation") {
                anyhow::anyhow!("Upgrade editing is unavailable; use the matching desktop and CLI from one portable distribution: {reason}")
            } else {
                anyhow::anyhow!("Cannot read project upgrade: {reason}")
            }
        })?;
        let context: UpgradeContext =
            serde_json::from_str(&text).context("Invalid upgrade context")?;
        ensure!(
            context.protocol == UPGRADE_PROTOCOL && context.project == project,
            "Unsupported upgrade worker or mismatched project; use a matching portable distribution"
        );
        Ok(context)
    }

    pub fn prepare_upgrade(
        &self,
        prepare: UpgradeContext,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<UpgradePreparation> {
        let project = prepare.project.clone();
        let report = match self.upgrade_operation(
            project,
            UpgradeAction::Prepare(prepare),
            cancellation,
            sink,
        ) {
            Ok(report) => report,
            Err(error) if error.is::<OwnedTeardownUnconfirmed>() => return Err(error),
            Err(error) => {
                return Ok(UpgradePreparation::Failed(format!(
                    "Preparation could not start; no project changes were made: {error:#}"
                )));
            }
        };
        if report.status == Some(UpgradeStatus::Cancelled) {
            return Ok(UpgradePreparation::Cancelled);
        }
        if report.status != Some(UpgradeStatus::Succeeded) {
            return Ok(UpgradePreparation::Failed(report.message.unwrap_or_else(
                || "Upgrade preparation failed; preparation made no project changes".into(),
            )));
        }
        Ok(UpgradePreparation::Prepared(Box::new(
            report
                .prepared
                .context("Upgrade preparation returned no plan")?,
        )))
    }

    pub fn apply_upgrade(
        &self,
        plan: UpgradeReview,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<UpgradeOutcome> {
        let project = plan.project.clone();
        let report = match self.upgrade_operation(
            project.clone(),
            UpgradeAction::Apply(plan),
            cancellation,
            sink,
        ) {
            Ok(report) => report,
            Err(error) if error.is::<OwnedTeardownUnconfirmed>() => return Err(error),
            Err(error) => {
                return Ok(UpgradeOutcome {
                    project,
                    status: UpgradeStatus::Failed,
                    writes: WriteState::NotStarted,
                    message: Some(format!(
                        "Apply could not start; no upgrade writes were made: {error:#}"
                    )),
                });
            }
        };
        Ok(UpgradeOutcome {
            project,
            status: report.status.unwrap_or(UpgradeStatus::Uncertain),
            writes: report.writes,
            message: report.message,
        })
    }

    fn upgrade_operation(
        &self,
        project: PathBuf,
        action: UpgradeAction,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<UpgradeReport> {
        let _guard = self
            .busy
            .try_lock()
            .map_err(|_| anyhow::anyhow!("Another operation is running"))?;
        ensure!(
            project.is_absolute(),
            "Upgrade requires an absolute captured project"
        );
        let directory = tempfile::Builder::new()
            .prefix("rproj-upgrade-")
            .tempdir()?;
        let request = UpgradeRequest {
            protocol: UPGRADE_PROTOCOL,
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
            let mut report = UpgradeReport::initial(&request);
            report.status = Some(UpgradeStatus::Cancelled);
            return Ok(report);
        }
        sink(OperationEvent::Started);
        let sink: Arc<dyn Fn(OperationEvent) + Send + Sync> = Arc::new(sink);
        // execute_owned_os marks every error after spawn with
        // OwnedTeardownUnconfirmed; only pre-spawn errors reach the recoverable
        // NotStarted branch. Keep that distinction when changing the runner.
        let outcome =
            run_owned_upgrade(&self.worker, directory.path(), cancellation, sink.clone())?;
        // Owned process teardown and both readers have completed. A terminal
        // report is still necessary to establish what happened to project files.
        let checked =
            read_json::<UpgradeReport>(&directory.path().join("report.json")).and_then(|report| {
                report.check(&request)?;
                ensure!(
                    outcome == OperationOutcome::Cancelled || report.status.is_some(),
                    "Upgrade worker exited without a terminal report"
                );
                ensure!(
                    outcome != OperationOutcome::Succeeded
                        || report.status == Some(UpgradeStatus::Succeeded),
                    "Upgrade worker exit and report disagree"
                );
                ensure!(
                    !matches!(outcome, OperationOutcome::Failed { .. })
                        || report.status != Some(UpgradeStatus::Succeeded),
                    "Upgrade reported success after an unsuccessful exit"
                );
                Ok(report)
            });
        let mut report = match checked {
            Ok(report) => report,
            Err(error) => {
                let mut report = UpgradeReport::initial(&request);
                let preparing = matches!(&request.action, UpgradeAction::Prepare(_));
                report.writes = if preparing {
                    WriteState::NotStarted
                } else {
                    WriteState::Unknown
                };
                report.status = Some(if outcome == OperationOutcome::Cancelled {
                    UpgradeStatus::Cancelled
                } else {
                    UpgradeStatus::Uncertain
                });
                report.message = Some(if preparing {
                    format!(
                        "Preparation returned no trustworthy report; no upgrade writes were requested: {error:#}"
                    )
                } else {
                    format!(
                        "No trustworthy upgrade report. Inspect {} for possible partial changes before retrying: {error:#}",
                        request.project.display()
                    )
                });
                report
            }
        };
        if report.status.is_none() {
            // Cancellation can interrupt the report write. Preserve its known
            // mutation boundary, but never turn an incomplete report into success.
            report.status = Some(UpgradeStatus::Cancelled);
            report.message = Some("Cancellation teardown completed. Inspect the captured project for possible partial changes before retrying.".into());
        }
        sink(OperationEvent::Finished { outcome });
        Ok(report)
    }
}

#[cfg(windows)]
fn run_owned_upgrade(
    worker: &Path,
    directory: &Path,
    cancellation: &Cancellation,
    sink: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    let args = [
        std::ffi::OsString::from(WORKER_FLAG),
        "upgrade-job".into(),
        directory.as_os_str().to_owned(),
    ];
    execute_owned_os(worker, &args, directory, cancellation, sink)
}

#[cfg(not(windows))]
fn run_owned_upgrade(
    _: &Path,
    _: &Path,
    _: &Cancellation,
    _: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    bail!("Desktop upgrade is currently supported on Windows; terminal editing remains available")
}
