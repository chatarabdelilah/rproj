//! Generates the `modules/` tree for the git-submodule package workflow.
//!
//! Layout (verified against littensy/fishing-minigame, a real project
//! consuming several of this catalog's packages exactly this way):
//!
//! ```text
//! modules/
//!   Charm.luau                  generated link: return require(script.Parent.submodules.Charm)
//!   Vide.luau
//!   submodules/
//!     default.project.json      generated; maps each cloned repo's real source
//!     charm/                    git submodule (the whole upstream repo)
//!     vide/
//! ```
//!
//! The root project maps `modules -> $path: "modules"` wholesale. Rojo
//! auto-detects `modules/submodules/default.project.json` and uses it for
//! the `submodules` folder - and because that file only ever `$path`s into
//! specific source subfolders (`./charm/packages/charm/src`), Rojo never
//! walks a vendored repo's root and therefore never sees the vendored
//! `default.project.json` that used to break the sync outright.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::catalog::wally_packages::{self, PackageSpec};
use crate::ui;

/// Test folders that ship inside some packages' source directories (e.g.
/// `vide-ripple/src/__tests__`). Excluded from the submodules project so
/// they don't end up mounted into the DataModel as if they were library
/// code - same exclusion the reference project uses.
const TEST_GLOBS: &[&str] = &["**/__tests__"];

/// Packages in `selected` that can actually be vendored as git submodules,
/// in catalog order.
pub fn vendorable(selected: &BTreeSet<String>) -> Vec<&'static PackageSpec> {
    wally_packages::PACKAGES
        .iter()
        .filter(|p| selected.contains(p.key) && p.submodule.is_some())
        .collect()
}

/// Writes `modules/submodules/default.project.json`, mapping every selected
/// package to its real source inside the cloned repo.
///
/// The instance names here are `PackageSpec::module_name` (canonical
/// upstream casing: `Charm`, `CharmSync`, `gt`), and that is load-bearing
/// rather than cosmetic: the monorepo packages cross-require each other
/// through this flat namespace. `charm-sync/src/client.luau` does
/// `require("../Charm")` and `vide-charm/src/init.luau` does
/// `require("./Charm")` - both resolve to a *sibling of the mounted
/// package*, i.e. `submodules.Charm`. Renaming these to catalog keys
/// (`charm`, `charmSync`) would break those requires at runtime.
pub fn submodules_project(selected: &BTreeSet<String>) -> Value {
    let mut tree = serde_json::Map::new();
    tree.insert("$className".to_string(), json!("Folder"));
    for spec in vendorable(selected) {
        let sub = spec.submodule.expect("vendorable() filtered to Some");
        tree.insert(
            spec.module_name.to_string(),
            json!({ "$path": format!("./{}/{}", sub.dir, sub.path) }),
        );
    }

    json!({
        "name": "submodules",
        "globIgnorePaths": TEST_GLOBS,
        "tree": tree,
    })
}

pub fn write_submodules_project(project_dir: &Path, selected: &BTreeSet<String>) -> Result<()> {
    let project = submodules_project(selected);

    let dir = project_dir.join("modules").join("submodules");
    fs::create_dir_all(&dir)?;
    let path = dir.join("default.project.json");
    fs::write(&path, serde_json::to_string_pretty(&project)?)
        .with_context(|| format!("failed to write {}", path.display()))?;
    ui::ok("wrote modules/submodules/default.project.json");
    Ok(())
}

/// Writes one `modules/<ModuleName>.luau` link file per selected package.
///
/// These exist so project code requires a short, stable path
/// (`ReplicatedStorage.modules.Charm`) instead of reaching through
/// `submodules` into vendored internals, and so a package can later be
/// swapped between the Wally and submodule workflows by editing one link
/// file rather than every call site.
pub fn link_file_contents(spec: &PackageSpec) -> String {
    format!(
        "--!strict\n-- {} ({})\n-- Link to the vendored package under modules/submodules.\nreturn require(script.Parent.submodules.{})\n",
        spec.module_name, spec.docs_url, spec.module_name
    )
}

