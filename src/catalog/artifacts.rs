//! Every file `rproj new` can write, as catalog data.
//!
//! # What this module stopped being
//!
//! It used to answer two questions per entry: `requires` said *may this be
//! offered*, and `entailed_by` said *is this still a question*. Both were
//! hand-written, and for four entries they were the **same condition** -
//! `selene.toml` was only offerable when Selene was pinned, and being pinned
//! was exactly what settled it. `rproj info` reported those as "always
//! settled", which was the model saying it had a level too many.
//!
//! That level is now `catalog::capabilities`. A capability owns its
//! artifacts, so "why does this project have this file" is answered by
//! naming the capability that asked for it. Nothing here declares what
//! selects it - the edge is recorded once, in the implementation - so the
//! two cannot disagree.
//!
//! # What is left
//!
//! Three kinds of entry, and that is the whole taxonomy:
//!
//! - **`mandatory`** - without it there is no Rojo project. Two entries.
//! - **`housekeeping`** - written for every project because every project
//!   wants them, and not worth a question nine users clear to serve one.
//!   Droppable from the summary's escape hatch, not from a prompt.
//! - everything else - **derived from a capability or the dependency
//!   strategy**, and never asked directly.
//!
//! `also_requires` is the residue: conditions that are not the capability
//! itself. `testez.yml` is a Selene standard library, so it needs the test
//! capability *and* the lint one. Four entries use it. It is deliberately
//! not a general mechanism.

/// An extra condition on an artifact, beyond whatever derived it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// Another capability must also be on.
    Capability(&'static str),
    /// A machine-wide system app key, e.g. `blender`.
    App(&'static str),
    /// A VS Code extension key, e.g. `testez-companion`.
    Extension(&'static str),
    /// The chosen dependency strategy.
    Strategy(Strategy),
}

/// How this project's dependencies arrive.
///
/// Mirrors `config::PackageWorkflow` without depending on it - `catalog`
/// depends on nothing, and inverting that to reach a config type would
/// break the layering the module tree exists to enforce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    Wally,
    GitSubmodules,
    /// No dependency manager at all. A tutorial project, or one that
    /// vendors by hand. Previously unrepresentable: with no packages the
    /// strategy silently became Wally, which then offered a `wally.toml`
    /// for dependencies that did not exist.
    None,
}

impl Requirement {
    /// One phrase naming what this needs, for `rproj info <artifact>`.
    pub fn describe(&self) -> String {
        match self {
            Requirement::Capability(key) => format!("the {key} capability"),
            Requirement::App(key) => format!("the {key} app"),
            Requirement::Extension(key) => format!("the {key} VS Code extension"),
            Requirement::Strategy(Strategy::Wally) => "Wally".to_string(),
            Requirement::Strategy(Strategy::GitSubmodules) => "git submodules".to_string(),
            Requirement::Strategy(Strategy::None) => "no dependency manager".to_string(),
        }
    }
}

/// Grouping for the summary, so a reader is not handed 21 flat lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactCategory {
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
    /// One line, shown on the summary.
    pub description: &'static str,
    pub category: ArtifactCategory,
    /// Conditions beyond whatever derived this. **All** of them.
    pub also_requires: &'static [Requirement],
    /// Written for every project without being asked. Not the same as
    /// mandatory: a project is still a project without a `.gitignore`.
    pub housekeeping: bool,
    /// Written unconditionally and never droppable.
    pub mandatory: bool,
}

