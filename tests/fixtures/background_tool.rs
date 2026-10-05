use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn lock(name: &str) -> std::fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new().read(true).write(true).create(true).truncate(false).share_mode(0).open(name).unwrap()
}

fn wait_for_release(path: &str) {
    let started = Instant::now();
    while Path::new(path).exists() {
        if started.elapsed() > Duration::from_secs(40) { std::process::exit(90); }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn main() {
    let exe = std::env::current_exe().unwrap();
    let name = exe.file_stem().unwrap().to_string_lossy();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["grandchild"] {
        let _lock = lock("grandchild.lock");
        fs::write("grandchild-started", "ready").unwrap();
        wait_for_release("hold-watch");
        return;
    }
    writeln!(OpenOptions::new().create(true).append(true).open("tools.log").unwrap(), "{name} {}", args.join(" ")).unwrap();
    if name == "rokit" {
        let _lock = lock("recovery.lock");
        fs::write("recovery-started", "ready").unwrap();
        wait_for_release("hold-recovery");
        fs::write("recovery-finished", "done").unwrap();
        return;
    }
    if !args.iter().any(|arg| arg == "--watch") {
        fs::write("sourcemap.json", "{\"fixture\":true}").unwrap();
        return;
    }
    let _lock = lock("watcher.lock");
    let _child = Command::new(&exe).arg("grandchild").stdin(Stdio::null()).spawn().unwrap();
    fs::write("watcher-started", "ready").unwrap();
    if Path::new("flood-output").exists() {
        let bytes = vec![b'x'; 8192];
        for _ in 0..400 {
            std::io::stdout().write_all(&bytes).unwrap();
            std::io::stderr().write_all(&bytes).unwrap();
        }
        fs::write("output-drained", "ready").unwrap();
    }
    if Path::new("fail-watch").exists() { std::process::exit(7); }
    wait_for_release("hold-watch");
}
