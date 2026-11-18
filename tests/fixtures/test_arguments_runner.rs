// Records UTF-8 argument bytes without any shell or third-party dependencies.
use std::{env, fs, io::Write, process::Command, thread, time::Duration};
fn main() {
    if env::var_os("RPROJ_ARGUMENT_SLEEPER").is_some() {
        thread::sleep(Duration::from_secs(60)); return;
    }
    let root = std::path::PathBuf::from(env::var_os("RPROJ_ARGUMENT_FIXTURE_ROOT").unwrap());
    let mut bytes = Vec::new();
    for arg in env::args().skip(1) {
        bytes.extend_from_slice(&(arg.len() as u64).to_le_bytes());
        bytes.extend_from_slice(arg.as_bytes());
    }
    fs::write(root.join("arguments.bin"), bytes).unwrap();
    println!("runner stdout"); eprintln!("runner stderr");
    if fs::read_to_string(root.join("mode")).unwrap() == "cancel" {
        let child = Command::new(env::current_exe().unwrap()).env("RPROJ_ARGUMENT_SLEEPER", "1").spawn().unwrap();
        fs::write(root.join("descendant.pid"), child.id().to_string()).unwrap();
        println!("cancel-ready"); std::io::stdout().flush().unwrap();
        thread::sleep(Duration::from_secs(60));
    }
    if fs::read_to_string(root.join("mode")).unwrap() == "runner-failure" { std::process::exit(7); }
}
