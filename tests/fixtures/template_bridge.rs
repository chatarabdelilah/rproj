// Fixture executable doubles as the matching CLI bridge and as a finite fake Rojo.
use std::{env, fs, io::Write, path::PathBuf, process::{Command, ExitCode}, thread, time::Duration};

fn main() -> ExitCode {
    let exe = env::current_exe().unwrap();
    let root = exe.ancestors().find(|p| p.join("fixture-mode").is_file()).unwrap();
    let mode = fs::read_to_string(root.join("fixture-mode")).unwrap();
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args == ["--descendant"] {
        thread::sleep(Duration::from_millis(250));
        fs::write(root.join("descendant-completed"), "done").unwrap();
        println!("final descendant output");
        return ExitCode::SUCCESS;
    }
    if exe.file_stem().unwrap() == "rojo" {
        assert!(args.len() == 4 && args[2] == "-o");
        assert!(PathBuf::from(&args[1]).is_file());
        let cwd = env::current_dir().unwrap();
        let label = cwd.file_name().unwrap().to_str().unwrap();
        writeln!(fs::OpenOptions::new().create(true).append(true).open(root.join("validations")).unwrap(),
                 "{label} {}", args[0].to_str().unwrap()).unwrap();
        if mode == "reject-rojo" { eprintln!("fixture Rojo rejection"); return ExitCode::FAILURE; }
        fs::write(PathBuf::from(&args[3]), "fixture output").unwrap();
        return ExitCode::SUCCESS;
    }
    if args == ["--version"] {
        println!("rproj {}", env!("RPROJ_TEST_WORKER_VERSION"));
        return ExitCode::SUCCESS;
    }
    assert_eq!(args[0], "--rproj-workbench");
    if args[1] == "template-support" {
        if mode == "old-worker" { return ExitCode::FAILURE; }
        println!("1");
        return ExitCode::SUCCESS;
    }
    assert_eq!(args[1], "template-job");
    let ipc = PathBuf::from(&args[2]);
    let report = ipc.join("report.json");
    if mode == "missing" { println!("Succeeded, according to an untrusted log"); return ExitCode::SUCCESS; }
    if mode == "malformed" { fs::write(report, "{").unwrap(); return ExitCode::SUCCESS; }
    let status = Command::new(env!("RPROJ_TEST_WORKER_BINARY"))
        .args(["--exact", "workbench::project_template::tests::driver", "--nocapture", "--test-threads=1"])
        .env("RPROJ_TEST_TEMPLATE_ROOT", root).env("RPROJ_TEST_TEMPLATE_IPC", ipc).status().unwrap();
    match mode.as_str() {
        "mismatched" => { let text = fs::read_to_string(&report).unwrap().replace("\"id\":\"", "\"id\":\"foreign-"); fs::write(report, text).unwrap(); }
        "lost-after-write" => fs::remove_file(report).unwrap(),
        "bad-exit" => return ExitCode::FAILURE,
        "descendant" => { let _child = Command::new(exe).arg("--descendant").spawn().unwrap(); }
        _ => {}
    }
    ExitCode::from(status.code().unwrap_or(1) as u8)
}
