use super::*;

fn delayed_control_acknowledgment(verb: &str) -> Result<Snapshot> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let snapshot = Snapshot {
        version: PROTOCOL,
        nonce: "delayed-ack-fixture".into(),
        project: "fixture".into(),
        state: if verb == "stop" {
            State::Stopping
        } else {
            State::Watching
        },
        message: String::new(),
        address: listener.local_addr().unwrap(),
        token: "fixture-token".into(),
        supervisor: std::process::id(),
    };
    let reply = snapshot.clone();
    let expected_verb = verb.to_owned();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut line = String::new();
        BufReader::new(&stream).read_line(&mut line).unwrap();
        let incoming: Request = serde_json::from_str(&line).unwrap();
        assert_eq!(incoming.version, PROTOCOL);
        assert_eq!(incoming.token, reply.token);
        assert_eq!(incoming.verb, expected_verb);
        // Model a durable session-state flush beyond the fast status-probe budget.
        std::thread::sleep(Duration::from_millis(750));
        let _ = stream.write_all(&serde_json::to_vec(&reply).unwrap());
    });
    let response = request(&snapshot, &snapshot.token, verb);
    server.join().unwrap();
    response
}

#[test]
fn mutation_acknowledgments_allow_durable_state_latency() {
    for verb in ["stop", "watching"] {
        let response = delayed_control_acknowledgment(verb).unwrap();
        assert_eq!(response.nonce, "delayed-ack-fixture");
        assert_eq!(
            response.state,
            if verb == "stop" {
                State::Stopping
            } else {
                State::Watching
            }
        );
    }
}

#[test]
fn status_probe_keeps_its_short_deadline() {
    let error = delayed_control_acknowledgment("status").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Watch control did not return an acknowledgment")
    );
}

#[test]
fn lost_control_reply_does_not_fail_the_supervisor() {
    struct AbortedPeer;
    impl Write for AbortedPeer {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::ConnectionAborted.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let snapshot = Snapshot {
        version: PROTOCOL,
        nonce: "aborted-peer-fixture".into(),
        project: "fixture".into(),
        state: State::Watching,
        message: String::new(),
        address: "127.0.0.1:1".parse().unwrap(),
        token: "fixture-token".into(),
        supervisor: std::process::id(),
    };
    write_control_reply(&mut AbortedPeer, &snapshot).unwrap();
    assert_eq!(snapshot.state, State::Watching);
}

fn exe() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("rproj.exe")
}