pub fn write_link_files(project_dir: &Path, selected: &BTreeSet<String>) -> Result<()> {
    let modules_dir = project_dir.join("modules");
    fs::create_dir_all(&modules_dir)?;

    let mut written = Vec::new();
    for spec in vendorable(selected) {
        let path = modules_dir.join(format!("{}.luau", spec.module_name));
        if path.exists() {
            continue;
        }
        fs::write(&path, link_file_contents(spec))
            .with_context(|| format!("failed to write {}", path.display()))?;
        written.push(spec.module_name);
    }

    if written.is_empty() {
        ui::ok("modules/ link files already present");
    } else {
        ui::ok(&format!("wrote modules/ link files: {}", written.join(", ")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::wally_packages::PACKAGES;

    fn selection(keys: &[&str]) -> BTreeSet<String> {
        keys.iter().map(|k| (*k).to_string()).collect()
    }

    /// The failure that made this whole layout necessary: mapping a repo
    /// root lets Rojo load the vendored default.project.json as a nested
    /// project, which then fails on paths only an npm install would create.
    /// Every mapped path must therefore reach *into* the repo.
    #[test]
    fn never_maps_a_vendored_repo_root() {
        let all: BTreeSet<String> = PACKAGES.iter().map(|p| p.key.to_string()).collect();
        let project = submodules_project(&all);
        let tree = project["tree"].as_object().unwrap();

        for (name, node) in tree.iter().filter(|(k, _)| !k.starts_with('$')) {
            let path = node["$path"].as_str().unwrap();
            let inside = path.trim_start_matches("./");
            assert!(
                inside.contains('/'),
                "{name} maps repo root {path:?}; Rojo would load the vendored project file"
            );
        }
    }

    /// charm-sync's own source does `require("../Charm")` and vide-charm's
    /// does `require("./Charm")`; both resolve to a sibling of the mounted
    /// package. That only works if the sibling is named exactly `Charm`,
    /// so these instance names are behaviour, not cosmetics.
    #[test]
    fn monorepo_siblings_are_mounted_under_the_names_they_require() {
        let project = submodules_project(&selection(&["charm", "charmSync", "videCharm", "vide"]));
        let tree = project["tree"].as_object().unwrap();

        for required in ["Charm", "Vide"] {
            assert!(tree.contains_key(required), "missing sibling {required}");
        }
        assert_eq!(tree["Charm"]["$path"], "./charm/packages/charm/src");
        assert_eq!(tree["CharmSync"]["$path"], "./charm/packages/charm-sync/src");
        assert_eq!(tree["VideCharm"]["$path"], "./charm/packages/vide-charm/src");
    }

    /// Packages needing an npm/pnpm install upstream can't be vendored as
    /// raw submodules, so they must never reach the project file.
    #[test]
    fn excludes_packages_that_cannot_be_vendored() {
        let project = submodules_project(&selection(&["react", "reactRoblox", "reactCharm", "vide"]));
        let tree = project["tree"].as_object().unwrap();

        assert!(tree.contains_key("Vide"));
        for excluded in ["React", "ReactRoblox", "ReactCharm"] {
            assert!(!tree.contains_key(excluded), "{excluded} is not vendorable");
        }
    }

    /// A link file's require path has to match the name the package is
    /// actually mounted under, or it resolves to nil at runtime.
    #[test]
    fn link_files_require_the_name_the_package_is_mounted_under() {
        let selected = selection(&["charm", "greentea", "lyra"]);
        let project = submodules_project(&selected);
        let tree = project["tree"].as_object().unwrap();

        for spec in vendorable(&selected) {
            let contents = link_file_contents(spec);
            assert!(
                contents.contains(&format!("require(script.Parent.submodules.{})", spec.module_name)),
                "link file for {} does not require its own mount name:\n{contents}",
                spec.key
            );
            assert!(tree.contains_key(spec.module_name), "{} is not mounted", spec.module_name);
        }
    }

    /// One clone per repo, even though several catalog entries share it.
    #[test]
    fn monorepo_packages_share_one_submodule_dir() {
        let dirs: BTreeSet<&str> = vendorable(&selection(&["charm", "charmSync", "videCharm"]))
            .iter()
            .map(|s| s.submodule.unwrap().dir)
            .collect();
        assert_eq!(dirs.len(), 1, "expected a single clone dir, got {dirs:?}");
    }
}

