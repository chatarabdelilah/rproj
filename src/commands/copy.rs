use std::fs;

use anyhow::{Context, Result};
use arboard::Clipboard;
use walkdir::WalkDir;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let src_dir = cwd.join("src");

    if !src_dir.exists() {
        println!("No 'src' folder found in the current directory.");
        return Ok(());
    }

    let mut output = String::new();
    let mut file_count = 0;

    for entry in WalkDir::new(&src_dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let relative = path.strip_prefix(&cwd).unwrap_or(path);
        let relative = relative.to_string_lossy().replace('\\', "/");

        match fs::read_to_string(path) {
            Ok(content) => {
                output.push_str(&format!("// --- {relative} --- \n"));
                output.push_str(&content);
                output.push_str("\n\n");
                file_count += 1;
            }
            Err(err) => eprintln!("skipped {relative}: {err}"),
        }
    }

    if file_count == 0 {
        println!("'src' folder is empty. Nothing to copy.");
        return Ok(());
    }

    println!("Copying {} characters from {file_count} file(s) to clipboard...", output.len());
    copy_to_clipboard(&output)?;
    println!("All files from 'src' successfully copied to clipboard!");
    Ok(())
}

fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new().context("failed to access the system clipboard")?;
    clipboard.set_text(text.to_string()).context("failed to write to the system clipboard")?;
    Ok(())
}
