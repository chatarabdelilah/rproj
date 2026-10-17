use super::*;
use crate::graph::ProjectGraph;

fn fixture() -> (tempfile::TempDir, ProjectEdit) {
    let root = tempfile::tempdir().unwrap();
    let graph = ProjectGraph {
        mode: "like:custom".into(),
        package_workflow: PackageWorkflow::None,
        ..Default::default()
    };
    let source = format!(
        "# exact original\n{}\n[metadata]\nowner = 'preserve'\n",
        toml::to_string(&graph).unwrap()
    );
    fs::write(root.path().join("rproj.toml"), &source).unwrap();
    fs::write(
        root.path().join("default.project.json"),
        serde_json::to_string(&rojo::builtin_project_template()).unwrap(),
    )
    .unwrap();
    let edit = ProjectEdit {
        source,
        original: graph.clone(),
        updated: graph,
        reviewed: None,
    };
    (root, edit)
}

#[test]
fn noop_keeps_exact_record_bytes_and_does_not_repair_files() {
    let (root, edit) = fixture();
    let plan = prepare(root.path(), &edit).unwrap();
    assert!(plan.files.is_empty());
    assert!(plan.external_operations.is_empty());
    execute(&plan, &mut || panic!("no mutation allowed")).unwrap();
    assert_eq!(
        fs::read_to_string(root.path().join("rproj.toml")).unwrap(),
        edit.source
    );
    assert!(!root.path().join(".gitignore").exists());
}

#[test]
fn preparation_is_read_only_and_plans_new_formatting_artifacts() {
    let (root, mut edit) = fixture();
    edit.updated.choose("format", Some("stylua"));
    let original = fs::read(root.path().join("default.project.json")).unwrap();
    let plan = prepare(root.path(), &edit).unwrap();
    for relative in ["stylua.toml", ".gitattributes", "rproj.toml"] {
        assert!(plan.files.iter().any(|f| f.relative == relative));
    }
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    assert_eq!(
        fs::read(root.path().join("default.project.json")).unwrap(),
        original
    );
    assert!(plan.tools_to_add.contains(&"stylua".into()));
}

#[test]
fn missing_excluded_prerequisite_blocks_addition_and_keeps_record() {
    let (root, mut edit) = fixture();
    edit.original.dropped.push("stylua.toml".into());
    edit.source = toml::to_string(&edit.original).unwrap();
    fs::write(root.path().join("rproj.toml"), &edit.source).unwrap();
    edit.updated = edit.original.clone();
    edit.updated.choose("format", None);
    assert!(
        prepare(root.path(), &edit)
            .unwrap_err()
            .to_string()
            .contains("excluded and missing")
    );
    fs::write(root.path().join("stylua.toml"), "# custom config\n").unwrap();
    let plan = prepare(root.path(), &edit).unwrap();
    assert!(!plan.files.iter().any(|f| f.relative == "stylua.toml"));
}

#[test]
fn stale_inputs_and_missing_target_races_are_refused_before_mutation() {
    for relative in ["rproj.toml", "default.project.json", "stylua.toml"] {
        let (root, mut edit) = fixture();
        edit.updated.choose("format", None);
        let plan = prepare(root.path(), &edit).unwrap();
        fs::write(root.path().join(relative), "externally changed").unwrap();
        assert!(execute(&plan, &mut || panic!("must refuse before writes")).is_err());
        assert!(!root.path().join(".gitattributes").exists());
    }
}

