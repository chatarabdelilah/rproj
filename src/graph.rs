//! The decisions a project was built from, as one value.
//!
//! # Why this is a type rather than four locals
//!
//! `rproj new` is not asking questions; it is **constructing a model**. Each
//! answer adds a node, and each node determines the next:
//!
//! ```text
//! Project
//! ├── Dependency strategy    Wally | git submodules | none
//! ├── Packages               constrained by the strategy
//! ├── Capabilities           what the project should do
//! └── Files                  derived from all of the above
//! ```
//!
//! Three things follow from making that explicit, and each of them was
//! either impossible or duplicated before:
//!
//! - **The summary is a render, not a screen.** It walks the same value the
//!   scaffolder does, so it cannot describe a project different from the one
//!   that gets written.
//! - **Revision is invalidation, not navigation.** Changing the strategy
//!   invalidates the packages (some are no longer vendorable) and therefore
//!   the file list; it leaves the capabilities alone. That is a table
//!   (`Node::invalidates`), not a pile of "go back two screens" logic.
//! - **`rproj.toml` records decisions rather than outcomes**, so `rproj
//!   upgrade` re-derives from intent - a changed default reaches an old
//!   project - and `--like` replays a graph straight to the summary.
//!
//! # What is deliberately *not* here
//!
//! No prompting, no filesystem, no process spawning. The graph is a value;
//! `commands::new` fills it in and `steps::*` acts on what it derives. That
//! split is what lets the same graph be built by a CLI today and by
//! something else later without moving any of this logic.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::catalog::artifacts::{self, Environment, Planned};
use crate::catalog::capabilities;
use crate::config::PackageWorkflow;

/// One editable decision. Ordered as they are asked, which is also the
/// order they constrain each other in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Node {
    Strategy,
    Packages,
    Capabilities,
    Files,
}

impl Node {
    pub const ALL: [Node; 4] = [Node::Strategy, Node::Packages, Node::Capabilities, Node::Files];

    pub fn label(&self) -> &'static str {
        match self {
            Node::Strategy => "dependencies",
            Node::Packages => "packages",
            Node::Capabilities => "capabilities",
            Node::Files => "files",
        }
    }

    /// What re-answering this node makes stale.
    ///
    /// **The whole invalidation model, and it is four rows.** Changing the
    /// strategy can make a chosen package unvendorable, so the packages go;
    /// anything upstream of the file list makes the file list stale. What is
    /// *absent* matters as much: the strategy does not touch capabilities
    /// (linting does not care where packages come from), and packages do not
    /// touch the strategy, which is what stopped React silently switching a
    /// project to Wally.
    pub fn invalidates(&self) -> &'static [Node] {
        match self {
            Node::Strategy => &[Node::Packages, Node::Files],
            Node::Packages => &[Node::Files],
            Node::Capabilities => &[Node::Files],
            Node::Files => &[],
        }
    }
}

/// Everything `rproj new` decided, and everything `rproj upgrade` needs to
/// decide it again.
///
/// Field order is load-bearing for TOML: `capabilities` serialises as a
/// `[capabilities]` table, and TOML requires every bare key to precede the
/// first table header. A test round-trips this rather than trusting it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectGraph {
    /// How the packages were chosen: `guided`, `expert`, `none`, or
    /// `like:<setup>`. A decision in its own right, so it lives here.
    #[serde(default)]
    pub mode: String,
    #[serde(default = "wally")]
    pub package_workflow: PackageWorkflow,
    #[serde(default)]
    pub packages: Vec<String>,
    /// Artifacts declined at the summary. Recorded so `rproj upgrade` does
    /// not helpfully restore a file the user deliberately removed.
    #[serde(default)]
    pub dropped: Vec<String>,
    /// Legacy, read-only. Pre-0.5 projects recorded the tools they pinned
    /// and nothing about *why*; `with_legacy_capabilities` reads this to
    /// reconstruct the intent. Never written by this version.
    #[serde(default, skip_serializing)]
    pub tools_at_creation: Vec<String>,
    /// Capability key -> the implementation actually used.
    ///
    /// Concrete rather than `Option`, deliberately: the file records what
    /// this project *has*, so a later change to which implementation is the
    /// default cannot silently re-point an existing project at a different
    /// test runner.
    #[serde(default)]
    pub capabilities: BTreeMap<String, String>,
}

