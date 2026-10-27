//! CLI worker entrypoint for the desktop's shared operation service.

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};
use rproj_core::operations::WORKER_FLAG;

mod composition;
mod creation;
mod saved_setups;
mod settings;
mod setup;
mod upgrade;

pub(crate) fn internal_entry() -> Option<Result<()>> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(OsStr::new(WORKER_FLAG)) {
        return None;
    }
    Some(parse_operation(args).and_then(dispatch))
}

fn dispatch(operation: Operation) -> Result<()> {
    match operation {
        Operation::Catalog => {
            println!("{}", serde_json::to_string(&catalog_entries()?)?);
            Ok(())
        }
        Operation::Status => {
            println!(
                "{}",
                serde_json::to_string(&crate::background_watch::structured_status()?)?
            );
            Ok(())
        }
        Operation::Logs => crate::background_watch::show_logs(),
        Operation::Stop => crate::background_watch::stop_and_wait(),
        Operation::Start => {
            let project = std::env::current_dir().context("could not read project directory")?;
            crate::background_watch::start_in(&project)
        }
        Operation::Check => {
            let project = std::env::current_dir().context("could not read project directory")?;
            run_check(&project)
        }
        Operation::Test => crate::commands::test::run(&[]),
        Operation::CreationContext => {
            let text = serde_json::to_string(&creation::context()?)?;
            ensure!(
                text.len() as u64 <= rproj_core::creation::protocol::IPC_LIMIT,
                "Creation context exceeded its size limit; use the terminal workflow or reduce the template/setup data"
            );
            println!("{text}");
            Ok(())
        }
        Operation::CreationJob(directory) => creation::job(&directory),
        Operation::CompositionContext => {
            let context = composition::context(&std::env::current_dir()?)?;
            let text = serde_json::to_string(&context)?;
            ensure!(
                text.len() as u64 <= rproj_core::creation::composition_protocol::IPC_LIMIT,
                "Composition context exceeded its size limit"
            );
            println!("{text}");
            Ok(())
        }
        Operation::CompositionJob(directory) => composition::job(&directory),
        Operation::SavedSetupsSupport => {
            println!(
                "{}",
                rproj_core::saved_setups::protocol::SAVED_SETUPS_PROTOCOL
            );
            Ok(())
        }
        Operation::SavedSetupsJob(directory) => saved_setups::job(&directory),
        Operation::SettingsSupport => {
            println!(
                "{}",
                rproj_core::tool_configuration::protocol::SETTINGS_PROTOCOL
            );
            Ok(())
        }
        Operation::SettingsJob(directory) => settings::job(&directory),
        Operation::SetupSupport => {
            println!("{}", rproj_core::machine_setup::protocol::SETUP_PROTOCOL);
            Ok(())
        }
        Operation::SetupJob(directory) => setup::job(&directory),
        Operation::UpgradeSupport => {
            println!("{}", rproj_core::upgrade::protocol::UPGRADE_PROTOCOL);
            Ok(())
        }
        Operation::UpgradeContext => {
            println!(
                "{}",
                serde_json::to_string(&upgrade::context(&std::env::current_dir()?)?)?
            );
            Ok(())
        }
        Operation::UpgradeJob(directory) => upgrade::job(&directory),
    }
}

#[cfg(all(test, windows))]
#[path = "workbench/worker_tests.rs"]
mod worker_tests;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Operation {
    Catalog,
    Status,
    Logs,
    Start,
    Stop,
    Check,
    Test,
    CreationContext,
    CreationJob(std::path::PathBuf),
    CompositionContext,
    CompositionJob(std::path::PathBuf),
    SavedSetupsSupport,
    SavedSetupsJob(std::path::PathBuf),
    SettingsSupport,
    SettingsJob(std::path::PathBuf),
    SetupSupport,
    SetupJob(std::path::PathBuf),
    UpgradeSupport,
    UpgradeContext,
    UpgradeJob(std::path::PathBuf),
}