#[track_caller]
fn wait(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !condition() {
        assert!(Instant::now() < deadline, "Watch fixture timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}

struct Session {
    root: PathBuf,
    project: PathBuf,
}

impl Session {
    #[track_caller]
    fn wait(&self, phase: &str, condition: impl FnMut() -> bool) {
        eprintln!("Watch phase: {phase}");
        wait(condition);
    }

    fn report_failure(&self) {
        // Display state without serializing the snapshot's authentication tokens.
        eprintln!(
            "Watch failure: project={} persisted={:?} live={:?}",
            self.project.display(),
            metadata(&self.root).map(|snapshot| snapshot.map(|snapshot| snapshot.display())),
            status_at(&self.root).map(|snapshot| snapshot.map(|snapshot| snapshot.display()))
        );
        for path in [
            self.project.join("tools.log"),
            self.root.join("watch.1.log"),
            self.root.join("watch.log"),
        ] {
            let tail = fs::read(&path).map(|bytes| {
                String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(4096)..]).into_owned()
            });
            eprintln!("{} tail: {tail:?}", path.display());
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.report_failure();
        }
        let _ = fs::remove_file(self.project.join("hold-recovery"));
        if let Ok(Some(snapshot)) = status_at(&self.root)
            && snapshot.state.active()
        {
            let _ = request(&snapshot, &snapshot.token, "stop");
        }
        let _ = fs::remove_file(self.project.join("hold-watch"));
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if status_at(&self.root)
                .ok()
                .flatten()
                .is_none_or(|snapshot| !snapshot.state.active())
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

#[track_caller]
fn stopped(session: &Session) {
    session.wait("terminal state after stopping", || {
        status_at(&session.root)
            .unwrap()
            .is_some_and(|snapshot| !snapshot.state.active())
    });
    for name in ["watcher.lock", "grandchild.lock", "recovery.lock"] {
        let path = session.project.join(name);
        if path.exists() {
            session.wait(&format!("released {name}"), || {
                OpenOptions::new().write(true).open(&path).is_ok()
            });
        }
    }
}

#[test]
fn process_driver() {
    let Some(root) = std::env::var_os("RPROJ_TEST_WATCH_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let project = PathBuf::from(std::env::var_os("RPROJ_TEST_WATCH_PROJECT").unwrap());
    if std::env::var_os("RPROJ_TEST_WATCH_BLOCKED").is_some() {
        let error = start_at(&root, &project, &exe()).unwrap_err();
        assert!(format!("{error:#}").contains("breakaway"), "{error:#}");
        return;
    }
    let competing = std::env::var_os("RPROJ_TEST_WATCH_READY");
    if let Some(ready) = &competing {
        fs::write(ready, "ready").unwrap();
        wait(|| root.join("release-starts").exists());
    }
    let snapshot = match start_at(&root, &project, &exe()) {
        Ok(snapshot) => snapshot,
        Err(error)
            if competing.is_some() && error.to_string().contains("Another rproj command") =>
        {
            fs::write(
                PathBuf::from(competing.unwrap()).with_extension("owner"),
                "busy",
            )
            .unwrap();
            return;
        }
        Err(error) => panic!("{error:#}"),
    };
    if let Some(ready) = competing {
        fs::write(
            PathBuf::from(ready).with_extension("owner"),
            snapshot.supervisor.to_string(),
        )
        .unwrap();
    }
    assert!(matches!(snapshot.state, State::Preparing | State::Watching));
}

fn driver(root: &Path, project: &Path, path: &std::ffi::OsStr) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "background_watch::windows::tests::process_driver",
            "--nocapture",
        ])
        .env("RPROJ_TEST_WATCH_ROOT", root)
        .env("RPROJ_TEST_WATCH_PROJECT", project)
        .env("PATH", path);
    command
}

fn outside_launcher(root: &Path, project: &Path, path: &std::ffi::OsStr) {
    let done = begin_outside_launcher(root, project, path, false);
    finish_launcher(&done);
}

fn begin_outside_launcher(
    root: &Path,
    project: &Path,
    path: &std::ffi::OsStr,
    competing: bool,
) -> PathBuf {
    // Codex and some CI launchers prohibit job breakaway. WMI creates this owned
    // test launcher outside their job; production has no escape fallback.
    fs::create_dir_all(root).unwrap();
    let quote =
        |value: &std::ffi::OsStr| format!("'{}'", value.to_string_lossy().replace('\'', "''"));
    let script = root.join(format!("launcher-{}.ps1", &random_token().unwrap()[..12]));
    let done = script.with_extension("exit");
    let pending = script.with_extension("pending");
    let barrier = if competing {
        format!(
            "$env:RPROJ_TEST_WATCH_READY={}\n",
            quote(done.with_extension("ready").as_os_str())
        )
    } else {
        String::new()
    };
    // Publish only after WriteAllText closes its exclusive handle; existence
    // must mean that the complete marker is immediately readable on Windows.
    fs::write(&script, format!("{barrier}$env:RPROJ_TEST_WATCH_ROOT={}\n$env:RPROJ_TEST_WATCH_PROJECT={}\n$env:PATH={}\n$info = New-Object System.Diagnostics.ProcessStartInfo\n$info.FileName={}\n$info.Arguments='--exact background_watch::windows::tests::process_driver --nocapture'\n$info.UseShellExecute=$false\n$info.CreateNoWindow=$true\n$process=[System.Diagnostics.Process]::Start($info)\nif ($process.WaitForExit(12000)) {{ $result=[string]$process.ExitCode }} else {{ $process.Kill(); $result='timeout' }}\n[System.IO.File]::WriteAllText({}, $result, [System.Text.Encoding]::ASCII)\n[System.IO.File]::Move({}, {})\n",
        quote(root.as_os_str()), quote(project.as_os_str()), quote(path), quote(std::env::current_exe().unwrap().as_os_str()), quote(pending.as_os_str()), quote(pending.as_os_str()), quote(done.as_os_str()))).unwrap();
    let command = format!(
        "powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File \"{}\"",
        script.display()
    );
    let output = Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command",
        &format!("$result = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{CommandLine={}}}; if ($result.ReturnValue -ne 0) {{ exit $result.ReturnValue }}", quote(std::ffi::OsStr::new(&command)))]).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    done
}

