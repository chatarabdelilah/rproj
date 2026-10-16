//! Shared project composition; no terminal, window, process or filesystem state.

mod draft;
pub mod graph;
#[doc(hidden)]
pub mod protocol;
use crate::catalog::{
    capabilities,
    wally_packages::{PackageSpec, companions_for},
};
pub use draft::*;
pub use graph::ProjectGraph;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// How a project gets dependencies: Wally, or no dependency manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageWorkflow {
    /// The default in the `Default` sense only - which strategy a *project*
    /// gets is always an explicit answer, never this.
    #[default]
    Wally,
    None,
}

impl PackageWorkflow {
    pub const ALL: &'static [Self] = &[Self::Wally, Self::None];
}

pub fn validate_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err("Use a single name without reserved path characters.");
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_uppercase();
    if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&stem.as_str())
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        })
    {
        return Err("This name is reserved by Windows.");
    }
    Ok(())
}

/// Test packages follow the selected capability rather than appearing twice in
/// creation. Derive ownership from the catalog so both interfaces agree.
pub fn capability_owned(key: &str) -> bool {
    capabilities::CAPABILITIES
        .iter()
        .any(|c| c.implementations.iter().any(|i| i.packages.contains(&key)))
}

pub fn apply_derived_packages(graph: &mut ProjectGraph) {
    graph.packages.retain(|key| !capability_owned(key));
    for key in graph.derived().packages {
        if !graph.packages.contains(&key) {
            graph.packages.push(key);
        }
    }
    graph.packages.sort();
}

/// Whether a package may be offered at all, given the strategy already
/// chosen. Offering one that cannot be installed is offering a broken
/// project.
pub fn offerable_package(spec: &PackageSpec, workflow: PackageWorkflow) -> bool {
    workflow == PackageWorkflow::Wally && !capability_owned(spec.key)
}

pub fn add_companions(packages: &mut BTreeSet<String>) {
    let primaries: Vec<String> = packages.iter().cloned().collect();
    let snapshot = packages.clone();
    for key in primaries {
        for companion in companions_for(&key, |k| snapshot.contains(k)) {
            packages.insert(companion.to_string());
        }
    }
}
