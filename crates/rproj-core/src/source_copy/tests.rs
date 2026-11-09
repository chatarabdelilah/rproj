use super::*;

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    dir
}

#[test]
fn exact_payload_and_ranges_preserve_text() {
    let dir = fixture();
    for (name, content) in [
        ("empty.luau", ""),
        ("雪.txt", "Café ☁\r\nsecond\n"),
        ("control", "\0\t\u{1b}[31m\u{202e}"),
    ] {
        fs::write(dir.path().join("src").join(name), content).unwrap();
    }
    let snap = collect(dir.path()).unwrap();
    assert_eq!(snap.files.len(), 3);
    assert!(snap.skipped.is_empty());
    let mut end = 0;
    for file in &snap.files {
        let content = fs::read_to_string(dir.path().join(&file.path)).unwrap();
        assert_eq!(
            &snap.text[file.range.clone()],
            format!("// --- {} --- \n{content}\n\n", file.path)
        );
        assert_eq!(file.range.start, end);
        end = file.range.end;
    }
    assert_eq!(end, snap.text.len());
    assert_eq!(snap.project, dir.path().canonicalize().unwrap());
}

#[test]
fn missing_empty_partial_and_all_skipped_are_distinct() {
    let dir = tempfile::tempdir().unwrap();
    assert!(collect(dir.path()).unwrap().source_missing);
    fs::create_dir(dir.path().join("src")).unwrap();
    let empty = collect(dir.path()).unwrap();
    assert!(!empty.source_missing && empty.files.is_empty() && empty.skipped.is_empty());
    fs::write(dir.path().join("src/binary"), [255, 254]).unwrap();
    let skipped = collect(dir.path()).unwrap();
    assert!(skipped.files.is_empty());
    assert_eq!(skipped.skipped[0].reason, "Not UTF-8 text");
    fs::write(dir.path().join("src/notes.txt"), "notes").unwrap();
    let partial = collect(dir.path()).unwrap();
    assert_eq!((partial.files.len(), partial.skipped.len()), (1, 1));
    assert!(!partial.text.contains("binary"));
}

#[test]
fn file_source_root_is_refused_without_changes() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("src"), "not a directory").unwrap();
    assert!(
        collect(dir.path())
            .unwrap_err()
            .to_string()
            .contains("not a directory")
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("src")).unwrap(),
        "not a directory"
    );
}

#[test]
fn byte_limit_includes_headers_separators_and_growing_reads() {
    let dir = fixture();
    let framing = "// --- src/a --- \n\n\n".len();
    fs::write(dir.path().join("src/a"), "123").unwrap();
    assert_eq!(
        collect_limited(dir.path(), framing + 3, 1, &mut || false)
            .unwrap()
            .text
            .len(),
        framing + 3
    );
    assert!(collect_limited(dir.path(), framing + 2, 1, &mut || false).is_err());
    assert!(collect_limited(dir.path(), framing - 1, 1, &mut || false).is_err());
    assert_eq!(
        read_bounded(io::Cursor::new(b"123"), 3, &mut || false)
            .unwrap()
            .unwrap(),
        b"123"
    );
    assert!(read_bounded(io::Cursor::new(b"1234"), 3, &mut || false).is_err());
    assert_eq!(MAX_PAYLOAD_BYTES, 8 * 1024 * 1024);
}

#[test]
fn production_payload_boundary_is_exact_and_not_truncated() {
    let dir = fixture();
    let path = dir.path().join("src/a");
    let framing = "// --- src/a --- \n\n\n".len();
    fs::write(&path, vec![b'a'; MAX_PAYLOAD_BYTES - framing]).unwrap();
    assert_eq!(collect(dir.path()).unwrap().text.len(), MAX_PAYLOAD_BYTES);
    use std::io::Write;
    fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(b"x")
        .unwrap();
    assert!(collect(dir.path()).is_err());
}

