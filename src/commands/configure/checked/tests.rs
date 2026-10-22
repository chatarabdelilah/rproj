use super::*;
use serde_json::json;

fn changed(context: &SettingsContext, key: &str, value: Value) -> SettingsDraft {
    let mut draft = SettingsDraft::new(values(context).unwrap());
    let index = context
        .target()
        .unwrap()
        .settings
        .iter()
        .position(|s| s.key == key)
        .unwrap();
    draft.set(index, value);
    draft
}

#[test]
fn field_diagnostics_identify_legacy_conflicts_without_changing_captured_bytes() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join(".vscode")).unwrap();
    let original = r#"{"luau-lsp.plugin.enabled":true,"luau-lsp.studioPlugin.enabled":true,"luau-lsp.types.roblox":true,"luau-lsp.platform.type":"roblox","editor.rulers":[90]}"#;
    fs::write(root.path().join(".vscode/settings.json"), original).unwrap();
    let context = capture(root.path(), "luau-lsp").unwrap();
    let blocked = field_problems(&context).unwrap();
    let keys: Vec<_> = blocked
        .keys()
        .map(|i| context.target().unwrap().settings[*i].key)
        .collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&"luau-lsp.plugin.enabled"));
    assert!(keys.contains(&"luau-lsp.types.roblox"));
    assert!(
        blocked
            .values()
            .all(|reason| reason.contains("replacement"))
    );
    assert_eq!(
        fs::read_to_string(root.path().join(".vscode/settings.json")).unwrap(),
        original
    );
    let other = capture(root.path(), "stylua-vscode").unwrap();
    assert!(field_problems(&other).unwrap().is_empty());
}

#[test]
fn preparation_is_read_only_for_every_target_without_setup_or_record() {
    for (tool, key, value) in [
        ("stylua", "column_width", json!(120)),
        ("selene", "std", json!("roblox")),
        ("luau-lsp", "luau-lsp.platform.type", json!("roblox")),
        ("stylua-vscode", "editor.formatOnSave", json!(true)),
    ] {
        let root = tempfile::Builder::new()
            .prefix("settings 東京 ")
            .tempdir()
            .unwrap();
        let context = capture(root.path(), tool).unwrap();
        let draft = changed(&context, key, value);
        let review = prepare(&context, &draft).unwrap();
        assert!(!review.diff().unwrap().is_empty());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        let mut started = false;
        let saved = execute(&review, &mut || {
            started = true;
            Ok(())
        })
        .unwrap();
        assert!(started);
        assert_eq!(saved.original, review.contents);
        assert!(!root.path().join("rproj.toml").exists());
    }
}

#[test]
fn ineffective_legacy_changes_are_refused_but_unrelated_edits_can_save() {
    for (old, replacement, value) in [
        (
            "luau-lsp.types.roblox",
            "luau-lsp.platform.type",
            json!("roblox"),
        ),
        (
            "luau-lsp.plugin.enabled",
            "luau-lsp.studioPlugin.enabled",
            json!(true),
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".vscode")).unwrap();
        let path = root.path().join(".vscode/settings.json");
        let original = json!({old:false,replacement:value,"custom":[1,2]}).to_string();
        fs::write(&path, &original).unwrap();
        let context = capture(root.path(), "luau-lsp").unwrap();
        let draft = changed(&context, old, json!(true));
        assert!(
            prepare(&context, &draft)
                .unwrap_err()
                .to_string()
                .contains("Revert this change")
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        let draft = changed(&context, "luau-lsp.sourcemap.enabled", json!(true));
        let saved = execute(&prepare(&context, &draft).unwrap(), &mut || Ok(())).unwrap();
        let actual: Value = serde_json::from_str(saved.original.as_ref().unwrap()).unwrap();
        assert!(actual.get(old).is_none());
        assert_eq!(actual["custom"], json!([1, 2]));
        let index = context
            .target()
            .unwrap()
            .settings
            .iter()
            .position(|s| s.key == old)
            .unwrap();
        assert_eq!(values(&saved).unwrap()[index], None);
    }
}

#[test]
fn stale_missing_targets_and_parents_refuse_before_mutation() {
    for create_file in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let context = capture(root.path(), "stylua-vscode").unwrap();
        let review = prepare(
            &context,
            &changed(&context, "editor.formatOnSave", json!(true)),
        )
        .unwrap();
        fs::create_dir(root.path().join(".vscode")).unwrap();
        if create_file {
            fs::write(root.path().join(".vscode/settings.json"), "{}").unwrap();
        }
        let mut wrote = false;
        assert!(
            execute(&review, &mut || {
                wrote = true;
                Ok(())
            })
            .is_err()
        );
        assert!(!wrote);
    }
}

