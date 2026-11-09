use anyhow::{Context, Result};
use arboard::Clipboard;
use rproj_core::source_copy;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    run_in(&cwd)
}

pub(super) fn run_in(cwd: &std::path::Path) -> Result<()> {
    run_with(cwd, copy_to_clipboard)
}

fn run_with(cwd: &std::path::Path, write: impl FnOnce(&str) -> Result<()>) -> Result<()> {
    let snapshot =
        source_copy::collect_with_abandonment(cwd, || crate::interrupt::check().is_err())?;
    if snapshot.source_missing {
        crate::diagnostics::event("copy.skip", "no src folder");
        println!("No 'src' folder found in the current directory.");
        return Ok(());
    }

    for skipped in &snapshot.skipped {
        crate::diagnostics::event("copy.skip", format!("{}: {}", skipped.path, skipped.reason));
        eprintln!("skipped {}: {}", skipped.path, skipped.reason);
    }
    if snapshot.files.is_empty() {
        crate::diagnostics::event("copy.skip", "no readable source files");
        if snapshot.skipped.is_empty() {
            println!("'src' contains no source files. Nothing to copy.");
        } else {
            println!(
                "No readable source files; {} path(s) skipped. Nothing copied.",
                snapshot.skipped.len()
            );
        }
        return Ok(());
    }
    println!(
        "Copying {} UTF-8 bytes from {} included file(s) to clipboard...",
        snapshot.text.len(),
        snapshot.files.len()
    );
    write(&snapshot.text)?;
    crate::diagnostics::event(
        "copy.complete",
        format!(
            "{} included files; {} skipped; {} bytes; content omitted",
            snapshot.files.len(),
            snapshot.skipped.len(),
            snapshot.text.len()
        ),
    );
    println!("{}", completion(&snapshot));
    Ok(())
}

fn completion(snapshot: &source_copy::Snapshot) -> String {
    format!(
        "Copied {} included file(s) from 'src'; {} path(s) skipped.",
        snapshot.files.len(),
        snapshot.skipped.len()
    )
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
    use std::fs;

    #[test]
    fn refusals_and_empty_collections_never_access_clipboard() {
        let temp = tempfile::tempdir().unwrap();
        run_with(temp.path(), |_| panic!("missing src")).unwrap();
        fs::create_dir(temp.path().join("src")).unwrap();
        run_with(temp.path(), |_| panic!("empty src")).unwrap();
        fs::write(temp.path().join("src/binary"), [255]).unwrap();
        run_with(temp.path(), |_| panic!("no readable files")).unwrap();
        fs::write(
            temp.path().join("src/large"),
            vec![b'x'; source_copy::MAX_PAYLOAD_BYTES],
        )
        .unwrap();
        assert!(run_with(temp.path(), |_| panic!("over limit")).is_err());
    }

    #[test]
    fn partial_reporting_and_clipboard_failures_are_honest() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("src/readable.txt"), "read me").unwrap();
        fs::write(temp.path().join("src/binary"), [255]).unwrap();
        let snapshot = source_copy::collect(temp.path()).unwrap();
        assert_eq!(
            completion(&snapshot),
            "Copied 1 included file(s) from 'src'; 1 path(s) skipped."
        );
        let mut copied = String::new();
        run_with(temp.path(), |s| {
            copied = s.into();
            Ok(())
        })
        .unwrap();
        assert_eq!(copied, snapshot.text);
        assert!(
            run_with(temp.path(), |_| anyhow::bail!("clipboard unavailable"))
                .unwrap_err()
                .to_string()
                .contains("clipboard unavailable")
        );
    }

    #[test]
    fn source_payload_framing_preserves_empty_unicode_and_mixed_endings() {
        for content in ["", "Café ☁\r\nsecond line\n", "tab\there\u{1b}[31m"] {
            let temp = tempfile::tempdir().unwrap();
            fs::create_dir(temp.path().join("src")).unwrap();
            fs::write(temp.path().join("src/notes.txt"), content).unwrap();
            let mut copied = None;
            run_with(temp.path(), |text| {
                copied = Some(text.to_owned());
                Ok(())
            })
            .unwrap();
            assert_eq!(
                copied.as_deref(),
                Some(format!("// --- src/notes.txt --- \n{content}\n\n").as_str())
            );
        }
    }

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
