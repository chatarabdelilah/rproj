// Test-only Rokit stand-in. Never installs, downloads, or reads user configuration.
use std::{env, fs, io::Write};

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let log = env::var_os("RPROJ_TOOL_FIXTURE_LOG").unwrap();
    writeln!(fs::OpenOptions::new().create(true).append(true).open(log).unwrap(), "{}", args.join(" ")).unwrap();
    let mode = env::var("RPROJ_TOOL_FIXTURE_MODE").unwrap();
    if mode == "failed-trust" && args.first().is_some_and(|a| a == "trust") { std::process::exit(7); }
    if mode == "failed-init" && args.first().is_some_and(|a| a == "init") { std::process::exit(8); }
    if args.first().is_some_and(|a| a == "add") {
        if mode == "failed-add" {
            eprintln!("fixture installation failed");
            std::process::exit(9);
        }
        if mode == "misleading-output" {
            eprintln!("already exists (diagnostic text, not a successful installation)");
            std::process::exit(9);
        }
        if mode != "missing-pin" {
            let mut file = fs::OpenOptions::new().append(true).open("rokit.toml").unwrap();
            writeln!(file, "asphalt = 'jackTabsCode/asphalt@0.9.0'").unwrap();
        }
        if mode == "changed-other-pin" { fs::write("rokit.toml", "[tools]\nasphalt='jacktabscode/asphalt@0.9.0'\nwally='unreviewed/replacement@1'\n").unwrap(); }
        if mode == "failed-after-pin" { eprintln!("download failed after manifest update"); std::process::exit(9); }
    } else if args.first().is_some_and(|a| a == "init") {
        fs::write("rokit.toml", "[tools]\n").unwrap();
    }
}
