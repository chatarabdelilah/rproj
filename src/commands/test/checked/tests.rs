use super::*;
use crate::config::{PackageWorkflow, project_file};
use tempfile::TempDir;

fn fixture() -> TempDir {
    let root = tempfile::Builder::new()
        .prefix("test 保存 space ")
        .tempdir()
        .unwrap();
    let mut graph = ProjectGraph {
        package_workflow: PackageWorkflow::None,
        ..Default::default()
    };
    graph.choose("test", Some("testez"));
    project_file::save_to(&graph, root.path()).unwrap();
    fs::write(
        root.path().join("default.project.json"),
        r#"{"name":"Fixture","tree":{}}"#,
    )
    .unwrap();
    root
}
fn isolated<T>(action: impl FnOnce() -> T) -> T {
    #[cfg(windows)]
    {
        let storage = TempDir::new().unwrap();
        crate::background_watch::with_test_root(storage.path(), action)
    }
    #[cfg(not(windows))]
    action()
}

#[test]
fn context_is_read_only_bounded_and_does_not_capture_test_source() {
    let root = fixture();
    let original = fs::read(root.path().join("rproj.toml")).unwrap();
    let first = prepare(root.path()).unwrap();
    assert_eq!(first.runner, TestRunner::TestEz);
    assert_eq!(first.workflow, PackageWorkflow::None);
    assert_eq!(first.inputs.len(), 5);
    assert!(first.inputs["wally.toml"].identity.is_none());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    assert_eq!(fs::read(root.path().join("rproj.toml")).unwrap(), original);
    fs::write(root.path().join("test.luau"), "changed test source").unwrap();
    assert_eq!(prepare(root.path()).unwrap(), first);
    fs::write(
        root.path().join("rokit.toml"),
        "x".repeat(2 * 1024 * 1024 + 1),
    )
    .unwrap();
    assert!(
        prepare(root.path())
            .unwrap_err()
            .to_string()
            .contains("2 MiB")
    );
}

#[test]
fn stale_or_missing_inputs_refuse_before_acceptance() {
    isolated(|| {
        for name in BASE {
            let root = fixture();
            let context = prepare(root.path()).unwrap();
            fs::write(root.path().join(name), "changed = true").unwrap();
            let mut accepted = false;
            let result = execute_with(
                &context,
                &[],
                &mut || {
                    accepted = true;
                    Ok(())
                },
                |_, _, _| panic!("must not run"),
            );
            assert!(result.is_err(), "{name}");
            assert!(!accepted);
        }
    });
}

#[test]
fn replaced_project_and_forged_context_are_rejected() {
    isolated(|| {
        let parent = TempDir::new().unwrap();
        let path = parent.path().join("project");
        fs::create_dir(&path).unwrap();
        let root = fixture();
        for name in ["rproj.toml", "default.project.json"] {
            fs::copy(root.path().join(name), path.join(name)).unwrap();
        }
        let context = prepare(&path).unwrap();
        fs::rename(&path, parent.path().join("previous")).unwrap();
        fs::create_dir(&path).unwrap();
        for name in ["rproj.toml", "default.project.json"] {
            fs::copy(root.path().join(name), path.join(name)).unwrap();
        }
        assert!(
            execute_with(
                &context,
                &[],
                &mut || panic!("must not accept"),
                |_, _, _| Ok(())
            )
            .is_err()
        );
        let mut forged = prepare(&path).unwrap();
        forged.runner = TestRunner::JestRoblox;
        assert!(
            execute_with(
                &forged,
                &[],
                &mut || panic!("must not accept"),
                |_, _, _| Ok(())
            )
            .is_err()
        );
    });
}

#[test]
fn checked_execution_reuses_graph_and_preserves_arguments() {
    isolated(|| {
        let root = fixture();
        let context = prepare(root.path()).unwrap();
        let args = vec!["quote\" space 保存\r\n".into(), String::new()];
        let mut accepted = false;
        execute_with(
            &context,
            &args,
            &mut || {
                accepted = true;
                Ok(())
            },
            |path, graph, received| {
                assert_eq!(path, context.project);
                assert_eq!(graph.test_runner(), Some(TestRunner::TestEz));
                assert_eq!(received, args);
                Ok(())
            },
        )
        .unwrap();
        assert!(accepted);
    });
}

