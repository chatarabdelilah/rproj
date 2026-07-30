//! Every file `rproj new` can write, as catalog data.
//!
//! # Why this exists
//!
//! Scaffolding used to be a hardcoded sequence with four ad-hoc gates, so
//! artifacts fell into two classes with no principle separating them: some
//! were conditional on a selection, and six were written whatever the user
//! answered - a CI workflow, a quality-gate script, editor settings, a
//! `.luaurc`, `.gitattributes`, and a Blender scene. There was no answer to
//! `rproj new` that omitted them.
//!
//! That is backwards for a tool whose whole purpose is composing a project
//! from choices. Here every artifact is an entry with **requirements**, the
//! same shape packages already have, so a minimal answer yields `src/` plus
//! `default.project.json` and nothing else.
//!
//! Two entries are `mandatory`: without them there is no Rojo project to
//! speak of, so they are not offered as questions.

/// A thing a selection can require.
///
/// Four kinds because the answers come from four different pickers, and
/// collapsing them into one namespace would let `blender` the app collide
/// with a package of the same name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// A Wally package key, e.g. `testez`.
    Package(&'static str),
    /// A rokit tool key, e.g. `stylua`.
    Tool(&'static str),
    /// A machine-wide system app key, e.g. `blender`.
    App(&'static str),
    /// Another artifact. `ci.yml` is worthless without the script it runs.
    Artifact(&'static str),
    /// The chosen dependency workflow.
    Workflow(Workflow),
}

/// Mirrors `config::PackageWorkflow` without depending on it - `catalog`
/// depends on nothing, and inverting that to reach a config type would
/// break the layering the module tree exists to enforce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workflow {
    Wally,
    GitSubmodules,
}

/// Grouping for the picker, so a reader is not handed 21 flat checkboxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactCategory {
    /// Not offered; the project is not a project without it.
    Core,
    Dependencies,
    Quality,
    Testing,
    Editor,
    Automation,
    Assets,
}

impl ArtifactCategory {
    pub fn label(&self) -> &'static str {
        match self {
            ArtifactCategory::Core => "Project structure",
            ArtifactCategory::Dependencies => "Dependencies",
            ArtifactCategory::Quality => "Linting & formatting",
            ArtifactCategory::Testing => "Testing",
            ArtifactCategory::Editor => "Editor integration",
            ArtifactCategory::Automation => "Automation",
            ArtifactCategory::Assets => "Assets",
        }
    }

}

pub struct Artifact {
    /// Stable identifier, also the path it writes where that is unambiguous.
    pub key: &'static str,
    /// One line, shown in the picker.
    pub description: &'static str,
    pub category: ArtifactCategory,
    /// Everything that must be selected for this to be *offerable*. All of
    /// them, not any - an artifact needing both a package and a tool is
    /// offered only when both are there.
    pub requires: &'static [Requirement],
    /// Pre-checked when the artifact is offerable.
    pub default_selected: bool,
    /// Written unconditionally and never offered.
    pub mandatory: bool,
}

