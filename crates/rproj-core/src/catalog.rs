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
