//! Catalog guidance shared by the CLI worker and desktop.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub key: String,
    pub title: String,
    pub description: String,
    pub docs_url: String,
    /// Canonical Wally `scope/name`, independent of a project's dependency alias.
    pub package: String,
    /// Luau using the catalog's default import aliases and companion packages.
    pub example: String,
    pub when_useful: String,
    pub context: String,
    pub caveats: String,
}

pub mod artifacts;
pub mod capabilities;
pub mod wally_packages;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Maintenance {
    Active,
    CommunityStable,
    Legacy,
}

impl Maintenance {
    /// Full explanation, used by `rproj info <key>`.
    pub fn badge(&self) -> &'static str {
        match self {
            Maintenance::Active => "actively maintained",
            Maintenance::CommunityStable => "community-stable",
            Maintenance::Legacy => "legacy, avoid for new projects",
        }
    }

    /// One word, used inline in pickers so option lines don't run long -
    /// `rproj info <key>` shows the full badge instead.
    pub fn short_badge(&self) -> &'static str {
        match self {
            Maintenance::Active => "active",
            Maintenance::CommunityStable => "stable",
            Maintenance::Legacy => "legacy",
        }
    }
}
