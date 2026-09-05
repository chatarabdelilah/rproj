//! Capability catalog: user intent maps to concrete tools, packages, and
//! generated artifacts. See `docs/architecture.md` §8.10 for the rationale.

use crate::config::PackageWorkflow;

/// One way of providing a capability.
///
/// Not all implementations are the same kind of thing: TestEZ is a Wally
/// package, Selene is a rokit tool, GitHub Actions is neither - it is a
/// hosted service the artifact targets. So this points at whichever the
/// capability needs, and the catalogs stay flat inventories that
/// capabilities reference into.
pub struct Implementation {
    /// Stable key, and what `rproj info` looks up.
    pub key: &'static str,
    /// Shown in the picker's badge slot, so the user always sees what they
    /// are actually enabling. A capability that hides its tool teaches the
    /// user nothing about the ecosystem they are now in.
    pub display: &'static str,
    /// Rokit tool keys this pins into the project.
    pub tools: &'static [&'static str],
    /// Wally package keys this adds to the manifest.
    pub packages: &'static [&'static str],
    /// Artifact keys this writes. The only place the capability -> artifact
    /// edge is recorded; `artifacts` does not name capabilities back, so the
    /// two cannot disagree.
    pub artifacts: &'static [&'static str],
    /// Dependency workflows under which this implementation can run.
    pub workflows: &'static [PackageWorkflow],
}

pub struct Capability {
    pub key: &'static str,
    /// What the user gets, in their words. Not "configures Selene".
    pub outcome: &'static str,
    /// Ordered; the first is the default when more than one exists.
    pub implementations: &'static [Implementation],
    /// Other capability keys that must also be on. A capability whose
    /// requirement is off is not offered at all.
    pub requires: &'static [&'static str],
    pub default_selected: bool,
}

fn chosen_has(chosen: &[String], key: &str) -> bool {
    chosen.iter().any(|candidate| candidate == key)
}

impl Capability {
    /// The implementation in use. Single-implementation capabilities never
    /// prompt, so this is the whole answer for all but one entry today.
    pub fn default_implementation(&self) -> &'static Implementation {
        &self.implementations[0]
    }

    pub fn implementation(&self, key: &str) -> Option<&'static Implementation> {
        self.implementations.iter().find(|i| i.key == key)
    }

    /// Whether choosing this capability is also choosing *how*.
    pub fn implementations_for(&self, workflow: PackageWorkflow) -> Vec<&'static Implementation> {
        self.implementations
            .iter()
            .filter(|implementation| implementation.workflows.contains(&workflow))
            .collect()
    }

    pub fn needs_an_implementation_prompt(&self, workflow: PackageWorkflow) -> bool {
        self.implementations_for(workflow).len() > 1
    }
}