/// The artifacts, in the order they would be written.
pub const ARTIFACTS: &[Artifact] = &[
    // --- Core: not offered ------------------------------------------------
    Artifact {
        key: "src",
        description: "The shared/server/client source tree",
        category: ArtifactCategory::Core,
        requires: &[],
        default_selected: true,
        mandatory: true,
    },
    Artifact {
        key: "default.project.json",
        description: "Rojo project definition - the tree Studio syncs against",
        category: ArtifactCategory::Core,
        requires: &[],
        default_selected: true,
        mandatory: true,
    },
    Artifact {
        key: "rokit.toml",
        description: "Pins the CLI tool versions this project uses, so a teammate gets the same ones",
        category: ArtifactCategory::Core,
        requires: &[],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: "rproj.toml",
        description: "Records what you picked, so `rproj upgrade` knows what this project is",
        category: ArtifactCategory::Core,
        requires: &[],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: ".gitignore",
        description: "Ignores regenerable output - Packages/, sourcemap.json, editor state",
        category: ArtifactCategory::Core,
        requires: &[],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: ".gitattributes",
        description: "Pins the working tree to LF, so a fresh Windows clone still passes its own format check",
        category: ArtifactCategory::Core,
        requires: &[],
        default_selected: true,
        mandatory: false,
    },
    // --- Dependencies -----------------------------------------------------
    Artifact {
        key: "wally.toml",
        description: "Wally manifest, split by realm",
        category: ArtifactCategory::Dependencies,
        requires: &[Requirement::Workflow(Workflow::Wally)],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: "modules",
        description: "Vendored packages as git submodules, with generated link files",
        category: ArtifactCategory::Dependencies,
        requires: &[Requirement::Workflow(Workflow::GitSubmodules)],
        default_selected: true,
        mandatory: false,
    },
    // --- Quality ----------------------------------------------------------
    Artifact {
        key: "selene.toml",
        description: "Selene lint configuration",
        category: ArtifactCategory::Quality,
        requires: &[Requirement::Tool("selene")],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: "stylua.toml",
        description: "StyLua formatting configuration",
        category: ArtifactCategory::Quality,
        requires: &[Requirement::Tool("stylua")],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: ".luaurc",
        description: "Puts Luau in strict mode, so type errors are errors",
        category: ArtifactCategory::Quality,
        requires: &[],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: "sourcemap.json",
        description: "Maps files to Roblox instances, so requires resolve in your editor",
        category: ArtifactCategory::Quality,
        requires: &[Requirement::Tool("rojo")],
        default_selected: true,
        mandatory: false,
    },
    // --- Testing ----------------------------------------------------------
    Artifact {
        key: "tests",
        description: "Test folders with a passing example spec per realm",
        category: ArtifactCategory::Testing,
        requires: &[Requirement::Package("testez")],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: "testez.yml",
        description: "Selene standard library covering TestEZ's globals, so specs don't lint as undefined",
        category: ArtifactCategory::Testing,
        requires: &[Requirement::Package("testez"), Requirement::Tool("selene")],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: "tests/.luaurc",
        description: "Declares TestEZ's globals to luau-lsp inside tests/ only",
        category: ArtifactCategory::Testing,
        requires: &[Requirement::Package("testez"), Requirement::Artifact("tests")],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: "testez-companion.toml",
        description: "Lets the TestEZ Companion Studio plugin find your specs",
        category: ArtifactCategory::Testing,
        requires: &[Requirement::Package("testez")],
        default_selected: true,
        mandatory: false,
    },
    // --- Editor -----------------------------------------------------------
    Artifact {
        key: ".vscode/settings.json",
        description: "Points luau-lsp at the sourcemap, enables the Studio bridge, sets the formatter",
        category: ArtifactCategory::Editor,
        requires: &[Requirement::App("vscode")],
        default_selected: true,
        mandatory: false,
    },
    // --- Automation -------------------------------------------------------
    Artifact {
        key: ".lute/check.luau",
        description: "One script that runs every quality check you selected - type, lint, format",
        category: ArtifactCategory::Automation,
        requires: &[Requirement::Tool("lute")],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: ".github/workflows/ci.yml",
        // Default off: this is the one artifact that changes what happens
        // on a push, and opting into that should be a decision rather than
        // something a scaffolder assumes.
        description: "Runs the same quality script on every push (default off - it changes what a push does)",
        category: ArtifactCategory::Automation,
        requires: &[Requirement::Artifact(".lute/check.luau")],
        default_selected: false,
        mandatory: false,
    },
    // --- Assets -----------------------------------------------------------
    Artifact {
        key: "blender",
        description: "Starter .blend scene at Roblox's unit scale (1 stud = 0.28 m)",
        category: ArtifactCategory::Assets,
        requires: &[Requirement::App("blender")],
        default_selected: false,
        mandatory: false,
    },
    Artifact {
        key: "figma",
        description: "Folder for exported design assets, wired to the Tarmac upload pipeline",
        category: ArtifactCategory::Assets,
        requires: &[Requirement::App("figma")],
        default_selected: false,
        mandatory: false,
    },
    Artifact {
        key: "tarmac.toml",
        description: "Tarmac asset-sync configuration",
        category: ArtifactCategory::Assets,
        requires: &[Requirement::Tool("tarmac")],
        default_selected: false,
        mandatory: false,
    },
];

