use super::*;
use crate::test_execution::{TestContext, protocol::*};

impl WorkbenchClient {
    pub fn test_arguments_support(&self) -> Result<()> {
        probe_support(&self.worker, &Cancellation::default())
    }

    pub fn test_context(
        &self,
        project: &Path,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<TestContext> {
        let metadata = std::fs::symlink_metadata(project)?;
        ensure!(
            metadata.is_dir() && !metadata.file_type().is_symlink(),
            "Choose a regular project directory"
        );
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            ensure!(
                metadata.file_attributes() & 0x400 == 0,
                "Project is a reparse point; choose a regular directory"
            );
        }
        let report =
            self.test_operation(project.canonicalize()?, Action::Context, cancellation, sink)?;
        ensure!(
            report.status == Some(Status::Succeeded),
            "{}",
            report
                .message
                .unwrap_or_else(|| "Could not read test context; Refresh explicitly".into())
        );
        report.context.context("Worker returned no test context")
    }

    /// The returned completion/error ends the request. Events are progress only;
    /// a pre-spawn refusal can return a completion without a Finished event.
    pub fn run_custom_tests(
        &self,
        context: TestContext,
        arguments: Vec<String>,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<Completion> {
        let project = context.project.clone();
        match self.test_operation(
            project.clone(),
            Action::Run { context, arguments },
            cancellation,
            sink,
        ) {
            Ok(report) => Ok(Completion {
                project,
                accepted: (report.effects != WriteState::Unknown).then_some(report.accepted),
                status: report.status.unwrap_or(Status::Uncertain),
                effects: report.effects,
                runner_exit: report.runner_exit,
                message: report.message,
            }),
            Err(error) if error.is::<OwnedTeardownUnconfirmed>() => Err(error),
            Err(error) => Ok(Completion {
                project,
                accepted: Some(false),
                status: Status::Failed,
                effects: WriteState::NotStarted,
                runner_exit: None,
                message: Some(format!("Custom tests could not start: {error:#}")),
            }),
        }
    }

    fn test_operation(
        &self,
        project: PathBuf,
        action: Action,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<Report> {
        let _guard = self
            .busy
            .try_lock()
            .map_err(|_| anyhow::anyhow!("Another operation is running"))?;
        let directory = tempfile::Builder::new().prefix("rproj-test-").tempdir()?;
        let request = Request {
            protocol: TEST_PROTOCOL,
            id: directory
                .path()
                .file_name()
                .context("Missing test operation identity")?
                .to_string_lossy()
                .into_owned(),
            project,
            action,
        };
        request.validate()?;
        write_atomic(&directory.path().join("request.json"), &request)?;
        if cancellation.is_cancelled() {
            let mut report = Report::initial(&request)?;
            report.status = Some(Status::Cancelled);
            return Ok(report);
        }
        let support = match probe_support(&self.worker, cancellation) {
            Err(error) if error.is::<OwnedTeardownUnconfirmed>() => return Err(error),
            other => other,
        };
        if cancellation.is_cancelled() {
            let mut report = Report::initial(&request)?;
            report.status = Some(Status::Cancelled);
            return Ok(report);
        }
        support?;
        sink(OperationEvent::Started);
        let sink: Arc<dyn Fn(OperationEvent) + Send + Sync> = Arc::new(sink);
        let exit = run_tests(&self.worker, directory.path(), cancellation, sink.clone())?;
        let report = resolve_report(
            &request,
            &exit,
            read_json(&directory.path().join("report.json")),
        )?;
        let outcome = match report.status {
            Some(Status::Succeeded) => OperationOutcome::Succeeded,
            Some(Status::Cancelled) => OperationOutcome::Cancelled,
            _ => OperationOutcome::Failed {
                code: report.runner_exit,
            },
        };
        sink(OperationEvent::Finished { outcome });
        Ok(report)
    }
}

fn resolve_report(
    request: &Request,
    exit: &OperationOutcome,
    result: Result<Report>,
) -> Result<Report> {
    let checked = result.and_then(|mut report| {
        report.check(request)?;
        // A terminal report wins a late cancellation race after owned teardown.
        if exit == &OperationOutcome::Cancelled && report.status.is_none() {
            report.status = Some(Status::Cancelled);
            report.message = Some(
                "Tests cancelled after owned teardown. Preparation changes may remain.".into(),
            );
        }
        ensure!(
            report.status.is_some(),
            "Worker exited without a terminal test report"
        );
        if exit != &OperationOutcome::Cancelled {
            ensure!(
                (exit == &OperationOutcome::Succeeded)
                    == (report.status == Some(Status::Succeeded)),
                "Test report and worker exit disagree"
            );
        }
        report.check(request)?;
        Ok(report)
    });
    match checked {
        Ok(report) => Ok(report),
        Err(error) => {
            let mut report = Report::initial(request)?;
            report.status = Some(Status::Uncertain);
            // This synthesized report is returned only locally. Unknown effects also
            // indicate that an acceptance acknowledgement cannot be trusted.
            if matches!(request.action, Action::Run { .. }) {
                report.effects = WriteState::Unknown;
            }
            report.message = Some(match request.action {
                Action::Context => format!(
                    "Could not confirm the read-only test-context refresh. Refresh explicitly: {error:#}"
                ),
                Action::Run { .. } => format!(
                    "No trustworthy test completion report; partial preparation changes may remain. Refresh context before retrying: {error:#}"
                ),
            });
            Ok(report)
        }
    }
}

#[cfg(windows)]
fn probe_support(worker: &Path, cancellation: &Cancellation) -> Result<()> {
    let output = Arc::new(std::sync::Mutex::new(String::new()));
    let captured = output.clone();
    let bounded = Cancellation::default();
    let stop = bounded.clone();
    let overflow = bounded.clone();
    let directory = worker.parent().context("Worker has no parent")?;
    let (finished, wait) = std::sync::mpsc::channel();
    let result = std::thread::scope(|scope| {
        scope.spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(8);
            loop {
                if cancellation.is_cancelled() || Instant::now() >= deadline {
                    stop.cancel();
                    break;
                }
                if wait.recv_timeout(Duration::from_millis(25)).is_ok() {
                    break;
                }
            }
        });
        let result = execute_owned_os(
            worker,
            &[WORKER_FLAG.into(), "test-support".into()],
            directory,
            &bounded,
            Arc::new(move |event| {
                if let OperationEvent::Output {
                    stderr: false,
                    text,
                } = event
                {
                    let mut output = captured.lock().unwrap();
                    if output.len().saturating_add(text.len()) <= IPC_LIMIT as usize {
                        output.push_str(&text);
                    } else {
                        overflow.cancel();
                    }
                }
            }),
        );
        let _ = finished.send(());
        result
    })?;
    ensure!(
        result == OperationOutcome::Succeeded && !bounded.is_cancelled(),
        "Custom-test support probe failed, timed out or was cancelled; use a matching desktop/CLI distribution"
    );
    ensure!(
        serde_json::from_str::<u32>(&output.lock().unwrap())? == TEST_PROTOCOL,
        "Unsupported custom-test worker; use a matching distribution"
    );
    Ok(())
}

#[cfg(not(windows))]
fn probe_support(worker: &Path, _: &Cancellation) -> Result<()> {
    let text = query(worker, &[WORKER_FLAG, "test-support"], None)?;
    ensure!(
        serde_json::from_str::<u32>(&text)? == TEST_PROTOCOL,
        "Unsupported custom-test worker; use a matching distribution"
    );
    Ok(())
}

#[cfg(windows)]
fn run_tests(
    worker: &Path,
    directory: &Path,
    cancellation: &Cancellation,
    sink: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    execute_owned_os(
        worker,
        &[
            WORKER_FLAG.into(),
            "test-job".into(),
            directory.as_os_str().to_owned(),
        ],
        directory,
        cancellation,
        sink,
    )
}
#[cfg(not(windows))]
fn run_tests(
    _: &Path,
    _: &Path,
    _: &Cancellation,
    _: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    bail!(
        "Custom test workers are currently supported on Windows; direct CLI tests remain available"
    )
}

#[cfg(test)]
mod tests;
