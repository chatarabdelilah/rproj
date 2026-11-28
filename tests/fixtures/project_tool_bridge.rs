// Disposable owned-worker fixture; never installs real tools or opens a window.
use std::{
    env, fs,
    process::{Command, Stdio},
    thread,
    time::Duration,
};
fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args == ["--sleeper"] { thread::sleep(Duration::from_secs(60)); return; }
    if args == ["--version"] { println!("rproj {}", env!("RPROJ_TEST_WORKER_VERSION")); return; }
    let exe = env::current_exe().unwrap();
    let root = exe.parent().unwrap();
    let mode = fs::read_to_string(root.join("mode")).unwrap();
    if mode == "old-worker" { eprintln!("unsupported worker"); std::process::exit(2); }
    assert_eq!(args[0], "--rproj-workbench");
    if args[1] == "project-tool-support" { println!("1"); return; }
    assert_eq!(args[1], "project-tool-job");
    let report = std::path::Path::new(&args[2]).join("report.json");
    if mode == "missing-report" { println!("Succeeded (untrustworthy fixture log)"); return; }
    if mode == "malformed-report" { fs::write(report, "{").unwrap(); return; }
    let status = Command::new(env!("RPROJ_TEST_WORKER_BINARY"))
        .args(["--exact", "workbench::project_tool::tests::driver", "--nocapture", "--test-threads=1"])
        .env("RPROJ_TEST_PROJECT_TOOL_ROOT", root).env("RPROJ_TEST_PROJECT_TOOL_IPC", &args[2])
        .env("PATH", root.join("tools"))
        .env("RPROJ_TOOL_FIXTURE_MODE", &mode)
        .env("RPROJ_TOOL_FIXTURE_LOG", root.join("tool-calls"))
        .status().unwrap();
    if mode == "mismatch-report" { fs::write(&report, fs::read_to_string(&report).unwrap().replace("\"id\":\"", "\"id\":\"wrong-")).unwrap(); }
    if mode == "descendant" {
        let child = Command::new(&exe)
            .arg("--sleeper")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        fs::write(root.join("descendant.pid"), child.id().to_string()).unwrap();
    }
    if mode == "exit-disagrees" { std::process::exit(7); }
    std::process::exit(status.code().unwrap_or(1));
}