fn wally() -> PackageWorkflow {
    PackageWorkflow::Wally
}

impl ProjectGraph {
    /// Records a capability choice, resolving `None` to whatever the default
    /// implementation is *now* so the file never carries an ambiguity.
    pub fn choose(&mut self, capability: &str, implementation: Option<&str>) {
        let Some(entry) = capabilities::find(capability) else { return };
        let resolved = implementation
            .and_then(|key| entry.implementation(key))
            .unwrap_or_else(|| entry.default_implementation());
        self.capabilities
            .insert(capability.to_string(), resolved.key.to_string());
    }

    /// The `(capability, implementation)` pairs the catalog's `derive` takes.
    pub fn choices(&self) -> Vec<(String, Option<String>)> {
        self.capabilities
            .iter()
            .map(|(k, v)| (k.clone(), Some(v.clone())))
            .collect()
    }

    pub fn capability_keys(&self) -> Vec<String> {
        self.capabilities.keys().cloned().collect()
    }

    pub fn package_set(&self) -> BTreeSet<String> {
        self.packages.iter().cloned().collect()
    }

    pub fn derived(&self) -> capabilities::Derived {
        capabilities::derive(&self.choices())
    }

    /// Every tool this project pins: what the capabilities need, plus what
    /// the dependency strategy needs.
    ///
    /// Wally is not a capability - nobody wants "Wally", they want their
    /// packages installed - so it is derived from the strategy that actually
    /// needs it. Merged here rather than at the call sites so `rokit.toml`,
    /// the check script and the summary cannot disagree about the list.
    pub fn tools(&self) -> Vec<String> {
        let mut tools = self.derived().tools;
        for tool in strategy_tools(self.package_workflow, self.packages.is_empty()) {
            if !tools.iter().any(|t| t == tool) {
                tools.push((*tool).to_string());
            }
        }
        tools
    }

    pub fn strategy(&self) -> artifacts::Strategy {
        match self.package_workflow {
            PackageWorkflow::Wally => artifacts::Strategy::Wally,
            PackageWorkflow::GitSubmodules => artifacts::Strategy::GitSubmodules,
            PackageWorkflow::None => artifacts::Strategy::None,
        }
    }

    /// The files this project gets, each with the reason it gets them.
    pub fn plan(&self, apps: &[String], extensions: &[String]) -> Vec<Planned> {
        let environment = Environment { apps, extensions, strategy: self.strategy() };
        artifacts::plan(
            &environment,
            &self.capability_keys(),
            &self.derived().artifacts,
            &self.tools(),
            &self.dropped,
        )
    }

    /// The plan for **maintaining** a project, as opposed to creating one.
    ///
    /// `rproj upgrade` must not consult the machine. A `.vscode/settings.json`
    /// that exists is a file this project asked for; whether *this* machine
    /// has VS Code installed today says nothing about that, and you might be
    /// upgrading on a different machine than you scaffolded on. So the app and
    /// extension conditions are treated as satisfied here, while the
    /// capability and strategy conditions - which are properties of the
    /// project rather than of the desk it is sitting on - still apply.
    pub fn maintenance_plan(&self) -> Vec<Planned> {
        let (apps, extensions) = artifacts::every_machine_requirement();
        self.plan(&apps, &extensions)
    }

    /// The plan with nothing dropped - what `customize` offers, so a file
    /// removed once can be restored without restarting the command.
    pub fn full_plan(&self, apps: &[String], extensions: &[String]) -> Vec<Planned> {
        let mut undropped = self.clone();
        undropped.dropped.clear();
        undropped.plan(apps, extensions)
    }