/// The artifacts, in the order they would be written.
pub const ARTIFACTS: &[Artifact] = &[
    // --- Core -------------------------------------------------------------
    Artifact {
        key: "src",
        description: "The shared/server/client source tree",
        category: ArtifactCategory::Core,
        also_requires: &[],
        housekeeping: false,
        mandatory: true,
    },
    Artifact {
        key: "default.project.json",
        description: "Rojo project definition - the tree Studio syncs against",
        category: ArtifactCategory::Core,
        also_requires: &[],
        housekeeping: false,
        mandatory: true,
    },
    Artifact {
        key: "rokit.toml",
        // Derived from having any tool to pin, which is itself derived from
        // the capabilities chosen. Not a question, and not housekeeping: a
        // project with no capabilities pins nothing and has no use for it.
        description: "Pins the CLI tool versions this project uses, so a teammate gets the same ones",
        category: ArtifactCategory::Core,
        also_requires: &[],
        housekeeping: false,
        mandatory: false,
    },
    Artifact {
        key: "rproj.toml",
        // Housekeeping rather than a question: it records the decisions this
        // project was built from, and without it `rproj upgrade` is blind.
        description: "Records the decisions this project was built from, so `rproj upgrade` can re-derive them",
        category: ArtifactCategory::Core,
        also_requires: &[],
        housekeeping: true,
        mandatory: false,
    },
    Artifact {
        key: ".gitignore",
        description: "Ignores regenerable output - Packages/, sourcemap.json, editor state",
        category: ArtifactCategory::Core,
        also_requires: &[],
        housekeeping: true,
        mandatory: false,
    },
    Artifact {
        key: ".gitattributes",
        // Derived from `format`, not housekeeping: it exists because StyLua
        // writes LF and Git for Windows checks out CRLF, so without a
        // formatter there is nothing for it to protect.
        description: "Pins the working tree to LF, so a fresh Windows clone still passes its own format check",
        category: ArtifactCategory::Core,
        also_requires: &[],
        housekeeping: false,
        mandatory: false,
    },
    // --- Dependencies -----------------------------------------------------
    Artifact {
        key: "wally.toml",
        description: "Wally manifest, split by realm",
        category: ArtifactCategory::Dependencies,
        also_requires: &[Requirement::Strategy(Strategy::Wally)],
        housekeeping: false,
        mandatory: false,
    },
    Artifact {
        key: "modules",
        description: "Vendored packages as git submodules, with generated link files",
        category: ArtifactCategory::Dependencies,
        also_requires: &[Requirement::Strategy(Strategy::GitSubmodules)],
        housekeeping: false,
        mandatory: false,
    },
    // --- Quality ----------------------------------------------------------
    Artifact {
        key: "selene.toml",
        description: "Selene lint configuration",
        category: ArtifactCategory::Quality,
        also_requires: &[],
        housekeeping: false,
        mandatory: false,
    },
    Artifact {
        key: "stylua.toml",
        description: "StyLua formatting configuration",
        category: ArtifactCategory::Quality,
        also_requires: &[],
        housekeeping: false,
        mandatory: false,
    },
    Artifact {
        key: ".luaurc",
        description: "Puts Luau in strict mode, so type errors are errors",
        category: ArtifactCategory::Quality,
        also_requires: &[],
        housekeeping: false,
        mandatory: false,
    },
    Artifact {
        key: "sourcemap.json",
        description: "Maps files to Roblox instances, so requires resolve in your editor",
        category: ArtifactCategory::Quality,
        also_requires: &[],
        housekeeping: false,
        mandatory: false,
    },
    // --- Testing ----------------------------------------------------------
    Artifact {
        key: "tests",
        description: "Test folders with a passing example spec per realm, typed for luau-lsp",
        category: ArtifactCategory::Testing,
        also_requires: &[],
        housekeeping: false,
        mandatory: false,
    },
    Artifact {
        key: "testez.yml",
        // Measured: with `std = "roblox+testez"` and no testez.yml, selene
        // prints "Could not find all standard library files" and exits 1
        // without linting anything - src/ included. So it travels with the
        // lint capability, not merely with the test one.
        description: "Selene standard library covering TestEZ's globals, so specs don't lint as undefined",
        category: ArtifactCategory::Testing,
        also_requires: &[Requirement::Capability("lint")],
        housekeeping: false,
        mandatory: false,
    },
    Artifact {
        key: "testez-companion.toml",
        // Gated on the extension that reads it. It used to be written for
        // everyone who picked TestEZ, so the usual outcome was a config file
        // for an extension the user had never installed.
        description: "Lets the TestEZ Companion extension find your specs",
        category: ArtifactCategory::Testing,
        also_requires: &[Requirement::Extension("testez-companion")],
        housekeeping: false,
        mandatory: false,
    },
    // --- Editor -----------------------------------------------------------
    Artifact {
        key: ".vscode/settings.json",
        description: "Points luau-lsp at the sourcemap, enables the Studio bridge, sets the formatter",
        category: ArtifactCategory::Editor,
        also_requires: &[Requirement::App("vscode")],
        housekeeping: false,
        mandatory: false,
    },
    // --- Automation -------------------------------------------------------
    Artifact {
        key: ".lute/check.luau",
        description: "One script that runs every quality check you selected - type, lint, format",
        category: ArtifactCategory::Automation,
        also_requires: &[],
        housekeeping: false,
        mandatory: false,
    },
    Artifact {
        key: ".github/workflows/ci.yml",
        description: "Runs the same quality script on every push",
        category: ArtifactCategory::Automation,
        also_requires: &[],
        housekeeping: false,
        mandatory: false,
    },
    // --- Assets -----------------------------------------------------------
    Artifact {
        key: "blender",
        description: "Starter .blend scene at Roblox's unit scale (1 stud = 0.28 m)",
        category: ArtifactCategory::Assets,
        also_requires: &[Requirement::App("blender")],
        housekeeping: false,
        mandatory: false,
    },
    Artifact {
        key: "figma",
        description: "Folder for exported design assets, wired to the Tarmac upload pipeline",
        category: ArtifactCategory::Assets,
        also_requires: &[],
        housekeeping: false,
        mandatory: false,
    },
    Artifact {
        key: "tarmac.toml",
        description: "Tarmac asset-sync configuration",
        category: ArtifactCategory::Assets,
        also_requires: &[],
        housekeeping: false,
        mandatory: false,
    },
];

