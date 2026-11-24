// Disposable test-only worker/tool bridge; never shipped or used for previews.
use std::{env, fs, process::{Command, Stdio}, thread, time::Duration};
fn main() {
    let exe = env::current_exe().unwrap();
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "--sleeper") { thread::sleep(Duration::from_secs(60)); return; }
    let packages = std::path::Path::new("Packages");
    match exe.file_stem().unwrap().to_str().unwrap() {
        "wally" => {
            assert_eq!(args, ["install"]);
            fs::remove_dir(packages).unwrap(); // Empty Wally installs remove the package directory.
            return;
        }
        "rojo" => {
            assert_eq!(args, ["sourcemap", "default.project.json", "-o", "sourcemap.json"]);
            assert!(packages.is_dir(), "missing Packages mount during sourcemap generation");
            fs::write("sourcemap.json", "{}").unwrap();
            return;
        }
        "wally-package-types" => {
            assert_eq!(args, ["-s", "sourcemap.json", "Packages"]);
            assert!(std::path::Path::new("sourcemap.json").is_file());
            return;
        }
        _ => {}
    }
    if exe.file_stem().unwrap() == "rokit" {
        let root = exe.parent().unwrap().parent().unwrap();
        let mode = fs::read_to_string(root.join("mode")).unwrap();
        println!("fixture Rokit stdout"); eprintln!("fixture Rokit stderr");
        if mode == "fail" { std::process::exit(9); }
        if mode == "cancel" {
            let mut child = Command::new(&exe).arg("--sleeper").spawn().unwrap();
            fs::write(root.join("ready"), child.id().to_string()).unwrap();
            let _ = child.wait();
        }
        if args.first().is_some_and(|a| a == "init") { fs::write("rokit.toml", "[tools]\n").unwrap(); }
        if args.first().is_some_and(|a| a == "add") {
            let mut text = fs::read_to_string("rokit.toml").unwrap();
            let source = args.last().unwrap().to_str().unwrap();
            let key = source.rsplit('/').next().unwrap().to_ascii_lowercase();
            text.push_str(&format!("{key} = '{source}@2.0.0'\n"));
            fs::write("rokit.toml", text).unwrap();
        }
        return;
    }
    if args == ["--version"] { println!("rproj {}", env!("RPROJ_TEST_WORKER_VERSION")); return; }
    assert_eq!(args[0], "--rproj-workbench");
    let root = exe.parent().unwrap();
    let context = args[1] == "composition-context";
    assert!(context || args[1] == "composition-job");
    let mode = fs::read_to_string(root.join("mode")).unwrap();
    if context && mode == "old-worker" { eprintln!("unknown workbench operation"); std::process::exit(2); }
    if !context && mode == "missing-report" { return; }
    if !context && mode == "malformed-report" { fs::write(std::path::Path::new(&args[2]).join("report.json"), "{").unwrap(); return; }
    let mut command = Command::new(env!("RPROJ_TEST_WORKER_BINARY"));
    command.args(["--exact", "workbench::composition::tests::driver", "--nocapture", "--test-threads=1"])
        .env("RPROJ_TEST_COMPOSITION_ROOT", root)
        .env("RPROJ_TEST_COMPOSITION_ACTION", if context { "context" } else { "job" })
        .env("PATH", root.join("tools"));
    if context { command.stdout(Stdio::null()); }
    else { command.env("RPROJ_TEST_COMPOSITION_IPC", &args[2]); }
    let status = command.status().unwrap();
    if status.success() && context { print!("{}", fs::read_to_string(root.join("context.json")).unwrap()); }
    std::process::exit(status.code().unwrap_or(1));
}
