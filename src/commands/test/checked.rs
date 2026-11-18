//! Capture only configuration inputs, never test/source contents or installations.
use crate::{
    graph::{ProjectGraph, TestRunner},
    path_identity::identity,
};
use anyhow::{Context, Result, ensure};
use rproj_core::test_execution::{Input, TestContext, protocol::IPC_LIMIT};
use std::{collections::BTreeMap, fs, io::Read, path::Path};

const BASE: &[&str] = &[
    "rproj.toml",
    "default.project.json",
    "rokit.toml",
    "wally.toml",
];

fn input(project: &Path, name: &str) -> Result<Input> {
    capture_input(project, name).map(|(input, _)| input)
}

fn capture_input(project: &Path, name: &str) -> Result<(Input, Option<Vec<u8>>)> {
    let path = project.join(name);
    let original = identity(&path, name.is_empty())?;
    let bytes = if original.is_some() && !name.is_empty() {
        Some(read(&path)?)
    } else {
        None
    };
    ensure!(
        identity(&path, name.is_empty())? == original,
        "Configuration changed while reading; Refresh test context"
    );
    let digest = bytes
        .as_ref()
        .map(|bytes| blake3::hash(bytes).to_hex().to_string());
    Ok((
        Input {
            identity: original,
            digest,
        },
        bytes,
    ))
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(IPC_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= IPC_LIMIT,
        "{} exceeds the 2 MiB test context limit",
        path.display()
    );
    Ok(bytes)
}

fn capture(project: &Path) -> Result<(TestContext, ProjectGraph)> {
    ensure!(
        identity(project, true)?.is_some(),
        "Project folder is unavailable"
    );
    let project = project.canonicalize()?;
    let mut inputs = BTreeMap::new();
    inputs.insert(String::new(), input(&project, "")?);
    let mut record = None;
    for name in BASE {
        let (input, bytes) = capture_input(&project, name)?;
        inputs.insert((*name).into(), input);
        if *name == "rproj.toml" {
            record = bytes;
        }
    }
    // Parse exactly the bytes whose digest is reviewed, not a second file read.
    let record =
        String::from_utf8(record.context("Missing rproj.toml; testing needs a project record")?)
            .context("Project record is not UTF-8")?;
    let graph: ProjectGraph = toml::from_str(&record).map_err(|error: toml::de::Error| {
        anyhow::anyhow!(
            "Malformed rproj.toml at byte range {:?}; repair it and Refresh context",
            error.span()
        )
    })?;
    let runner = graph
        .test_runner()
        .context("Testing is not configured for this project")?;
    ensure!(
        graph.testing_is_compatible(),
        "Jest Roblox requires the Wally workflow"
    );
    if runner == TestRunner::JestRoblox {
        for name in [
            crate::steps::jest::CONFIG_FILE,
            crate::steps::jest::PROJECT_FILE,
        ] {
            inputs.insert(name.into(), input(&project, name)?);
        }
    }
    let snapshot = rproj_core::inspect(&project)?;
    ensure!(
        snapshot.actions.test.available,
        "{}",
        snapshot
            .actions
            .test
            .reason
            .unwrap_or_else(|| "Testing is unavailable".into())
    );
    let context = TestContext {
        name: snapshot.name.unwrap_or_else(|| {
            project
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        }),
        project,
        runner,
        backend: graph.jest_backend(),
        workflow: graph.package_workflow,
        inputs,
    };
    verify(&context)?;
    Ok((context, graph))
}

pub(crate) fn prepare(project: &Path) -> Result<TestContext> {
    capture(project).map(|(context, _)| context)
}

fn verify(context: &TestContext) -> Result<()> {
    for (name, expected) in &context.inputs {
        ensure!(
            input(&context.project, name)? == *expected,
            "{} changed; explicitly Refresh test context",
            if name.is_empty() {
                "Project identity"
            } else {
                name
            }
        );
    }
    Ok(())
}

pub(crate) fn execute(
    context: &TestContext,
    arguments: &[String],
    before_execution: &mut impl FnMut() -> Result<()>,
) -> Result<()> {
    execute_with(context, arguments, before_execution, super::run_project)
}

fn execute_with(
    context: &TestContext,
    arguments: &[String],
    before_execution: &mut impl FnMut() -> Result<()>,
    run: impl FnOnce(&Path, &ProjectGraph, &[String]) -> Result<()>,
) -> Result<()> {
    context.invocation(arguments)?;
    let _guard = crate::background_watch::mutation_guard(Some(&context.project), true)?;
    let (actual, graph) = capture(&context.project)?;
    ensure!(
        actual == *context,
        "Project configuration changed; explicitly Refresh test context"
    );
    if graph.test_runner() == Some(TestRunner::JestRoblox)
        && checked_studio_required(arguments, graph.jest_backend())
    {
        super::ensure_jest_runtime()?;
    }
    crate::interrupt::check()?;
    before_execution()?;
    verify(&actual)?;
    // Use the checked graph, not a fresh runner selection after acceptance.
    run(&context.project, &graph, arguments)
}

fn checked_studio_required(arguments: &[String], backend: crate::graph::JestBackend) -> bool {
    // Positional filters after -- cannot waive the custom worker's preflight.
    // Keep the legacy terminal parser unchanged in this compatibility extraction.
    let end = arguments
        .iter()
        .position(|value| value == "--")
        .unwrap_or(arguments.len());
    super::uses_studio_cli(&arguments[..end], backend)
}

#[cfg(test)]
mod tests;