fn finish_launcher(done: &Path) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !done.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(done.exists(), "fixture launcher did not finish");
    assert_eq!(
        fs::read_to_string(done).unwrap().trim(),
        "0",
        "fixture launcher failed"
    );
}

#[test]
fn slow_log_writer_does_not_block_the_control_runtime() {
    let root = tempfile::tempdir().unwrap();
    let log = Arc::new(Mutex::new(RotatingLog::new(root.path()).unwrap()));
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let busy_log = log.clone();
    let writer = std::thread::spawn(move || {
        let _guard = busy_log.lock().unwrap();
        ready_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(2)).is_ok()
    });
    ready_rx.recv().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let released = runtime.block_on(async {
        let (mut input, output) = tokio::io::duplex(64);
        let draining = tokio::spawn(drain(output, log));
        input.write_all(b"fixture output").await.unwrap();
        drop(input);
        tokio::time::sleep(Duration::from_millis(20)).await;
        let released = release_tx.send(()).is_ok();
        draining.await.unwrap().unwrap();
        released
    });
    assert!(
        writer.join().unwrap() && released,
        "log writer blocked control"
    );
    assert_eq!(
        fs::read_to_string(root.path().join("watch.log")).unwrap(),
        "fixture output"
    );
}

#[test]
fn concurrent_session_readers_do_not_break_atomic_replacement() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    let root = tempfile::tempdir().unwrap();
    private_directory(root.path()).unwrap();
    let snapshot = Snapshot {
        version: PROTOCOL,
        nonce: "probe".into(),
        project: root.path().into(),
        state: State::Watching,
        message: "probe".into(),
        address: "127.0.0.1:1".parse().unwrap(),
        token: "probe".into(),
        supervisor: 0,
    };
    save(root.path(), &snapshot).unwrap();
    let done = AtomicBool::new(false);
    let errors = AtomicUsize::new(0);
    let reads = AtomicUsize::new(0);
    let result = std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                while !done.load(Ordering::Relaxed) {
                    match metadata(root.path()) {
                        Ok(Some(current)) => {
                            assert_eq!(current.nonce, snapshot.nonce);
                            assert_eq!(current.state, State::Watching);
                            reads.fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(None) => panic!("snapshot disappeared during replacement"),
                        Err(error)
                            if error.downcast_ref::<std::io::Error>().is_some_and(|error| {
                                error.kind() == std::io::ErrorKind::WouldBlock
                            }) => {}
                        Err(error) => {
                            if errors.fetch_add(1, Ordering::Relaxed) == 0 {
                                eprintln!("reader error: {error:#}");
                            }
                        }
                    }
                }
            });
        }
        let result = (0..500).try_for_each(|iteration| {
            save(root.path(), &snapshot).with_context(|| format!("replacement {iteration}"))
        });
        done.store(true, Ordering::Relaxed);
        result
    });
    result.unwrap();
    assert_eq!(errors.load(Ordering::Relaxed), 0);
    assert!(reads.load(Ordering::Relaxed) > 0);
    assert_eq!(
        metadata(root.path()).unwrap().unwrap().state,
        State::Watching
    );
}

