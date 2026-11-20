use super::*;

#[test]
fn update_preserves_comments_crlf_unrelated_aliases_and_forks() {
    let source = "# tools\r\n[tools]\r\nfmt = 'fork/editor@1.2.0' # unrelated\r\neditor = \"owner/editor@1.2.0\" # used in CI\r\n";
    assert_eq!(
        edit(
            source,
            "tools",
            "editor",
            "owner/editor@1.2.0",
            "owner/editor@2.0.0"
        )
        .unwrap(),
        source.replace("owner/editor@1.2.0", "owner/editor@2.0.0")
    );
    assert!(
        edit(
            source,
            "tools",
            "editor",
            "other/editor@1.2.0",
            "owner/editor@2.0.0"
        )
        .is_err()
    );
    assert_eq!(
        edit(
            source,
            "tools",
            "fmt",
            "fork/editor@1.2.0",
            "fork/editor@1.2.0"
        )
        .unwrap(),
        source
    );
}

#[test]
fn atomic_commit_preserves_changed_or_new_user_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("custom ç.toml");
    let missing = file_state(&path, false).unwrap();
    fs::write(&path, "user file").unwrap();
    assert!(replace(&path, b"new", &missing).is_err());
    let before = file_state(&path, false).unwrap();
    fs::write(&path, "edited").unwrap();
    assert!(replace(&path, b"new", &before).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"edited");
}

#[test]
fn linked_or_external_mounts_refuse_preparation() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("default.project.json"),
        r#"{"tree":{"Workspace":{"$path":"../outside"}}}"#,
    )
    .unwrap();
    assert!(packages::source_tree(dir.path()).is_err());
}

#[cfg(windows)]
#[test]
fn package_directory_move_never_replaces_a_new_destination() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("staged");
    let to = dir.path().join("Packages");
    fs::create_dir(&from).unwrap();
    fs::create_dir(&to).unwrap();
    fs::write(to.join("local.luau"), "local edit").unwrap();
    assert!(move_exclusive(&from, &to).is_err());
    assert!(from.is_dir());
    assert_eq!(fs::read(to.join("local.luau")).unwrap(), b"local edit");
}