    /// Clears whatever re-answering `node` made stale.
    ///
    /// The packages are cleared rather than filtered on a strategy change:
    /// the user is about to be asked again, and silently keeping the subset
    /// that survives would be a third party editing their answer.
    pub fn invalidate(&mut self, node: Node) {
        for stale in node.invalidates() {
            match stale {
                Node::Strategy => self.package_workflow = PackageWorkflow::Wally,
                Node::Packages => self.packages.clear(),
                Node::Capabilities => self.capabilities.clear(),
                Node::Files => self.dropped.clear(),
            }
        }
    }

    /// Reconstructs the intent of a project scaffolded before capabilities
    /// existed.
    ///
    /// A pre-0.5 `rproj.toml` records the tools it pinned and nothing about
    /// why. Without this, `rproj upgrade` on such a project would derive an
    /// empty capability set and conclude the project wants no linter, no
    /// formatter and no gate - then rewrite its config to match. That is not
    /// an upgrade, it is a demolition.
    ///
    /// The mapping is the inverse of §8.10's table, and it is only consulted
    /// when `capabilities` is empty, so a 0.5 project is never second-guessed.
    pub fn with_legacy_capabilities(mut self) -> Self {
        if !self.capabilities.is_empty() {
            return self;
        }
        const FROM_TOOL: &[(&str, &str)] = &[
            ("selene", "lint"),
            ("stylua", "format"),
            ("luau-lsp-cli", "typecheck"),
            ("lute", "gate"),
            ("tarmac", "assets-2d"),
        ];
        for (tool, capability) in FROM_TOOL {
            if self.tools_at_creation.iter().any(|t| t == tool) {
                self.choose(capability, None);
            }
        }
        // The one capability whose evidence is a package rather than a tool.
        if self.packages.iter().any(|p| p == "testez") {
            self.choose("test", None);
        }
        // Unconditional, unlike the rest: pre-0.5 rproj wrote
        // `.vscode/settings.json` and `sourcemap.json` for *every* project
        // that had VS Code, gated on the app rather than on any recorded
        // choice - so there is no tool in `tools_at_creation` that
        // distinguishes a project which had them from one which did not.
        // Assuming yes is the safe direction: the alternative is an upgrade
        // that stops maintaining editor settings it wrote itself.
        self.choose("editor", None);
        self
    }
}