pub fn find(key: &str) -> Option<&'static Artifact> {
    ARTIFACTS.iter().find(|a| a.key == key)
}

/// What the user has chosen everywhere else, as the artifact layer needs it.
///
/// Borrowed rather than owned: every field is already held by the caller,
/// and copying four collections per resolution to satisfy a struct would be
/// work in service of the struct.
#[derive(Debug, Clone, Copy)]
pub struct Selections<'a> {
    pub packages: &'a [String],
    pub tools: &'a [String],
    pub apps: &'a [String],
    pub workflow: Workflow,
}

impl<'a> Selections<'a> {
    fn has(&self, requirement: &Requirement, chosen_artifacts: &[&str]) -> bool {
        match requirement {
            Requirement::Package(key) => self.packages.iter().any(|p| p == key),
            Requirement::Tool(key) => self.tools.iter().any(|t| t == key),
            Requirement::App(key) => self.apps.iter().any(|a| a == key),
            Requirement::Workflow(workflow) => self.workflow == *workflow,
            Requirement::Artifact(key) => chosen_artifacts.contains(key),
        }
    }
}

/// Artifacts whose requirements the selection satisfies, ignoring
/// artifact-to-artifact requirements.
///
/// Split from `resolve` because "could this be offered" and "is this
/// actually written" are different questions: the picker needs the first to
/// know what to show, and only the second depends on what the user then
/// ticked.
pub fn offerable(selections: &Selections) -> Vec<&'static Artifact> {
    // Artifact requirements are checked against everything offerable, so a
    // dependent is shown whenever its dependency *could* be chosen.
    let candidate_keys: Vec<&str> = ARTIFACTS
        .iter()
        .filter(|a| {
            a.requires
                .iter()
                .filter(|r| !matches!(r, Requirement::Artifact(_)))
                .all(|r| selections.has(r, &[]))
        })
        .map(|a| a.key)
        .collect();

    ARTIFACTS
        .iter()
        .filter(|a| a.requires.iter().all(|r| selections.has(r, &candidate_keys)))
        .collect()
}