fn parse_operation(mut args: impl Iterator<Item = OsString>) -> Result<Operation> {
    let verb = args.next().context("missing workbench operation")?;
    if verb == "creation-job"
        || verb == "composition-job"
        || verb == "upgrade-job"
        || verb == "saved-setups-job"
        || verb == "settings-job"
        || verb == "setup-job"
    {
        let directory = args.next().context("missing creation IPC directory")?;
        ensure!(args.next().is_none(), "unexpected workbench argument");
        return Ok(if verb == "creation-job" {
            Operation::CreationJob(directory.into())
        } else if verb == "composition-job" {
            Operation::CompositionJob(directory.into())
        } else if verb == "saved-setups-job" {
            Operation::SavedSetupsJob(directory.into())
        } else if verb == "settings-job" {
            Operation::SettingsJob(directory.into())
        } else if verb == "setup-job" {
            Operation::SetupJob(directory.into())
        } else {
            Operation::UpgradeJob(directory.into())
        });
    }
    ensure!(args.next().is_none(), "unexpected workbench argument");
    match verb.to_str() {
        Some("creation-context") => Ok(Operation::CreationContext),
        Some("composition-context") => Ok(Operation::CompositionContext),
        Some("saved-setups-support") => Ok(Operation::SavedSetupsSupport),
        Some("settings-support") => Ok(Operation::SettingsSupport),
        Some("setup-support") => Ok(Operation::SetupSupport),
        Some("upgrade-support") => Ok(Operation::UpgradeSupport),
        Some("upgrade-context") => Ok(Operation::UpgradeContext),
        Some("catalog") => Ok(Operation::Catalog),
        Some("status") => Ok(Operation::Status),
        Some("logs") => Ok(Operation::Logs),
        Some("start") => Ok(Operation::Start),
        Some("stop") => Ok(Operation::Stop),
        Some("check") => Ok(Operation::Check),
        Some("test") => Ok(Operation::Test),
        _ => bail!("unknown workbench operation"),
    }
}

fn catalog_entries() -> Result<Vec<rproj_core::catalog::CatalogEntry>> {
    use crate::catalog::{package_usage, wally_packages};
    wally_packages::PACKAGES
        .iter()
        .map(|package| {
            let guide = package_usage::find(package.key)
                .with_context(|| format!("missing catalog guidance for {}", package.key))?;
            let (identity, _) = package
                .source
                .split_once('@')
                .with_context(|| format!("invalid catalog package identity for {}", package.key))?;
            Ok(rproj_core::catalog::CatalogEntry {
                key: package.key.into(),
                title: package.module_name.into(),
                description: package.description.into(),
                docs_url: package.docs_url.into(),
                package: identity.into(),
                example: package_usage::example(&guide),
                when_useful: guide.when.into(),
                context: guide.context.into(),
                caveats: guide.caveats.into(),
            })
        })
        .collect()
}

fn run_check(project: &Path) -> Result<()> {
    ensure!(
        project.join(".lute/check.luau").is_file(),
        "No .lute/check.luau quality script in this project"
    );
    let _guard = crate::background_watch::mutation_guard(Some(project), true)?;
    crate::interrupt::check()?;
    let status = Command::new("lute")
        .args(["run", "check"])
        .current_dir(project)
        .status()
        .context(
            "could not run the quality check; ensure Lute is installed and available on PATH",
        )?;
    crate::diagnostics::tool_exit("lute", status);
    crate::interrupt::check()?;
    ensure!(status.success(), "lute run check failed ({status})");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_accepts_exact_operations_and_refuses_extra_arguments() {
        for (verb, expected) in [
            ("catalog", Operation::Catalog),
            ("status", Operation::Status),
            ("logs", Operation::Logs),
            ("start", Operation::Start),
            ("stop", Operation::Stop),
            ("check", Operation::Check),
            ("test", Operation::Test),
            ("settings-support", Operation::SettingsSupport),
            ("setup-support", Operation::SetupSupport),
        ] {
            assert_eq!(
                parse_operation([OsString::from(verb)].into_iter()).unwrap(),
                expected
            );
            assert!(
                parse_operation([OsString::from(verb), OsString::from("extra")].into_iter())
                    .is_err()
            );
        }
        assert!(parse_operation(std::iter::empty()).is_err());
        assert!(parse_operation([OsString::from("unknown")].into_iter()).is_err());
    }

    #[test]
    fn missing_quality_script_does_not_create_project_files() {
        let root = tempfile::tempdir().unwrap();
        let error = run_check(root.path()).unwrap_err();
        assert!(error.to_string().contains("No .lute/check.luau"));
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn catalog_guidance_matches_canonical_packages_and_preserves_real_examples() {
        let entries = catalog_entries().unwrap();
        assert_eq!(
            entries.len(),
            crate::catalog::wally_packages::PACKAGES.len()
        );
        let renderer = entries
            .iter()
            .find(|entry| entry.package == "jsdotlua/react-roblox")
            .unwrap();
        assert_eq!(renderer.key, "reactRoblox");
        assert_eq!(renderer.title, "ReactRoblox");
        assert!(renderer.docs_url.starts_with("https://"));
        let profile = entries
            .iter()
            .find(|entry| entry.package == "lm-loleris/profilestore")
            .unwrap();
        assert!(
            profile
                .example
                .contains("ServerScriptService\").serverPackages.profilestore")
        );
        let guide = crate::catalog::package_usage::find("profilestore").unwrap();
        assert_eq!(profile.caveats, guide.caveats);
        assert_eq!(profile.context, guide.context);
        assert!(!profile.when_useful.is_empty());
        let serialized = serde_json::to_string(&entries).unwrap();
        let decoded: Vec<rproj_core::catalog::CatalogEntry> =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(decoded, entries);
        assert!(
            entries
                .iter()
                .all(|entry| entry.package != "unknown/third-party")
        );
    }
}