/// In picker order: the checks a project runs, then how it is tested, then
/// what automates it, then the editor, then asset pipelines. Roughly the
/// order a project acquires them.
pub const CAPABILITIES: &[Capability] = &[
    Capability {
        key: "lint",
        outcome: "Catch bugs and risky patterns before they ship",
        implementations: &[Implementation {
            key: "selene",
            display: "Selene",
            tools: &["selene"],
            packages: &[],
            artifacts: &["selene.toml"],
            workflows: PackageWorkflow::ALL,
        }],
        requires: &[],
        default_selected: true,
    },
    Capability {
        key: "format",
        outcome: "One consistent code style, applied automatically",
        implementations: &[Implementation {
            key: "stylua",
            display: "StyLua",
            tools: &["stylua"],
            packages: &[],
            // `.gitattributes` is here rather than in housekeeping because
            // it exists for exactly one reason: StyLua formats to LF, Git
            // for Windows checks out CRLF, and without it every fresh clone
            // fails `stylua --check` on every file. No formatter, no need.
            artifacts: &["stylua.toml", ".gitattributes"],
            workflows: PackageWorkflow::ALL,
        }],
        requires: &[],
        default_selected: true,
    },
    Capability {
        key: "typecheck",
        outcome: "Strict Luau, so type errors are errors and not surprises",
        implementations: &[Implementation {
            key: "luau-lsp",
            display: "luau-lsp",
            // The rokit key is `luau-lsp-cli`; `luau-lsp` alone is the VS
            // Code extension. A plausible-looking guess picks the wrong
            // entry, so the catalog spells it out.
            tools: &["luau-lsp-cli"],
            packages: &[],
            artifacts: &[".luaurc"],
            workflows: PackageWorkflow::ALL,
        }],
        requires: &[],
        default_selected: true,
    },
    Capability {
        key: "test",
        // Default off, deliberately: a project without tests is valid, and
        // once jest-lua lands the runner becomes an implementation choice.
        outcome: "Write and run tests against your game's own code",
        implementations: &[
            // Kept first as the compatibility fallback for old or manually
            // incomplete rproj.toml files. The picker sorts independently.
            Implementation {
                key: "testez",
                display: "TestEZ",
                tools: &[],
                packages: &["testez"],
                artifacts: &[
                    "tests",
                    "test-examples",
                    "testez.yml",
                    "testez-companion.toml",
                ],
                workflows: PackageWorkflow::ALL,
            },
            Implementation {
                key: "jest-roblox",
                display: "Jest Roblox",
                tools: &["jest-roblox"],
                packages: &["jest", "jest-globals"],
                artifacts: &[
                    "tests",
                    "test-examples",
                    "jest.project.json",
                    "jest.config.json",
                ],
                workflows: &[PackageWorkflow::Wally],
            },
        ],
        requires: &[],
        default_selected: false,
    },
    Capability {
        key: "gate",
        outcome: "One command that runs every check above, in one pass",
        implementations: &[Implementation {
            key: "lute",
            display: "Lute",
            tools: &["lute"],
            packages: &[],
            artifacts: &[".lute/check.luau"],
            workflows: PackageWorkflow::ALL,
        }],
        requires: &[],
        default_selected: true,
    },
    Capability {
        key: "ci",
        // Off by default: the one capability that changes what happens on a
        // push, and opting into that should be a decision rather than
        // something a scaffolder assumes.
        outcome: "Run that same gate on GitHub for every push (default off)",
        implementations: &[Implementation {
            key: "github-actions",
            display: "GitHub Actions",
            tools: &[],
            packages: &[],
            artifacts: &[".github/workflows/ci.yml"],
            workflows: PackageWorkflow::ALL,
        }],
        // Not merely "better with": the workflow's entire body is the gate
        // script. Without it the first command of every CI run is missing.
        requires: &["gate"],
        default_selected: false,
    },
    Capability {
        key: "editor",
        outcome: "VS Code resolves requires and sees what you build in Studio",
        implementations: &[Implementation {
            key: "vscode",
            display: "VS Code + luau-lsp",
            // rojo generates the sourcemap luau-lsp reads.
            tools: &["rojo"],
            packages: &[],
            artifacts: &[".vscode/settings.json", "sourcemap.json"],
            workflows: PackageWorkflow::ALL,
        }],
        requires: &[],
        default_selected: true,
    },
    Capability {
        key: "asset-pipeline",
        outcome: "Upload Roblox assets and reference them by name instead of by id",
        implementations: &[
            Implementation {
                key: "asphalt",
                display: "Asphalt",
                tools: &["asphalt"],
                packages: &[],
                artifacts: &["figma", "asphalt.toml"],
                workflows: PackageWorkflow::ALL,
            },
            Implementation {
                key: "tungsten",
                display: "Tungsten",
                tools: &["tungsten"],
                packages: &[],
                artifacts: &["figma", "tungsten.toml"],
                workflows: PackageWorkflow::ALL,
            },
        ],
        requires: &[],
        default_selected: false,
    },
    Capability {
        key: "assets-3d",
        outcome: "A Blender scene at Roblox's unit scale (1 stud = 0.28 m)",
        implementations: &[Implementation {
            key: "blender",
            display: "Blender",
            tools: &[],
            packages: &[],
            artifacts: &["blender"],
            workflows: PackageWorkflow::ALL,
        }],
        requires: &[],
        default_selected: false,
    },
];

pub fn find(key: &str) -> Option<&'static Capability> {
    CAPABILITIES.iter().find(|c| c.key == key)
}

/// Capabilities whose own requirements are met by `chosen`, in catalog
/// order.
///
/// Requirements are between capabilities only, so this is a single pass
/// rather than a fixpoint: `requires` names entries earlier in the list and
/// a test holds that ordering.
pub fn offerable(chosen: &[String]) -> Vec<&'static Capability> {
    CAPABILITIES
        .iter()
        .filter(|c| c.requires.iter().all(|r| chosen_has(chosen, r)))
        .collect()
}

