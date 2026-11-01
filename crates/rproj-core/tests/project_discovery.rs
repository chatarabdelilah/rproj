use rproj_core::project_discovery::{Source, discover};
use std::fs;

#[test]
fn skyline_in_root_and_at_launch_is_one_row_with_both_sources() {
    let root = tempfile::tempdir().unwrap();
    let skyline = root.path().join("Skyline");
    fs::create_dir(&skyline).unwrap();
    fs::write(skyline.join("default.project.json"), "{}").unwrap();
    let scan = discover(&Ok(root.path().to_owned()), Some(&skyline));
    assert!(scan.warnings.is_empty());
    assert_eq!(scan.entries.len(), 1);
    assert_eq!(scan.entries[0].source, Source::Both);
    assert_eq!(scan.entries[0].path, skyline.canonicalize().unwrap());
}

#[test]
fn duplicate_names_keep_distinct_paths_and_launch_survives_a_root_error() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("projects");
    let local = root.join("灯台 Relay");
    let outside = fixture.path().join("shared/灯台 Relay");
    for dir in [&local, &outside, &root.join("container/nested")] {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("rproj.toml"), "broken = [").unwrap();
    }
    let scan = discover(&Ok(root), Some(&outside));
    assert_eq!(scan.entries.len(), 2);
    assert_eq!(scan.entries[0].name, scan.entries[1].name);
    assert!(scan.entries[0].path < scan.entries[1].path);
    assert_eq!(scan.entries[0].source, Source::ProjectsFolder);
    assert_eq!(scan.entries[1].source, Source::LaunchFolder);
    let missing = fixture.path().join("missing");
    for unavailable in [
        Ok(missing.clone()),
        Err("Invalid machine configuration".into()),
    ] {
        let scan = discover(&unavailable, Some(&outside));
        assert_eq!(scan.entries.len(), 1);
        assert_eq!(scan.entries[0].source, Source::LaunchFolder);
        assert!(!scan.warnings.is_empty());
        assert!(!missing.exists());
    }
}

#[cfg(unix)]
#[test]
fn linked_children_are_skipped() {
    let root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    fs::write(target.path().join("rproj.toml"), "").unwrap();
    std::os::unix::fs::symlink(target.path(), root.path().join("Linked")).unwrap();
    let scan = discover(&Ok(root.path().into()), None);
    assert!(scan.entries.is_empty());
    assert!(
        scan.warnings
            .iter()
            .any(|warning| warning.contains("linked"))
    );
}
