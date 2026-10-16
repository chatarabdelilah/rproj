use super::{
    PackageWorkflow, ProjectGraph, add_companions, apply_derived_packages, capability_owned,
    graph::Node, offerable_package, validate_name,
};
use crate::catalog::{artifacts, capabilities, wally_packages};
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationDraft {
    pub name: String,
    pub graph: ProjectGraph,
    pub save_setup: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationEnvironment {
    pub projects_root: PathBuf,
    pub apps: Vec<String>,
    pub extensions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationFile {
    pub key: String,
    pub reason: String,
    pub required: bool,
    pub included: bool,
}

/// A pure composition review. Filesystem/source identity is checked by the CLI
/// before this can become an executable request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationPlan {
    pub draft: CreationDraft,
    pub destination: PathBuf,
    pub files: Vec<CreationFile>,
    pub tools: Vec<String>,
    pub packages: Vec<String>,
}

impl CreationDraft {
    pub fn new(name: String) -> Self {
        Self {
            name,
            graph: ProjectGraph {
                mode: "guided".into(),
                ..Default::default()
            },
            save_setup: None,
        }
    }

    pub fn set_workflow(&mut self, workflow: PackageWorkflow) {
        if self.graph.package_workflow != workflow {
            self.graph.invalidate(Node::Strategy);
            self.graph.package_workflow = workflow;
            apply_derived_packages(&mut self.graph);
        }
    }

    pub fn toggle_package(&mut self, key: &str, guided: bool) -> Result<()> {
        let package =
            wally_packages::find(key).ok_or_else(|| anyhow::anyhow!("Unknown package: {key}"))?;
        ensure!(
            offerable_package(package, self.graph.package_workflow),
            "This package is controlled by its capability or requires Wally"
        );
        ensure!(
            !guided || package.primary_choice,
            "Guided companions follow their primary choices"
        );
        let selected = self.graph.packages.iter().any(|k| k == key);
        if selected {
            self.graph.packages.retain(|k| k != key);
        } else {
            if guided && !package.category.allows_multiple() {
                self.graph.packages.retain(|k| {
                    wally_packages::find(k).is_none_or(|p| p.category != package.category)
                });
            }
            self.graph.packages.push(key.into());
        }
        if guided {
            let mut choices: BTreeSet<String> = self
                .graph
                .packages
                .iter()
                .filter(|k| {
                    wally_packages::find(k).is_some_and(|p| p.primary_choice)
                        && !capability_owned(k)
                })
                .cloned()
                .collect();
            add_companions(&mut choices);
            self.graph.packages = choices.into_iter().collect();
        }
        apply_derived_packages(&mut self.graph);
        self.graph.invalidate(Node::Packages);
        Ok(())
    }

    pub fn set_capability(&mut self, key: &str, implementation: Option<&str>) -> Result<()> {
        let capability =
            capabilities::find(key).ok_or_else(|| anyhow::anyhow!("Unknown capability: {key}"))?;
        if let Some(value) = implementation {
            let implementation = capability
                .implementation(value)
                .ok_or_else(|| anyhow::anyhow!("Unknown implementation: {value}"))?;
            ensure!(
                implementation
                    .workflows
                    .contains(&self.graph.package_workflow),
                "{value} requires a compatible dependency workflow"
            );
            ensure!(
                capability
                    .requires
                    .iter()
                    .all(|k| self.graph.capabilities.contains_key(*k)),
                "{key} has missing capability requirements"
            );
            self.graph.capabilities.insert(key.into(), value.into());
        } else {
            self.graph.capabilities.remove(key);
            loop {
                let before = self.graph.capabilities.len();
                let selected = self.graph.capabilities.clone();
                self.graph.capabilities.retain(|key, _| {
                    capabilities::find(key)
                        .is_none_or(|c| c.requires.iter().all(|k| selected.contains_key(*k)))
                });
                if self.graph.capabilities.len() == before {
                    break;
                }
            }
        }
        apply_derived_packages(&mut self.graph);
        self.graph.invalidate(Node::Capabilities);
        Ok(())
    }

    pub fn toggle_file(&mut self, key: &str, environment: &CreationEnvironment) -> Result<()> {
        ensure!(
            self.graph
                .full_plan(&environment.apps, &environment.extensions)
                .iter()
                .any(|p| p.key == key),
            "This file is not part of the current composition"
        );
        ensure!(
            artifacts::find(key).is_some_and(artifacts::Artifact::droppable),
            "This file is required by the current composition"
        );
        if self.graph.dropped.iter().any(|k| k == key) {
            self.graph.dropped.retain(|k| k != key);
        } else {
            self.graph.dropped.push(key.into());
        }
        Ok(())
    }

    /// Strict native validation is separate from the terminal's legacy loading
    /// adapter: unfamiliar choices must remain visible instead of being removed.
    pub fn validate(&self) -> Result<()> {
        validate_name(&self.name).map_err(anyhow::Error::msg)?;
        if let Some(name) = &self.save_setup {
            validate_name(name).map_err(anyhow::Error::msg)?;
        }
        for key in &self.graph.packages {
            ensure!(
                wally_packages::find(key).is_some(),
                "Unsupported package: {key}"
            );
            ensure!(
                self.graph.package_workflow == PackageWorkflow::Wally || capability_owned(key),
                "Package choices require Wally"
            );
        }
        for (key, value) in &self.graph.capabilities {
            let Some(capability) = capabilities::find(key) else {
                bail!("Unsupported capability: {key}");
            };
            let Some(implementation) = capability.implementation(value) else {
                bail!("Unsupported implementation for {key}: {value}");
            };
            ensure!(
                implementation
                    .workflows
                    .contains(&self.graph.package_workflow),
                "{value} requires a compatible dependency workflow; choose Wally, compatible testing, or no testing explicitly"
            );
            ensure!(
                capability
                    .requires
                    .iter()
                    .all(|k| self.graph.capabilities.contains_key(*k)),
                "{key} has missing capability requirements"
            );
        }
        for key in &self.graph.dropped {
            ensure!(
                artifacts::find(key).is_some_and(artifacts::Artifact::droppable),
                "Unsupported or required excluded file: {key}"
            );
        }
        Ok(())
    }

    pub fn plan(&self, environment: &CreationEnvironment) -> Result<CreationPlan> {
        self.validate()?;
        let mut draft = self.clone();
        apply_derived_packages(&mut draft.graph);
        let files = draft
            .graph
            .full_plan(&environment.apps, &environment.extensions)
            .into_iter()
            .map(|file| {
                let required = !artifacts::find(file.key)
                    .expect("canonical artifact")
                    .droppable();
                CreationFile {
                    key: file.key.into(),
                    reason: file.reason.describe(),
                    required,
                    included: required || !draft.graph.dropped.iter().any(|k| k == file.key),
                }
            })
            .collect();
        Ok(CreationPlan {
            destination: environment.projects_root.join(&draft.name),
            tools: draft.graph.tools(),
            packages: draft.graph.packages.clone(),
            draft,
            files,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn environment() -> CreationEnvironment {
        CreationEnvironment {
            projects_root: PathBuf::from("fixture"),
            apps: vec![],
            extensions: vec![],
        }
    }

    #[test]
    fn changing_guided_framework_removes_only_stale_companions() {
        let mut draft = CreationDraft::new("Example".into());
        for key in ["react", "charm", "ripple", "janitor"] {
            draft.toggle_package(key, true).unwrap();
        }
        assert!(draft.graph.packages.iter().any(|p| p == "reactCharm"));
        draft.toggle_package("vide", true).unwrap();
        for key in ["videCharm", "videRipple", "charmSync", "janitor"] {
            assert!(draft.graph.packages.iter().any(|p| p == key), "{key}");
        }
        for key in ["react", "reactRoblox", "reactCharm", "reactRipple"] {
            assert!(!draft.graph.packages.iter().any(|p| p == key), "{key}");
        }
    }

    #[test]
    fn workflow_revision_keeps_testing_visible_until_an_explicit_repair() {
        let mut draft = CreationDraft::new("Example".into());
        draft
            .set_capability("test", Some("jest-roblox-open-cloud"))
            .unwrap();
        draft.set_capability("lint", Some("selene")).unwrap();
        draft.toggle_file("rproj.toml", &environment()).unwrap();
        draft.set_workflow(PackageWorkflow::None);
        assert!(draft.graph.dropped.is_empty());
        assert_eq!(draft.graph.capabilities["test"], "jest-roblox-open-cloud");
        assert!(draft.validate().is_err());
        draft.set_capability("test", Some("testez")).unwrap();
        assert!(draft.plan(&environment()).is_ok());
        assert_eq!(draft.graph.capabilities["lint"], "selene");
    }

    #[test]
    fn workflow_round_trip_keeps_derived_packages_and_file_choices_coherent() {
        let mut draft = CreationDraft::new("Example".into());
        draft.set_capability("test", Some("testez")).unwrap();
        draft.set_workflow(PackageWorkflow::None);
        draft.set_workflow(PackageWorkflow::Wally);
        assert_eq!(draft.graph.packages, ["testez"]);
        let plan = draft.plan(&environment()).unwrap();
        assert_eq!(draft.graph.tools(), plan.tools);
        draft.toggle_file("rokit.toml", &environment()).unwrap();
        assert!(
            draft
                .plan(&environment())
                .unwrap()
                .files
                .iter()
                .any(|p| p.key == "rokit.toml" && !p.included)
        );
    }

    #[test]
    fn saved_graph_unknown_choices_are_not_normalized_and_required_files_cannot_drop() {
        let mut draft = CreationDraft::new("Example".into());
        draft.graph.packages.push("future-package".into());
        let original = draft.clone();
        assert!(draft.plan(&environment()).is_err());
        assert_eq!(draft, original);
        draft.graph.packages.clear();
        draft.set_capability("test", Some("jest-roblox")).unwrap();
        for key in ["src", "tests", "jest.project.json", "jest.config.json"] {
            assert!(draft.toggle_file(key, &environment()).is_err());
        }
        draft.toggle_file("rproj.toml", &environment()).unwrap();
        let plan = draft.plan(&environment()).unwrap();
        assert!(
            plan.files
                .iter()
                .any(|p| p.key == "rproj.toml" && !p.included)
        );
        assert!(plan.files.iter().filter(|p| p.required).all(|p| p.included));
    }

    #[test]
    fn dependency_changes_invalidate_optional_files_and_required_capabilities() {
        let mut draft = CreationDraft::new("Example".into());
        assert!(draft.set_capability("ci", Some("github-actions")).is_err());
        draft.set_capability("gate", Some("lute")).unwrap();
        draft.set_capability("ci", Some("github-actions")).unwrap();
        draft.set_capability("gate", None).unwrap();
        assert!(!draft.graph.capabilities.contains_key("ci"));
        draft.toggle_file("rproj.toml", &environment()).unwrap();
        draft.toggle_package("react", false).unwrap();
        assert!(draft.graph.dropped.is_empty());
        assert!(draft.toggle_package("testez", false).is_err());
    }
}
