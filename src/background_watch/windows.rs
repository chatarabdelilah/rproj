use std::cell::Cell;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use process_wrap::tokio::{CommandWrap, CreationFlags, JobObject, KillOnDrop};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

const PROTOCOL: u8 = 1;
const MAX_MESSAGE: u64 = 16 * 1024;
const LOG_SIZE: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_millis(300);
const INTERNAL: &str = "--rproj-watch-internal";
const NONCE: &str = "RPROJ_WATCH_NONCE";
const ENGINE_TOKEN: &str = "RPROJ_WATCH_ENGINE_TOKEN";
const DETACHED_BREAKAWAY: u32 = 0x00000008 | 0x01000000;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum State {
    Preparing,
    Watching,
    Stopping,
    Stopped,
    Failed,
    Unresponsive,
}

impl State {
    fn active(self) -> bool {
        matches!(
            self,
            Self::Preparing | Self::Watching | Self::Stopping | Self::Unresponsive
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Snapshot {
    version: u8,
    nonce: String,
    project: PathBuf,
    state: State,
    message: String,
    address: SocketAddr,
    token: String,
    supervisor: u32,
}

impl Snapshot {
    fn display(&self) -> String {
        format!(
            "{:?}: {}\n{}",
            self.state,
            self.project.display(),
            self.message
        )
    }
}

#[derive(Serialize, Deserialize)]
struct Request {
    version: u8,
    token: String,
    verb: String,
}

fn root() -> Result<PathBuf> {
    #[cfg(test)]
    if let Some(root) = TEST_ROOT.with(|root| root.borrow().clone()) {
        return Ok(root);
    }
    Ok(directories::ProjectDirs::from("", "", "rproj")
        .context("could not find the user's local data directory")?
        .data_local_dir()
        .join("watch"))
}

#[cfg(test)]
thread_local! { static TEST_ROOT: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) }; }

#[cfg(test)]
pub(crate) fn with_unresponsive_fixture<R>(project: &Path, action: impl FnOnce() -> R) -> R {
    struct Restore(Option<PathBuf>);
    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_ROOT.with(|root| *root.borrow_mut() = self.0.take());
        }
    }
    let root = tempfile::tempdir().unwrap();
    private_directory(root.path()).unwrap();
    let owner = open_lock(root.path(), "owner.lock").unwrap();
    assert!(acquire(&owner).unwrap());
    let snapshot = Snapshot {
        version: PROTOCOL,
        nonce: "fixture".into(),
        project: fs::canonicalize(project).unwrap(),
        state: State::Watching,
        message: String::new(),
        address: "127.0.0.1:1".parse().unwrap(),
        token: "fixture".into(),
        supervisor: 0,
    };
    save(root.path(), &snapshot).unwrap();
    let _restore = Restore(TEST_ROOT.with(|slot| slot.replace(Some(root.path().into()))));
    action()
}

fn private_directory(path: &Path) -> Result<()> {
    use std::os::windows::fs::MetadataExt;
    use windows_permissions::{
        LocalBox, SecurityDescriptor,
        constants::{SeObjectType::SE_FILE_OBJECT, SecurityInformation},
        utilities, wrappers,
    };
    fs::create_dir_all(path)?;
    ensure!(
        fs::symlink_metadata(path)?.file_attributes() & 0x400 == 0,
        "Watch storage cannot be a link"
    );
    let sid = utilities::current_process_sid()?;
    let descriptor: LocalBox<SecurityDescriptor> = format!("D:P(A;OICI;FA;;;{sid})").parse()?;
    wrappers::SetNamedSecurityInfo(
        path,
        SE_FILE_OBJECT,
        SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        None,
        None,
        descriptor.dacl(),
        None,
    )?;
    Ok(())
}

fn open_lock(root: &Path, name: &str) -> Result<File> {
    Ok(OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join(name))?)
}

fn acquire(file: &File) -> Result<bool> {
    match file.try_lock() {
        Ok(()) => Ok(true),
        Err(std::fs::TryLockError::WouldBlock) => Ok(false),
        Err(std::fs::TryLockError::Error(error)) => Err(error.into()),
    }
}

fn acquire_shared(file: &File) -> Result<bool> {
    match file.try_lock_shared() {
        Ok(()) => Ok(true),
        Err(std::fs::TryLockError::WouldBlock) => Ok(false),
        Err(std::fs::TryLockError::Error(error)) => Err(error.into()),
    }
}