/// What a set of chosen capabilities derives: the tools to pin, the packages
/// to add, and the artifacts to write.
///
/// One function because the three always travel together - deriving them
/// separately is how the project's `rokit.toml` and its check script came to
/// disagree about which tools existed.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Derived {
    pub tools: Vec<String>,
    pub packages: Vec<String>,
    pub artifacts: Vec<String>,
}

/// `selected` pairs a capability key with the implementation key in use.
/// Passing `None` for the implementation takes the default, which is the
/// whole answer for every capability that has only one.
pub fn derive(selected: &[(String, Option<String>)]) -> Derived {
    let mut out = Derived::default();
    // Requirements first: a capability whose requirement was dropped
    // contributes nothing, or CI would still write its workflow after the
    // gate that workflow runs was turned off.
    let keys: Vec<String> = selected.iter().map(|(k, _)| k.to_string()).collect();
    let live = offerable(&keys);

    for (key, implementation) in selected {
        let Some(capability) = live.iter().find(|c| c.key == key) else {
            continue;
        };
        let implementation = match implementation {
            Some(i) => match capability.implementation(i) {
                Some(found) => found,
                // An unknown implementation key means a stale `rproj.toml`
                // or a hand-edit. Falling back beats failing the scaffold.
                None => capability.default_implementation(),
            },
            None => capability.default_implementation(),
        };
        push_new(&mut out.tools, implementation.tools);
        push_new(&mut out.packages, implementation.packages);
        push_new(&mut out.artifacts, implementation.artifacts);
    }
    out
}

