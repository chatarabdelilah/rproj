use super::*;

fn exe() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("rproj.exe")
}

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

impl Drop for Session {
    fn drop(&mut self) {
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

fn stopped(session: &Session) {
    wait(|| {
        status_at(&session.root)
            .unwrap()
            .is_some_and(|snapshot| !snapshot.state.active())
    });
    for name in ["watcher.lock", "grandchild.lock", "recovery.lock"] {
        let path = session.project.join(name);
        if path.exists() {
            wait(|| OpenOptions::new().write(true).open(&path).is_ok());
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
    let snapshot = start_at(&root, &project, &exe()).unwrap();
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
    // Codex and some CI launchers prohibit job breakaway. WMI creates this owned
    // test launcher outside their job; production has no escape fallback.
    fs::create_dir_all(root).unwrap();
    let quote =
        |value: &std::ffi::OsStr| format!("'{}'", value.to_string_lossy().replace('\'', "''"));
    let script = root.join("launcher.ps1");
    let done = root.join("launcher.exit");
    fs::write(&script, format!("$env:RPROJ_TEST_WATCH_ROOT={}\n$env:RPROJ_TEST_WATCH_PROJECT={}\n$env:PATH={}\n$info = New-Object System.Diagnostics.ProcessStartInfo\n$info.FileName={}\n$info.Arguments='--exact background_watch::windows::tests::process_driver --nocapture'\n$info.UseShellExecute=$false\n$info.CreateNoWindow=$true\n$process=[System.Diagnostics.Process]::Start($info)\nif ($process.WaitForExit(12000)) {{ $process.ExitCode | Set-Content -Encoding ASCII -LiteralPath {} }} else {{ $process.Kill(); 'timeout' | Set-Content -Encoding ASCII -LiteralPath {} }}\n",
        quote(root.as_os_str()), quote(project.as_os_str()), quote(path), quote(std::env::current_exe().unwrap().as_os_str()), quote(done.as_os_str()), quote(done.as_os_str()))).unwrap();
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
        outside_launcher(&session.root, &session.project, &path);
        if scenario == "simultaneous" {
            let output = driver(&session.root, &session.project, &path)
                .output()
                .unwrap();
            assert!(
                output.status.success()
                    || String::from_utf8_lossy(&output.stderr).contains("Another rproj command"),
                "{output:?}"
            );
        }
        if scenario == "stop-recovery" {
            wait(|| session.project.join("recovery-started").exists());
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
        wait(|| status_at(&session.root).unwrap().unwrap().state == State::Watching);
        wait(|| session.project.join("grandchild-started").exists());
        let snapshot = status_at(&session.root).unwrap().unwrap();
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
            wait(|| {
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