#[test]
fn held_state_lock_refuses_reads_and_writes_without_changing_the_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Snapshot {
        version: PROTOCOL,
        nonce: "fixture".into(),
        project: root.path().into(),
        state: State::Watching,
        message: String::new(),
        address: "127.0.0.1:1".parse().unwrap(),
        token: "fixture".into(),
        supervisor: 0,
    };
    save(root.path(), &snapshot).unwrap();
    let before = fs::read(root.path().join("session.json")).unwrap();
    let held = open_lock(root.path(), "state.lock").unwrap();
    assert!(acquire(&held).unwrap());
    for error in [
        metadata(root.path()).unwrap_err(),
        save(root.path(), &snapshot).unwrap_err(),
    ] {
        assert!(error.to_string().contains("state is busy"));
        assert_eq!(
            error.downcast_ref::<std::io::Error>().unwrap().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
    assert_eq!(fs::read(root.path().join("session.json")).unwrap(), before);
    drop(held);
    assert_eq!(
        metadata(root.path()).unwrap().unwrap().state,
        State::Watching
    );
}

#[test]
fn supervisor_lifecycle_detachment_recovery_and_tree_cleanup() {
    let fixtures = tempfile::tempdir().unwrap();
    let bin = fixtures.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let output = Command::new("rustc")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/background_tool.rs"
        ))
        .arg("-o")
        .arg(bin.join("rojo.exe"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::copy(bin.join("rojo.exe"), bin.join("rokit.exe")).unwrap();
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    assert!(
        exe().is_file(),
        "cargo build --locked must produce the fixture's rproj binary"
    );
    for scenario in [
        "normal",
        "stop-recovery",
        "failure",
        "crash",
        "flood",
        "blocked",
        "simultaneous",
    ] {
        eprintln!("Watch lifecycle: {scenario}");
        let project = fixtures.path().join(scenario);
        fs::create_dir(&project).unwrap();
        fs::write(project.join("default.project.json"), "{}").unwrap();
        fs::write(project.join("hold-watch"), "").unwrap();
        let session = Session {
            root: fixtures.path().join(format!("{scenario}-session")),
            project,
        };
        if scenario == "stop-recovery" {
            fs::write(session.project.join("rokit.toml"), "[tools]").unwrap();
            fs::write(session.project.join("hold-recovery"), "").unwrap();
        }
        if scenario == "failure" {
            fs::write(session.project.join("fail-watch"), "").unwrap();
        }
        if scenario == "flood" {
            fs::write(session.project.join("flood-output"), "").unwrap();
        }
        if scenario == "blocked" {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let command = driver(&session.root, &session.project, &path);
                let mut command = tokio::process::Command::from(command);
                command.env("RPROJ_TEST_WATCH_BLOCKED", "1");
                let mut wrapped: CommandWrap = command.into();
                let mut child = wrapped.wrap(KillOnDrop).wrap(JobObject).spawn().unwrap();
                assert!(child.wait().await.unwrap().success());
            });
            assert!(!session.root.join("session.json").exists());
            continue;
        }
        if scenario == "simultaneous" {
            let first = begin_outside_launcher(&session.root, &session.project, &path, true);
            let second = begin_outside_launcher(&session.root, &session.project, &path, true);
            session.wait("both competing launchers ready", || {
                first.with_extension("ready").exists() && second.with_extension("ready").exists()
            });
            fs::write(session.root.join("release-starts"), "go").unwrap();
            finish_launcher(&first);
            finish_launcher(&second);
            let owner = status_at(&session.root)
                .unwrap()
                .unwrap()
                .supervisor
                .to_string();
            let outcomes = [
                fs::read_to_string(first.with_extension("owner")).unwrap(),
                fs::read_to_string(second.with_extension("owner")).unwrap(),
            ];
            assert!(outcomes.iter().any(|result| result == &owner));
            assert!(
                outcomes
                    .iter()
                    .all(|result| result == "busy" || result == &owner)
            );
        } else {
            outside_launcher(&session.root, &session.project, &path);
        }
        if scenario == "stop-recovery" {
            session.wait("recovery tool started", || {
                session.project.join("recovery-started").exists()
            });
            let snapshot = status_at(&session.root).unwrap().unwrap();
            assert_eq!(
                request(&snapshot, &snapshot.token, "stop").unwrap().state,
                State::Stopping
            );
            std::thread::sleep(Duration::from_millis(150));
            assert!(!session.project.join("recovery-finished").exists());
            assert!(
                OpenOptions::new()
                    .write(true)
                    .open(session.project.join("recovery.lock"))
                    .is_err()
            );
            fs::remove_file(session.project.join("hold-recovery")).unwrap();
            stopped(&session);
            assert_eq!(
                status_at(&session.root).unwrap().unwrap().state,
                State::Stopped
            );
            assert!(session.project.join("recovery-finished").exists());
            assert!(!session.project.join("watcher-started").exists());
            continue;
        }
        if scenario == "failure" {
            stopped(&session);
            assert_eq!(
                status_at(&session.root).unwrap().unwrap().state,
                State::Failed
            );
            assert!(
                fs::read_to_string(session.root.join("watch.log"))
                    .unwrap()
                    .contains("Rojo sourcemap Watch exited"),
                "state={} log={:?}",
                status_at(&session.root).unwrap().unwrap().display(),
                fs::read_to_string(session.root.join("watch.log"))
            );
            continue;
        }
        session.wait("Watching acknowledgement", || {
            status_at(&session.root).unwrap().unwrap().state == State::Watching
        });
        session.wait("grandchild started", || {
            session.project.join("grandchild-started").exists()
        });
        let snapshot = status_at(&session.root).unwrap().unwrap();
        if scenario == "simultaneous" {
            assert_eq!(
                fs::read_to_string(session.project.join("tools.log"))
                    .unwrap()
                    .lines()
                    .filter(|line| line.contains("--watch"))
                    .count(),
                1
            );
        }
        assert_eq!(
            start_at(&session.root, &session.project, &exe())
                .unwrap()
                .supervisor,
            snapshot.supervisor
        );
        let other = fixtures.path().join("other");
        fs::create_dir_all(&other).unwrap();
        fs::write(other.join("default.project.json"), "{}").unwrap();
        assert!(
            start_at(&session.root, &other, &exe())
                .unwrap_err()
                .to_string()
                .contains("already owns")
        );
        assert!(request(&snapshot, "wrong-token", "stop").is_err());
        assert_eq!(
            status_at(&session.root).unwrap().unwrap().state,
            State::Watching
        );
        if scenario == "flood" {
            session.wait("flood output drained", || {
                let snapshot = status_at(&session.root).unwrap().unwrap();
                assert_ne!(snapshot.state, State::Failed, "{}", snapshot.display());
                session.project.join("output-drained").exists()
            });
            for name in ["watch.log", "watch.1.log"] {
                assert!(fs::metadata(session.root.join(name)).unwrap().len() <= LOG_SIZE as u64);
            }
        }
        if scenario == "crash" {
            // Only this freshly authenticated fixture PID is used to simulate death; production never kills cached PIDs.
            let output = Command::new("taskkill")
                .args(["/F", "/PID", &snapshot.supervisor.to_string()])
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            stopped(&session);
            assert_eq!(
                status_at(&session.root).unwrap().unwrap().state,
                State::Failed
            );
        } else {
            assert_eq!(
                request(&snapshot, &snapshot.token, "stop").unwrap().state,
                State::Stopping
            );
            stopped(&session);
            assert_eq!(
                status_at(&session.root).unwrap().unwrap().state,
                State::Stopped
            );
        }
    }
}