/// Appends without duplicating. Two capabilities can want the same tool -
/// `rojo` is the obvious future case - and a doubled `rokit add` is a
/// confusing line of output for no reason.
fn push_new(into: &mut Vec<String>, items: &[&str]) {
    for item in items {
        if !into.iter().any(|existing| existing == item) {
            into.push((*item).to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chosen(keys: &[&str]) -> Vec<(String, Option<String>)> {
        keys.iter().map(|k| (k.to_string(), None)).collect()
    }

    #[test]
    fn every_key_is_unique() {
        let mut keys: Vec<&str> = CAPABILITIES.iter().map(|c| c.key).collect();
        let before = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(before, keys.len(), "duplicate capability key");
    }

    #[test]
    fn every_capability_has_at_least_one_implementation() {
        for capability in CAPABILITIES {
            assert!(
                !capability.implementations.is_empty(),
                "{} has no implementation, so choosing it would do nothing",
                capability.key
            );
        }
    }

    /// A requirement naming a capability that does not exist would silently
    /// make its dependent unofferable forever.
    #[test]
    fn every_requirement_resolves_and_points_backwards() {
        for (i, capability) in CAPABILITIES.iter().enumerate() {
            for required in capability.requires {
                let at = CAPABILITIES.iter().position(|c| c.key == *required);
                let at =
                    at.unwrap_or_else(|| panic!("{} requires unknown {required}", capability.key));
                // Backwards-only keeps `offerable` a single pass. A forward
                // edge would need a fixpoint, and nothing here wants one.
                assert!(
                    at < i,
                    "{} requires {required}, which is listed after it",
                    capability.key
                );
            }
        }
    }

    /// **The rule the model exists to give.** An implementation prompt is
    /// permitted exactly when a capability has a real choice to offer.
    /// Today one does; if this count changes, a prompt appears or vanishes,
    /// and that should be a deliberate edit rather than a surprise.
    #[test]
    fn exactly_two_capabilities_offer_a_choice_of_implementation() {
        let with_a_choice: Vec<&str> = CAPABILITIES
            .iter()
            .filter(|c| c.implementations.len() > 1)
            .map(|c| c.key)
            .collect();
        assert_eq!(with_a_choice, ["test", "asset-pipeline"]);
    }

    #[test]
    fn jest_is_only_available_with_wally() {
        let testing = find("test").unwrap();
        assert_eq!(testing.implementations_for(PackageWorkflow::Wally).len(), 2);
        for workflow in [PackageWorkflow::GitSubmodules, PackageWorkflow::None] {
            let implementations = testing.implementations_for(workflow);
            assert_eq!(implementations.len(), 1);
            assert_eq!(implementations[0].key, "testez");
        }
    }

    #[test]
    fn jest_derives_its_runner_packages_and_managed_files() {
        let derived = derive(&[("test".into(), Some("jest-roblox".into()))]);
        assert_eq!(derived.tools, ["jest-roblox"]);
        assert_eq!(derived.packages, ["jest", "jest-globals"]);
        assert!(derived.artifacts.contains(&"jest.project.json".into()));
        assert!(!derived.artifacts.contains(&"testez.yml".into()));
    }

    /// The picker shows the tool in the badge slot, so it has to be there.
    #[test]
    fn every_implementation_names_itself() {
        for capability in CAPABILITIES {
            for implementation in capability.implementations {
                assert!(!implementation.display.is_empty(), "{}", capability.key);
                assert!(!implementation.key.is_empty(), "{}", capability.key);
            }
        }
    }

    /// A capability that derives nothing is a checkbox with no effect.
    #[test]
    fn every_implementation_derives_something() {
        for capability in CAPABILITIES {
            for implementation in capability.implementations {
                assert!(
                    !implementation.artifacts.is_empty()
                        || !implementation.tools.is_empty()
                        || !implementation.packages.is_empty(),
                    "{}/{} derives nothing",
                    capability.key,
                    implementation.key
                );
            }
        }
    }

    // ---------------------------------------------------------------
    // Derivation.
    // ---------------------------------------------------------------

    #[test]
    fn choosing_lint_derives_selene_and_its_config() {
        let derived = derive(&chosen(&["lint"]));
        assert_eq!(derived.tools, ["selene"]);
        assert_eq!(derived.artifacts, ["selene.toml"]);
        assert!(derived.packages.is_empty());
    }

    /// The user never picks a package here; the capability does. That is
    /// what stops "do I want tests" being asked in the package step and
    /// answered again in the files step.
    #[test]
    fn choosing_test_derives_the_package_not_just_the_files() {
        let derived = derive(&chosen(&["test"]));
        assert_eq!(derived.packages, ["testez"]);
        assert!(derived.artifacts.contains(&"tests".to_string()));
        assert!(derived.artifacts.contains(&"testez.yml".to_string()));
    }

    #[test]
    fn choosing_an_asset_pipeline_implementation_derives_that_tool_and_config() {
        let derived = derive(&[("asset-pipeline".to_string(), Some("asphalt".to_string()))]);
        assert_eq!(derived.tools, ["asphalt"]);
        assert_eq!(derived.artifacts, ["figma", "asphalt.toml"]);
    }

    /// CI's whole body is the gate script, so without the gate it must
    /// contribute nothing - not a workflow whose first command is missing.
    #[test]
    fn ci_without_the_gate_derives_nothing() {
        let with = derive(&chosen(&["gate", "ci"]));
        assert!(
            with.artifacts
                .contains(&".github/workflows/ci.yml".to_string())
        );

        let without = derive(&chosen(&["ci"]));
        assert!(
            !without
                .artifacts
                .contains(&".github/workflows/ci.yml".to_string()),
            "{without:?}"
        );
        assert!(!offerable(&["ci".to_string()]).iter().any(|c| c.key == "ci"));
    }

    /// Nothing chosen is a valid answer, and it has to derive nothing at
    /// all - this is what keeps "just the Rojo basics" reachable.
    #[test]
    fn choosing_nothing_derives_nothing() {
        assert_eq!(derive(&[]), Derived::default());
    }

    #[test]
    fn a_tool_wanted_by_two_capabilities_is_pinned_once() {
        let mut derived = Derived::default();
        push_new(&mut derived.tools, &["rojo", "selene"]);
        push_new(&mut derived.tools, &["rojo"]);
        assert_eq!(derived.tools, ["rojo", "selene"]);
    }

    /// A stale `rproj.toml` naming an implementation that no longer exists
    /// must not take the scaffold down with it.
    #[test]
    fn an_unknown_implementation_falls_back_to_the_default() {
        let selected = vec![("lint".to_string(), Some("clippy".to_string()))];
        assert_eq!(derive(&selected).tools, ["selene"]);
    }

    /// The defaults are what a user gets by pressing enter, so the ones
    /// deliberately off must stay off. `test` is here because neither
    /// runner is rproj's choice to make.
    #[test]
    fn the_deliberately_off_capabilities_are_off() {
        for key in ["test", "ci", "asset-pipeline", "assets-3d"] {
            assert!(
                !find(key).expect(key).default_selected,
                "{key} must not be pre-checked"
            );
        }
    }
}
