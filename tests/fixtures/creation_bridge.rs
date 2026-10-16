// Fixture-only bridge: production helpers run in a test child with explicitly
// injected storage. No production environment/config-root override is added.
use std::{env, fs, process::{Command, Stdio}, thread, time::Duration};
fn main() {
    let exe = env::current_exe().unwrap();
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "--sleeper") { thread::sleep(Duration::from_secs(60)); return; }
    if exe.file_stem().unwrap() == "git" {
        let root = exe.parent().unwrap().parent().unwrap();
        let mode = fs::read_to_string(root.join("mode")).unwrap();
        println!("fixture Git stdout"); eprintln!("fixture Git stderr");
        if mode == "fail" { std::process::exit(9); }
        if mode == "save-race" {
            fs::create_dir_all(root.join("setups")).unwrap();
            fs::write(root.join("setups/Race setup.toml"), "preserved concurrent setup").unwrap();
        }
        fs::create_dir_all(".git").unwrap();
        if mode == "cancel-after" {
            let mut child = Command::new(&exe).arg("--sleeper").spawn().unwrap();
            fs::write(root.join("ready"), "claimed").unwrap();
            let _ = child.wait();
        }
        return;
    }
    if args == ["--version"] { println!("rproj {}", env!("RPROJ_TEST_WORKER_VERSION")); return; }
    assert_eq!(args[0], "--rproj-workbench");
    let root = exe.parent().unwrap();
    let context = args[1] == "creation-context";
    let mode = fs::read_to_string(root.join("mode")).unwrap();
    if context && mode == "old-worker" { eprintln!("unknown workbench operation"); std::process::exit(1); }
    if !context && mode == "missing-report" { return; }
    if !context && mode == "wrong-report" {
        fs::write(std::path::Path::new(&args[2]).join("report.json"), r#"{"protocol":1,"id":"wrong","claim":"Claimed","status":"Succeeded","prepared":null,"error":null}"#).unwrap();
        return;
    }
    if !context && mode == "false-success" {
        let directory = std::path::Path::new(&args[2]);
        let id = directory.file_name().unwrap().to_str().unwrap();
        fs::write(directory.join("report.json"), format!(r#"{{"protocol":1,"id":"{id}","claim":"NotClaimed","status":"Succeeded","prepared":null,"error":null}}"#)).unwrap();
        return;
    }
    let mut command = Command::new(env!("RPROJ_TEST_WORKER_BINARY"));
    command.args(["--exact", "workbench::creation::worker_tests::driver", "--nocapture", "--test-threads=1"])
        .env("RPROJ_TEST_CREATION_ROOT", root)
        .env("RPROJ_TEST_CREATION_ACTION", if context { "context" } else { "job" })
        .env("PATH", root.join("tools"));
    if context { command.stdout(Stdio::null()); }
    else { command.env("RPROJ_TEST_CREATION_IPC", &args[2]); }
    let status = command.status().unwrap();
    if status.success() && context { print!("{}", fs::read_to_string(root.join("context.json")).unwrap()); }
    std::process::exit(status.code().unwrap_or(1));
}