pub fn find(key: &str) -> Option<&'static Artifact> {
    ARTIFACTS.iter().find(|a| a.key == key)
}

/// What the environment offers, as the artifact layer needs it. Everything
/// *chosen* arrives through capabilities instead.
#[derive(Debug, Clone, Copy)]
pub struct Environment<'a> {
    pub apps: &'a [String],
    pub extensions: &'a [String],
    pub strategy: Strategy,
}

impl<'a> Environment<'a> {
    fn has(&self, requirement: &Requirement, capabilities: &[String]) -> bool {
        match requirement {
            Requirement::Capability(key) => capabilities.iter().any(|c| c == key),
            Requirement::App(key) => self.apps.iter().any(|a| a == key),
            Requirement::Extension(key) => self.extensions.iter().any(|e| e == key),
            Requirement::Strategy(strategy) => self.strategy == *strategy,
        }
    }
}

/// Why an artifact is being written, in the user's terms.
///
/// Carried alongside the artifact rather than derived later, because the
/// summary's whole job is showing the work: a file listed with no reason is
/// a receipt, and this tool exists partly to teach the ecosystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// Every Rojo project has this.
    Mandatory,
    /// Written for every project; not worth a question.
    Housekeeping,
    /// A capability asked for it. Holds the capability key.
    Capability(String),
    /// The dependency strategy asked for it.
    Strategy(Strategy),
    /// Something pins tool versions, and this is where they go. Holds how
    /// many, because "pins 4 tool versions" is more use than "pins tools".
    Pins(usize),
}

impl Reason {
    /// Reads after the artifact key on the summary.
    pub fn describe(&self) -> String {
        match self {
            Reason::Mandatory => "every Rojo project has this".to_string(),
            Reason::Housekeeping => "written for every project".to_string(),
            Reason::Capability(key) => format!("you chose {key}"),
            Reason::Strategy(Strategy::Wally) => "this project uses Wally".to_string(),
            Reason::Strategy(Strategy::GitSubmodules) => {
                "this project vendors packages as git submodules".to_string()
            }
            Reason::Strategy(Strategy::None) => "no dependency manager".to_string(),
            Reason::Pins(n) => {
                format!("pins {n} tool version{} so teammates get the same ones", if *n == 1 { "" } else { "s" })
            }
        }
    }
}

/// An artifact together with why it is being written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    pub key: &'static str,
    pub reason: Reason,
}

