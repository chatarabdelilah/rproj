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
//!
//! # Two different questions
//!
//! `requires` answers *may this be offered* - a Selene config is meaningless
//! on a project that never pins Selene. That is the only question this
//! module used to answer, and it left a real hole: **every offerable
//! artifact was a free checkbox, including ones an earlier answer had
//! already decided.** You could pick six packages and then untick
//! `wally.toml`, and the six packages silently became nothing. You could
//! have Selene pinned and untick `selene.toml`, and get a lint that reports
//! every Roblox global as undefined.
//!
//! So `entailed_by` answers the second question: *is this still a choice?*
//! When an earlier answer entails an artifact, it is written and **reported
//! with the answer that caused it**, not offered as a checkbox with one
//! acceptable answer. The way to not have it is to change the answer it
//! follows from, which is the honest place for that decision - unticking
//! `wally.toml` was never a way to not want packages.
//!
//! The bar for entailment is deliberately narrow: declining it must make an
//! earlier answer **do nothing** or **produce a tool that cannot run**.
//! "Would be nice to have" is not entailment - `stylua.toml` stays a
//! question because StyLua formats fine on its defaults, and `.luaurc`
//! stays a question because non-strict Luau is a real way to work.

/// A thing a selection can require.
///
/// Split by *which picker answered it* rather than collapsed into one
/// namespace, so `blender` the app cannot collide with a package of the same
/// name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// A Wally package key, e.g. `testez`.
    Package(&'static str),
    /// A rokit tool key, e.g. `stylua`.
    Tool(&'static str),
    /// A machine-wide system app key, e.g. `blender`.
    App(&'static str),
    /// A VS Code extension key, e.g. `testez-companion`.
    Extension(&'static str),
    /// Another artifact. `ci.yml` is worthless without the script it runs.
    Artifact(&'static str),
    /// The chosen dependency workflow.
    Workflow(Workflow),
    /// At least one package was selected, whichever it was. Used for
    /// entailment: what makes a dependency manifest non-negotiable is
    /// having dependencies, not having any particular one.
    AnyPackage,
    /// At least one tool will be pinned into this project.
    AnyTool,
}

impl Requirement {
    /// One phrase naming what this needs, for `rproj info <artifact>`.
    ///
    /// Reads after "needs" or "when": `needs the Wally workflow`, `when any
    /// package at all is selected`.
    pub fn describe(&self) -> String {
        match self {
            Requirement::Package(key) => format!("the {key} package"),
            Requirement::Tool(key) => format!("the {key} tool"),
            Requirement::App(key) => format!("the {key} app"),
            Requirement::Extension(key) => format!("the {key} VS Code extension"),
            Requirement::Artifact(key) => format!("{key} to also be generated"),
            Requirement::Workflow(Workflow::Wally) => "the Wally workflow".to_string(),
            Requirement::Workflow(Workflow::GitSubmodules) => {
                "the git-submodule workflow".to_string()
            }
            Requirement::AnyPackage => "any package at all".to_string(),
            Requirement::AnyTool => "any tool to pin".to_string(),
        }
    }
}

/// A condition under which an artifact stops being a question.
///
/// Carries its own explanation because the user is owed the *reason*, not
/// just the fact. A line saying `wally.toml` is included is a tool being
/// bossy; a line saying it is included because the three packages you picked
/// install from it is a tool showing its work - and it tells you which
/// answer to change if you disagree.
#[derive(Debug, Clone, Copy)]
pub struct Entailment {
    pub when: Requirement,
    /// Reads after the key: `wally.toml - the packages you picked ...`.
    pub because: &'static str,
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
    /// Conditions that turn this from a question into a consequence. **Any**
    /// of them, not all: one sufficient reason is enough to settle it.
    ///
    /// Evaluated only for artifacts that are already offerable, so an
    /// entailment does not have to restate the requirements - `testez.yml`
    /// is entailed by Selene being pinned, and separately requires TestEZ,
    /// and both hold before it is written.
    pub entailed_by: &'static [Entailment],
    /// Pre-checked when the artifact is offered. Meaningless for entries
    /// that are mandatory or entailed, which are never in the picker.
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
        entailed_by: &[],
        default_selected: true,
        mandatory: true,
    },
    Artifact {
        key: "default.project.json",
        description: "Rojo project definition - the tree Studio syncs against",
        category: ArtifactCategory::Core,
        requires: &[],
        entailed_by: &[],
        default_selected: true,
        mandatory: true,
    },
    Artifact {
        key: "rokit.toml",
        description: "Pins the CLI tool versions this project uses, so a teammate gets the same ones",
        category: ArtifactCategory::Core,
        requires: &[],
        // Not a file *about* the pinned tools - it *is* the pin. Declining it
        // while pinning tools asks for a pin with nowhere to write it, and
        // `rokit add` would create the file anyway, so the answer was never
        // going to be honoured.
        entailed_by: &[Entailment {
            when: Requirement::AnyTool,
            because: "it is where this project's tool versions are pinned",
        }],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: "rproj.toml",
        description: "Records what you picked, so `rproj upgrade` knows what this project is",
        category: ArtifactCategory::Core,
        requires: &[],
        // Deliberately still a question. Nothing else needs it: declining it
        // costs you `rproj upgrade`, which is a loss the user can weigh, not
        // a contradiction.
        entailed_by: &[],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: ".gitignore",
        description: "Ignores regenerable output - Packages/, sourcemap.json, editor state",
        category: ArtifactCategory::Core,
        requires: &[],
        entailed_by: &[],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: ".gitattributes",
        description: "Pins the working tree to LF, so a fresh Windows clone still passes its own format check",
        category: ArtifactCategory::Core,
        requires: &[],
        entailed_by: &[],
        default_selected: true,
        mandatory: false,
    },
    // --- Dependencies -----------------------------------------------------
    Artifact {
        key: "wally.toml",
        description: "Wally manifest, split by realm",
        category: ArtifactCategory::Dependencies,
        requires: &[Requirement::Workflow(Workflow::Wally)],
        // With packages selected this is the whole point of having selected
        // them: no manifest, no install, and the package answers evaporate.
        // With *no* packages it stays a question - an empty manifest to fill
        // in by hand later is a legitimate thing to want.
        entailed_by: &[Entailment {
            when: Requirement::AnyPackage,
            because: "the packages you picked are installed from it",
        }],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: "modules",
        description: "Vendored packages as git submodules, with generated link files",
        category: ArtifactCategory::Dependencies,
        requires: &[Requirement::Workflow(Workflow::GitSubmodules)],
        entailed_by: &[Entailment {
            when: Requirement::AnyPackage,
            because: "the packages you picked are vendored into it",
        }],
        default_selected: true,
        mandatory: false,
    },
    // --- Quality ----------------------------------------------------------
    Artifact {
        key: "selene.toml",
        description: "Selene lint configuration",
        category: ArtifactCategory::Quality,
        requires: &[Requirement::Tool("selene")],
        // Measured, not assumed: `selene .` with no config on a file using
        // `game`, `script` and `workspace` reports all three as
        // `undefined_variable` and exits 1, because the default standard
        // library is Lua 5.1. So the config is not a refinement - without it
        // the lint fails on every Roblox file in the project.
        entailed_by: &[Entailment {
            when: Requirement::Tool("selene"),
            because: "selene defaults to the Lua 5.1 std, so every Roblox global lints as undefined",
        }],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: "stylua.toml",
        description: "StyLua formatting configuration",
        category: ArtifactCategory::Quality,
        requires: &[Requirement::Tool("stylua")],
        // The contrast with selene.toml, and the reason entailment is a
        // per-entry judgement rather than a rule about config files: StyLua
        // has working defaults. Declining this gives you StyLua's house
        // style instead of rproj's, which is a preference, not a breakage.
        entailed_by: &[],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: ".luaurc",
        description: "Puts Luau in strict mode, so type errors are errors",
        category: ArtifactCategory::Quality,
        requires: &[],
        entailed_by: &[],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: "sourcemap.json",
        description: "Maps files to Roblox instances, so requires resolve in your editor",
        category: ArtifactCategory::Quality,
        requires: &[Requirement::Tool("rojo")],
        // Generated output, and `rproj watch` regenerates it on demand, so
        // declining it costs a command rather than a capability.
        entailed_by: &[],
        default_selected: true,
        mandatory: false,
    },
    // --- Testing ----------------------------------------------------------
    Artifact {
        key: "tests",
        // Writes `tests/.luaurc` too. That was a separate entry until it
        // turned out not to be a separate decision: its only job is to
        // declare TestEZ's globals so the specs written alongside it do not
        // light up red in the editor. Nobody wants the folder *and* wants
        // its contents to be wrong, so it was a checkbox with one sensible
        // answer - and it could not be entailed either, because entailment
        // is computed before the picker runs and so cannot read an answer
        // the picker has not collected yet.
        description: "Test folders with a passing example spec per realm, typed for luau-lsp",
        category: ArtifactCategory::Testing,
        requires: &[Requirement::Package("testez")],
        // Still a question: someone who picked TestEZ to write their own
        // layout does not need rproj's example specs. The dependency is
        // theirs either way.
        entailed_by: &[],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: "testez.yml",
        description: "Selene standard library covering TestEZ's globals, so specs don't lint as undefined",
        category: ArtifactCategory::Testing,
        requires: &[Requirement::Package("testez"), Requirement::Tool("selene")],
        // Measured: with `std = "roblox+testez"` and no testez.yml, selene
        // prints "Could not find all standard library files" and exits 1
        // without linting anything. Not "tests lint badly" - the whole lint
        // is dead, including src/. Whenever selene.toml is written for a
        // TestEZ project it names this file, so the two travel together.
        entailed_by: &[Entailment {
            when: Requirement::Tool("selene"),
            because: "selene.toml sets std = roblox+testez, and selene will not run without it",
        }],
        default_selected: true,
        mandatory: false,
    },
    Artifact {
        key: "testez-companion.toml",
        description: "Lets the TestEZ Companion extension find your specs",
        category: ArtifactCategory::Testing,
        // Now gated on the extension, which is what reads it. It used to be
        // offered to everyone who picked TestEZ, so the usual answer was a
        // config file for an extension the user had never installed.
        requires: &[
            Requirement::Package("testez"),
            Requirement::Extension("testez-companion"),
        ],
        entailed_by: &[Entailment {
            when: Requirement::Extension("testez-companion"),
            because: "the extension you installed cannot find your specs without it",
        }],
        default_selected: true,
        mandatory: false,
    },
    // --- Editor -----------------------------------------------------------
    Artifact {
        key: ".vscode/settings.json",
        description: "Points luau-lsp at the sourcemap, enables the Studio bridge, sets the formatter",
        category: ArtifactCategory::Editor,
        requires: &[Requirement::App("vscode")],
        // Having VS Code installed says nothing about wanting per-project
        // settings committed to the repo - some teams keep .vscode/ out of
        // version control entirely.
        entailed_by: &[],
        default_selected: true,
        mandatory: false,
    },
    // --- Automation -------------------------------------------------------
    Artifact {
        key: ".lute/check.luau",
        description: "One script that runs every quality check you selected - type, lint, format",
        category: ArtifactCategory::Automation,
        requires: &[Requirement::Tool("lute")],
        // Lute is a general Luau runtime, not rproj's gate runner: a project
        // can pin it and run its own scripts. So this stays a question,
        // unlike selene.toml where the tool has nothing else to read.
        entailed_by: &[],
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
        entailed_by: &[],
        default_selected: false,
        mandatory: false,
    },
    // --- Assets -----------------------------------------------------------
    Artifact {
        key: "blender",
        description: "Starter .blend scene at Roblox's unit scale (1 stud = 0.28 m)",
        category: ArtifactCategory::Assets,
        requires: &[Requirement::App("blender")],
        entailed_by: &[],
        default_selected: false,
        mandatory: false,
    },
    Artifact {
        key: "figma",
        description: "Folder for exported design assets, wired to the Tarmac upload pipeline",
        category: ArtifactCategory::Assets,
        requires: &[Requirement::App("figma")],
        entailed_by: &[],
        default_selected: false,
        mandatory: false,
    },
    Artifact {
        key: "tarmac.toml",
        description: "Tarmac asset-sync configuration",
        category: ArtifactCategory::Assets,
        requires: &[Requirement::Tool("tarmac")],
        // Tarmac has no default config: `tarmac sync` with no tarmac.toml
        // has no inputs and nothing to upload. Same shape as selene.toml.
        entailed_by: &[Entailment {
            when: Requirement::Tool("tarmac"),
            because: "tarmac has no default config, so `tarmac sync` would have no inputs",
        }],
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
    pub extensions: &'a [String],
    pub workflow: Workflow,
}

impl<'a> Selections<'a> {
    fn has(&self, requirement: &Requirement, chosen_artifacts: &[&str]) -> bool {
        match requirement {
            Requirement::Package(key) => self.packages.iter().any(|p| p == key),
            Requirement::Tool(key) => self.tools.iter().any(|t| t == key),
            Requirement::App(key) => self.apps.iter().any(|a| a == key),
            Requirement::Extension(key) => self.extensions.iter().any(|e| e == key),
            Requirement::Workflow(workflow) => self.workflow == *workflow,
            Requirement::Artifact(key) => chosen_artifacts.contains(key),
            Requirement::AnyPackage => !self.packages.is_empty(),
            Requirement::AnyTool => !self.tools.is_empty(),
        }
    }

    /// Why this artifact is no longer a question, or `None` if it still is.
    ///
    /// The first matching reason, not all of them: one sufficient cause is
    /// what the user needs to know, and listing two makes the line longer
    /// without making it more actionable.
    fn entails(&self, artifact: &Artifact) -> Option<&'static str> {
        artifact
            .entailed_by
            .iter()
            .find(|e| self.has(&e.when, &[]))
            .map(|e| e.because)
    }
}

/// Offerable artifacts an earlier answer has already decided, each with the
/// reason - what the picker reports instead of offering.
pub fn entailed(selections: &Selections) -> Vec<(&'static Artifact, &'static str)> {
    offerable(selections)
        .into_iter()
        .filter(|a| !a.mandatory)
        .filter_map(|a| selections.entails(a).map(|why| (a, why)))
        .collect()
}

/// Offerable artifacts that are still a genuine choice - what the picker
/// actually asks about.
pub fn offered(selections: &Selections) -> Vec<&'static Artifact> {
    offerable(selections)
        .into_iter()
        .filter(|a| !a.mandatory && selections.entails(a).is_none())
        .collect()
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
/// `chosen` is consulted only for entries that were genuinely offered.
/// Mandatory ones are in regardless, and so are entailed ones - a caller
/// passing a `chosen` list that omits an entailed artifact is describing a
/// project that cannot exist, and silently honouring it is what let six
/// selected packages resolve to no manifest.
///
/// An artifact requiring another that was *not* chosen is dropped,
/// transitively - so unticking the quality script also drops the CI
/// workflow that only exists to run it, rather than writing a workflow
/// whose first line fails.
pub fn resolve(selections: &Selections, chosen: &[String]) -> Vec<&'static Artifact> {
    let mut keys: Vec<&str> = offerable(selections)
        .into_iter()
        .filter(|a| {
            a.mandatory || selections.entails(a).is_some() || chosen.iter().any(|c| c == a.key)
        })
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

    /// A `Selections` over the four answer sources, so a test names only the
    /// ones it cares about instead of spelling out four empty slices.
    #[derive(Clone)]
    struct Answers {
        packages: Vec<String>,
        tools: Vec<String>,
        apps: Vec<String>,
        extensions: Vec<String>,
        workflow: Workflow,
    }

    impl Answers {
        fn new() -> Self {
            Self {
                packages: Vec::new(),
                tools: Vec::new(),
                apps: Vec::new(),
                extensions: Vec::new(),
                workflow: Workflow::Wally,
            }
        }
        fn packages(mut self, keys: &[&str]) -> Self {
            self.packages = owned(keys);
            self
        }
        fn tools(mut self, keys: &[&str]) -> Self {
            self.tools = owned(keys);
            self
        }
        fn apps(mut self, keys: &[&str]) -> Self {
            self.apps = owned(keys);
            self
        }
        fn extensions(mut self, keys: &[&str]) -> Self {
            self.extensions = owned(keys);
            self
        }
        fn workflow(mut self, workflow: Workflow) -> Self {
            self.workflow = workflow;
            self
        }
        fn selections(&self) -> Selections<'_> {
            Selections {
                packages: &self.packages,
                tools: &self.tools,
                apps: &self.apps,
                extensions: &self.extensions,
                workflow: self.workflow,
            }
        }
    }

    fn keys_of(artifacts: &[&'static Artifact]) -> Vec<&'static str> {
        artifacts.iter().map(|a| a.key).collect()
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
        let answers = Answers::new();
        let written = keys_of(&resolve(&answers.selections(), &[]));
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
        let answers = Answers::new().tools(&["selene"]);
        let offerable = keys_of(&offerable(&answers.selections()));

        assert!(offerable.contains(&"selene.toml"), "{offerable:?}");
        assert!(!offerable.contains(&"stylua.toml"), "no stylua selected: {offerable:?}");
        assert!(!offerable.contains(&".lute/check.luau"), "no lute: {offerable:?}");
    }

    /// Both requirements, not either: the TestEZ selene definition is
    /// pointless without selene, and meaningless without TestEZ.
    #[test]
    fn an_artifact_with_two_requirements_needs_both() {
        let can_offer = |packages: &[&str], tools: &[&str]| {
            let answers = Answers::new().packages(packages).tools(tools);
            offerable(&answers.selections()).iter().any(|a| a.key == "testez.yml")
        };
        assert!(can_offer(&["testez"], &["selene"]));
        assert!(!can_offer(&["testez"], &[]));
        assert!(!can_offer(&[], &["selene"]));
    }

    #[test]
    fn the_workflow_decides_between_wally_and_modules() {
        for (workflow, expected, absent) in [
            (Workflow::Wally, "wally.toml", "modules"),
            (Workflow::GitSubmodules, "modules", "wally.toml"),
        ] {
            let answers = Answers::new().workflow(workflow);
            let offerable = keys_of(&offerable(&answers.selections()));
            assert!(offerable.contains(&expected), "{workflow:?}: {offerable:?}");
            assert!(!offerable.contains(&absent), "{workflow:?}: {offerable:?}");
        }
    }

    /// Unticking the quality script must drop the CI workflow with it -
    /// otherwise the workflow's first command is missing and every push
    /// fails on a file the user never asked for.
    #[test]
    fn dropping_a_dependency_drops_its_dependent() {
        let answers = Answers::new().tools(&["lute"]);
        let selections = answers.selections();

        let with = keys_of(&resolve(
            &selections,
            &owned(&[".lute/check.luau", ".github/workflows/ci.yml"]),
        ));
        assert!(with.contains(&".github/workflows/ci.yml"), "{with:?}");

        // Same tick for CI, but the script it runs is not chosen.
        let without = keys_of(&resolve(&selections, &owned(&[".github/workflows/ci.yml"])));
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
        // Every subset of a representative selection, over both workflows,
        // and - since entailment forces artifacts in whatever the user said -
        // both the tick-everything and tick-nothing extremes.
        for mask in 0u32..(1 << 4) {
            let mut answers = Answers::new();
            if mask & 1 != 0 {
                answers = answers.packages(&["testez"]);
            }
            if mask & 2 != 0 {
                answers = answers.tools(&["selene", "stylua", "lute", "rojo", "tarmac"]);
            }
            if mask & 4 != 0 {
                answers = answers.apps(&["vscode", "blender"]);
            }
            if mask & 8 != 0 {
                answers = answers.extensions(&["testez-companion"]);
            }

            for workflow in [Workflow::Wally, Workflow::GitSubmodules] {
                let answers = Answers { workflow, ..answers.clone() };
                let selections = answers.selections();
                let everything: Vec<String> =
                    offerable(&selections).iter().map(|a| a.key.to_string()).collect();

                for chosen in [everything, Vec::new()] {
                    let written = resolve(&selections, &chosen);
                    let keys = keys_of(&written);

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
    }

    /// **Every offered artifact must have something that writes it.**
    ///
    /// `tarmac.toml` was added to this catalog with no writer, so the picker
    /// offered it and ticking it did nothing - a silent no-op, which is the
    /// worst kind. Scanning the scaffolder's source for a `writes("key")`
    /// call is crude, and it is the check that catches this: the alternative
    /// is noticing by hand that one of twenty-two entries never appears in
    /// the output.
    ///
    /// Mandatory artifacts are exempt - they are written unconditionally and
    /// have no gate to find.
    #[test]
    fn every_offered_artifact_is_gated_in_the_scaffolder() {
        let scaffolder = include_str!("../commands/new.rs");
        let missing: Vec<&str> = ARTIFACTS
            .iter()
            .filter(|a| !a.mandatory)
            .map(|a| a.key)
            .filter(|key| !scaffolder.contains(&format!("writes(\"{key}\")")))
            .collect();
        assert!(
            missing.is_empty(),
            "offered but never written: {missing:?}"
        );
    }

    // ---------------------------------------------------------------
    // Entailment: the incoherence this half of the module exists for.
    // ---------------------------------------------------------------

    /// **The complaint, as a test.** Pick packages, then be offered the
    /// chance to decline the manifest they install from - and have the
    /// packages silently evaporate when you do.
    ///
    /// Both halves matter: `wally.toml` must not be *asked about* (a
    /// checkbox whose only sane answer is yes is not a question), and it
    /// must be written even if a caller passes a `chosen` list without it.
    #[test]
    fn picking_packages_settles_the_manifest_they_install_from() {
        let answers = Answers::new().packages(&["testez"]);
        let selections = answers.selections();

        assert!(
            !keys_of(&offered(&selections)).contains(&"wally.toml"),
            "must not be offered as a choice: {:?}",
            keys_of(&offered(&selections))
        );
        let (_, why) = entailed(&selections)
            .into_iter()
            .find(|(a, _)| a.key == "wally.toml")
            .expect("reported as entailed, with a reason");
        assert!(why.contains("packages you picked"), "{why}");

        // Ticked nothing at all, which is what unticking every box means.
        let written = keys_of(&resolve(&selections, &[]));
        assert!(written.contains(&"wally.toml"), "{written:?}");
    }

    /// The same shape for the submodule workflow - the manifest differs, the
    /// contradiction does not.
    #[test]
    fn picking_packages_settles_the_submodule_folder_too() {
        let answers = Answers::new()
            .packages(&["testez"])
            .workflow(Workflow::GitSubmodules);
        let written = keys_of(&resolve(&answers.selections(), &[]));
        assert!(written.contains(&"modules"), "{written:?}");
        assert!(!keys_of(&offered(&answers.selections())).contains(&"modules"));
    }

    /// With no packages there is nothing to contradict, so the manifest goes
    /// back to being a real question - an empty `wally.toml` to fill in by
    /// hand is a legitimate thing to want, and "just the Rojo basics" has to
    /// stay reachable.
    #[test]
    fn with_no_packages_the_manifest_is_a_question_again() {
        let answers = Answers::new();
        let selections = answers.selections();
        assert!(keys_of(&offered(&selections)).contains(&"wally.toml"));
        assert!(entailed(&selections).is_empty(), "nothing is settled yet");
        assert_eq!(
            keys_of(&resolve(&selections, &[])),
            ["src", "default.project.json"],
            "declining everything must still be possible"
        );
    }

    /// Pinning a tool is exactly what rokit.toml *is*, so declining it while
    /// pinning tools is asking for the pin without the file holding it -
    /// and `rokit add` would have written the file regardless.
    #[test]
    fn pinning_tools_settles_the_file_that_pins_them() {
        let answers = Answers::new().tools(&["rojo"]);
        let selections = answers.selections();
        assert!(!keys_of(&offered(&selections)).contains(&"rokit.toml"));
        assert!(keys_of(&resolve(&selections, &[])).contains(&"rokit.toml"));

        // No tools: a real choice again.
        let empty = Answers::new();
        assert!(keys_of(&offered(&empty.selections())).contains(&"rokit.toml"));
    }

    /// Both measured against the real binary, and both are why these two are
    /// entailed rather than merely recommended:
    ///
    /// - no `selene.toml` at all: `game`, `script` and `workspace` all report
    ///   `undefined_variable` and selene exits 1, because the default std is
    ///   Lua 5.1.
    /// - `std = "roblox+testez"` with no `testez.yml`: "Could not find all
    ///   standard library files", exit 1, nothing linted at all - src/
    ///   included.
    #[test]
    fn a_pinned_linter_settles_the_config_without_which_it_cannot_run() {
        let answers = Answers::new().packages(&["testez"]).tools(&["selene"]);
        let selections = answers.selections();

        let settled: Vec<&str> = entailed(&selections).iter().map(|(a, _)| a.key).collect();
        assert!(settled.contains(&"selene.toml"), "{settled:?}");
        assert!(settled.contains(&"testez.yml"), "{settled:?}");

        let written = keys_of(&resolve(&selections, &[]));
        assert!(written.contains(&"selene.toml"), "{written:?}");
        assert!(written.contains(&"testez.yml"), "{written:?}");
    }

    /// The other side of the same judgement, and the reason entailment is
    /// per-entry rather than a blanket rule about config files: StyLua has
    /// working defaults, so its config is a preference and stays a question.
    /// Without this contrast, "the tool needs a config" would have swallowed
    /// half the picker.
    #[test]
    fn a_tool_with_working_defaults_keeps_its_config_optional() {
        let answers = Answers::new().tools(&["stylua", "lute", "rojo"]);
        let selections = answers.selections();
        let offered = keys_of(&offered(&selections));

        assert!(offered.contains(&"stylua.toml"), "{offered:?}");
        assert!(offered.contains(&".lute/check.luau"), "{offered:?}");
        assert!(offered.contains(&"sourcemap.json"), "{offered:?}");

        // And declining them is honoured, not quietly overridden.
        let written = keys_of(&resolve(&selections, &[]));
        for key in ["stylua.toml", ".lute/check.luau", "sourcemap.json"] {
            assert!(!written.contains(&key), "{key} was declined: {written:?}");
        }
    }

    /// A config file for an extension the user never installed is the same
    /// class of nonsense in the other direction: it was offered to everyone
    /// who picked TestEZ, and the usual answer was a file nothing reads.
    #[test]
    fn the_companion_config_needs_the_companion_extension() {
        let without = Answers::new().packages(&["testez"]);
        assert!(!keys_of(&offerable(&without.selections())).contains(&"testez-companion.toml"));

        let with = Answers::new()
            .packages(&["testez"])
            .extensions(&["testez-companion"]);
        assert!(keys_of(&resolve(&with.selections(), &[])).contains(&"testez-companion.toml"));
    }

    /// Every artifact is in exactly one of three states, and the picker
    /// shows exactly one of them. An entry that was both offered and settled
    /// would be a checkbox whose answer is discarded - which is the whole
    /// defect this module now exists to prevent.
    #[test]
    fn offered_and_entailed_never_overlap_and_cover_everything_offerable() {
        let answers = Answers::new()
            .packages(&["testez"])
            .tools(&["selene", "stylua", "lute", "rojo", "tarmac"])
            .apps(&["vscode", "blender", "figma"])
            .extensions(&["testez-companion"]);
        let selections = answers.selections();

        let offered = keys_of(&offered(&selections));
        let settled: Vec<&str> = entailed(&selections).iter().map(|(a, _)| a.key).collect();
        for key in &offered {
            assert!(!settled.contains(key), "{key} is both offered and settled");
        }

        let mut covered: Vec<&str> = offered
            .iter()
            .chain(settled.iter())
            .copied()
            .chain(ARTIFACTS.iter().filter(|a| a.mandatory).map(|a| a.key))
            .collect();
        covered.sort_unstable();
        let mut all = keys_of(&offerable(&selections));
        all.sort_unstable();
        assert_eq!(covered, all, "every offerable artifact must be in one bucket");
    }

    /// Entailment is evaluated *before* the picker runs, so it can only read
    /// answers already collected. An `Artifact` condition would be checked
    /// against an empty list and silently never fire - a rule that looks
    /// present in the table and does nothing.
    ///
    /// `tests/.luaurc` was written this way first. The fix was not a smarter
    /// evaluator: it was noticing the file is not a separate decision, and
    /// folding it into `tests`.
    #[test]
    fn no_entailment_depends_on_another_artifact() {
        for artifact in ARTIFACTS {
            for entailment in artifact.entailed_by {
                assert!(
                    !matches!(entailment.when, Requirement::Artifact(_)),
                    "{}'s entailment reads an artifact, which is always empty here",
                    artifact.key
                );
            }
        }
    }

    /// An entailed artifact is declared non-negotiable, so the fixpoint loop
    /// must have no way to drop it. It only drops entries with unmet
    /// *artifact* requirements, so the two must never coexist.
    #[test]
    fn nothing_entailed_can_be_dropped_by_resolution() {
        for artifact in ARTIFACTS.iter().filter(|a| !a.entailed_by.is_empty()) {
            for requirement in artifact.requires {
                assert!(
                    !matches!(requirement, Requirement::Artifact(_)),
                    "{} is entailed but depends on another artifact, so resolution could drop it",
                    artifact.key
                );
            }
        }
    }

    /// Mandatory means unconditional; entailment is a condition. An entry
    /// with both would carry a reason nothing ever reads.
    #[test]
    fn mandatory_artifacts_are_not_also_entailed() {
        for artifact in ARTIFACTS.iter().filter(|a| a.mandatory) {
            assert!(
                artifact.entailed_by.is_empty(),
                "{} is mandatory, so entailment is dead weight",
                artifact.key
            );
        }
    }

    /// The reason is printed straight after the key, so it has to read as a
    /// clause rather than a restated sentence.
    #[test]
    fn every_entailment_reason_reads_as_a_because_clause() {
        for artifact in ARTIFACTS {
            for entailment in artifact.entailed_by {
                let because = entailment.because;
                assert!(!because.is_empty(), "{}", artifact.key);
                assert!(
                    because.chars().next().is_some_and(|c| c.is_lowercase()),
                    "{}: `{because}` should continue the line, not start a sentence",
                    artifact.key
                );
                assert!(
                    !because.ends_with('.'),
                    "{}: `{because}` should not end in a full stop",
                    artifact.key
                );
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
