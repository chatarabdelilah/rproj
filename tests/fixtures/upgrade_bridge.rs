// Disposable process-tree/report fixtures; never shipped or used for previews.
use std::{env, fs, process::{Command, Stdio}, thread, time::Duration};
fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args == ["--sleeper"] { thread::sleep(Duration::from_secs(60)); return; }
    if args == ["--version"] { println!("rproj {}", env!("RPROJ_TEST_WORKER_VERSION")); return; }
    assert_eq!(args[0], "--rproj-workbench");
    let exe = env::current_exe().unwrap();
    let root = exe.parent().unwrap();
    let mode = fs::read_to_string(root.join("mode")).unwrap();
    if mode == "old-worker" { eprintln!("unknown workbench operation"); std::process::exit(2); }
    if args[1] == "upgrade-support" { println!("1"); return; }
    let context = args[1] == "upgrade-context";
    assert!(context || args[1] == "upgrade-job");
    if !context && mode == "missing-report" { println!("Succeeded (untrustworthy fixture log)"); return; }
    if !context && mode == "malformed-report" { fs::write(std::path::Path::new(&args[2]).join("report.json"), "{").unwrap(); return; }
    if !context && mode == "cancel" {
        let mut child = Command::new(&exe).arg("--sleeper").spawn().unwrap();
        fs::write(root.join("ready"), child.id().to_string()).unwrap();
        let _ = child.wait(); return;
    }
    let mut command = Command::new(env!("RPROJ_TEST_WORKER_BINARY"));
    command.args(["--exact", "workbench::upgrade::tests::driver", "--nocapture", "--test-threads=1"])
        .env("RPROJ_TEST_UPGRADE_ROOT", root)
        .env("RPROJ_TEST_UPGRADE_ACTION", if context { "context" } else { "job" });
    if context { command.stdout(Stdio::null()); }
    else { command.env("RPROJ_TEST_UPGRADE_IPC", &args[2]); }
    let status = command.status().unwrap();
    if status.success() && context { print!("{}", fs::read_to_string(root.join("context.json")).unwrap()); }
    if !context && mode == "mismatch-report" {
        let path = std::path::Path::new(&args[2]).join("report.json");
        fs::write(&path, fs::read_to_string(&path).unwrap().replace("\"id\":\"", "\"id\":\"wrong-")).unwrap();
    }
    std::process::exit(status.code().unwrap_or(1));
}