/// The artifacts this project gets, in write order, each with its reason.
///
/// `derived` is what the capabilities produced (see `capabilities::derive`);
/// `capability_keys` is what was chosen, for `Requirement::Capability`;
/// `dropped` is the summary's escape hatch, and is the only way anything
/// non-mandatory leaves.
pub fn plan(
    environment: &Environment,
    capability_keys: &[String],
    derived: &[String],
    pinned_tools: &[String],
    dropped: &[String],
) -> Vec<Planned> {
    let mut planned = Vec::new();
    for artifact in ARTIFACTS {
        if !artifact.mandatory && dropped.iter().any(|d| d == artifact.key) {
            continue;
        }
        if !artifact
            .also_requires
            .iter()
            .all(|r| environment.has(r, capability_keys))
        {
            continue;
        }

        let reason = if artifact.mandatory {
            Reason::Mandatory
        } else if artifact.key == "rokit.toml" {
            // Owned by nothing in particular, and needed whenever anything
            // pins a version. Skipping this is how a project derived four
            // tools from its capabilities and then wrote no manifest to pin
            // them in - `rokit add` never ran at all, so the "same versions
            // as your teammate" promise silently became nothing.
            if pinned_tools.is_empty() {
                continue;
            }
            Reason::Pins(pinned_tools.len())
        } else if let Some(key) = capability_that_wrote(artifact.key, capability_keys) {
            Reason::Capability(key)
        } else if strategy_wants(artifact.key, environment.strategy) {
            Reason::Strategy(environment.strategy)
        } else if derived.iter().any(|d| d == artifact.key) {
            // Derived by a capability whose key we could not name - only
            // reachable if the catalogs disagree, which a test forbids.
            Reason::Housekeeping
        } else if artifact.housekeeping {
            Reason::Housekeeping
        } else {
            continue;
        };
        planned.push(Planned { key: artifact.key, reason });
    }
    planned
}

/// Which capability's implementation lists this artifact.
///
/// Filters through `capabilities::offerable` rather than testing raw
/// membership, so a capability whose requirement was not chosen writes
/// nothing. Without that, ticking CI without the gate still produced a
/// workflow file - `capabilities::derive` dropped it correctly and this
/// function put it back, because the two disagreed about what "chosen"
/// meant.
fn capability_that_wrote(key: &str, chosen: &[String]) -> Option<String> {
    use crate::catalog::capabilities;
    for capability in capabilities::offerable(chosen) {
        if !chosen.iter().any(|c| c == capability.key) {
            continue;
        }
        for implementation in capability.implementations {
            if implementation.artifacts.contains(&key) {
                return Some(capability.key.to_string());
            }
        }
    }
    None
}