#[test]
fn additions_preserve_manual_wally_pins_and_custom_seed_files() {
    let (root, mut edit) = fixture();
    edit.original.package_workflow = PackageWorkflow::Wally;
    edit.original.packages.push("vide".into());
    edit.source = toml::to_string(&edit.original).unwrap();
    fs::write(root.path().join("rproj.toml"), &edit.source).unwrap();
    let wally =
        "# manual pin\n[dependencies]\nvide = 'owner/fork@1.0.0'\nmanual = 'owner/custom@2.0.0'\n";
    fs::write(root.path().join("wally.toml"), wally).unwrap();
    fs::write(root.path().join("stylua.toml"), "custom = true\n").unwrap();
    edit.updated = edit.original.clone();
    edit.updated.packages.push("promise".into());
    edit.updated.choose("format", None);
    let plan = prepare(root.path(), &edit).unwrap();
    let file = plan
        .files
        .iter()
        .find(|f| f.relative == "wally.toml")
        .unwrap();
    assert!(file.contents.contains("# manual pin"));
    assert!(file.contents.contains("owner/fork@1.0.0"));
    assert!(file.contents.contains("owner/custom@2.0.0"));
    assert!(!plan.files.iter().any(|f| f.relative == "stylua.toml"));
    assert_eq!(
        fs::read_to_string(root.path().join("wally.toml")).unwrap(),
        wally
    );
}

#[test]
fn preparation_rejects_unsafe_mount_changes() {
    let (root, mut edit) = fixture();
    edit.updated.choose("test", Some("testez"));
    let mut project = rojo::builtin_project_template();
    project["tree"]["ReplicatedStorage"]["test"] = serde_json::json!({"$path":"my/custom-tests"});
    fs::write(
        root.path().join("default.project.json"),
        serde_json::to_string(&project).unwrap(),
    )
    .unwrap();
    // None workflow TestEZ package must follow canonical capability derivation.
    edit.updated.packages = edit.updated.derived().packages;
    assert!(
        prepare(root.path(), &edit)
            .unwrap_err()
            .to_string()
            .contains("custom data")
    );
}

#[test]
fn excluded_housekeeping_is_never_regenerated() {
    let (root, mut edit) = fixture();
    edit.original.dropped = vec![".gitignore".into(), ".luaurc".into()];
    edit.source = toml::to_string(&edit.original).unwrap();
    fs::write(root.path().join("rproj.toml"), &edit.source).unwrap();
    edit.updated = edit.original.clone();
    edit.updated.choose("format", None);
    let plan = prepare(root.path(), &edit).unwrap();
    assert!(
        !plan
            .files
            .iter()
            .any(|f| [".gitignore", ".luaurc"].contains(&f.relative.as_str()))
    );
}

#[test]
fn required_step_failure_leaves_original_record_and_reports_partial_files() {
    let (root, mut edit) = fixture();
    edit.updated.choose("format", None);
    let plan = prepare(root.path(), &edit).unwrap();
    let error = execute_changes_with(&plan, |_| bail!("required tool failed")).unwrap_err();
    assert!(error.to_string().contains("required tool failed"));
    assert!(root.path().join("stylua.toml").exists());
    assert_eq!(
        fs::read_to_string(root.path().join("rproj.toml")).unwrap(),
        edit.source
    );
}

