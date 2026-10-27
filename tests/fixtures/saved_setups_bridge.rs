// Disposable owned-worker/report fixtures; never used for native previews.
use std::{env, fs, process::Command, thread, time::Duration};
fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args == ["--sleeper"] { thread::sleep(Duration::from_secs(60)); return; }
    if args == ["--version"] { println!("rproj {}", env!("RPROJ_TEST_WORKER_VERSION")); return; }
    let exe = env::current_exe().unwrap();
    let root = exe.parent().unwrap();
    let mode = fs::read_to_string(root.join("mode")).unwrap();
    if mode == "old-worker" { eprintln!("unknown workbench operation"); std::process::exit(2); }
    assert_eq!(args[0], "--rproj-workbench");
    if args[1] == "saved-setups-support" { println!("1"); return; }
    assert_eq!(args[1], "saved-setups-job");
    let report = std::path::Path::new(&args[2]).join("report.json");
    if mode == "missing-report" { println!("Succeeded (untrustworthy fixture log)"); return; }
    if mode == "malformed-report" { fs::write(report, "{").unwrap(); return; }
    let status = Command::new(env!("RPROJ_TEST_WORKER_BINARY"))
        .args(["--exact", "workbench::saved_setups::tests::owned::driver", "--nocapture", "--test-threads=1"])
        .env("RPROJ_TEST_SAVED_ROOT", root)
        .env("RPROJ_TEST_SAVED_IPC", &args[2]).status().unwrap();
    if mode == "mismatch-report" {
        fs::write(&report, fs::read_to_string(&report).unwrap().replace("\"id\":\"", "\"id\":\"wrong-")).unwrap();
    }
    if mode == "descendant" {
        let child = Command::new(&exe).arg("--sleeper").spawn().unwrap();
        fs::write(root.join("descendant.pid"), child.id().to_string()).unwrap();
        // The parent exits while its child retains the pipes; the owning job must close it.
    }
    if mode == "exit-disagrees" { std::process::exit(7); }
    std::process::exit(status.code().unwrap_or(1));
}
