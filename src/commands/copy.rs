use std::fs;

use anyhow::{Context, Result};
use arboard::Clipboard;
use walkdir::WalkDir;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    run_in(&cwd)
}

pub(super) fn run_in(cwd: &std::path::Path) -> Result<()> {
    run_with(cwd, copy_to_clipboard)
}

fn run_with(cwd: &std::path::Path, write: impl FnOnce(&str) -> Result<()>) -> Result<()> {
    let src_dir = cwd.join("src");

    if !src_dir.exists() {
        crate::diagnostics::event("copy.skip", "no src folder");
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
        let relative = path.strip_prefix(cwd).unwrap_or(path);
        let relative = relative.to_string_lossy().replace('\\', "/");

        match fs::read_to_string(path) {
            Ok(content) => {
                output.push_str(&format!("// --- {relative} --- \n"));
                output.push_str(&content);
                output.push_str("\n\n");
                file_count += 1;
            }
            Err(err) => {
                crate::diagnostics::event("copy.skip", format!("{relative}: {err}"));
                eprintln!("skipped {relative}: {err}");
            }
        }
    }

    if file_count == 0 {
        crate::diagnostics::event("copy.skip", "no readable source files");
        println!("'src' folder is empty. Nothing to copy.");
        return Ok(());
    }

    println!(
        "Copying {} characters from {file_count} file(s) to clipboard...",
        output.len()
    );
    write(&output)?;
    crate::diagnostics::event(
        "copy.complete",
        format!(
            "{file_count} files; {} bytes; content omitted",
            output.len()
        ),
    );
    println!("All files from 'src' successfully copied to clipboard!");
    Ok(())
}

fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new().context("failed to access the system clipboard")?;
    clipboard
        .set_text(text.to_string())
        .context("failed to write to the system clipboard")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_directory_controls_copy_without_using_the_clipboard() {
        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("A");
        let b = temp.path().join("B");
        for path in [&a, &b] {
            fs::create_dir_all(path.join("src")).unwrap();
        }
        fs::write(a.join("src/main.luau"), "selected project").unwrap();
        fs::write(b.join("src/main.luau"), "other project").unwrap();
        let cwd = std::env::current_dir().unwrap();
        let mut copied = String::new();
        run_with(&a, |text| {
            copied = text.into();
            Ok(())
        })
        .unwrap();
        assert!(copied.contains("// --- src/main.luau ---"));
        assert!(copied.contains("selected project"));
        assert!(!copied.contains("other project"));
        assert_eq!(std::env::current_dir().unwrap(), cwd);
    }
}
