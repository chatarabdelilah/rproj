// Test-only subprocess bridge. Its child runs the shared production dispatcher
// under the existing cfg(test) Watch-root scope; no production override exists.
use std::{env, fs, process::Command};

fn main() {
    let executable = env::current_exe().unwrap();
    let args: Vec<_> = env::args().skip(1).collect();
    if executable.file_stem().unwrap() == "lute" {
        println!("\nfixture Lute stdout: {args:?}");
        println!("fixture project: {}", env::current_dir().unwrap().display());
        eprintln!("fixture Lute stderr");
        std::process::exit(env::var("RPROJ_TEST_LUTE_EXIT").unwrap().parse().unwrap());
    }
    if args == ["--version"] {
        println!("rproj {}", env!("RPROJ_TEST_WORKER_VERSION"));
        return;
    }
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], "--rproj-workbench");
    assert!(matches!(args[1].as_str(), "check" | "test"));
    let root = executable.parent().unwrap();
    let status = Command::new(env!("RPROJ_TEST_WORKER_BINARY"))
        .args([
            "--exact",
            "workbench::worker_tests::isolated_worker_driver",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("RPROJ_TEST_WORKBENCH_ROOT", root.join("watch"))
        .env("RPROJ_TEST_WORKBENCH_OPERATION", &args[1])
        .env(
            "RPROJ_TEST_LUTE_EXIT",
            fs::read_to_string(root.join("lute-exit-code")).unwrap(),
        )
        .env("PATH", root.join("tools"))
        .status()
        .unwrap();
    std::process::exit(status.code().unwrap_or(1));
}
