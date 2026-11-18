//! Exact test invocations and captured context for the matching desktop/CLI.
use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

use crate::creation::{
    PackageWorkflow,
    graph::{JestBackend, TestRunner},
};

pub mod protocol;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    pub identity: Option<String>,
    pub digest: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestContext {
    pub project: PathBuf,
    pub name: String,
    pub runner: TestRunner,
    pub backend: JestBackend,
    pub workflow: PackageWorkflow,
    pub inputs: BTreeMap<String, Input>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invocation {
    pub program: String,
    pub arguments: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    ExecuteProjectTests,
    SynchronizeRokitTools,
    SynchronizeWallyDependencies,
    RefreshJestConfiguration,
}

impl TestContext {
    pub fn invocation(&self, arguments: &[String]) -> Result<Invocation> {
        validate_arguments(self.runner, arguments)?;
        let (program, arguments) = invocation(self.runner, arguments);
        Ok(Invocation {
            program: program.into(),
            arguments,
        })
    }

    pub fn effects(&self) -> Vec<Effect> {
        let mut effects = vec![Effect::ExecuteProjectTests];
        if self
            .inputs
            .get("rokit.toml")
            .is_some_and(|input| input.identity.is_some())
        {
            effects.push(Effect::SynchronizeRokitTools);
        }
        if self.workflow == PackageWorkflow::Wally {
            effects.push(Effect::SynchronizeWallyDependencies);
        }
        if self.runner == TestRunner::JestRoblox {
            effects.push(Effect::RefreshJestConfiguration);
        }
        effects
    }
}

pub fn invocation(runner: TestRunner, arguments: &[String]) -> (&'static str, Vec<String>) {
    let (program, prefix) = match runner {
        TestRunner::TestEz => ("lute", "test"),
        TestRunner::JestRoblox => ("jest-roblox-cli", "--passWithNoTests"),
    };
    (
        program,
        std::iter::once(prefix.into())
            .chain(arguments.iter().cloned())
            .collect(),
    )
}

pub fn uses_studio_cli(arguments: &[String], backend: JestBackend) -> bool {
    for (index, argument) in arguments.iter().enumerate() {
        if argument == "--backend" {
            return arguments
                .get(index + 1)
                .is_none_or(|backend| backend == "studio-cli");
        }
        if let Some(backend) = argument.strip_prefix("--backend=") {
            return backend == "studio-cli";
        }
    }
    backend == JestBackend::Studio
}

/// Bound JSON first, then conservatively account for Windows argument escaping.
/// Reserve conventional executable-path space; the OS still checks the resolved
/// executable at spawn time, and a spawn refusal is reported as a failure.
pub fn validate_arguments(runner: TestRunner, arguments: &[String]) -> Result<()> {
    ensure!(
        arguments.iter().all(|value| !value.contains('\0')),
        "NUL cannot be passed as a test argument"
    );
    ensure!(
        serde_json::to_vec(arguments)?.len() as u64 <= protocol::IPC_LIMIT,
        "Test arguments exceed the 2 MiB worker limit"
    );
    let (_, argv) = invocation(runner, arguments);
    let mut units = 261usize;
    for value in &argv {
        // Quoting every argument is conservative; each quote/backslash may be escaped.
        let length = value.encode_utf16().count();
        let escapes = value.chars().filter(|c| matches!(c, '\\' | '"')).count();
        units = units
            .saturating_add(length)
            .saturating_add(escapes)
            .saturating_add(3);
    }
    ensure!(
        units < 32_767,
        "Test arguments exceed the Windows command-line limit; shorten them before running"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_values_and_backend_precedence() {
        let args = vec![
            String::new(),
            " 保存 \"quoted\"\r\n\t".into(),
            "$HOME; echo unchanged".into(),
        ];
        validate_arguments(TestRunner::TestEz, &args).unwrap();
        assert_eq!(&invocation(TestRunner::TestEz, &args).1[1..], args);
        assert!(!uses_studio_cli(
            &["--backend=open-cloud".into(), "--backend=studio-cli".into()],
            JestBackend::Studio
        ));
        assert!(uses_studio_cli(
            &["--backend".into()],
            JestBackend::OpenCloud
        ));
    }

    #[test]
    fn invalid_or_unrepresentable_values_are_refused() {
        assert!(validate_arguments(TestRunner::TestEz, &["a\0b".into()]).is_err());
        assert!(validate_arguments(TestRunner::TestEz, &["x".repeat(32_767)]).is_err());
        assert!(validate_arguments(TestRunner::TestEz, &["\\".repeat(17_000)]).is_err());
        assert!(validate_arguments(TestRunner::TestEz, &["🙂".repeat(17_000)]).is_err());
        assert!(validate_arguments(TestRunner::TestEz, &["x".repeat(2 * 1024 * 1024)]).is_err());
    }
}