#[test]
fn successful_required_steps_commit_record_last_with_metadata() {
    let (root, mut edit) = fixture();
    edit.updated.choose("format", None);
    let plan = prepare(root.path(), &edit).unwrap();
    execute_changes_with(&plan, |_| {
        assert_eq!(
            fs::read_to_string(root.path().join("rproj.toml"))?,
            edit.source
        );
        assert!(root.path().join("stylua.toml").is_file());
        Ok(())
    })
    .unwrap();
    let record = fs::read_to_string(root.path().join("rproj.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&record).unwrap();
    assert_eq!(parsed["metadata"]["owner"].as_str(), Some("preserve"));
    assert_eq!(
        toml::from_str::<ProjectGraph>(&record).unwrap(),
        edit.updated
    );
}

#[test]
fn concurrent_record_edit_at_commit_is_preserved() {
    let (root, mut edit) = fixture();
    edit.updated.choose("format", None);
    let plan = prepare(root.path(), &edit).unwrap();
    let external = "# concurrent edit\n";
    assert!(
        execute_changes_with(&plan, |_| {
            fs::write(root.path().join("rproj.toml"), external)?;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(
        fs::read_to_string(root.path().join("rproj.toml")).unwrap(),
        external
    );
}

#[test]
fn both_asset_pipelines_plan_figma_first_and_keep_custom_configs() {
    for pipeline in ["asphalt", "tungsten"] {
        let (root, mut edit) = fixture();
        edit.updated.choose("asset-pipeline", Some(pipeline));
        let plan = prepare(root.path(), &edit).unwrap();
        let name = format!("{pipeline}.toml");
        let config = plan
            .files
            .iter()
            .find(|file| file.relative == name)
            .unwrap();
        assert!(config.contents.contains("figma/exports/**/*.png"));
        assert!(
            plan.files
                .iter()
                .any(|file| file.relative == "figma/exports/.gitkeep")
        );
        execute_changes_with(&plan, |_| Ok(())).unwrap();
        assert!(root.path().join("figma/README.md").is_file());
        assert!(root.path().join(&name).is_file());
    }
}

#[test]
fn new_tests_use_the_selected_runner_and_preserve_existing_specs() {
    for implementation in ["testez", "jest-roblox", "jest-roblox-open-cloud"] {
        let (root, mut edit) = fixture();
        if implementation != "testez" {
            edit.original.package_workflow = PackageWorkflow::Wally;
            edit.source = toml::to_string(&edit.original).unwrap();
            fs::write(root.path().join("rproj.toml"), &edit.source).unwrap();
            fs::write(root.path().join("wally.toml"), "[dependencies]\n").unwrap();
            edit.updated = edit.original.clone();
        }
        fs::create_dir_all(root.path().join("tests/shared")).unwrap();
        fs::write(
            root.path().join("tests/shared/hello.spec.luau"),
            "-- owned spec\n",
        )
        .unwrap();
        edit.updated.choose("test", Some(implementation));
        edit.updated.packages = edit.updated.derived().packages;
        let plan = prepare(root.path(), &edit).unwrap();
        assert!(
            !plan
                .files
                .iter()
                .any(|f| f.relative == "tests/shared/hello.spec.luau")
        );
        let spec = plan
            .files
            .iter()
            .find(|f| f.relative == "tests/client/hello.spec.luau")
            .unwrap();
        assert_eq!(
            spec.contents.contains("JestGlobals"),
            implementation != "testez"
        );
        assert_eq!(
            plan.files.iter().any(|f| f.relative == "jest.config.json"),
            implementation != "testez"
        );
        assert_eq!(
            fs::read_to_string(root.path().join("rproj.toml")).unwrap(),
            edit.source
        );
    }
}

#[test]
fn unsafe_configuration_merge_is_refused_before_writes() {
    let (root, mut edit) = fixture();
    fs::write(root.path().join("selene.toml"), "std = [\n").unwrap();
    edit.updated.choose("lint", None);
    assert!(prepare(root.path(), &edit).is_err());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 3);
    assert_eq!(
        fs::read_to_string(root.path().join("rproj.toml")).unwrap(),
        edit.source
    );
}

#[test]
fn absent_tool_manifest_is_never_created_when_excluded() {
    let (root, mut edit) = fixture();
    edit.original.dropped.push("rokit.toml".into());
    edit.source = toml::to_string(&edit.original).unwrap();
    fs::write(root.path().join("rproj.toml"), &edit.source).unwrap();
    edit.updated = edit.original.clone();
    edit.updated.choose("format", None);
    assert!(prepare(root.path(), &edit).is_err());
    assert!(!root.path().join("rokit.toml").exists());
}

#[test]
fn excluded_optional_examples_do_not_block_testing_or_reappear() {
    let (root, mut edit) = fixture();
    edit.original.dropped.push("test-examples".into());
    edit.source = toml::to_string(&edit.original).unwrap();
    fs::write(root.path().join("rproj.toml"), &edit.source).unwrap();
    edit.updated = edit.original.clone();
    edit.updated.choose("test", Some("testez"));
    edit.updated.packages = edit.updated.derived().packages;
    let plan = prepare(root.path(), &edit).unwrap();
    assert!(
        !plan
            .files
            .iter()
            .any(|f| f.relative.ends_with("hello.spec.luau"))
    );
    assert!(
        plan.files
            .iter()
            .any(|f| f.relative == "tests/shared/.gitkeep")
    );
}