/// The artifacts actually written, given what the user ticked.
///
/// `chosen` is ignored for mandatory entries and consulted for everything
/// else. An artifact requiring another that was *not* chosen is dropped,
/// transitively - so unticking the quality script also drops the CI
/// workflow that only exists to run it, rather than writing a workflow
/// whose first line fails.
pub fn resolve(selections: &Selections, chosen: &[String]) -> Vec<&'static Artifact> {
    let mut keys: Vec<&str> = offerable(selections)
        .into_iter()
        .filter(|a| a.mandatory || chosen.iter().any(|c| c == a.key))
        .map(|a| a.key)
        .collect();

    // Drop anything whose artifact requirements are now unmet, repeatedly,
    // because dropping one can unmeet another's. Bounded by the list length
    // since each pass removes at least one entry or stops.
    loop {
        // Evaluated against the state at the start of the pass, which is
        // both what a fixpoint iteration means and what the borrow checker
        // will allow - `retain`'s closure cannot read the vector it is
        // draining.
        let surviving = keys.clone();
        let before = keys.len();
        keys.retain(|key| {
            let artifact = find(key).expect("keys come from ARTIFACTS");
            artifact
                .requires
                .iter()
                .all(|r| !matches!(r, Requirement::Artifact(_)) || selections.has(r, &surviving))
        });
        if keys.len() == before {
            break;
        }
    }

    ARTIFACTS.iter().filter(|a| keys.contains(&a.key)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|s| s.to_string()).collect()
    }

    /// Nothing selected anywhere: the minimum a project can be.
    fn nothing() -> (Vec<String>, Vec<String>, Vec<String>) {
        (Vec::new(), Vec::new(), Vec::new())
    }

    // ---------------------------------------------------------------
    // Structural invariants - these hold for any future entry.
    // ---------------------------------------------------------------

    #[test]
    fn every_key_is_unique() {
        let mut keys: Vec<&str> = ARTIFACTS.iter().map(|a| a.key).collect();
        let before = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(before, keys.len(), "duplicate artifact key");
    }

    /// A requirement naming an artifact that does not exist would silently
    /// make its dependent unofferable forever.
    #[test]
    fn every_artifact_requirement_resolves() {
        for artifact in ARTIFACTS {
            for requirement in artifact.requires {
                if let Requirement::Artifact(key) = requirement {
                    assert!(
                        find(key).is_some(),
                        "{} requires unknown artifact {key}",
                        artifact.key
                    );
                }
            }
        }
    }

    /// A cycle would make `resolve`'s fixpoint loop drop both entries, so
    /// neither would ever be written and nothing would say why.
    #[test]
    fn the_artifact_requirement_graph_is_acyclic() {
        fn reaches(from: &str, target: &str, depth: usize) -> bool {
            assert!(depth < ARTIFACTS.len() + 1, "cycle through {from}");
            let Some(artifact) = find(from) else { return false };
            artifact.requires.iter().any(|r| match r {
                Requirement::Artifact(next) => *next == target || reaches(next, target, depth + 1),
                _ => false,
            })
        }
        for artifact in ARTIFACTS {
            assert!(
                !reaches(artifact.key, artifact.key, 0),
                "{} reaches itself",
                artifact.key
            );
        }
    }

    /// A mandatory artifact with a requirement could be required-away, which
    /// contradicts being mandatory.
    #[test]
    fn mandatory_artifacts_require_nothing() {
        for artifact in ARTIFACTS.iter().filter(|a| a.mandatory) {
            assert!(
                artifact.requires.is_empty(),
                "{} is mandatory but has requirements",
                artifact.key
            );
        }
    }

    // ---------------------------------------------------------------
    // The user story that used to fail.
    // ---------------------------------------------------------------

    /// **"I want just the Rojo basics."**
    ///
    /// No packages, no tools, no apps, nothing ticked - and the result is
    /// the source tree and the project file. Not a CI workflow, not a
    /// quality script, not editor settings, not a Blender scene.
    #[test]
    fn a_minimal_answer_writes_only_the_mandatory_artifacts() {
        let (packages, tools, apps) = nothing();
        let selections = Selections {
            packages: &packages,
            tools: &tools,
            apps: &apps,
            workflow: Workflow::Wally,
        };
        let written: Vec<&str> = resolve(&selections, &[]).iter().map(|a| a.key).collect();
        assert_eq!(written, ["src", "default.project.json"]);
    }

    /// The six that used to be unconditional are all droppable now.
    #[test]
    fn the_previously_unconditional_artifacts_are_all_optional() {
        for key in [
            ".lute/check.luau",
            ".github/workflows/ci.yml",
            ".vscode/settings.json",
            ".luaurc",
            ".gitattributes",
            "blender",
        ] {
            let artifact = find(key).unwrap_or_else(|| panic!("{key} missing"));
            assert!(!artifact.mandatory, "{key} is still mandatory");
        }
    }

    // ---------------------------------------------------------------
    // Requirements.
    // ---------------------------------------------------------------

    #[test]
    fn an_artifact_is_not_offered_when_its_tool_is_absent() {
        let (packages, apps) = (Vec::new(), Vec::new());
        let tools = owned(&["selene"]);
        let selections = Selections {
            packages: &packages,
            tools: &tools,
            apps: &apps,
            workflow: Workflow::Wally,
        };
        let offered: Vec<&str> = offerable(&selections).iter().map(|a| a.key).collect();

        assert!(offered.contains(&"selene.toml"), "{offered:?}");
        assert!(!offered.contains(&"stylua.toml"), "no stylua selected: {offered:?}");
        assert!(!offered.contains(&".lute/check.luau"), "no lute: {offered:?}");
    }

    /// Both requirements, not either: the TestEZ selene definition is
    /// pointless without selene, and meaningless without TestEZ.
    #[test]
    fn an_artifact_with_two_requirements_needs_both() {
        let apps = Vec::new();
        let offered = |packages: &[&str], tools: &[&str]| {
            let (p, t) = (owned(packages), owned(tools));
            let selections = Selections {
                packages: &p,
                tools: &t,
                apps: &apps,
                workflow: Workflow::Wally,
            };
            offerable(&selections)
                .iter()
                .any(|a| a.key == "testez.yml")
        };
        assert!(offered(&["testez"], &["selene"]));
        assert!(!offered(&["testez"], &[]));
        assert!(!offered(&[], &["selene"]));
    }

    #[test]
    fn the_workflow_decides_between_wally_and_modules() {
        let (packages, tools, apps) = nothing();
        for (workflow, expected, absent) in [
            (Workflow::Wally, "wally.toml", "modules"),
            (Workflow::GitSubmodules, "modules", "wally.toml"),
        ] {
            let selections = Selections {
                packages: &packages,
                tools: &tools,
                apps: &apps,
                workflow,
            };
            let offered: Vec<&str> = offerable(&selections).iter().map(|a| a.key).collect();
            assert!(offered.contains(&expected), "{workflow:?}: {offered:?}");
            assert!(!offered.contains(&absent), "{workflow:?}: {offered:?}");
        }
    }

    /// Unticking the quality script must drop the CI workflow with it -
    /// otherwise the workflow's first command is missing and every push
    /// fails on a file the user never asked for.
    #[test]
    fn dropping_a_dependency_drops_its_dependent() {
        let (packages, apps) = (Vec::new(), Vec::new());
        let tools = owned(&["lute"]);
        let selections = Selections {
            packages: &packages,
            tools: &tools,
            apps: &apps,
            workflow: Workflow::Wally,
        };

        let with = resolve(&selections, &owned(&[".lute/check.luau", ".github/workflows/ci.yml"]));
        let with: Vec<&str> = with.iter().map(|a| a.key).collect();
        assert!(with.contains(&".github/workflows/ci.yml"), "{with:?}");

        // Same tick for CI, but the script it runs is not chosen.
        let without = resolve(&selections, &owned(&[".github/workflows/ci.yml"]));
        let without: Vec<&str> = without.iter().map(|a| a.key).collect();
        assert!(
            !without.contains(&".github/workflows/ci.yml"),
            "CI must not survive without its script: {without:?}"
        );
    }

    /// The property that has to hold for every combination, not just the
    /// ones with a named test: nothing is written whose requirements are
    /// unmet.
    #[test]
    fn no_resolved_artifact_has_an_unmet_requirement() {
        let every_package = owned(&["testez"]);
        let every_tool = owned(&["selene", "stylua", "lute", "rojo", "tarmac"]);
        let every_app = owned(&["vscode", "blender"]);

        // Every subset of a representative selection, over both workflows.
        for mask in 0u32..(1 << 3) {
            let packages = if mask & 1 != 0 { every_package.clone() } else { Vec::new() };
            let tools = if mask & 2 != 0 { every_tool.clone() } else { Vec::new() };
            let apps = if mask & 4 != 0 { every_app.clone() } else { Vec::new() };

            for workflow in [Workflow::Wally, Workflow::GitSubmodules] {
                let selections = Selections {
                    packages: &packages,
                    tools: &tools,
                    apps: &apps,
                    workflow,
                };
                // Tick everything offerable, which is the worst case for
                // requirements being satisfied.
                let chosen: Vec<String> =
                    offerable(&selections).iter().map(|a| a.key.to_string()).collect();
                let written = resolve(&selections, &chosen);
                let keys: Vec<&str> = written.iter().map(|a| a.key).collect();

                for artifact in &written {
                    for requirement in artifact.requires {
                        assert!(
                            selections.has(requirement, &keys),
                            "{} written with unmet {requirement:?} (mask {mask}, {workflow:?})",
                            artifact.key
                        );
                    }
                }
                // And the mandatory two are always there.
                assert!(keys.contains(&"src"), "mask {mask}");
                assert!(keys.contains(&"default.project.json"), "mask {mask}");
            }
        }
    }

    /// Defaults are what a user gets by pressing enter, so the two that are
    /// deliberately off must stay off.
    #[test]
    fn the_two_deliberately_off_by_default_artifacts_are_off() {
        for key in [".github/workflows/ci.yml", "blender"] {
            assert!(
                !find(key).expect(key).default_selected,
                "{key} must not be pre-checked"
            );
        }
    }
}