#[test]
fn many_empty_files_and_deep_directories_are_bounded() {
    let dir = fixture();
    let mut nested = dir.path().join("src");
    for _ in 0..32 {
        nested.push("d");
        fs::create_dir(&nested).unwrap();
    }
    for n in 0..64 {
        fs::write(nested.join(n.to_string()), "").unwrap();
    }
    let snapshot = collect_limited(dir.path(), 32_768, 96, &mut || false).unwrap();
    assert_eq!(snapshot.files.len(), 64);
    assert!(collect_limited(dir.path(), 32_768, 95, &mut || false).is_err());
}

#[test]
fn entries_include_directories_and_empty_files_but_not_source_root() {
    let dir = fixture();
    fs::create_dir(dir.path().join("src/nested")).unwrap();
    fs::write(dir.path().join("src/nested/empty"), "").unwrap();
    assert!(collect_limited(dir.path(), 1024, 2, &mut || false).is_ok());
    assert!(collect_limited(dir.path(), 1024, 1, &mut || false).is_err());
    assert_eq!(MAX_VISITED_ENTRIES, 10_000);
}

#[test]
fn abandonment_is_checked_between_read_chunks() {
    let mut calls = 0;
    assert!(
        read_bounded(io::Cursor::new(vec![0; 32_768]), 32_768, &mut || {
            calls += 1;
            calls > 2
        })
        .is_err()
    );
    assert_eq!(calls, 3);
    let dir = fixture();
    assert!(
        collect_with_abandonment(dir.path(), || true)
            .unwrap_err()
            .to_string()
            .contains("abandoned")
    );
}

#[test]
fn read_errors_return_skips_instead_of_source_bytes() {
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        }
    }
    let error = read_bounded(Broken, 10, &mut || false)
        .unwrap()
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn replacements_and_during_read_changes_are_detected() {
    let dir = fixture();
    let source = dir.path().join("src").canonicalize().unwrap();
    let path = source.join("a");
    fs::write(&path, "before").unwrap();
    let before = fs::metadata(&path).unwrap();
    let mut changed = false;
    let result = read_file(&path, &source, &before, 1024, &mut || {
        if !changed {
            fs::write(&path, "changed length").unwrap();
            changed = true;
        }
        false
    })
    .unwrap();
    assert!(result.unwrap_err().contains("changed"));
    let captured = Directory::capture(&dir.path().join("src")).unwrap();
    fs::rename(dir.path().join("src"), dir.path().join("old")).unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    assert!(captured.verify().is_err());
}

#[cfg(windows)]
fn link_directory(link: &Path, target: &Path) {
    let output = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link.to_string_lossy().replace('/', "\\"))
        .arg(target.to_string_lossy().replace('/', "\\"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "junction prerequisite failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[cfg(unix)]
fn link_directory(link: &Path, target: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}

#[cfg(any(windows, unix))]
#[test]
fn linked_roots_refuse_and_linked_descendants_skip() {
    let dir = fixture();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), "outside source").unwrap();
    let link = dir.path().join("src/external");
    link_directory(&link, outside.path());
    let snap = collect(dir.path()).unwrap();
    assert!(snap.text.is_empty());
    assert_eq!(snap.skipped.len(), 1);
    assert_eq!(snap.skipped[0].reason, "Linked path skipped");
    let root_link = dir.path().join("alias");
    link_directory(&root_link, outside.path());
    assert!(collect(&root_link).is_err());
    let linked_source_project = dir.path().join("linked-source");
    fs::create_dir(&linked_source_project).unwrap();
    link_directory(&linked_source_project.join("src"), outside.path());
    assert!(collect(&linked_source_project).is_err());
}

#[cfg(any(windows, unix))]
#[test]
fn linked_ancestor_outside_project_is_canonicalized_before_collection() {
    let parent = tempfile::tempdir().unwrap();
    let real = parent.path().join("real");
    fs::create_dir_all(real.join("project/src")).unwrap();
    fs::write(real.join("project/src/a"), "inside").unwrap();
    let alias = parent.path().join("alias");
    link_directory(&alias, &real);
    let snapshot = collect(&alias.join("project")).unwrap();
    assert_eq!(
        snapshot.project,
        real.join("project").canonicalize().unwrap()
    );
    assert_eq!(snapshot.text, "// --- src/a --- \ninside\n\n");
}