fn random_token() -> Result<String> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes)
        .map_err(|error| anyhow::anyhow!("OS randomness unavailable: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn save(root: &Path, snapshot: &Snapshot) -> Result<()> {
    let mut pending =
        tempfile::NamedTempFile::new_in(root).context("could not stage Watch session state")?;
    pending
        .write_all(&serde_json::to_vec(snapshot)?)
        .context("could not write staged Watch session state")?;
    pending
        .as_file()
        .sync_all()
        .context("could not flush staged Watch session state")?;
    let _state = lock_state(root, true)?;
    pending
        .persist(root.join("session.json"))
        .map_err(|error| error.error)
        .context("could not replace Watch session state")?;
    Ok(())
}

fn metadata(root: &Path) -> Result<Option<Snapshot>> {
    if !root.exists() {
        return Ok(None);
    }
    let _state = lock_state(root, false)?;
    match File::open(root.join("session.json")) {
        Ok(file) => {
            let snapshot: Snapshot =
                serde_json::from_reader(BufReader::new(file).take(MAX_MESSAGE))?;
            ensure!(
                snapshot.version == PROTOCOL && snapshot.address.ip().is_loopback(),
                "Unsupported Watch session metadata"
            );
            Ok(Some(snapshot))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn lock_state(root: &Path, writing: bool) -> Result<File> {
    let state = open_lock(root, "state.lock")?;
    let gate = open_lock(root, "state-writer.lock")?;
    let deadline = Instant::now() + TIMEOUT;
    let pause = || -> Result<()> {
        if Instant::now() >= deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "Watch session state is busy; retry the command",
            )
            .into());
        }
        std::thread::sleep(Duration::from_millis(1));
        Ok(())
    };
    if writing {
        // Stop new readers before waiting for existing read handles to close.
        // Staging and flushing happen before this short replacement interval.
        while !acquire(&gate)? {
            pause()?;
        }
        while !acquire(&state)? {
            pause()?;
        }
        return Ok(state);
    }
    loop {
        if acquire_shared(&gate)? {
            let acquired = acquire_shared(&state)?;
            gate.unlock()?;
            if acquired {
                return Ok(state);
            }
        }
        pause()?;
    }
}

fn request(snapshot: &Snapshot, token: &str, verb: &str) -> Result<Snapshot> {
    let mut stream = TcpStream::connect_timeout(&snapshot.address, TIMEOUT)?;
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    serde_json::to_writer(
        &mut stream,
        &Request {
            version: PROTOCOL,
            token: token.into(),
            verb: verb.into(),
        },
    )?;
    stream.write_all(b"\n")?;
    let result: Snapshot = serde_json::from_reader(BufReader::new(stream).take(MAX_MESSAGE))?;
    ensure!(
        result.version == PROTOCOL && result.nonce == snapshot.nonce,
        "Watch owner changed; retry the command"
    );
    Ok(result)
}

fn status_at(root: &Path) -> Result<Option<Snapshot>> {
    if !root.exists() {
        return Ok(None);
    }
    let lock = open_lock(root, "owner.lock")?;
    let held = !acquire_shared(&lock)?;
    let cached = metadata(root);
    if !held {
        let mut snapshot = cached?;
        if let Some(snapshot) = &mut snapshot
            && snapshot.state.active()
        {
            snapshot.state = State::Failed;
            snapshot.message = "The supervisor exited unexpectedly. Its owned processes were terminated; restart manually.".into();
        }
        return Ok(snapshot);
    }
    let mut snapshot = cached?.context(
        "Watch ownership is held but metadata is unavailable; do not start a replacement",
    )?;
    match request(&snapshot, &snapshot.token, "status") {
        Ok(live) => Ok(Some(live)),
        Err(_) => {
            snapshot.state = State::Unresponsive;
            snapshot.message = "Watch ownership is still held, but control is unavailable. No replacement will start.".into();
            Ok(Some(snapshot))
        }
    }
}

pub(crate) fn summary() -> String {
    root()
        .and_then(|root| status_at(&root))
        .map(|snapshot| {
            snapshot
                .map(|snapshot| snapshot.display())
                .unwrap_or_else(|| "Stopped: no background Watch".into())
        })
        .unwrap_or_else(|error| format!("Watch status unavailable: {error:#}"))
}

pub(crate) fn show_status() -> Result<()> {
    let root = root()?;
    match status_at(&root)? {
        Some(snapshot) => println!("{}", snapshot.display()),
        None => println!("Stopped: no background Watch"),
    }
    Ok(())
}

pub(crate) fn show_logs() -> Result<()> {
    logs_at(&root()?)
}

fn logs_at(root: &Path) -> Result<()> {
    println!("Watch logs: {}", root.display());
    for name in ["watch.1.log", "watch.log"] {
        match File::open(root.join(name)) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(LOG_SIZE as u64).read_to_end(&mut bytes)?;
                // Tool output is untrusted terminal data; never replay control sequences.
                let safe: String = String::from_utf8_lossy(&bytes)
                    .chars()
                    .filter(|ch| !ch.is_control() || matches!(ch, '\n' | '\t'))
                    .collect();
                println!("{safe}");
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub(crate) fn stop() -> Result<()> {
    if let Some(snapshot) = status_at(&root()?)? {
        if snapshot.state.active() {
            let live = request(&snapshot, &snapshot.token, "stop")
                .context("Watch could not acknowledge Stop; it has not been reported stopped")?;
            println!("{}", live.display());
        } else {
            println!("{}", snapshot.display());
        }
    } else {
        println!("Stopped: no background Watch");
    }
    Ok(())
}

thread_local! { static MUTATION_DEPTH: Cell<usize> = const { Cell::new(0) }; }

pub(crate) struct MutationGuard {
    _lock: Option<File>,
}

impl Drop for MutationGuard {
    fn drop(&mut self) {
        MUTATION_DEPTH.with(|depth| depth.set(depth.get() - 1));
    }
}

pub(crate) fn mutation_guard(
    project: Option<&Path>,
    allow_watching: bool,
) -> Result<MutationGuard> {
    if MUTATION_DEPTH.with(|depth| depth.get() > 0) {
        MUTATION_DEPTH.with(|depth| depth.set(depth.get() + 1));
        return Ok(MutationGuard { _lock: None });
    }
    let root = root()?;
    private_directory(&root)?;
    let lock = open_lock(&root, "start.lock")?;
    lock.try_lock_shared()
        .context("Background Watch is starting; retry after its acknowledgement")?;
    if let Some(snapshot) = status_at(&root)?
        && snapshot.state.active()
    {
        let same_project = project
            .map(fs::canonicalize)
            .transpose()?
            .is_none_or(|project| project == snapshot.project);
        check_operation(&snapshot, same_project, allow_watching)?;
    }
    MUTATION_DEPTH.with(|depth| depth.set(1));
    Ok(MutationGuard { _lock: Some(lock) })
}

fn check_operation(snapshot: &Snapshot, same_project: bool, allow_watching: bool) -> Result<()> {
    ensure!(
        !snapshot.state.active()
            || if allow_watching {
                snapshot.state == State::Watching
            } else {
                !same_project
            },
        "Stop background Watch before changing this project or machine setup, or wait until recovery finishes before testing. {}. Run `rproj watch stop`.",
        snapshot.display()
    );
    Ok(())
}

fn gate(root: &Path) -> Result<File> {
    let lock = open_lock(root, "start.lock")?;
    ensure!(
        acquire(&lock)?,
        "Another rproj command is preparing or changing a project; retry when it finishes"
    );
    Ok(lock)
}

pub(crate) fn start_in(project: &Path) -> Result<()> {
    let snapshot = start_at(&root()?, project, &std::env::current_exe()?)?;
    println!(
        "{}\nClosing rproj leaves Watch running. Use `rproj watch status`, `logs`, or `stop`.",
        snapshot.display()
    );
    Ok(())
}

fn start_at(root: &Path, project: &Path, exe: &Path) -> Result<Snapshot> {
    ensure!(
        project.join("default.project.json").is_file(),
        "Background Watch requires default.project.json in the current project"
    );
    let project = fs::canonicalize(project)?;
    private_directory(root)?;
    let _gate = gate(root)?;
    // An unlocked damaged cache is replaceable; a held owner lock always wins.
    let owner = open_lock(root, "owner.lock")?;
    let occupied = !acquire_shared(&owner)?;
    drop(owner);
    if occupied {
        let snapshot =
            status_at(root)?.context("Watch owner is starting; retry after acknowledgement")?;
        ensure!(
            snapshot.project == project
                && snapshot.state.active()
                && snapshot.state != State::Unresponsive,
            "Background Watch already owns {} ({:?}); run `rproj watch stop` before starting another project",
            snapshot.project.display(),
            snapshot.state
        );
        return Ok(snapshot);
    }
    let nonce = random_token()?;
    let mut child = Command::new(exe).arg(INTERNAL).arg("supervisor").arg(root).arg(&project)
        .env(NONCE, &nonce).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .creation_flags(DETACHED_BREAKAWAY).spawn()
        .context("Could not detach background Watch. The launcher may prohibit job breakaway; launch rproj from a normal Windows terminal and retry")?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(Some(snapshot)) = status_at(root)
            && snapshot.nonce == nonce
        {
            ensure!(snapshot.state != State::Failed, "{}", snapshot.display());
            if matches!(
                snapshot.state,
                State::Preparing | State::Watching | State::Stopping
            ) {
                return Ok(snapshot);
            }
        }
        if let Some(exit) = child.try_wait()? {
            bail!(
                "Background Watch supervisor exited before acknowledgement ({exit}); inspect `rproj watch logs`"
            );
        }
        if Instant::now() >= deadline {
            bail!(
                "Background Watch has not acknowledged startup. Check `rproj watch status` before retrying"
            );
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

pub(crate) fn internal_entry() -> Option<Result<()>> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new(INTERNAL)) {
        return None;
    }
    Some((|| {
        let mode = args.next().context("missing worker mode")?;
        let root = PathBuf::from(args.next().context("missing session directory")?);
        let project = PathBuf::from(args.next().context("missing project directory")?);
        let nonce = std::env::var(NONCE).context("internal Watch requires a startup nonce")?;
        ensure!(args.next().is_none(), "unexpected internal worker argument");
        match mode.to_str() {
            Some("supervisor") => supervisor(&root, &project, nonce),
            Some("engine") => engine(&root, &project),
            _ => bail!("unknown Watch worker mode"),
        }
    })())
}

struct RotatingLog {
    root: PathBuf,
    file: File,
    size: usize,
}

impl RotatingLog {
    fn new(root: &Path) -> Result<Self> {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("watch.log"))?;
        let size = file.metadata()?.len() as usize;
        file.seek(SeekFrom::End(0))?;
        Ok(Self {
            root: root.into(),
            file,
            size,
        })
    }
    fn append(&mut self, bytes: &[u8]) -> Result<()> {
        for chunk in bytes.chunks(8192) {
            if self.size + chunk.len() > LOG_SIZE {
                // Copy while retaining the open handle, then truncate; log readers cannot block a rename.
                self.file.flush()?;
                fs::copy(self.root.join("watch.log"), self.root.join("watch.1.log"))?;
                self.file.set_len(0)?;
                self.file.seek(SeekFrom::Start(0))?;
                self.size = 0;
            }
            self.file.write_all(chunk)?;
            self.size += chunk.len();
        }
        self.file.flush()?;
        Ok(())
    }
}

async fn drain(mut pipe: impl AsyncRead + Unpin, log: Arc<Mutex<RotatingLog>>) -> Result<()> {
    let mut bytes = [0; 8192];
    loop {
        let count = pipe.read(&mut bytes).await?;
        if count == 0 {
            return Ok(());
        }
        let chunk = bytes[..count].to_vec();
        let log = log.clone();
        // Disk writes and rotation must not block the single control-runtime thread.
        tokio::task::spawn_blocking(move || {
            log.lock()
                .map_err(|_| anyhow::anyhow!("Watch log lock failed"))?
                .append(&chunk)
        })
        .await??;
    }
}

fn supervisor(root: &Path, project: &Path, nonce: String) -> Result<()> {
    private_directory(root)?;
    let owner = open_lock(root, "owner.lock")?;
    let deadline = Instant::now() + Duration::from_secs(1);
    while !acquire(&owner)? {
        ensure!(
            Instant::now() < deadline,
            "another Watch supervisor owns this session"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    listener.set_nonblocking(true)?;
    let mut snapshot = Snapshot {
        version: PROTOCOL,
        nonce,
        project: project.into(),
        state: State::Preparing,
        message: "Restoring tools and packages. Stop finishes the active recovery command.".into(),
        address: listener.local_addr()?,
        token: random_token()?,
        supervisor: std::process::id(),
    };
    let engine_token = random_token()?;
    save(root, &snapshot)?;
    let log = Arc::new(Mutex::new(RotatingLog::new(root)?));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(async {
        let mut command = CommandWrap::with_new(std::env::current_exe()?, |command| {
            command.arg(INTERNAL).arg("engine").arg(root).arg(project).env(NONCE, &snapshot.nonce)
                .env(ENGINE_TOKEN, &engine_token).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        });
        command.wrap(KillOnDrop).wrap(JobObject).wrap(CreationFlags(windows::Win32::System::Threading::CREATE_NO_WINDOW));
        let mut child = command.spawn().context("could not start the owned Watch engine")?;
        let mut input = child.stdin().take().context("missing engine control input")?;
        let mut stdout = Some(tokio::spawn(drain(child.stdout().take().context("missing engine output")?, log.clone())));
        let mut stderr = Some(tokio::spawn(drain(child.stderr().take().context("missing engine errors")?, log.clone())));
        let mut stopping = false;
        loop {
            if let Some(exit) = child.try_wait().context("could not poll owned Watch engine")? {
                // Closing the armed JobObject also terminates surviving grandchildren.
                drop(child);
                if let Some(task) = stdout.take() { task.await??; }
                if let Some(task) = stderr.take() { task.await??; }
                snapshot.state = if stopping { State::Stopped } else { State::Failed };
                snapshot.message = if stopping { "Watch stopped; owned processes exited.".into() } else { format!("Watch engine exited ({exit}). Inspect logs and restart manually.") };
                break;
            }
            if stdout.as_ref().is_some_and(|task| task.is_finished()) { stdout.take().unwrap().await??; }
            if stderr.as_ref().is_some_and(|task| task.is_finished()) { stderr.take().unwrap().await??; }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_read_timeout(Some(TIMEOUT))?;
                    stream.set_write_timeout(Some(TIMEOUT))?;
                    let mut line = String::new();
                    let read = BufReader::new(&stream).take(MAX_MESSAGE).read_line(&mut line);
                    if read.is_ok() && let Ok(request) = serde_json::from_str::<Request>(&line) && request.version == PROTOCOL {
                        if request.token == snapshot.token {
                            if request.verb == "stop" && !stopping {
                                stopping = true;
                                let watching = snapshot.state == State::Watching;
                                snapshot.state = State::Stopping;
                                snapshot.message = "Stopping after the active recovery command; no later steps will run.".into();
                                save(root, &snapshot)?;
                                if watching { child.start_kill()?; } else { input.write_all(b"stop\n").await?; }
                            }
                            if matches!(request.verb.as_str(), "status" | "stop") { serde_json::to_writer(&mut stream, &snapshot)?; }
                        } else if request.token == engine_token && request.verb == "watching" {
                            if stopping { child.start_kill()?; } else {
                                snapshot.state = State::Watching;
                                snapshot.message = "Sourcemap Watch is running. Closing rproj leaves it running.".into();
                                save(root, &snapshot)?;
                            }
                            serde_json::to_writer(&mut stream, &snapshot)?;
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error).context("could not accept Watch control connection"),
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Ok::<_, anyhow::Error>(())
    });
    if let Err(error) = result {
        snapshot.state = State::Failed;
        snapshot.message = format!("{error:#}");
        log.lock()
            .map_err(|_| anyhow::anyhow!("Watch log lock failed"))?
            .append(snapshot.message.as_bytes())?;
    }
    save(root, &snapshot)?;
    drop(owner);
    Ok(())
}

fn engine(root: &Path, project: &Path) -> Result<()> {
    let token = std::env::var(ENGINE_TOKEN).context("missing engine authentication")?;
    std::thread::spawn(|| {
        let mut line = String::new();
        let _ = std::io::stdin().lock().read_line(&mut line);
        crate::interrupt::request();
    });
    let file = crate::commands::watch::prepare_in(project)?;
    crate::steps::rojo::generate_sourcemap_from(project, file)?;
    crate::interrupt::check()?;
    let mut watcher = Command::new("rojo")
        .args(["sourcemap", "--watch", file, "-o", "sourcemap.json"])
        .current_dir(project)
        .stdin(Stdio::null())
        .spawn()
        .context("could not start Rojo sourcemap Watch")?;
    let snapshot = metadata(root)?.context("missing supervisor metadata")?;
    if let Err(error) = request(&snapshot, &token, "watching") {
        let _ = watcher.kill();
        let _ = watcher.wait();
        return Err(error.context("supervisor did not acknowledge Watching"));
    }
    let exit = watcher.wait()?;
    bail!("Rojo sourcemap Watch exited ({exit})")
}