#[test]
fn held_owner_refuses_unresponsive_or_corrupt_session_replacement() {
    let root = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("default.project.json"), "{}").unwrap();
    let owner = open_lock(root.path(), "owner.lock").unwrap();
    assert!(acquire(&owner).unwrap());
    assert!(
        start_at(root.path(), project.path(), &exe())
            .unwrap_err()
            .to_string()
            .contains("ownership")
    );
    let snapshot = Snapshot {
        version: PROTOCOL,
        nonce: "fixture".into(),
        project: project.path().into(),
        state: State::Preparing,
        message: String::new(),
        address: "127.0.0.1:1".parse().unwrap(),
        token: "fixture".into(),
        supervisor: 0,
    };
    save(root.path(), &snapshot).unwrap();
    assert_eq!(
        status_at(root.path()).unwrap().unwrap().state,
        State::Unresponsive
    );
    assert!(start_at(root.path(), project.path(), &exe()).is_err());
    fs::write(root.path().join("session.json"), "{").unwrap();
    assert!(start_at(root.path(), project.path(), &exe()).is_err());
    drop(owner);
}

#[test]
fn mutation_policy_preserves_testing_and_blocks_conflicting_writes() {
    let mut snapshot = Snapshot {
        version: PROTOCOL,
        nonce: "fixture".into(),
        project: "fixture".into(),
        state: State::Preparing,
        message: String::new(),
        address: "127.0.0.1:1".parse().unwrap(),
        token: "fixture".into(),
        supervisor: 0,
    };
    for state in [
        State::Preparing,
        State::Watching,
        State::Stopping,
        State::Unresponsive,
    ] {
        snapshot.state = state;
        assert!(check_operation(&snapshot, true, false).is_err());
        assert!(check_operation(&snapshot, false, false).is_ok());
        assert_eq!(
            check_operation(&snapshot, true, true).is_ok(),
            state == State::Watching
        );
        assert_eq!(
            check_operation(&snapshot, false, true).is_ok(),
            state == State::Watching
        );
    }
    for state in [State::Stopped, State::Failed] {
        snapshot.state = state;
        assert!(check_operation(&snapshot, true, false).is_ok());
    }
}

