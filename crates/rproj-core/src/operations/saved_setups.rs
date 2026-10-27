use super::*;
use crate::saved_setups::{protocol::*, *};

impl WorkbenchClient {
    pub fn saved_setups_support(&self) -> Result<()> {
        let text = query(&self.worker, &[WORKER_FLAG, "saved-setups-support"], None)
            .context("Saved Setups is unavailable; use a matching desktop/CLI distribution")?;
        ensure!(
            serde_json::from_str::<u32>(&text)? == SAVED_SETUPS_PROTOCOL,
            "Unsupported Saved Setups worker; use a matching distribution"
        );
        Ok(())
    }
    pub fn saved_setups(
        &self,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<SetupListing> {
        successful(self.saved_operation(SavedAction::Browse, cancellation, sink)?)?
            .listing
            .context("Missing setup listing")
    }
    pub fn load_saved_setup(
        &self,
        name: String,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<LoadedSetup> {
        successful(self.saved_operation(SavedAction::Load(name), cancellation, sink)?)?
            .loaded
            .context("Missing saved document")
    }
    pub fn prepare_saved_setup(
        &self,
        source: CapturedSetup,
        action: SetupAction,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<PreparedSetup> {
        successful(self.saved_operation(
            SavedAction::Prepare { source, action },
            cancellation,
            sink,
        )?)?
        .prepared
        .context("Missing saved setup review")
    }
    pub fn reuse_saved_setup(
        &self,
        source: CapturedSetup,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<ReusedSetup> {
        successful(self.saved_operation(SavedAction::Reuse(source), cancellation, sink)?)?
            .reused
            .context("Missing reusable composition")
    }
    /// Mutations cannot be cancelled. Closing waits for owned completion.
    pub fn manage_saved_setup(
        &self,
        review: ManagementReview,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<ManagementOutcome> {
        let source = review
            .source
            .root
            .join(format!("{}.toml", review.source.name));
        let destination = review.destination.clone();
        let report = match self.saved_operation(
            SavedAction::Execute(Box::new(review)),
            &Cancellation::default(),
            sink,
        ) {
            Ok(report) => report,
            Err(e) if e.is::<OwnedTeardownUnconfirmed>() => return Err(e),
            Err(e) => {
                return Ok(ManagementOutcome {
                    status: ManagementStatus::Failed,
                    mutation: MutationState::NotStarted,
                    source,
                    destination,
                    saved: None,
                    message: Some(format!(
                        "Saved setup operation could not start; no writes were made: {e:#}"
                    )),
                });
            }
        };
        Ok(report.outcome.unwrap_or(ManagementOutcome {
            status: report.status.unwrap_or(ManagementStatus::Uncertain),
            mutation: report.mutation,
            source,
            destination,
            saved: None,
            message: report.message,
        }))
    }
    fn saved_operation(
        &self,
        action: SavedAction,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<SavedReport> {
        let _guard = self
            .busy
            .try_lock()
            .map_err(|_| anyhow::anyhow!("Another operation is running"))?;
        let directory = tempfile::Builder::new()
            .prefix("rproj-saved-setups-")
            .tempdir()?;
        let request = SavedRequest {
            protocol: SAVED_SETUPS_PROTOCOL,
            id: directory
                .path()
                .file_name()
                .context("Missing operation identity")?
                .to_string_lossy()
                .into_owned(),
            action,
        };
        ensure!(
            serde_json::to_vec(&request)?.len() as u64 <= IPC_LIMIT,
            "Saved Setups request exceeds the 2 MiB worker limit; use terminal management. Nothing written."
        );
        request.validate()?;
        write_atomic(&directory.path().join("request.json"), &request)?;
        if cancellation.is_cancelled() {
            bail!("Saved setup preparation cancelled; nothing written");
        }
        sink(OperationEvent::Started);
        let sink: Arc<dyn Fn(OperationEvent) + Send + Sync> = Arc::new(sink);
        let outcome = run_owned_saved(&self.worker, directory.path(), cancellation, sink.clone())?;
        let checked =
            read_json::<SavedReport>(&directory.path().join("report.json")).and_then(|report| {
                report.check(&request)?;
                ensure!(
                    report.status.is_some(),
                    "Saved Setups exited without a terminal report"
                );
                ensure!(
                    (outcome == OperationOutcome::Succeeded)
                        == (report.status == Some(ManagementStatus::Succeeded)),
                    "Saved Setups exit and report disagree"
                );
                Ok(report)
            });
        // Once spawned, an absent/invalid report never means unchanged documents.
        let report = match checked {
            Ok(report) => report,
            Err(e) => {
                let mut report = SavedReport::initial(&request);
                let mutation = matches!(request.action, SavedAction::Execute(_));
                report.status = Some(ManagementStatus::Uncertain);
                report.mutation = if mutation {
                    MutationState::Unknown
                } else {
                    MutationState::NotStarted
                };
                report.message = Some(format!(
                    "No trustworthy Saved Setups completion report. {}: {e:#}",
                    if mutation {
                        "Refresh storage and Reload from file before retrying; files may have changed"
                    } else {
                        "No document writes were requested"
                    }
                ));
                report
            }
        };
        sink(OperationEvent::Finished { outcome });
        Ok(report)
    }
}
fn successful(report: SavedReport) -> Result<SavedReport> {
    ensure!(
        report.status == Some(ManagementStatus::Succeeded),
        "{}",
        report
            .message
            .as_deref()
            .unwrap_or("Saved Setups did not complete; retry explicitly")
    );
    Ok(report)
}
#[cfg(windows)]
fn run_owned_saved(
    worker: &Path,
    directory: &Path,
    cancellation: &Cancellation,
    sink: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    execute_owned_os(
        worker,
        &[
            WORKER_FLAG.into(),
            "saved-setups-job".into(),
            directory.as_os_str().to_owned(),
        ],
        directory,
        cancellation,
        sink,
    )
}
#[cfg(not(windows))]
fn run_owned_saved(
    _: &Path,
    _: &Path,
    _: &Cancellation,
    _: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    bail!(
        "Desktop Saved Setups workers are currently supported on Windows; terminal management remains available"
    )
}