#[test]
fn forged_output_and_baseline_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let context = capture(root.path(), "stylua").unwrap();
    let draft = changed(&context, "column_width", json!(120));
    let review = prepare(&context, &draft).unwrap();
    let mut forged = review.clone();
    forged.contents = Some("column_width = 9\n".into());
    assert!(execute(&forged, &mut || panic!("must not write")).is_err());
    forged = review;
    forged.draft.current[0] = Some(json!("invented"));
    assert!(execute(&forged, &mut || panic!("must not write")).is_err());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn change_during_mutation_callback_is_not_overwritten() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("stylua.toml");
    fs::write(&path, "column_width=80\n").unwrap();
    let context = capture(root.path(), "stylua").unwrap();
    let review = prepare(&context, &changed(&context, "column_width", json!(120))).unwrap();
    assert!(
        execute(&review, &mut || {
            fs::write(&path, "column_width=99\n")?;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(fs::read_to_string(path).unwrap(), "column_width=99\n");
}

#[test]
fn malformed_inputs_and_unsafe_merges_never_write() {
    for (tool, text, key, value) in [
        ("stylua", "bad = [", "column_width", json!(120)),
        (
            "stylua-vscode",
            "{// comment\n}",
            "editor.formatOnSave",
            json!(true),
        ),
        (
            "stylua-vscode",
            "{\"custom\":1,}",
            "editor.formatOnSave",
            json!(true),
        ),
        (
            "selene",
            "std = 'roblox'\nnotes = '''\nstd = 'test'\n'''\n",
            "std",
            json!("roblox+testez"),
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let tool_spec = tool_settings::find(tool).unwrap();
        let path = root.path().join(tool_spec.target.relative());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        let context = capture(root.path(), tool).unwrap();
        if values(&context).is_ok() {
            assert!(prepare(&context, &changed(&context, key, value)).is_err());
        }
        assert_eq!(fs::read_to_string(path).unwrap(), text);
    }
}

#[test]
fn callback_failure_occurs_before_staging_or_parent_creation() {
    let root = tempfile::tempdir().unwrap();
    let context = capture(root.path(), "stylua-vscode").unwrap();
    let review = prepare(
        &context,
        &changed(&context, "editor.formatOnSave", json!(true)),
    )
    .unwrap();
    assert!(execute(&review, &mut || anyhow::bail!("report write failed")).is_err());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn replacement_with_identical_bytes_still_invalidates_review() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("stylua.toml");
    fs::write(&path, "column_width=80\n").unwrap();
    let context = capture(root.path(), "stylua").unwrap();
    let review = prepare(&context, &changed(&context, "column_width", json!(120))).unwrap();
    fs::rename(&path, root.path().join("old.toml")).unwrap();
    fs::write(&path, "column_width=80\n").unwrap();
    assert!(execute(&review, &mut || panic!("must not write")).is_err());
}

#[test]
fn replaced_project_directory_invalidates_review() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    fs::create_dir(&project).unwrap();
    let context = capture(&project, "stylua").unwrap();
    let review = prepare(&context, &changed(&context, "column_width", json!(120))).unwrap();
    fs::rename(&project, root.path().join("old")).unwrap();
    fs::create_dir(&project).unwrap();
    assert!(execute(&review, &mut || panic!("must not write")).is_err());
    assert_eq!(fs::read_dir(&project).unwrap().count(), 0);
}

#[cfg(windows)]
#[test]
fn junction_parent_and_project_alias_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let junction = root.path().join(".vscode");
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&junction)
        .arg(external.path())
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert!(capture(root.path(), "stylua-vscode").is_err());
    assert!(capture(&junction, "stylua").is_err());
    assert_eq!(fs::read_dir(external.path()).unwrap().count(), 0);
    fs::create_dir(external.path().join("regular-project")).unwrap();
    assert!(capture(&junction.join("regular-project"), "stylua").is_ok());
    fs::remove_dir(junction).unwrap();
}

#[cfg(unix)]
#[test]
fn symbolic_file_and_parent_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(external.path(), root.path().join(".vscode")).unwrap();
    assert!(capture(root.path(), "stylua-vscode").is_err());
    let target = external.path().join("stylua.toml");
    fs::write(&target, "column_width=80\n").unwrap();
    std::os::unix::fs::symlink(&target, root.path().join("stylua.toml")).unwrap();
    assert!(capture(root.path(), "stylua").is_err());
    fs::create_dir(external.path().join("regular-project")).unwrap();
    assert!(capture(&root.path().join(".vscode/regular-project"), "stylua").is_ok());
}
