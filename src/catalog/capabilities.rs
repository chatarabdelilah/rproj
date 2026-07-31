//! What a project *does*, as catalog data.
//!
//! # The level this module exists to add
//!
//! rproj used to ask one decision at three abstraction levels, and never at
//! the level the user thinks in:
//!
//! | level          | the user thinks       | rproj asked          |
//! |----------------|-----------------------|----------------------|
//! | intent         | "I want code linted"  | never                |
//! | implementation | `selene`              | "Tools to pin"       |
//! | artifact       | `selene.toml`         | "Files to generate"  |
//!
//! Only the first row is a decision. The other two derive from it. Asking
//! the bottom two and never the top forced the user to reverse-engineer
//! their own intent into tool names and filenames - twice - and any
//! disagreement between those two answers became a contradiction the CLI
//! then had to patch. (`artifacts::Entailment` was that patch. It made the
//! contradiction legible rather than unrepresentable, which is why it is
//! gone now: this module makes it unrepresentable.)
//!
//! A capability is the unit of choice. It owns an **implementation**, and
//! the implementation owns the tools, packages and artifacts. Swap the
//! implementation and everything below re-derives; the capability does not
//! change.
//!
//! ```text
//! Capability        Testing
//!     -> Implementation   TestEZ  (or jest-lua, or none)
//!         -> Packages     testez
//!         -> Artifacts    tests/, testez.yml, testez-companion.toml
//!         -> Commands     lute test
//! ```
//!
//! # The rule this gives for free
//!
//! **An implementation prompt appears only when a capability has more than
//! one implementation.** Same rule as everywhere else - never ask a question
//! with one answer - and it says exactly when a new prompt is permitted.
//! Today exactly one capability qualifies (`test`), and it is the reason
//! adding jest-lua is one catalog entry rather than a new gate step.

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
    pub fn needs_an_implementation_prompt(&self) -> bool {
        self.implementations.len() > 1
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
        }],
        requires: &[],
        default_selected: true,
    },
    Capability {
        key: "test",
        // Default off, and deliberately: neither runner is rproj's choice to
        // make, and `none` is a valid answer. Once jest-lua lands this is the
        // one capability that asks which.
        outcome: "Write and run tests against your game's own code",
        implementations: &[Implementation {
            key: "testez",
            display: "TestEZ",
            tools: &[],
            packages: &["testez"],
            artifacts: &["tests", "testez.yml", "testez-companion.toml"],
        }],
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
        }],
        requires: &[],
        default_selected: true,
    },
    Capability {
        key: "assets-2d",
        outcome: "Upload images and reference them by name instead of by id",
        implementations: &[Implementation {
            key: "tarmac",
            display: "Tarmac",
            tools: &["tarmac"],
            packages: &[],
            // Both, because they are one pipeline: `figma/exports/` is where
            // designs land and `tarmac.toml` is what uploads them. Splitting
            // them into two checkboxes was two folders that ignored each
            // other.
            artifacts: &["figma", "tarmac.toml"],
        }],
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
        .filter(|c| c.requires.iter().all(|r| chosen.iter().any(|k| k == r)))
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
    let keys: Vec<String> = selected.iter().map(|(k, _)| k.clone()).collect();
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
                let at = at.unwrap_or_else(|| panic!("{} requires unknown {required}", capability.key));
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
    fn exactly_one_capability_offers_a_choice_of_implementation() {
        let with_a_choice: Vec<&str> = CAPABILITIES
            .iter()
            .filter(|c| c.needs_an_implementation_prompt())
            .map(|c| c.key)
            .collect();
        assert!(
            with_a_choice.is_empty() || with_a_choice == ["test"],
            "only `test` may ask which implementation (jest-lua is the peer): {with_a_choice:?}"
        );
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

    /// CI's whole body is the gate script, so without the gate it must
    /// contribute nothing - not a workflow whose first command is missing.
    #[test]
    fn ci_without_the_gate_derives_nothing() {
        let with = derive(&chosen(&["gate", "ci"]));
        assert!(with.artifacts.contains(&".github/workflows/ci.yml".to_string()));

        let without = derive(&chosen(&["ci"]));
        assert!(
            !without.artifacts.contains(&".github/workflows/ci.yml".to_string()),
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
        for key in ["test", "ci", "assets-2d", "assets-3d"] {
            assert!(
                !find(key).expect(key).default_selected,
                "{key} must not be pre-checked"
            );
        }
    }
}