#[test]
#[ignore = "requires installed Rojo"]
fn installed_rojo_keeps_updating_after_launcher_exit() {
    let fixtures = tempfile::tempdir().unwrap();
    let project = fixtures.path().join("project");
    fs::create_dir_all(project.join("src/shared")).unwrap();
    fs::write(project.join("default.project.json"), r#"{"name":"WatchAcceptance","tree":{"$className":"DataModel","ReplicatedStorage":{"$className":"ReplicatedStorage","shared":{"$path":"src/shared"}}}}"#).unwrap();
    let session = Session {
        root: fixtures.path().join("session"),
        project,
    };
    outside_launcher(
        &session.root,
        &session.project,
        &std::env::var_os("PATH").unwrap(),
    );
    wait(|| status_at(&session.root).unwrap().unwrap().state == State::Watching);
    fs::write(
        session.project.join("src/shared/persistent.luau"),
        "return {}\n",
    )
    .unwrap();
    wait(|| {
        fs::read_to_string(session.project.join("sourcemap.json"))
            .unwrap()
            .contains("persistent")
    });
    let snapshot = status_at(&session.root).unwrap().unwrap();
    request(&snapshot, &snapshot.token, "stop").unwrap();
    stopped(&session);
    assert_eq!(
        status_at(&session.root).unwrap().unwrap().state,
        State::Stopped
    );
}

#[test]
#[ignore = "requires installed Rojo, Wally and wally-package-types, plus network access"]
fn installed_wally_and_jest_background_recovery_preserves_types_and_mounts() {
    let fixtures = tempfile::tempdir().unwrap();
    for jest in [false, true] {
        let project = fixtures.path().join(if jest { "jest" } else { "wally" });
        fs::create_dir_all(project.join("src/shared")).unwrap();
        fs::create_dir_all(project.join("src/server")).unwrap();
        fs::create_dir_all(project.join("src/client")).unwrap();
        let mut graph = crate::graph::ProjectGraph {
            package_workflow: crate::config::PackageWorkflow::Wally,
            packages: vec!["charm".into()],
            ..Default::default()
        };
        if jest {
            graph.choose("test", Some("jest-roblox"));
            for package in graph.derived().packages {
                if !graph.packages.contains(&package) {
                    graph.packages.push(package);
                }
            }
            crate::steps::jest::ensure_test_tree(&project, false).unwrap();
        }
        crate::config::project_file::save_to(&graph, &project).unwrap();
        let document = crate::steps::rojo::project_document(
            "WatchAcceptance",
            crate::config::PackageWorkflow::Wally,
            false,
            false,
            None,
        )
        .unwrap();
        fs::write(
            project.join("default.project.json"),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
        fs::write(
            project.join("wally.toml"),
            crate::steps::wally::render_wally_toml("rproj/background-watch", &graph.packages)
                .unwrap(),
        )
        .unwrap();
        let session = Session {
            root: fixtures.path().join(if jest {
                "jest-session"
            } else {
                "wally-session"
            }),
            project,
        };
        outside_launcher(
            &session.root,
            &session.project,
            &std::env::var_os("PATH").unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(180);
        loop {
            let snapshot = status_at(&session.root).unwrap().unwrap();
            assert_ne!(
                snapshot.state,
                State::Failed,
                "{}\n{:?}",
                snapshot.display(),
                fs::read_to_string(session.root.join("watch.log"))
            );
            if snapshot.state == State::Watching {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "background package recovery timed out"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
        let link = fs::read_dir(session.project.join("Packages"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.is_file()
                    && path
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .eq_ignore_ascii_case("charm")
            })
            .unwrap();
        assert!(fs::read_to_string(link).unwrap().contains("export type"));
        if jest {
            assert!(
                fs::read_dir(session.project.join("DevPackages"))
                    .unwrap()
                    .any(|entry| {
                        let path = entry.unwrap().path();
                        path.is_file()
                            && path
                                .file_stem()
                                .unwrap()
                                .to_string_lossy()
                                .eq_ignore_ascii_case("Jest")
                    })
            );
            let generated: serde_json::Value = serde_json::from_slice(
                &fs::read(session.project.join("jest.project.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(
                generated["tree"]["ReplicatedStorage"]["devPackages"]["$path"],
                "DevPackages"
            );
        }
        fs::write(
            session.project.join("src/shared/persistent.luau"),
            "return {}\n",
        )
        .unwrap();
        wait(|| {
            fs::read_to_string(session.project.join("sourcemap.json"))
                .unwrap()
                .contains("persistent")
        });
        let snapshot = status_at(&session.root).unwrap().unwrap();
        request(&snapshot, &snapshot.token, "stop").unwrap();
        stopped(&session);
    }
}
