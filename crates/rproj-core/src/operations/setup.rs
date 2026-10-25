use super::*;
use crate::machine_setup::{Selection, SetupContext, SetupReview, protocol::*};

/// Cooperative request only; it never terminates an installer process.
#[derive(Clone, Default)]
pub struct SetupStop(Arc<AtomicBool>);
impl SetupStop {
    pub fn request(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn requested(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Debug)]
pub enum SetupEvent {
    Output(OperationEvent),
    Progress(Box<SetupReport>),
}

impl WorkbenchClient {
    pub fn setup_support(&self) -> Result<()> {
        let response = query(&self.worker, &[WORKER_FLAG, "setup-support"], None)
            .context("Machine Setup is unavailable; use a matching desktop/CLI distribution")?;
        ensure!(
            serde_json::from_str::<u32>(&response)? == SETUP_PROTOCOL,
            "Unsupported Machine Setup worker; use a matching distribution"
        );
        Ok(())
    }
    pub fn setup_context(
        &self,
        sink: impl Fn(SetupEvent) + Send + Sync + 'static,
    ) -> Result<SetupContext> {
        let report = self.setup_operation(SetupAction::Context, &SetupStop::default(), sink)?;
        report.loaded.context(
            report
                .message
                .unwrap_or_else(|| "Could not load Machine Setup; reload explicitly".into()),
        )
    }
    pub fn prepare_setup(
        &self,
        context: SetupContext,
        selection: Selection,
        sink: impl Fn(SetupEvent) + Send + Sync + 'static,
    ) -> Result<SetupReview> {
        let report = self.setup_operation(
            SetupAction::Prepare { context, selection },
            &SetupStop::default(),
            sink,
        )?;
        report.prepared.context(
            report
                .message
                .unwrap_or_else(|| "Could not prepare Machine Setup; review again".into()),
        )
    }
    pub fn apply_setup(
        &self,
        review: SetupReview,
        stop: &SetupStop,
        sink: impl Fn(SetupEvent) + Send + Sync + 'static,
    ) -> Result<SetupReport> {
        self.setup_operation(SetupAction::Apply(review), stop, sink)
    }
    fn setup_operation(
        &self,
        action: SetupAction,
        stop: &SetupStop,
        sink: impl Fn(SetupEvent) + Send + Sync + 'static,
    ) -> Result<SetupReport> {
        let _guard = self
            .busy
            .try_lock()
            .map_err(|_| anyhow::anyhow!("Another operation is running"))?;
        let directory = tempfile::Builder::new()
            .prefix("rproj-machine-setup-")
            .tempdir()?;
        let request = SetupRequest {
            protocol: SETUP_PROTOCOL,
            id: directory
                .path()
                .file_name()
                .context("Missing operation identity")?
                .to_string_lossy()
                .into_owned(),
            action,
        };
        request.validate()?;
        ensure!(
            serde_json::to_vec(&request)?.len() as u64 <= IPC_LIMIT,
            "Setup request exceeds the 2 MiB limit; nothing installed"
        );
        write_atomic(&directory.path().join("request.json"), &request)?;
        let sink: Arc<dyn Fn(SetupEvent) + Send + Sync> = Arc::new(sink);
        sink(SetupEvent::Output(OperationEvent::Started));
        let outcome = match run_setup(&self.worker, directory.path(), &request, stop, sink.clone())
        {
            Ok(outcome) => outcome,
            Err(error) if error.is::<OwnedTeardownUnconfirmed>() => {
                // An active worker may still need its control/report directory.
                let _ = directory.keep();
                return Err(error);
            }
            Err(error) => return Err(error),
        };
        let checked =
            read_json::<SetupReport>(&directory.path().join("report.json")).and_then(|report| {
                report.check(&request)?;
                ensure!(
                    report.status.is_some(),
                    "Worker exited without a terminal Machine Setup report"
                );
                ensure!(
                    (outcome == OperationOutcome::Succeeded)
                        == (report.status == Some(SetupStatus::Succeeded)),
                    "Setup exit and completion report disagree"
                );
                Ok(report)
            });
        let report = checked.unwrap_or_else(|error| {
            let mut report = SetupReport::initial(&request);
            report.status = Some(SetupStatus::Uncertain);
            report.effects = if matches!(request.action, SetupAction::Apply(_)) { WriteState::Unknown } else { WriteState::NotStarted };
            report.message = Some(format!("No trustworthy Machine Setup completion report: {error:#}. Completed installations may remain; reload before retrying."));
            report
        });
        sink(SetupEvent::Progress(Box::new(report.clone())));
        let presented = match report.status {
            Some(SetupStatus::Succeeded) => OperationOutcome::Succeeded,
            Some(SetupStatus::Stopped) => OperationOutcome::Cancelled,
            _ => OperationOutcome::Failed {
                code: match outcome {
                    OperationOutcome::Failed { code } => code,
                    _ => None,
                },
            },
        };
        sink(SetupEvent::Output(OperationEvent::Finished {
            outcome: presented,
        }));
        Ok(report)
    }
}

#[cfg(windows)]
fn run_setup(
    worker: &Path,
    directory: &Path,
    request: &SetupRequest,
    stop: &SetupStop,
    sink: Arc<dyn Fn(SetupEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    use process_wrap::tokio::{CommandWrap, CreationFlags, JobObject};
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let outcome = runtime.block_on(async {
        let mut command = CommandWrap::with_new(worker, |command| {
            command
                .args([
                    std::ffi::OsStr::new(WORKER_FLAG),
                    std::ffi::OsStr::new("setup-job"),
                    directory.as_os_str(),
                ])
                .current_dir(directory)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        });
        // Never opt this installer job into killing on Drop or normal Stop.
        command.wrap(JobObject).wrap(CreationFlags(
            windows::Win32::System::Threading::CREATE_NO_WINDOW,
        ));
        let mut child = command
            .spawn()
            .context("Could not start owned Machine Setup worker")?;
        async {
            let forward = sink.clone();
            let output: Arc<dyn Fn(OperationEvent) + Send + Sync> =
                Arc::new(move |event| forward(SetupEvent::Output(event)));
            let stdout = tokio::spawn(drain(
                child.stdout().take().context("Missing setup stdout")?,
                false,
                output.clone(),
            ));
            let stderr = tokio::spawn(drain(
                child.stderr().take().context("Missing setup stderr")?,
                true,
                output,
            ));
            let mut sent_stop = false;
            let mut sequence = None;
            // Preserve one future: recreating wait after root exit could skip the
            // job's remaining descendants. try_wait proves only root exit.
            let mut wait = child.wait();
            let status = loop {
                if stop.requested() && !sent_stop && matches!(request.action, SetupAction::Apply(_))
                {
                    write_atomic(
                        &directory.join("stop.json"),
                        &StopAfterItem {
                            protocol: SETUP_PROTOCOL,
                            id: request.id.clone(),
                        },
                    )?;
                    sent_stop = true;
                }
                if let Ok(report) = read_json::<SetupReport>(&directory.join("report.json"))
                    && report.check(request).is_ok()
                    && sequence.is_none_or(|last| report.sequence > last)
                {
                    sequence = Some(report.sequence);
                    // A terminal file does not prove descendant/stream teardown.
                    if report.status.is_none() {
                        sink(SetupEvent::Progress(Box::new(report)));
                    }
                }
                if let Ok(status) =
                    tokio::time::timeout(Duration::from_millis(50), wait.as_mut()).await
                {
                    break status?;
                }
            };
            drop(wait);
            tokio::time::timeout(Duration::from_secs(5), async {
                stdout.await??;
                stderr.await??;
                Ok::<_, anyhow::Error>(())
            })
            .await
            .context("Setup output did not finish draining")??;
            Ok::<_, anyhow::Error>(if status.success() {
                OperationOutcome::Succeeded
            } else {
                OperationOutcome::Failed {
                    code: status.code(),
                }
            })
        }
        .await
        .context(OwnedTeardownUnconfirmed)
    });
    if outcome.is_err() {
        // Windows pipe reads may occupy Tokio blocking threads. On unconfirmed
        // teardown, dropping the runtime would wait indefinitely for those reads.
        // Do not kill the installer or pretend the tree/streams have completed.
        runtime.shutdown_background();
    }
    outcome
}

#[cfg(not(windows))]
fn run_setup(
    _: &Path,
    _: &Path,
    _: &SetupRequest,
    _: &SetupStop,
    _: Arc<dyn Fn(SetupEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    bail!(
        "Desktop Machine Setup execution currently requires Windows; terminal commands remain available"
    )
}