/// The tools the **dependency strategy** pins, as opposed to the ones
/// capabilities pin.
///
/// `wally-package-types` travels with `wally` because a plain `wally install`
/// writes link files with no `export type` lines, so without it every
/// package silently degrades to `any`.
///
/// Empty with no packages: a manifest with nothing in it needs no installer.
/// Empty under submodules: there is no `wally.toml` to install from and no
/// thunks to retype, so pinning either would be noise in `rokit.toml`.
pub fn strategy_tools(workflow: PackageWorkflow, no_packages: bool) -> &'static [&'static str] {
    match workflow {
        PackageWorkflow::Wally if !no_packages => &["wally", "wally-package-types"],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(workflow: PackageWorkflow, packages: &[&str], caps: &[&str]) -> ProjectGraph {
        let mut g = ProjectGraph {
            mode: "expert".into(),
            package_workflow: workflow,
            packages: packages.iter().map(|p| p.to_string()).collect(),
            ..Default::default()
        };
        for key in caps {
            g.choose(key, None);
        }
        g
    }

    // ---------------------------------------------------------------
    // Invalidation - the reason this is a graph and not a wizard.
    // ---------------------------------------------------------------

    /// Changing the strategy clears the packages, because some may no longer
    /// be vendorable - but leaves the capabilities alone, because linting
    /// does not care where packages come from.
    #[test]
    fn changing_the_strategy_invalidates_packages_but_not_capabilities() {
        let mut g = graph(PackageWorkflow::Wally, &["react"], &["lint", "format"]);
        g.dropped.push(".gitignore".into());

        g.invalidate(Node::Strategy);

        assert!(g.packages.is_empty(), "packages must be re-asked");
        assert!(g.dropped.is_empty(), "the file list is downstream of everything");
        assert_eq!(g.capability_keys(), ["format", "lint"], "capabilities are orthogonal");
    }

    /// Everything upstream of the file list makes the file list stale, and
    /// nothing else does.
    #[test]
    fn only_the_file_list_is_downstream_of_everything() {
        for node in [Node::Strategy, Node::Packages, Node::Capabilities] {
            assert!(
                node.invalidates().contains(&Node::Files),
                "{:?} must invalidate the file list",
                node
            );
        }
        assert!(Node::Files.invalidates().is_empty(), "nothing is downstream of files");
    }

    /// Invalidation must only ever point *forward*, or re-answering one node
    /// could clear an answer the user gave before it - which is the "go back
    /// three screens and lose your work" behaviour this replaces.
    #[test]
    fn invalidation_only_points_forward() {
        for (i, node) in Node::ALL.iter().enumerate() {
            for stale in node.invalidates() {
                let at = Node::ALL.iter().position(|n| n == stale).expect("in ALL");
                assert!(at > i, "{node:?} invalidates {stale:?}, which comes before it");
            }
        }
    }

    #[test]
    fn changing_capabilities_keeps_the_packages() {
        let mut g = graph(PackageWorkflow::Wally, &["reflex"], &["lint"]);
        g.invalidate(Node::Capabilities);
        assert_eq!(g.packages, ["reflex"], "packages are upstream");
    }

    // ---------------------------------------------------------------
    // Derivation.
    // ---------------------------------------------------------------

    #[test]
    fn a_capability_choice_is_recorded_concretely() {
        let g = graph(PackageWorkflow::None, &[], &["lint"]);
        assert_eq!(g.capabilities.get("lint").map(String::as_str), Some("selene"));
    }

    #[test]
    fn an_unknown_capability_is_not_recorded() {
        let mut g = ProjectGraph::default();
        g.choose("teleportation", None);
        assert!(g.capabilities.is_empty());
    }

    #[test]
    fn tools_merge_the_capability_and_strategy_halves() {
        let g = graph(PackageWorkflow::Wally, &["reflex"], &["lint", "gate"]);
        let tools = g.tools();
        assert!(tools.contains(&"selene".to_string()), "{tools:?}");
        assert!(tools.contains(&"lute".to_string()), "{tools:?}");
        assert!(tools.contains(&"wally".to_string()), "{tools:?}");
        assert!(tools.contains(&"wally-package-types".to_string()), "{tools:?}");
    }

    #[test]
    fn a_wally_project_with_no_packages_pins_no_installer() {
        let g = graph(PackageWorkflow::Wally, &[], &[]);
        assert!(g.tools().is_empty(), "{:?}", g.tools());
    }

    #[test]
    fn submodule_projects_never_pin_wally() {
        let g = graph(PackageWorkflow::GitSubmodules, &["charm"], &["lint"]);
        assert!(!g.tools().iter().any(|t| t.starts_with("wally")), "{:?}", g.tools());
    }

    /// The bare project, from the graph rather than from the command.
    #[test]
    fn an_empty_graph_plans_only_the_basics_and_housekeeping() {
        let g = graph(PackageWorkflow::None, &[], &[]);
        let keys: Vec<&str> = g.plan(&[], &[]).iter().map(|p| p.key).collect();
        assert_eq!(keys, ["src", "default.project.json", "rproj.toml", ".gitignore"]);
    }

    /// And `dropped` reaches the housekeeping entries, so the absolute
    /// minimum stays reachable.
    #[test]
    fn dropping_housekeeping_leaves_only_the_mandatory_two() {
        let mut g = graph(PackageWorkflow::None, &[], &[]);
        g.dropped = vec!["rproj.toml".into(), ".gitignore".into()];
        let keys: Vec<&str> = g.plan(&[], &[]).iter().map(|p| p.key).collect();
        assert_eq!(keys, ["src", "default.project.json"]);
    }

    /// `customize` has to offer what was dropped, or removing a file would
    /// make it unrecoverable without restarting the command.
    #[test]
    fn the_full_plan_ignores_what_was_dropped() {
        let mut g = graph(PackageWorkflow::None, &[], &[]);
        g.dropped = vec![".gitignore".into()];
        let full: Vec<&str> = g.full_plan(&[], &[]).iter().map(|p| p.key).collect();
        assert!(full.contains(&".gitignore"), "{full:?}");
    }

    // ---------------------------------------------------------------
    // Persistence.
    // ---------------------------------------------------------------

    /// TOML requires bare keys to precede the first `[table]` header, and
    /// `capabilities` is a table. Field order in the struct is what puts it
    /// last, so this asserts the file is valid rather than trusting it.
    #[test]
    fn the_graph_round_trips_through_toml() {
        let mut g = graph(PackageWorkflow::GitSubmodules, &["charm", "vide"], &["lint", "test"]);
        g.dropped = vec![".gitignore".into()];

        let text = toml::to_string_pretty(&g).expect("serialise");
        assert!(!text.contains("tools_at_creation"), "legacy field must not be written: {text}");

        let back: ProjectGraph = toml::from_str(&text).expect("parse back");
        assert_eq!(back.package_workflow, PackageWorkflow::GitSubmodules);
        assert_eq!(back.packages, ["charm", "vide"]);
        assert_eq!(back.dropped, [".gitignore"]);
        assert_eq!(back.capability_keys(), ["lint", "test"]);
        assert_eq!(back.capabilities.get("test").map(String::as_str), Some("testez"));
    }

    /// A pre-0.5 file has none of the new keys. It must still load, and must
    /// not silently become a project that wants nothing.
    #[test]
    fn a_pre_capability_file_still_loads() {
        let legacy = r#"
mode = "guided"
package_workflow = "wally"
packages = ["testez", "reflex"]
tools_at_creation = ["rojo", "selene", "stylua", "lute"]
"#;
        let g: ProjectGraph = toml::from_str(legacy).expect("parse legacy");
        assert_eq!(g.packages, ["testez", "reflex"]);
        assert!(g.capabilities.is_empty(), "nothing recorded yet");

        let migrated = g.with_legacy_capabilities();
        let keys = migrated.capability_keys();
        assert_eq!(keys, ["editor", "format", "gate", "lint", "test"], "{keys:?}");
    }

    /// **The demolition this prevents.** Without the bridge, upgrading a
    /// pre-0.5 project derives no capabilities, so its selene and stylua
    /// configs stop being wanted and the gate disappears.
    #[test]
    fn migration_keeps_what_the_old_project_had() {
        let legacy = ProjectGraph {
            package_workflow: PackageWorkflow::Wally,
            packages: vec!["reflex".into()],
            tools_at_creation: vec!["selene".into(), "stylua".into(), "lute".into()],
            ..Default::default()
        };
        let before: Vec<&str> = legacy.clone().plan(&[], &[]).iter().map(|p| p.key).collect();
        assert!(!before.contains(&"selene.toml"), "no capabilities recorded yet: {before:?}");

        let after: Vec<&str> = legacy
            .with_legacy_capabilities()
            .plan(&[], &[])
            .iter()
            .map(|p| p.key)
            .collect();
        assert!(after.contains(&"selene.toml"), "{after:?}");
        assert!(after.contains(&"stylua.toml"), "{after:?}");
        assert!(after.contains(&".lute/check.luau"), "{after:?}");
    }

    /// A 0.5 project records its capabilities, so the bridge must not touch
    /// it - inferring on top would resurrect something deliberately dropped.
    #[test]
    fn migration_never_second_guesses_a_recorded_graph() {
        let g = graph(PackageWorkflow::Wally, &["testez"], &["lint"]);
        let migrated = g.clone().with_legacy_capabilities();
        assert_eq!(migrated.capability_keys(), g.capability_keys());
        assert!(!migrated.capability_keys().contains(&"test".to_string()));
    }
}
