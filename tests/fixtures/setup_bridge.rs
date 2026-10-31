// Finite fake installers only. No native preview or machine configuration.
use std::{env, fs, process::Command, thread, time::Duration};
fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let exe = env::current_exe().unwrap();
    let root = exe.parent().unwrap();
    if args == ["--descendant"] {
        thread::sleep(Duration::from_millis(350));
        fs::write(root.join("descendant-finished"), "finished normally").unwrap();
        println!("descendant output drained");
        return;
    }
    if args == ["--version"] { println!("rproj {}", env!("RPROJ_TEST_WORKER_VERSION")); return; }
    let mode = fs::read_to_string(root.join("mode")).unwrap();
    if mode == "old-worker" { std::process::exit(2); }
    assert_eq!(args[0], "--rproj-workbench");
    if args[1] == "setup-support" { println!("2"); return; }
    assert_eq!(args[1], "setup-job");
    let report = std::path::Path::new(&args[2]).join("report.json");
    if mode == "missing-report" { println!("Succeeded (untrustworthy log)"); return; }
    if mode == "malformed-report" { fs::write(report, "{").unwrap(); return; }
    let status = Command::new(env!("RPROJ_TEST_WORKER_BINARY"))
        .args(["--exact", "workbench::setup::tests::driver", "--nocapture", "--test-threads=1"])
        .env("RPROJ_TEST_SETUP_ROOT", root).env("RPROJ_TEST_SETUP_IPC", &args[2]).status().unwrap();
    if mode == "mismatch-report" {
        fs::write(&report, fs::read_to_string(&report).unwrap().replace("\"id\":\"", "\"id\":\"wrong-")).unwrap();
    }
    if mode == "descendant" || mode == "late-stop" {
        // Parent exits first. The owned job must wait, not kill this descendant.
        let child = Command::new(&exe).arg("--descendant").spawn().unwrap();
        fs::write(root.join("descendant.pid"), child.id().to_string()).unwrap();
        if mode == "late-stop" { println!("fixture saved before late Stop"); }
    }
    if mode == "exit-disagrees" { std::process::exit(7); }
    fs::write(root.join("bridge-finished"), "finished").unwrap();
    std::process::exit(status.code().unwrap_or(1));
}
