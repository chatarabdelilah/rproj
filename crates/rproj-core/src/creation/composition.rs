//! Additions-only editing. The original intent is immutable, including exclusions.
use super::{CreationDraft, ProjectGraph};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionFile {
    pub relative: String,
    pub contents: String,
    pub creating: bool,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionInput {
    pub relative: String,
    pub fingerprint: Option<u64>,
}

/// Private workbench payload and shared terminal execution plan, not project config.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionPlan {
    pub project: PathBuf,
    pub draft: CompositionDraft,
    pub inputs: Vec<CompositionInput>,
    pub files: Vec<CompositionFile>,
    pub tools_to_add: Vec<String>,
    pub external_operations: Vec<String>,
    pub create_blender: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionDraft {
    pub original: ProjectGraph,
    pub graph: ProjectGraph,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionAdditions {
    pub packages: Vec<String>,
    pub capabilities: Vec<(String, String)>,
    pub tools: Vec<String>,
}

impl CompositionDraft {
    pub fn new(original: ProjectGraph) -> Self {
        Self {
            graph: original.clone(),
            original,
        }
    }

    pub fn validate(&self) -> Result<()> {
        // Existing project names need not satisfy New Project's Windows name rules.
        for graph in [&self.original, &self.graph] {
            CreationDraft {
                name: "existing-project".into(),
                graph: graph.clone(),
                save_setup: None,
            }
            .validate()?;
        }
        ensure!(
            self.original.mode == self.graph.mode,
            "Existing composition provenance is locked"
        );
        ensure!(
            self.original.package_workflow == self.graph.package_workflow,
            "Existing dependency workflow is locked"
        );
        ensure!(
            self.original.dropped == self.graph.dropped,
            "Existing file exclusions are locked"
        );
        ensure!(
            self.original
                .packages
                .iter()
                .all(|k| self.graph.packages.contains(k)),
            "Existing packages are locked"
        );
        ensure!(
            self.original
                .capabilities
                .iter()
                .all(|(k, v)| self.graph.capabilities.get(k) == Some(v)),
            "Existing capabilities and implementations are locked"
        );
        let derived = self.graph.derived().packages;
        ensure!(
            derived.iter().all(|k| self.graph.packages.contains(k)),
            "Capability-derived packages are missing"
        );
        Ok(())
    }

    fn revise(&mut self, change: impl FnOnce(&mut CreationDraft) -> Result<()>) -> Result<()> {
        self.validate()?;
        let mut draft = CreationDraft {
            name: "existing-project".into(),
            graph: self.graph.clone(),
            save_setup: None,
        };
        change(&mut draft)?;
        draft.graph.mode = self.original.mode.clone();
        draft.graph.dropped = self.original.dropped.clone();
        // New Project derivation can replace capability-owned packages. Editing
        // must preserve older explicit declarations as well as their order.
        let additions = draft.graph.packages.clone();
        draft.graph.packages = self.original.packages.clone();
        for key in additions {
            if !draft.graph.packages.contains(&key) {
                draft.graph.packages.push(key);
            }
        }
        let revised = Self {
            original: self.original.clone(),
            graph: draft.graph,
        };
        revised.validate()?;
        *self = revised;
        Ok(())
    }

    pub fn toggle_package(&mut self, key: &str) -> Result<()> {
        ensure!(
            !self.original.packages.iter().any(|k| k == key),
            "Existing packages are locked"
        );
        self.revise(|draft| draft.toggle_package(key, false))
    }

    pub fn set_capability(&mut self, key: &str, implementation: Option<&str>) -> Result<()> {
        ensure!(
            !self.original.capabilities.contains_key(key),
            "Existing capabilities and implementations are locked"
        );
        self.revise(|draft| draft.set_capability(key, implementation))
    }

    pub fn additions(&self) -> Result<CompositionAdditions> {
        self.validate()?;
        let original_tools = self.original.tools();
        Ok(CompositionAdditions {
            packages: self
                .graph
                .packages
                .iter()
                .filter(|k| !self.original.packages.contains(k))
                .cloned()
                .collect(),
            capabilities: self
                .graph
                .capabilities
                .iter()
                .filter(|(k, _)| !self.original.capabilities.contains_key(*k))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            tools: self
                .graph
                .tools()
                .into_iter()
                .filter(|k| !original_tools.contains(k))
                .collect(),
        })
    }

    /// Patch only editor-owned values, keeping unrelated TOML metadata. An
    /// unchanged composition keeps its exact source bytes, including comments.
    pub fn record(&self, source: &str) -> Result<String> {
        self.validate()?;
        let parsed: ProjectGraph = toml::from_str(source)?;
        ensure!(
            parsed == self.original,
            "rproj.toml no longer matches the opened composition"
        );
        if self.graph == self.original {
            return Ok(source.into());
        }
        let mut record: toml::Value = toml::from_str(source)?;
        let graph = toml::Value::try_from(&self.graph)?;
        for key in ["packages", "capabilities"] {
            record[key] = graph[key].clone();
        }
        Ok(toml::to_string_pretty(&record)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn original() -> ProjectGraph {
        let mut graph = ProjectGraph {
            mode: "like:custom".into(),
            packages: vec!["vide".into()],
            dropped: vec![".gitignore".into()],
            ..Default::default()
        };
        graph.choose("lint", Some("selene"));
        graph
    }

    #[test]
    fn additions_preserve_original_intent_and_can_be_undone() {
        let mut draft = CompositionDraft::new(original());
        draft.toggle_package("promise").unwrap();
        draft.set_capability("format", Some("stylua")).unwrap();
        assert_eq!(draft.graph.dropped, draft.original.dropped);
        assert_eq!(draft.graph.mode, "like:custom");
        assert_eq!(draft.additions().unwrap().packages, ["promise"]);
        draft.toggle_package("promise").unwrap();
        draft.set_capability("format", None).unwrap();
        assert_eq!(draft.graph, draft.original);
    }

    #[test]
    fn original_choices_and_implementations_cannot_be_changed() {
        let mut draft = CompositionDraft::new(original());
        assert!(draft.toggle_package("vide").is_err());
        assert!(draft.set_capability("lint", None).is_err());
        draft.graph.dropped.clear();
        assert!(draft.validate().is_err());
    }

    #[test]
    fn unsupported_choices_are_preserved_and_block_validation() {
        let mut graph = original();
        graph
            .capabilities
            .insert("future".into(), "unfamiliar".into());
        let draft = CompositionDraft::new(graph.clone());
        assert!(
            draft
                .validate()
                .unwrap_err()
                .to_string()
                .contains("Unsupported")
        );
        assert_eq!(draft.graph, graph);
    }

    #[test]
    fn record_patch_keeps_metadata_and_noop_bytes() {
        let mut draft = CompositionDraft::new(original());
        let source = format!(
            "# retained when unchanged\n{}\n[metadata]\nowner = 'custom'\n",
            toml::to_string(&draft.original).unwrap()
        );
        assert_eq!(draft.record(&source).unwrap(), source);
        draft.toggle_package("promise").unwrap();
        let patched: toml::Value = toml::from_str(&draft.record(&source).unwrap()).unwrap();
        assert_eq!(patched["metadata"]["owner"].as_str(), Some("custom"));
        assert_eq!(
            toml::from_str::<ProjectGraph>(&draft.record(&source).unwrap()).unwrap(),
            draft.graph
        );
    }

    #[test]
    fn capability_derivation_and_none_compatibility_are_shared() {
        let mut draft = CompositionDraft::new(ProjectGraph::default());
        draft
            .set_capability("test", Some("jest-roblox-open-cloud"))
            .unwrap();
        assert!(draft.graph.packages.iter().any(|k| k == "jest"));
        let mut none = CompositionDraft::new(ProjectGraph {
            package_workflow: super::super::PackageWorkflow::None,
            ..Default::default()
        });
        assert!(none.toggle_package("promise").is_err());
        assert!(
            none.set_capability("test", Some("jest-roblox-open-cloud"))
                .is_err()
        );
        none.set_capability("format", Some("stylua")).unwrap();
    }
}
