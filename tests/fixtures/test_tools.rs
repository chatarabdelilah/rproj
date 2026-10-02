use std::{fs, io::Write, path::Path, process};

fn main() {
    let executable = std::env::current_exe().unwrap();
    let name = executable.file_stem().unwrap().to_str().unwrap();
    let args: Vec<String> = std::env::args().skip(1).collect();
    writeln!(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("tool-calls.txt")
            .unwrap(),
        "{name} {}",
        args.join(" ")
    )
    .unwrap();
    match name {
        "rojo" => fs::write("sourcemap.json", "{}").unwrap(),
        "wally" => {
            let manifest = fs::read_to_string("wally.toml").unwrap();
            let folder = if manifest.contains("[dev-dependencies]") {
                "DevPackages"
            } else {
                "Packages"
            };
            fs::create_dir_all(Path::new(folder).join("_Index")).unwrap();
            fs::write("package-installed", "").unwrap();
        }
        "wally-package-types" if Path::new("package-installed").exists() => {}
        "wally-package-types" => {
            eprintln!("already retyped links cannot be processed again");
            process::exit(9);
        }
        "jest-roblox-cli" | "lute" => println!("fixture runner completed"),
        _ => process::exit(10),
    }
}
