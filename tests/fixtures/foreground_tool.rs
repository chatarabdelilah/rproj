use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

fn main() {
    let program = std::env::current_exe().unwrap();
    let name = program.file_stem().unwrap().to_str().unwrap();
    let mut log = OpenOptions::new().create(true).append(true).open("tools.log").unwrap();
    writeln!(log, "{name}").unwrap();
    if Path::new("fail-tool").exists() {
        eprintln!("fixture tool failed");
        std::process::exit(9);
    }
    if Path::new("hold-tool").exists() {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0);
        }
        let _lock = options.open("active-child.lock").unwrap();
        println!("fixture child running");
        let started = std::time::Instant::now();
        while started.elapsed() < std::time::Duration::from_secs(90) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        eprintln!("fixture child timed out waiting for interruption");
        std::process::exit(10);
    }
    println!("fixture tool completed");
}
