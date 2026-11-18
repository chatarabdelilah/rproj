// Disposable worker: uses the real job implementation with isolated Watch storage.
use std::{env, fs, process::Command, thread, time::Duration};
fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args == ["--sleeper"] { thread::sleep(Duration::from_secs(60)); return; }
    if args == ["--version"] { println!("rproj {}", env!("RPROJ_TEST_WORKER_VERSION")); return; }
    let exe = env::current_exe().unwrap();
    let root = exe.parent().unwrap();
    let mode = fs::read_to_string(root.join("mode")).unwrap();
    if mode == "old-worker" { std::process::exit(2); }
    assert_eq!(args[0], "--rproj-workbench");
    if args[1] == "test-support" {
        if mode == "support-stall" {
            let child = Command::new(&exe).arg("--sleeper").spawn().unwrap();
            fs::write(root.join("descendant.pid"), child.id().to_string()).unwrap();
            fs::write(root.join("probe-ready"), "ready").unwrap();
            thread::sleep(Duration::from_secs(60));
        }
        println!("1"); return;
    }
    assert_eq!(args[1], "test-job");
    let report = std::path::Path::new(&args[2]).join("report.json");
    if mode == "missing-report" { println!("Succeeded (untrustworthy log)"); return; }
    if mode == "malformed-report" { fs::write(report, "{").unwrap(); return; }
    let status = Command::new(env!("RPROJ_TEST_WORKER_BINARY"))
        .args(["--exact", "workbench::test_execution::tests::process::driver", "--nocapture", "--test-threads=1"])
        .env("RPROJ_TEST_ARGUMENT_ROOT", root).env("RPROJ_TEST_ARGUMENT_IPC", &args[2])
        .env("PATH", root.join("tools"))
        .env("RPROJ_ARGUMENT_FIXTURE_ROOT", root)
        .status().unwrap();
    if mode == "mismatch-report" { fs::write(&report, fs::read_to_string(&report).unwrap().replace("\"id\":\"", "\"id\":\"wrong-")).unwrap(); }
    if mode == "descendant" {
        let child = Command::new(&exe).arg("--sleeper").spawn().unwrap();
        fs::write(root.join("descendant.pid"), child.id().to_string()).unwrap();
    }
    if mode == "exit-disagrees" { std::process::exit(7); }
    std::process::exit(status.code().unwrap_or(1));
}
