use super::*;

#[test]
fn composition_golden_serialization_preserves_existing_file_format() {
    const RECORD: &str = "mode = \"like:starter\"\npackage_workflow = \"wally\"\npackages = [\n    \"react\",\n    \"reactRoblox\",\n]\ndropped = [\"rproj.toml\"]\n\n[capabilities]\nformat = \"stylua\"\ntest = \"jest-roblox-open-cloud\"\n";
    let graph: ProjectGraph = toml::from_str(RECORD).unwrap();
    assert_eq!(toml::to_string_pretty(&graph).unwrap(), RECORD);
    assert_eq!(graph.jest_backend(), JestBackend::OpenCloud);
    assert_eq!(graph.test_runner(), Some(TestRunner::JestRoblox));
    let legacy: ProjectGraph = toml::from_str("packages = ['testez']\n").unwrap();
    assert_eq!(legacy.package_workflow, PackageWorkflow::Wally);
    assert_eq!(legacy.test_runner(), Some(TestRunner::TestEz));
}

#[test]
fn composition_golden_empty_workflows_plan_only_their_requested_files() {
    for (workflow, expected) in [
        (
            PackageWorkflow::None,
            vec!["src", "default.project.json", "rproj.toml", ".gitignore"],
        ),
        (
            PackageWorkflow::Wally,
            vec![
                "src",
                "default.project.json",
                "rproj.toml",
                ".gitignore",
                "wally.toml",
            ],
        ),
    ] {
        let graph = ProjectGraph {
            package_workflow: workflow,
            ..Default::default()
        };
        assert_eq!(
            graph
                .plan(&[], &[])
                .iter()
                .map(|p| p.key)
                .collect::<Vec<_>>(),
            expected
        );
        assert!(graph.tools().is_empty());
    }
}

#[test]
fn composition_golden_runner_files_exclusions_and_machine_gates() {
    let mut graph = ProjectGraph {
        mode: "expert".into(),
        packages: vec!["react".into(), "reactRoblox".into()],
        ..Default::default()
    };
    for (capability, implementation) in [
        ("lint", "selene"),
        ("format", "stylua"),
        ("test", "jest-roblox-open-cloud"),
        ("assets-3d", "blender"),
    ] {
        graph.choose(capability, Some(implementation));
    }
    graph.dropped = vec![
        "rproj.toml".into(),
        "tests".into(),
        "jest.config.json".into(),
    ];
    let plan = graph.plan(&[], &[]);
    assert_eq!(
        plan.iter().map(|p| p.key).collect::<Vec<_>>(),
        [
            "src",
            "default.project.json",
            "rokit.toml",
            ".gitignore",
            ".gitattributes",
            "wally.toml",
            "selene.toml",
            "stylua.toml",
            "tests",
            "test-examples",
            "jest.project.json",
            "jest.config.json",
            "blender"
        ]
    );
    assert!(
        graph
            .full_plan(&[], &[])
            .iter()
            .any(|p| p.key == "rproj.toml")
    );
    assert_eq!(
        graph.tools(),
        [
            "stylua",
            "selene",
            "jest-roblox",
            "wally",
            "wally-package-types"
        ]
    );
}