#[test]
fn acceptance_failure_and_changes_during_acknowledgement_prevent_runner() {
    isolated(|| {
        let root = fixture();
        let context = prepare(root.path()).unwrap();
        assert!(
            execute_with(
                &context,
                &[],
                &mut || anyhow::bail!("report unavailable"),
                |_, _, _| panic!("must not run")
            )
            .is_err()
        );
        assert!(
            execute_with(
                &context,
                &[],
                &mut || {
                    fs::write(root.path().join("wally.toml"), "[package]")?;
                    Ok(())
                },
                |_, _, _| panic!("must not run")
            )
            .is_err()
        );
    });
}

#[test]
fn malformed_configuration_is_actionable() {
    let root = fixture();
    fs::write(root.path().join("rproj.toml"), "[").unwrap();
    assert!(
        prepare(root.path())
            .unwrap_err()
            .to_string()
            .contains("Malformed")
    );
}

#[test]
fn jest_context_captures_missing_outputs_and_rejects_later_targets() {
    isolated(|| {
        let root = fixture();
        let mut graph = ProjectGraph::default();
        graph.choose("test", Some("jest-roblox-open-cloud"));
        project_file::save_to(&graph, root.path()).unwrap();
        fs::write(
            root.path().join("wally.toml"),
            "[package]\nname='fixture/test'\nversion='0.1.0'\nrealm='shared'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("rokit.toml"),
            "[tools]\nrojo='rojo-rbx/rojo@7.5.1'\n",
        )
        .unwrap();
        let context = prepare(root.path()).unwrap();
        assert_eq!(context.runner, TestRunner::JestRoblox);
        assert_eq!(context.backend, crate::graph::JestBackend::OpenCloud);
        assert!(context.inputs["jest.config.json"].identity.is_none());
        assert!(context.inputs["jest.project.json"].identity.is_none());
        assert_eq!(
            context.effects(),
            vec![
                rproj_core::test_execution::Effect::ExecuteProjectTests,
                rproj_core::test_execution::Effect::SynchronizeRokitTools,
                rproj_core::test_execution::Effect::SynchronizeWallyDependencies,
                rproj_core::test_execution::Effect::RefreshJestConfiguration,
            ]
        );
        fs::write(root.path().join("jest.config.json"), "{}").unwrap();
        assert!(
            execute_with(
                &context,
                &[],
                &mut || panic!("must not accept"),
                |_, _, _| Ok(())
            )
            .is_err()
        );
    });
}

#[test]
fn positional_backend_filter_cannot_waive_checked_studio_preflight() {
    assert!(checked_studio_required(
        &["--".into(), "--backend=open-cloud".into()],
        crate::graph::JestBackend::Studio
    ));
    assert!(!checked_studio_required(
        &["--backend=open-cloud".into(), "--".into(), "filter".into()],
        crate::graph::JestBackend::Studio
    ));
}

#[test]
fn captured_record_digest_is_of_the_exact_bytes_used_for_parsing() {
    let root = fixture();
    let (captured, bytes) = capture_input(root.path(), "rproj.toml").unwrap();
    let bytes = bytes.unwrap();
    fs::write(root.path().join("rproj.toml"), "packages=['changed']").unwrap();
    let graph: ProjectGraph = toml::from_str(std::str::from_utf8(&bytes).unwrap()).unwrap();
    assert!(graph.packages.is_empty());
    assert_eq!(
        captured.digest,
        Some(blake3::hash(&bytes).to_hex().to_string())
    );
    assert_ne!(captured, input(root.path(), "rproj.toml").unwrap());
}

#[cfg(windows)]
#[test]
fn unresponsive_watch_refuses_test_before_acceptance() {
    let root = fixture();
    let context = prepare(root.path()).unwrap();
    crate::background_watch::with_unresponsive_fixture(root.path(), || {
        assert!(
            execute_with(
                &context,
                &[],
                &mut || panic!("must not accept"),
                |_, _, _| Ok(())
            )
            .is_err()
        )
    });
}
