use super::*;

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("default.project.json"), "{}\n").unwrap();
    fs::write(
        root.path().join("rokit.toml"),
        "# custom\n[tools]\nasphalt='owner/fork@1'\ntungsten='owner/other@2'\n",
    )
    .unwrap();
    root
}

fn silent() -> Reporter {
    Reporter(std::sync::Arc::new(|_, _| {}))
}

#[test]
fn preparation_is_read_only_and_uses_exact_asset_generators() {
    for (tool, folder) in [("asphalt", "assets"), ("tungsten", "figma/exports")] {
        let root = fixture();
        if folder == "figma/exports" {
            fs::create_dir_all(root.path().join(folder)).unwrap();
        }
        let before = crate::test_common::project_snapshot(root.path());
        let plan = prepare(root.path(), tool).unwrap();
        assert_eq!(before, crate::test_common::project_snapshot(root.path()));
        assert!(plan.prerequisites.is_empty());
        let expected = if tool == "asphalt" {
            asphalt::render("assets/**/*.png")
        } else {
            tungsten::render("figma/exports/**/*.png")
        };
        assert!(plan.steps.contains(&Step::CreateFile {
            relative: format!("{tool}.toml"),
            contents: expected
        }));
        let mut marked = 0;
        let outcome = execute_with(
            &plan,
            &silent(),
            &mut || {
                marked += 1;
                Ok(())
            },
            |_| panic!("existing pins must not invoke tools"),
        )
        .unwrap();
        assert!(outcome.succeeded());
        assert!(marked > 0);
        assert!(root.path().join(format!("{folder}/.gitkeep")).is_file());
        assert_eq!(
            fs::read(root.path().join("rokit.toml")).unwrap(),
            before[Path::new("rokit.toml")].clone().unwrap()
        );
    }
}

#[test]
fn preserved_config_is_opaque_and_noop_is_byte_exact() {
    let root = fixture();
    fs::write(root.path().join("asphalt.toml"), b"custom malformed [\xff").unwrap();
    let before = crate::test_common::project_snapshot(root.path());
    let plan = prepare(root.path(), "asphalt").unwrap();
    assert!(plan.steps.iter().all(|s| !s.mutates()));
    assert!(
        execute(&plan, &silent(), &mut || panic!("no effects"))
            .unwrap()
            .succeeded()
    );
    assert_eq!(before, crate::test_common::project_snapshot(root.path()));
}

#[test]
fn rojo_comments_and_trailing_commas_are_preserved_without_parser_migration() {
    let root = fixture();
    let text = "{\"name\":\"Demo\", /* owner comment */ \"tree\":{},}\n";
    fs::write(root.path().join("default.project.json"), text).unwrap();
    let plan = prepare(root.path(), "asphalt").unwrap();
    assert!(
        execute(&plan, &silent(), &mut || Ok(()))
            .unwrap()
            .succeeded()
    );
    assert_eq!(
        fs::read_to_string(root.path().join("default.project.json")).unwrap(),
        text
    );
}

#[test]
fn source_changes_missing_targets_and_tampered_plans_are_refused() {
    for path in [
        "default.project.json",
        "rokit.toml",
        "asphalt.toml",
        "assets",
    ] {
        let root = fixture();
        let plan = prepare(root.path(), "asphalt").unwrap();
        if path == "assets" {
            fs::create_dir(root.path().join(path)).unwrap();
        } else {
            fs::write(root.path().join(path), "changed").unwrap();
        }
        assert!(execute(&plan, &silent(), &mut || panic!("before effects")).is_err());
    }
    let root = fixture();
    let mut plan = prepare(root.path(), "asphalt").unwrap();
    plan.steps.push(Step::CreateFile {
        relative: "../escape".into(),
        contents: "not allowed".into(),
    });
    assert!(execute(&plan, &silent(), &mut || panic!("before effects")).is_err());
}

#[test]
fn malformed_manifest_and_non_rojo_folders_fail_without_writes() {
    let root = fixture();
    for text in ["[", "tools=2", "[tools]\nasphalt=false"] {
        fs::write(root.path().join("rokit.toml"), text).unwrap();
        assert!(prepare(root.path(), "asphalt").is_err());
        assert!(!root.path().join("asphalt.toml").exists());
    }
    fs::remove_file(root.path().join("default.project.json")).unwrap();
    assert!(prepare(root.path(), "blender").is_err());
}

#[test]
fn blender_existing_scene_needs_no_executable_and_machine_entries_only_guide() {
    let root = fixture();
    fs::create_dir(root.path().join("blender")).unwrap();
    fs::write(root.path().join("blender/scene.blend"), b"existing scene").unwrap();
    let plan = prepare(root.path(), "blender").unwrap();
    assert!(plan.prerequisites.is_empty());
    assert!(
        execute(&plan, &silent(), &mut || panic!("no effects"))
            .unwrap()
            .succeeded()
    );
    let guide = prepare(root.path(), "luau-lsp").unwrap();
    assert!(
        guide
            .steps
            .iter()
            .all(|s| matches!(s, Step::Guidance { .. }))
    );
}

#[test]
fn races_after_mutation_notice_are_checked_before_staging() {
    let root = fixture();
    let plan = prepare(root.path(), "asphalt").unwrap();
    assert!(
        execute(&plan, &silent(), &mut || {
            fs::write(root.path().join("asphalt.toml"), "other writer")?;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(
        fs::read_to_string(root.path().join("asphalt.toml")).unwrap(),
        "other writer"
    );
    assert!(!root.path().join("assets").exists());
}

#[test]
fn replaced_project_identity_and_oversized_configuration_are_refused() {
    let parent = tempfile::tempdir().unwrap();
    let project = parent.path().join("project");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("default.project.json"), "{}").unwrap();
    let plan = prepare(&project, "luau-lsp").unwrap();
    fs::rename(&project, parent.path().join("old-project")).unwrap();
    fs::create_dir(&project).unwrap();
    fs::write(project.join("default.project.json"), "{}").unwrap();
    assert!(execute(&plan, &silent(), &mut || panic!("no writes")).is_err());
    fs::write(
        project.join("default.project.json"),
        vec![b' '; LIMIT as usize + 1],
    )
    .unwrap();
    assert!(
        prepare(&project, "luau-lsp")
            .unwrap_err()
            .to_string()
            .contains("2 MiB")
    );
}

#[cfg(windows)]
#[test]
fn watch_on_the_captured_project_blocks_execution_but_not_review() {
    let root = fixture();
    crate::background_watch::with_unresponsive_fixture(root.path(), || {
        let plan = prepare(root.path(), "asphalt").unwrap();
        assert!(
            execute(&plan, &silent(), &mut || panic!("Watch must block effects"))
                .unwrap_err()
                .to_string()
                .contains("Watch")
        );
        let other = fixture();
        let plan = prepare(other.path(), "asphalt").unwrap();
        assert!(
            execute(&plan, &silent(), &mut || Ok(()))
                .unwrap()
                .succeeded()
        );
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(
    windows,
    ignore = "requires Windows symlink privilege or Developer Mode"
)]
#[test]
fn linked_asset_parents_are_refused_without_traversal() {
    let root = fixture();
    let external = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(external.path(), root.path().join("figma")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(external.path(), root.path().join("figma")).unwrap();
    assert!(prepare(root.path(), "asphalt").is_err());
    assert_eq!(fs::read_dir(external.path()).unwrap().count(), 0);
}
