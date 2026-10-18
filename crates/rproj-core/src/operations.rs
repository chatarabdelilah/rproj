//! Blocking, UI-independent control of an explicitly located, matching CLI worker.
//! Call these services off the UI thread; output is progress, never a state protocol.

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub const WORKER_FLAG: &str = "--rproj-workbench";
const QUERY_LIMIT: usize = 2 * 1024 * 1024 + 4096;

// Only failures after a successful spawn can leave owned teardown unconfirmed.
// Keep startup errors distinct so frontends can recover without a close lock.
#[derive(Debug)]
struct OwnedTeardownUnconfirmed;
impl std::fmt::Display for OwnedTeardownUnconfirmed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Owned worker teardown was not confirmed")
    }
}
impl std::error::Error for OwnedTeardownUnconfirmed {}

#[path = "operations/composition.rs"]
mod composition;
#[path = "operations/creation.rs"]
mod creation;
#[path = "operations/upgrade.rs"]
mod upgrade;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum WatchState {
    Stopped,
    Preparing,
    Watching,
    Stopping,
    Failed,
    Unresponsive,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchStatus {
    pub supported: bool,
    pub project: Option<PathBuf>,
    pub state: WatchState,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    StartWatch,
    StopWatch,
    SwitchWatch,
    Check,
    Test,
}

impl Operation {
    pub fn cancellable(self) -> bool {
        matches!(self, Self::Check | Self::Test)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationOutcome {
    Succeeded,
    Failed { code: Option<i32> },
    Cancelled,
}

#[derive(Clone, Debug)]
pub enum OperationEvent {
    Started,
    Output { stderr: bool, text: String },
    Finished { outcome: OperationOutcome },
}

#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone)]
pub struct WorkbenchClient {
    worker: PathBuf,
    busy: Arc<Mutex<()>>,
}

impl WorkbenchClient {
    pub fn new(worker: PathBuf) -> Result<Self> {
        let worker = worker
            .canonicalize()
            .context("Could not locate the bundled rproj CLI")?;
        let version = query(&worker, &["--version"], None)?;
        ensure!(
            version.trim() == concat!("rproj ", env!("CARGO_PKG_VERSION")),
            "Desktop and CLI versions differ; install both from the same portable archive"
        );
        Ok(Self {
            worker,
            busy: Arc::new(Mutex::new(())),
        })
    }

    pub fn watch_status(&self) -> Result<WatchStatus> {
        let output = query(&self.worker, &[WORKER_FLAG, "status"], None)?;
        serde_json::from_str(&output).context("CLI returned an invalid Watch status")
    }

    pub fn watch_logs(&self) -> Result<String> {
        query(&self.worker, &[WORKER_FLAG, "logs"], None)
    }

    pub fn catalog(&self) -> Result<Vec<crate::catalog::CatalogEntry>> {
        let output = query(&self.worker, &[WORKER_FLAG, "catalog"], None)?;
        serde_json::from_str(&output).context("CLI returned an invalid package catalog")
    }

    pub fn run(
        &self,
        operation: Operation,
        project: &Path,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<OperationOutcome> {
        let _guard = self
            .busy
            .try_lock()
            .map_err(|_| anyhow::anyhow!("Another operation is running"))?;
        let project = if operation == Operation::StopWatch {
            project.to_path_buf()
        } else {
            project
                .canonicalize()
                .context("Project folder is unavailable")?
        };
        sink(OperationEvent::Started);
        let sink: Arc<dyn Fn(OperationEvent) + Send + Sync> = Arc::new(sink);
        let outcome = if cancellation.is_cancelled() {
            OperationOutcome::Cancelled
        } else if operation.cancellable() {
            execute(
                &self.worker,
                operation,
                &project,
                cancellation,
                sink.clone(),
            )?
        } else {
            // Control calls wait for acknowledgements. Cancelling their launcher cannot
            // establish that a detached Watch was stopped, so controls are not cancellable.
            if matches!(operation, Operation::StopWatch | Operation::SwitchWatch) {
                let output = query(&self.worker, &[WORKER_FLAG, "stop"], None)?;
                sink(OperationEvent::Output {
                    stderr: false,
                    text: output,
                });
                let stopped = self.watch_status()?;
                ensure!(
                    matches!(stopped.state, WatchState::Stopped | WatchState::Failed),
                    "Watch has not acknowledged Stop: {}",
                    stopped.message
                );
            }
            if matches!(operation, Operation::StartWatch | Operation::SwitchWatch) {
                let output = query(&self.worker, &[WORKER_FLAG, "start"], Some(&project))?;
                sink(OperationEvent::Output {
                    stderr: false,
                    text: output,
                });
                let active = self.watch_status()?;
                ensure!(
                    matches!(active.state, WatchState::Preparing | WatchState::Watching)
                        && active.project.as_deref() == Some(&project),
                    "Watch did not acknowledge the requested project: {}",
                    active.message
                );
            }
            OperationOutcome::Succeeded
        };
        sink(OperationEvent::Finished {
            outcome: outcome.clone(),
        });
        Ok(outcome)
    }
}

fn read_bounded(mut pipe: impl Read) -> std::io::Result<Vec<u8>> {
    let mut kept = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        let n = pipe.read(&mut chunk)?;
        if n == 0 {
            return Ok(kept);
        }
        let remaining = QUERY_LIMIT.saturating_sub(kept.len());
        kept.extend_from_slice(&chunk[..n.min(remaining)]);
    }
}

fn query(worker: &Path, args: &[&str], project: Option<&Path>) -> Result<String> {
    let mut command = Command::new(worker);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(project) = project {
        command.current_dir(project);
    } else if let Some(directory) = worker.parent() {
        command.current_dir(directory);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .context("Could not launch rproj CLI worker")?;
    let stdout = child.stdout.take().context("Missing worker stdout")?;
    let stderr = child.stderr.take().context("Missing worker stderr")?;
    let out = std::thread::spawn(move || read_bounded(stdout));
    let err = std::thread::spawn(move || read_bounded(stderr));
    let deadline = Instant::now() + Duration::from_secs(25);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("CLI control timed out; inspect Watch status before retrying");
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let out = out
        .join()
        .map_err(|_| anyhow::anyhow!("Worker output reader failed"))??;
    let err = err
        .join()
        .map_err(|_| anyhow::anyhow!("Worker error reader failed"))??;
    ensure!(
        status.success(),
        "CLI worker failed: {}",
        String::from_utf8_lossy(&err)
    );
    ensure!(
        out.len() < QUERY_LIMIT,
        "CLI response exceeded its size limit"
    );
    String::from_utf8(out).context("CLI worker returned invalid UTF-8")
}

#[cfg(not(windows))]
fn execute(
    _: &Path,
    _: Operation,
    _: &Path,
    _: &Cancellation,
    _: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    bail!(
        "Desktop execution is currently supported on Windows; direct CLI commands remain available"
    )
}

#[cfg(windows)]
fn execute(
    worker: &Path,
    operation: Operation,
    project: &Path,
    cancellation: &Cancellation,
    sink: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    let verb = if operation == Operation::Check {
        "check"
    } else {
        "test"
    };
    execute_owned(worker, &[WORKER_FLAG, verb], project, cancellation, sink)
}

#[cfg(windows)]
fn execute_owned(
    worker: &Path,
    args: &[&str],
    project: &Path,
    cancellation: &Cancellation,
    sink: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    let args: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
    execute_owned_os(worker, &args, project, cancellation, sink)
}

#[cfg(windows)]
fn execute_owned_os(
    worker: &Path,
    args: &[std::ffi::OsString],
    project: &Path,
    cancellation: &Cancellation,
    sink: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    use process_wrap::tokio::{CommandWrap, CreationFlags, JobObject, KillOnDrop};
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let mut command = CommandWrap::with_new(worker, |command| {
            command
                .args(args)
                .current_dir(project)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        });
        command.wrap(KillOnDrop).wrap(JobObject).wrap(CreationFlags(
            windows::Win32::System::Threading::CREATE_NO_WINDOW,
        ));
        let mut child = command
            .spawn()
            .context("Could not start owned CLI operation")?;
        async move {
            let stdout = tokio::spawn(drain(
                child.stdout().take().context("Missing worker stdout")?,
                false,
                sink.clone(),
            ));
            let stderr = tokio::spawn(drain(
                child.stderr().take().context("Missing worker stderr")?,
                true,
                sink,
            ));
            let result = loop {
                if let Some(status) = child.try_wait()? {
                    break if status.success() {
                        OperationOutcome::Succeeded
                    } else {
                        OperationOutcome::Failed {
                            code: status.code(),
                        }
                    };
                }
                if cancellation.is_cancelled() {
                    break OperationOutcome::Cancelled;
                }
                tokio::time::sleep(Duration::from_millis(30)).await;
            };
            // Closing the job also terminates grandchildren and releases inherited pipes.
            drop(child);
            stdout.await??;
            stderr.await??;
            Ok::<_, anyhow::Error>(result)
        }
        .await
        .context(OwnedTeardownUnconfirmed)
    })
}

#[cfg(windows)]
async fn drain(
    mut pipe: impl tokio::io::AsyncRead + Unpin,
    stderr: bool,
    sink: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<()> {
    use tokio::io::AsyncReadExt;
    let mut bytes = [0; 8192];
    let mut pending = Vec::new();
    loop {
        let n = pipe.read(&mut bytes).await?;
        pending.extend_from_slice(&bytes[..n]);
        let keep = match std::str::from_utf8(&pending) {
            Ok(_) => pending.len(),
            Err(error) if error.error_len().is_none() && n != 0 => error.valid_up_to(),
            Err(_) => pending.len(),
        };
        let text: String = String::from_utf8_lossy(&pending[..keep])
            .chars()
            .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
            .collect();
        pending.drain(..keep);
        if !text.is_empty() {
            sink(OperationEvent::Output { stderr, text });
        }
        if n == 0 {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_is_shared_and_controls_require_acknowledgement() {
        let flag = Cancellation::default();
        let other = flag.clone();
        other.cancel();
        assert!(flag.is_cancelled());
        assert!(!Operation::SwitchWatch.cancellable());
        assert!(Operation::Test.cancellable());
    }
    #[test]
    fn capture_is_bounded_but_drains_both_pipes() {
        let bytes = vec![b'x'; QUERY_LIMIT + 100];
        let mut reader = std::io::Cursor::new(bytes);
        assert_eq!(read_bounded(&mut reader).unwrap().len(), QUERY_LIMIT);
        assert_eq!(reader.position(), (QUERY_LIMIT + 100) as u64);
    }

    #[cfg(windows)]
    #[test]
    fn owned_operation_preserves_failure_and_both_streams() {
        let root = tempfile::tempdir().unwrap();
        let output = Arc::new(Mutex::new(Vec::new()));
        let copy = output.clone();
        let result = execute_owned(
            Path::new("cmd.exe"),
            &[
                "/D",
                "/C",
                "echo worker-out & echo worker-err 1>&2 & exit /B 7",
            ],
            root.path(),
            &Cancellation::default(),
            Arc::new(move |event| copy.lock().unwrap().push(event)),
        )
        .unwrap();
        assert_eq!(result, OperationOutcome::Failed { code: Some(7) });
        let events = output.lock().unwrap();
        assert!(events.iter().any(|e| matches!(e, OperationEvent::Output { stderr: false, text } if text.contains("worker-out"))));
        assert!(events.iter().any(|e| matches!(e, OperationEvent::Output { stderr: true, text } if text.contains("worker-err"))));
    }

    #[cfg(windows)]
    #[test]
    fn cancellation_closes_inherited_descendant_pipes() {
        let root = tempfile::tempdir().unwrap();
        let cancellation = Cancellation::default();
        let trigger = cancellation.clone();
        let thread = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            trigger.cancel();
        });
        let start = Instant::now();
        let result = execute_owned(
            Path::new("cmd.exe"),
            &["/D", "/C", "ping.exe -n 30 127.0.0.1"],
            root.path(),
            &cancellation,
            Arc::new(|_| {}),
        )
        .unwrap();
        thread.join().unwrap();
        assert_eq!(result, OperationOutcome::Cancelled);
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "Inherited output pipes remained open after cancellation"
        );
    }

    #[cfg(windows)]
    #[test]
    fn owned_errors_distinguish_pre_spawn_from_post_spawn_reader_failure() {
        let root = tempfile::tempdir().unwrap();
        let missing = execute_owned(
            &root.path().join("missing.exe"),
            &[],
            root.path(),
            &Cancellation::default(),
            Arc::new(|_| {}),
        )
        .unwrap_err();
        assert!(!missing.is::<OwnedTeardownUnconfirmed>());
        let reader_failure = execute_owned(
            Path::new("cmd.exe"),
            &["/D", "/C", "echo fault-fixture"],
            root.path(),
            &Cancellation::default(),
            Arc::new(|_| panic!("fixture output sink failure")),
        )
        .unwrap_err();
        assert!(
            reader_failure.is::<OwnedTeardownUnconfirmed>(),
            "{reader_failure:#}"
        );
    }
}