fn strategy_wants(key: &str, strategy: Strategy) -> bool {
    matches!(
        (key, strategy),
        ("wally.toml", Strategy::Wally) | ("modules", Strategy::GitSubmodules)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::capabilities;

    fn owned(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|s| s.to_string()).collect()
    }

    fn env(strategy: Strategy) -> (Vec<String>, Vec<String>, Strategy) {
        (Vec::new(), Vec::new(), strategy)
    }

    /// Plans a project from capability keys, the way `commands::new` does.
    fn plan_from(
        capabilities_chosen: &[&str],
        strategy: Strategy,
        apps: &[&str],
        extensions: &[&str],
    ) -> Vec<Planned> {
        let (apps, extensions) = (owned(apps), owned(extensions));
        let environment = Environment { apps: &apps, extensions: &extensions, strategy };
        let selected: Vec<(String, Option<String>)> = capabilities_chosen
            .iter()
            .map(|k| (k.to_string(), None))
            .collect();
        let derived = capabilities::derive(&selected);
        let keys = owned(capabilities_chosen);
        plan(&environment, &keys, &derived.artifacts, &derived.tools, &[])
    }

    fn keys_of(planned: &[Planned]) -> Vec<&'static str> {
        planned.iter().map(|p| p.key).collect()
    }

    // ---------------------------------------------------------------
    // Structure.
    // ---------------------------------------------------------------

    #[test]
    fn every_key_is_unique() {
        let mut keys: Vec<&str> = ARTIFACTS.iter().map(|a| a.key).collect();
        let before = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(before, keys.len(), "duplicate artifact key");
    }

    /// A mandatory artifact with a condition could be conditioned away,
    /// which contradicts being mandatory.
    #[test]
    fn mandatory_artifacts_require_nothing() {
        for artifact in ARTIFACTS.iter().filter(|a| a.mandatory) {
            assert!(artifact.also_requires.is_empty(), "{}", artifact.key);
            assert!(!artifact.housekeeping, "{} is both", artifact.key);
        }
    }

    /// **The edge is recorded once.** Every non-mandatory, non-housekeeping
    /// artifact must be reachable from some capability or from the
    /// dependency strategy - otherwise nothing can ever write it, and the
    /// entry is a promise the CLI silently fails to keep.
    ///
    /// This replaces the old scaffolder source-scan: the catalog now knows
    /// its own edges, so the check is data rather than a grep.
    #[test]
    fn every_artifact_is_reachable_from_something() {
        let strategy_owned = ["wally.toml", "modules"];
        for artifact in ARTIFACTS {
            if artifact.mandatory || artifact.housekeeping {
                continue;
            }
            if strategy_owned.contains(&artifact.key) || artifact.key == "rokit.toml" {
                continue;
            }
            let claimed = capabilities::CAPABILITIES.iter().any(|c| {
                c.implementations
                    .iter()
                    .any(|i| i.artifacts.contains(&artifact.key))
            });
            assert!(
                claimed,
                "{} is not derived by any capability, so nothing can write it",
                artifact.key
            );
        }
    }

    /// And the reverse: a capability naming an artifact that does not exist
    /// would be a silent no-op at scaffold time.
    #[test]
    fn every_capability_artifact_exists() {
        for capability in capabilities::CAPABILITIES {
            for implementation in capability.implementations {
                for key in implementation.artifacts {
                    assert!(
                        find(key).is_some(),
                        "{}/{} names unknown artifact {key}",
                        capability.key,
                        implementation.key
                    );
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // The user story.
    // ---------------------------------------------------------------

    /// **"I want just the Rojo basics."** No capabilities, no packages, no
    /// dependency manager - and the result is the source tree, the project
    /// file, and the two housekeeping entries.
    #[test]
    fn a_minimal_answer_writes_almost_nothing() {
        let keys = keys_of(&plan_from(&[], Strategy::None, &[], &[]));
        assert_eq!(
            keys,
            ["src", "default.project.json", "rproj.toml", ".gitignore"],
            "the mandatory two plus housekeeping, and nothing else"
        );
    }

    /// And the escape hatch reaches even the housekeeping entries, so the
    /// absolute minimum is still available.
    #[test]
    fn dropping_the_housekeeping_leaves_only_the_mandatory_two() {
        let (apps, extensions, strategy) = env(Strategy::None);
        let environment = Environment { apps: &apps, extensions: &extensions, strategy };
        let planned = plan(&environment, &[], &[], &[], &owned(&["rproj.toml", ".gitignore"]));
        assert_eq!(keys_of(&planned), ["src", "default.project.json"]);
    }

    // ---------------------------------------------------------------
    // Reasons - the summary's whole point.
    // ---------------------------------------------------------------

    #[test]
    fn every_planned_artifact_carries_a_reason_naming_its_cause() {
        let planned = plan_from(&["lint", "format"], Strategy::Wally, &[], &[]);
        let reason = |key: &str| {
            planned
                .iter()
                .find(|p| p.key == key)
                .unwrap_or_else(|| panic!("{key} not planned: {:?}", keys_of(&planned)))
                .reason
                .clone()
        };
        assert_eq!(reason("src"), Reason::Mandatory);
        assert_eq!(reason("selene.toml"), Reason::Capability("lint".into()));
        assert_eq!(reason("stylua.toml"), Reason::Capability("format".into()));
        assert_eq!(reason("wally.toml"), Reason::Strategy(Strategy::Wally));
        assert_eq!(reason(".gitignore"), Reason::Housekeeping);
    }

    #[test]
    fn reasons_read_as_clauses_after_the_key() {
        assert_eq!(Reason::Capability("lint".into()).describe(), "you chose lint");
        assert_eq!(Reason::Strategy(Strategy::Wally).describe(), "this project uses Wally");
        for reason in [Reason::Mandatory, Reason::Housekeeping] {
            let text = reason.describe();
            assert!(text.chars().next().is_some_and(|c| c.is_lowercase()), "{text}");
            assert!(!text.ends_with('.'), "{text}");
        }
    }

    // ---------------------------------------------------------------
    // Cross-capability conditions.
    // ---------------------------------------------------------------

    /// `testez.yml` is a Selene standard library: it needs the test
    /// capability that names it *and* the lint capability that reads it.
    #[test]
    fn the_testez_selene_library_needs_both_capabilities() {
        let with = plan_from(&["test", "lint"], Strategy::Wally, &[], &[]);
        assert!(keys_of(&with).contains(&"testez.yml"), "{:?}", keys_of(&with));

        let without = plan_from(&["test"], Strategy::Wally, &[], &[]);
        assert!(!keys_of(&without).contains(&"testez.yml"), "{:?}", keys_of(&without));
        // The test folders still arrive - only the lint bridge is skipped.
        assert!(keys_of(&without).contains(&"tests"));
    }

    /// A config file for an extension the user never installed is a file
    /// nothing reads.
    #[test]
    fn the_companion_config_needs_the_companion_extension() {
        let without = plan_from(&["test"], Strategy::Wally, &[], &[]);
        assert!(!keys_of(&without).contains(&"testez-companion.toml"));

        let with = plan_from(&["test"], Strategy::Wally, &[], &["testez-companion"]);
        assert!(keys_of(&with).contains(&"testez-companion.toml"));
    }

    #[test]
    fn the_blender_scene_needs_blender_installed() {
        let without = plan_from(&["assets-3d"], Strategy::None, &[], &[]);
        assert!(!keys_of(&without).contains(&"blender"));

        let with = plan_from(&["assets-3d"], Strategy::None, &["blender"], &[]);
        assert!(keys_of(&with).contains(&"blender"));
    }

    #[test]
    fn the_strategy_decides_between_wally_and_modules() {
        for (strategy, expected, absent) in [
            (Strategy::Wally, "wally.toml", "modules"),
            (Strategy::GitSubmodules, "modules", "wally.toml"),
        ] {
            let keys = keys_of(&plan_from(&[], strategy, &[], &[]));
            assert!(keys.contains(&expected), "{strategy:?}: {keys:?}");
            assert!(!keys.contains(&absent), "{strategy:?}: {keys:?}");
        }
        let none = keys_of(&plan_from(&[], Strategy::None, &[], &[]));
        assert!(!none.contains(&"wally.toml"), "{none:?}");
        assert!(!none.contains(&"modules"), "{none:?}");
    }

    /// CI's body is the gate script, so turning the gate off must take the
    /// workflow with it rather than leaving one whose first command fails.
    #[test]
    fn ci_cannot_outlive_the_gate_it_runs() {
        let with = keys_of(&plan_from(&["gate", "ci"], Strategy::None, &[], &[]));
        assert!(with.contains(&".github/workflows/ci.yml"), "{with:?}");

        let without = keys_of(&plan_from(&["ci"], Strategy::None, &[], &[]));
        assert!(!without.contains(&".github/workflows/ci.yml"), "{without:?}");
    }

    /// The property that must hold for every combination: nothing is
    /// written whose conditions are unmet, and the mandatory two are always
    /// there.
    #[test]
    fn no_planned_artifact_has_an_unmet_condition() {
        let all: Vec<&str> = capabilities::CAPABILITIES.iter().map(|c| c.key).collect();
        for mask in 0u32..(1 << all.len().min(9)) {
            let chosen: Vec<&str> = all
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(_, k)| *k)
                .collect();
            for strategy in [Strategy::Wally, Strategy::GitSubmodules, Strategy::None] {
                for apps in [vec![], vec!["vscode", "blender"]] {
                    let planned = plan_from(&chosen, strategy, &apps, &["testez-companion"]);
                    let keys = keys_of(&planned);
                    let chosen_owned = owned(&chosen);
                    let (apps_owned, ext) = (owned(&apps), owned(&["testez-companion"]));
                    let environment =
                        Environment { apps: &apps_owned, extensions: &ext, strategy };
                    for entry in &planned {
                        let artifact = find(entry.key).expect("from ARTIFACTS");
                        for requirement in artifact.also_requires {
                            assert!(
                                environment.has(requirement, &chosen_owned),
                                "{} written with unmet {requirement:?} (mask {mask})",
                                entry.key
                            );
                        }
                    }
                    assert!(keys.contains(&"src"), "mask {mask}");
                    assert!(keys.contains(&"default.project.json"), "mask {mask}");
                }
            }
        }
    }
}
