use super::*;

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("default.project.json"), "{}\n").unwrap();
    fs::write(
        root.path().join("rproj.toml"),
        "mode='expert'\npackage_workflow='none'\n",
    )
    .unwrap();
    root
}

#[test]
fn housekeeping_baseline_is_read_only_and_byte_preserving_after_execution() {
    let root = fixture();
    fs::write(root.path().join(".gitignore"), "# custom\r\ncustom/").unwrap();
    let before = fs::read(root.path().join("rproj.toml")).unwrap();
    let prepared = prepare(root.path()).unwrap();
    assert_eq!(
        prepared
            .plan
            .rewrites
            .iter()
            .map(|r| r.relative.as_str())
            .collect::<Vec<_>>(),
        [".gitignore", ".luaurc"]
    );
    assert!(!root.path().join(".luaurc").exists());
    assert_eq!(
        fs::read(root.path().join(".gitignore")).unwrap(),
        b"# custom\r\ncustom/"
    );
    prepared.execute().unwrap();
    assert_eq!(fs::read(root.path().join("rproj.toml")).unwrap(), before);
    assert!(
        fs::read_to_string(root.path().join(".gitignore"))
            .unwrap()
            .starts_with("# custom\r\ncustom/\n")
    );
    assert_eq!(
        fs::read_to_string(root.path().join(".luaurc")).unwrap(),
        "{\n  \"languageMode\": \"strict\"\n}\n"
    );
    assert!(prepare(root.path()).unwrap().plan.rewrites.is_empty());
}

#[test]
fn skipped_only_does_not_mean_up_to_date() {
    let root = fixture();
    prepare(root.path()).unwrap().execute().unwrap();
    fs::write(root.path().join(".luaurc"), "// custom JSON comments\n{}").unwrap();
    let prepared = prepare(root.path()).unwrap();
    assert!(prepared.plan.rewrites.is_empty());
    assert_eq!(prepared.plan.skipped.len(), 1);
    assert!(prepared.plan.skipped[0].contains(".luaurc"));
    prepared.execute().unwrap();
    assert_eq!(
        fs::read_to_string(root.path().join(".luaurc")).unwrap(),
        "// custom JSON comments\n{}"
    );
}

#[test]
fn stale_missing_targets_and_tampered_plans_are_refused_before_mutation() {
    let root = fixture();
    let reviewed = prepare(root.path()).unwrap().review();
    let mut tampered = reviewed.clone();
    tampered.files[0].contents = "arbitrary".into();
    let mut claimed = false;
    assert!(
        execute_review(&tampered, &mut || {
            claimed = true;
            Ok(())
        })
        .is_err()
    );
    assert!(!claimed);
    fs::write(root.path().join(".luaurc"), "{\"custom\":true}").unwrap();
    assert!(
        execute_review(&reviewed, &mut || {
            claimed = true;
            Ok(())
        })
        .is_err()
    );
    assert!(!claimed);
    assert!(!root.path().join(".gitignore").exists());
}

#[test]
fn mutation_boundary_precedes_staging_and_failure_replaces_no_targets() {
    let root = fixture();
    fs::write(
        root.path().join("rproj.toml"),
        "mode='expert'\npackage_workflow='none'\n[capabilities]\neditor='vscode'\n",
    )
    .unwrap();
    let reviewed = prepare(root.path()).unwrap().review();
    assert!(!root.path().join(".vscode").exists());
    assert!(execute_review(&reviewed, &mut || anyhow::bail!("report unavailable")).is_err());
    assert!(!root.path().join(".vscode").exists());
    let error = execute_review(&reviewed, &mut || {
        fs::write(root.path().join(".vscode"), "obstruction")?;
        Ok(())
    })
    .unwrap_err();
    assert!(format!("{error:#}").contains("prepare"));
    assert!(!root.path().join(".gitignore").exists());
    assert!(!root.path().join(".luaurc").exists());
}

#[test]
fn changed_sources_during_staging_leave_targets_unreplaced() {
    let root = fixture();
    let reviewed = prepare(root.path()).unwrap().review();
    let error = execute_review(&reviewed, &mut || {
        fs::write(root.path().join("rproj.toml"), "mode='guided'\n")?;
        Ok(())
    })
    .unwrap_err();
    assert!(format!("{error:#}").contains("changed"));
    assert!(!root.path().join(".gitignore").exists());
    assert!(!root.path().join(".luaurc").exists());
}

#[cfg(windows)]
#[test]
fn linked_parent_is_refused_during_read_only_preparation() {
    let root = fixture();
    let external = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("rproj.toml"),
        "mode='expert'\npackage_workflow='none'\n[capabilities]\neditor='vscode'\n",
    )
    .unwrap();
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(root.path().join(".vscode"))
        .arg(external.path())
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let error = prepare(root.path()).err().unwrap();
    assert!(format!("{error:#}").contains("linked"));
    assert_eq!(fs::read_dir(external.path()).unwrap().count(), 0);
}

#[test]
fn failure_after_a_replacement_reports_partial_changes_and_preserves_record() {
    let root = fixture();
    let prepared = prepare(root.path()).unwrap();
    let mut saved = 0;
    let error = prepared
        .plan
        .apply_with(
            &prepared.project_dir,
            &prepared.identity,
            &mut || Ok(()),
            &mut |pending, path| {
                if saved == 1 {
                    anyhow::bail!("fixture replacement failure");
                }
                pending.persist(path).map_err(|e| e.error)?;
                saved += 1;
                Ok(())
            },
        )
        .unwrap_err();
    assert!(format!("{error:#}").contains("1 earlier upgrade files were updated"));
    assert!(root.path().join(".gitignore").exists());
    assert!(!root.path().join(".luaurc").exists());
    assert_eq!(
        fs::read_to_string(root.path().join("rproj.toml")).unwrap(),
        "mode='expert'\npackage_workflow='none'\n"
    );
    assert_eq!(prepare(root.path()).unwrap().plan.rewrites.len(), 1);
}
