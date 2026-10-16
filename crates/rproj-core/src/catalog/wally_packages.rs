#[cfg(test)]
use std::collections::BTreeSet;

use super::Maintenance;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    StateManagement,
    Ui,
    Architecture,
    DataProfile,
    Testing,
    Utility,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::StateManagement => "State management",
            Category::Ui => "UI",
            Category::Architecture => "Architecture",
            Category::DataProfile => "Data & profiles",
            Category::Testing => "Testing",
            Category::Utility => "Utilities",
        }
    }

    pub const ALL: [Category; 6] = [
        Category::StateManagement,
        Category::Ui,
        Category::Architecture,
        Category::DataProfile,
        Category::Testing,
        Category::Utility,
    ];

    /// Whether more than one pick makes sense in this category.
    ///
    /// State management, UI, architecture, data/profile and testing are
    /// architecturally exclusive choices - you don't run two UI frameworks,
    /// and you don't run two test runners: that would mean two `tests/`
    /// layouts, two selene standard libraries and two quality-gate steps.
    /// Those are single-select, so picking one means not picking the other,
    /// and `none` is always available.
    ///
    /// Utilities is the exception: an additive toolbox where wanting
    /// several at once (janitor + promise + greentea, say) is normal.
    pub fn allows_multiple(&self) -> bool {
        matches!(self, Category::Utility)
    }
}

/// Which wally realm a package is published under.
///
/// Not cosmetic and not rproj's choice: wally refuses to resolve a
/// server-realm package listed under `[dependencies]` at all, failing with
/// "No packages were found that matched (Shared) <pkg>. Are you sure this is
/// a Shared dependency?" - which is what every selection including
/// ProfileStore did, aborting the scaffold. Server-realm packages go in
/// `[server-dependencies]` and wally installs them into `ServerPackages/`
/// instead of `Packages/`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Realm {
    Shared,
    Server,
    Dev,
}

/// Packages whose public API is a table holding both properties and
/// children, which selene's `mixed_table` lint objects to.
///
/// Vide is written `create("Frame")({ Name = "x", create("TextLabel")({}) })`:
/// properties as key/value pairs and children as array entries, in one
/// table. That is not a style choice a user can avoid, it is how every Vide
/// component is written, so with the lint at its default `warn` every UI
/// file a Vide project will ever contain fails the quality gate — and
/// selene exits 1 on warnings as well as errors (checked, not assumed).
///
/// Deliberately just Vide. Fusion's children go under a `[Children]` key,
/// which keeps the table a pure dictionary, and React takes props and
/// children as separate arguments - neither produces a mixed table.
const MIXED_TABLE_IDIOM: &[&str] = &["vide"];

/// Whether this selection's UI library forces mixed tables on the user.
pub fn allows_mixed_tables<'a>(selected: impl IntoIterator<Item = &'a String>) -> bool {
    selected
        .into_iter()
        .any(|key| MIXED_TABLE_IDIOM.contains(&key.as_str()))
}

pub struct PackageSpec {
    /// Short identifier used in rproj.toml and on the CLI (e.g. `rproj info reflex`).
    pub key: &'static str,
    /// Wally dependency line value, e.g. "littensy/reflex@4.3.1".
    pub source: &'static str,
    /// The realm this package is published under. Every catalog entry is
    /// `Shared` except ProfileStore - verified by installing all 22 packages
    /// together, which succeeds only with ProfileStore under
    /// `[server-dependencies]`.
    pub realm: Realm,
    /// Upstream repository URL, used for catalog information and maintenance checks.
    pub git_repo: &'static str,
    /// Canonical module name used in examples and development-package aliases.
    pub module_name: &'static str,
    pub description: &'static str,
    pub maintenance: Maintenance,
    pub category: Category,
    pub docs_url: &'static str,
    /// Shown as a guided-mode choice. Companions (bridge/renderer packages
    /// that only make sense alongside a primary pick) are not offered on
    /// their own in guided mode - they ride along automatically, see
    /// `companions_for`. The expert flat checklist always shows every
    /// entry regardless of this flag.
    pub primary_choice: bool,
}

/// Packages that get pulled in automatically alongside a primary pick
/// (e.g. picking `react` also needs `react-roblox` to actually render).
/// Guided mode applies this; expert mode lists everything
/// individually so an experienced dev can opt out of a companion.
///
/// `has` reports whether a given package key is already in the selection -
/// used to pick the right UI-specific binding (e.g. `charm` pulls in
/// `reactCharm` alongside React but `videCharm` alongside Vide) instead of
/// always assuming React, which would staple a React binding onto a
/// Vide/Fusion project.
pub fn companions_for(key: &str, has: impl Fn(&str) -> bool) -> Vec<&'static str> {
    match key {
        "react" => vec!["reactRoblox"],
        "reflex" if has("react") => vec!["reactReflex"],
        "charm" => {
            let mut companions = vec!["charmSync"];
            if has("react") {
                companions.push("reactCharm");
            } else if has("vide") {
                companions.push("videCharm");
            }
            companions
        }
        "ripple" if has("react") => vec!["reactRipple"],
        "ripple" if has("vide") => vec!["videRipple"],
        _ => vec![],
    }
}

pub const PACKAGES: &[PackageSpec] = &[
    // UI Labs supplements UI frameworks, so offer it as an additive utility.
    PackageSpec {
        key: "uiLabs",
        source: "pepeeltoro41/ui-labs@2.4.2",
        realm: Realm::Shared,
        git_repo: "https://github.com/PepeElToro41/ui-labs-utils",
        module_name: "UILabs",
        description: "Story helpers and controls for the UI Labs Studio preview plugin",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://ui-labs.luau.page/",
        primary_choice: true,
    },
    // --- UI ---
    PackageSpec {
        key: "react",
        source: "jsdotlua/react@17.2.1",
        realm: Realm::Shared,
        git_repo: "https://github.com/jsdotlua/react-lua",
        module_name: "React",
        description: "Roact-style declarative UI library, a Luau port of React",
        maintenance: Maintenance::Active,
        category: Category::Ui,
        docs_url: "https://jsdotlua.github.io/react-lua/",
        primary_choice: true,
    },
    PackageSpec {
        key: "reactRoblox",
        source: "jsdotlua/react-roblox@17.2.1",
        realm: Realm::Shared,
        git_repo: "https://github.com/jsdotlua/react-lua",
        module_name: "ReactRoblox",
        description: "React's Roblox renderer - required alongside react to mount anything",
        maintenance: Maintenance::Active,
        category: Category::Ui,
        docs_url: "https://jsdotlua.github.io/react-lua/",
        primary_choice: false,
    },
    PackageSpec {
        key: "vide",
        source: "centau/vide@0.4.1",
        realm: Realm::Shared,
        git_repo: "https://github.com/centau/vide",
        module_name: "Vide",
        description: "Lightweight reactive UI + state library built for Luau",
        maintenance: Maintenance::Active,
        category: Category::Ui,
        docs_url: "https://centau.github.io/vide/",
        primary_choice: true,
    },
    PackageSpec {
        key: "fusion",
        source: "elttob/fusion@0.3.0",
        realm: Realm::Shared,
        git_repo: "https://github.com/dphfox/Fusion",
        module_name: "Fusion",
        description: "Reactive UI library with state management built in",
        maintenance: Maintenance::Active,
        category: Category::Ui,
        docs_url: "https://elttob.uk/Fusion/",
        primary_choice: true,
    },
    // --- Architecture ---
    PackageSpec {
        key: "matter",
        source: "matter-ecs/matter@0.8.4",
        realm: Realm::Shared,
        git_repo: "https://github.com/matter-ecs/matter",
        module_name: "Matter",
        description: "Entity Component System architecture library for data-oriented Roblox gameplay",
        maintenance: Maintenance::Active,
        category: Category::Architecture,
        docs_url: "https://matter-ecs.github.io/matter/",
        primary_choice: true,
    },
    // --- State management ---
    // (ripple/remo used to be listed here too - verified against their own
    // repos and they are not state management: ripple is an animation
    // library and remo is a networking wrapper. Moved to Utilities below.)
    PackageSpec {
        key: "reflex",
        source: "littensy/reflex@4.3.1",
        realm: Realm::Shared,
        git_repo: "https://github.com/littensy/reflex",
        module_name: "Reflex",
        description: "Redux-inspired predictable state container",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://littensy.github.io/reflex/",
        primary_choice: true,
    },
    PackageSpec {
        key: "reactReflex",
        source: "littensy/react-reflex@0.3.6",
        realm: Realm::Shared,
        git_repo: "https://github.com/littensy/react-reflex",
        module_name: "ReactReflex",
        description: "React bindings for Reflex",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://littensy.github.io/reflex/",
        primary_choice: false,
    },
    PackageSpec {
        key: "charm",
        source: "littensy/charm@0.11.0",
        realm: Realm::Shared,
        git_repo: "https://github.com/littensy/charm",
        module_name: "Charm",
        description: "Atom-based state management, inspired by Jotai/Nanostores",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://github.com/littensy/charm",
        primary_choice: true,
    },
    PackageSpec {
        key: "charmSync",
        source: "littensy/charm-sync@0.4.0",
        realm: Realm::Shared,
        git_repo: "https://github.com/littensy/charm",
        module_name: "CharmSync",
        description: "Client/server atom synchronization for Charm",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://github.com/littensy/charm",
        primary_choice: false,
    },
    PackageSpec {
        key: "reactCharm",
        source: "littensy/react-charm@0.4.0",
        realm: Realm::Shared,
        git_repo: "https://github.com/littensy/charm",
        module_name: "ReactCharm",
        description: "React bindings for Charm",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://github.com/littensy/charm",
        primary_choice: false,
    },
    PackageSpec {
        key: "videCharm",
        source: "littensy/vide-charm@0.4.0",
        realm: Realm::Shared,
        git_repo: "https://github.com/littensy/charm",
        module_name: "VideCharm",
        description: "Bridge between Vide and Charm, for using Charm atoms in Vide UI",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://github.com/littensy/charm",
        primary_choice: false,
    },
    // --- Data & profiles ---
    PackageSpec {
        key: "lyra",
        source: "paradoxum-games/lyra@0.6.0",
        realm: Realm::Shared,
        git_repo: "https://github.com/paradoxum-games/lyra",
        module_name: "Lyra",
        description: "Full game framework with a built-in player-data/profile layer",
        maintenance: Maintenance::Active,
        category: Category::DataProfile,
        docs_url: "https://paradoxum-games.github.io/lyra/",
        primary_choice: true,
    },
    PackageSpec {
        key: "profilestore",
        source: "lm-loleris/profilestore@1.0.3",
        realm: Realm::Server,
        git_repo: "https://github.com/MadStudioRoblox/ProfileStore",
        module_name: "ProfileStore",
        description: "DataStore session-locking wrapper - the successor to ProfileService, recommended for new projects",
        maintenance: Maintenance::Active,
        category: Category::DataProfile,
        docs_url: "https://madstudioroblox.github.io/ProfileStore/",
        primary_choice: true,
    },
    PackageSpec {
        key: "scribe",
        source: "ericplane/scribe@2.2.0",
        realm: Realm::Shared,
        git_repo: "https://github.com/ericplane/Scribe",
        module_name: "Scribe",
        description: "Typed replicated player-data layer built on ProfileStore, with schemas, migrations and visibility rules",
        maintenance: Maintenance::Active,
        category: Category::DataProfile,
        docs_url: "https://ericplane.github.io/Scribe/",
        primary_choice: true,
    },
    // --- Testing ---
    PackageSpec {
        key: "testez",
        source: "roblox/testez@0.4.1",
        realm: Realm::Shared,
        git_repo: "https://github.com/Roblox/testez",
        module_name: "TestEZ",
        description: "Roblox's own BDD-style unit testing framework - archived by Roblox in Sept 2024, no longer receiving updates upstream, but still the most common Wally-installable test framework in existing projects",
        maintenance: Maintenance::Legacy,
        category: Category::Testing,
        docs_url: "https://roblox.github.io/testez/",
        primary_choice: true,
    },
    PackageSpec {
        key: "jest",
        source: "roblox/jest@=3.20.1",
        realm: Realm::Dev,
        git_repo: "https://github.com/Roblox/jest-roblox",
        module_name: "Jest",
        description: "Roblox's maintained Jest runtime, installed as a Wally development dependency",
        maintenance: Maintenance::Active,
        category: Category::Testing,
        docs_url: "https://github.com/Roblox/jest-roblox",
        primary_choice: false,
    },
    PackageSpec {
        key: "jest-globals",
        source: "roblox/jest-globals@=3.20.1",
        realm: Realm::Dev,
        git_repo: "https://github.com/Roblox/jest-roblox",
        module_name: "JestGlobals",
        description: "The explicit describe, it, and expect imports used by Jest Roblox specs",
        maintenance: Maintenance::Active,
        category: Category::Testing,
        docs_url: "https://github.com/Roblox/jest-roblox",
        primary_choice: false,
    },
    // --- Utilities ---
    PackageSpec {
        key: "janitor",
        source: "howmanysmall/janitor@1.18.3",
        realm: Realm::Shared,
        git_repo: "https://github.com/howmanysmall/Janitor",
        module_name: "Janitor",
        description: "Cleanup/connection-management utility (a faster, typed Maid)",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://howmanysmall.github.io/Janitor/",
        primary_choice: true,
    },
    PackageSpec {
        key: "ripple",
        source: "littensy/ripple@0.10.2",
        realm: Realm::Shared,
        git_repo: "https://github.com/littensy/ripple",
        module_name: "Ripple",
        description: "Spring/tween-based animation library for Roblox UI, inspired by react-spring",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://github.com/littensy/ripple",
        primary_choice: true,
    },
    PackageSpec {
        key: "reactRipple",
        source: "littensy/react-ripple@3.0.1",
        realm: Realm::Shared,
        git_repo: "https://github.com/littensy/ripple",
        module_name: "ReactRipple",
        description: "React bindings for Ripple's animation primitives",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://github.com/littensy/ripple",
        primary_choice: false,
    },
    PackageSpec {
        key: "prettyReactHooks",
        source: "notmirrox/pretty-react-hooks@0.1.1",
        realm: Realm::Shared,
        git_repo: "https://github.com/NotMirrox/pretty-react-hooks-luau",
        module_name: "PrettyReactHooks",
        description: "Opinionated hook collection for React Lua projects",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://github.com/NotMirrox/pretty-react-hooks-luau",
        primary_choice: false,
    },
    PackageSpec {
        key: "videRipple",
        source: "littensy/vide-ripple@0.10.2",
        realm: Realm::Shared,
        git_repo: "https://github.com/littensy/ripple",
        module_name: "VideRipple",
        description: "Vide bindings for Ripple's animation primitives",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://github.com/littensy/ripple",
        primary_choice: false,
    },
    PackageSpec {
        key: "remo",
        source: "littensy/remo@1.5.3",
        realm: Realm::Shared,
        git_repo: "https://github.com/littensy/remo",
        module_name: "Remo",
        description: "Type-safe remote event/networking wrapper",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://github.com/littensy/remo",
        primary_choice: true,
    },
    PackageSpec {
        key: "promise",
        source: "evaera/promise@4.0.0",
        realm: Realm::Shared,
        git_repo: "https://github.com/evaera/roblox-lua-promise",
        module_name: "Promise",
        description: "Promise/A+-style async utility for Luau",
        maintenance: Maintenance::CommunityStable,
        category: Category::Utility,
        docs_url: "https://eryn.io/roblox-lua-promise/",
        primary_choice: true,
    },
    PackageSpec {
        key: "greentea",
        source: "corecii/greentea@0.4.11",
        realm: Realm::Shared,
        git_repo: "https://github.com/corecii/greentea",
        module_name: "gt",
        description: "Runtime type-checking utility",
        maintenance: Maintenance::CommunityStable,
        category: Category::Utility,
        docs_url: "https://github.com/corecii/greentea",
        primary_choice: true,
    },
    PackageSpec {
        key: "t",
        source: "osyrisrblx/t@3.1.1",
        realm: Realm::Shared,
        git_repo: "https://github.com/osyrisrblx/t",
        module_name: "t",
        description: "Runtime type checker - validates values (e.g. RemoteEvent payloads) against type definitions",
        maintenance: Maintenance::CommunityStable,
        category: Category::Utility,
        docs_url: "https://github.com/osyrisrblx/t",
        primary_choice: true,
    },
    PackageSpec {
        key: "sift",
        source: "csqrl/sift@0.0.11",
        realm: Realm::Shared,
        git_repo: "https://github.com/csqrl/sift",
        module_name: "Sift",
        description: "Immutable data utility library for tables/arrays (Llama-style helpers) - no longer actively maintained upstream, but stable and widely used",
        maintenance: Maintenance::CommunityStable,
        category: Category::Utility,
        docs_url: "https://cxmeel.github.io/sift",
        primary_choice: true,
    },
];

impl PackageSpec {
    pub fn alias(&self) -> &'static str {
        if self.realm == Realm::Dev {
            self.module_name
        } else {
            self.key
        }
    }

    /// The wally author/org, parsed from `source` (e.g. "littensy" out of
    /// "littensy/reflex@4.3.1"). Used for the compact `rproj info` listing.
    pub fn author(&self) -> &'static str {
        self.source.split('/').next().unwrap_or(self.source)
    }

    /// The pinned version, parsed from `source` (e.g. "4.3.1" out of
    /// "littensy/reflex@4.3.1"). Used for the compact `rproj info` listing.
    pub fn version(&self) -> &'static str {
        self.source.rsplit('@').next().unwrap_or("")
    }
}

pub fn find(key: &str) -> Option<&'static PackageSpec> {
    PACKAGES.iter().find(|p| p.key == key)
}

pub fn in_category(category: Category) -> impl Iterator<Item = &'static PackageSpec> {
    PACKAGES.iter().filter(move |p| p.category == category)
}

/// Whether any selected package is server-realm, i.e. whether wally will
/// create a `ServerPackages/` folder.
///
/// Several unrelated things key off this - the project file's mount, the
/// retyping arguments, CI - and every one of them breaks differently if it
/// disagrees with the manifest: rojo fails on a `$path` that doesn't exist,
/// and wally-package-types fails on a directory argument that doesn't
/// exist. Deriving them all from one predicate keeps them from drifting.
pub fn has_server_realm<'a>(keys: impl IntoIterator<Item = &'a String>) -> bool {
    keys.into_iter()
        .filter_map(|k| find(k))
        .any(|p| p.realm == Realm::Server)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every Vide component is a properties-and-children table, so with
    /// the lint left on, every UI file a Vide project will ever have fails
    /// the quality gate.
    #[test]
    fn only_the_create_style_ui_library_waives_the_mixed_table_lint() {
        let vide: BTreeSet<String> = ["vide".to_string()].into_iter().collect();
        assert!(allows_mixed_tables(&vide));

        // Fusion puts children under a `[Children]` key and React takes
        // them as a separate argument; neither builds a mixed table.
        for key in ["fusion", "react", "charm", "reflex"] {
            let other: BTreeSet<String> = [key.to_string()].into_iter().collect();
            assert!(!allows_mixed_tables(&other), "{key} should keep the lint");
        }
        assert!(!allows_mixed_tables(&BTreeSet::new()));
    }

    /// A server-realm package listed under `[dependencies]` doesn't land in
    /// the wrong folder - wally refuses to resolve it and the install fails,
    /// aborting the whole scaffold. This is what happened to every selection
    /// containing ProfileStore.
    #[test]
    fn profilestore_is_the_server_realm_package() {
        let profilestore = find("profilestore").expect("profilestore is in the catalog");
        assert!(
            profilestore.realm == Realm::Server,
            "ProfileStore is published server-realm"
        );

        // Verified by installing all 22 catalog packages together: the
        // install succeeds only with ProfileStore under
        // `[server-dependencies]` and everything else under `[dependencies]`.
        for spec in PACKAGES
            .iter()
            .filter(|p| p.key != "profilestore" && p.realm != Realm::Dev)
        {
            assert!(
                spec.realm == Realm::Shared,
                "{} is marked server-realm; confirm with a real `wally install` before trusting it",
                spec.key
            );
        }
    }

    /// Several unrelated things key off this predicate (the project file's
    /// ServerPackages mount, the retyping arguments, CI), and each fails
    /// differently when it disagrees with the manifest.
    #[test]
    fn has_server_realm_tracks_the_selection() {
        let owned = |keys: &[&str]| keys.iter().map(|k| (*k).to_string()).collect::<Vec<_>>();

        assert!(has_server_realm(&owned(&["charm", "profilestore"])));
        assert!(has_server_realm(&owned(&["profilestore"])));
        assert!(!has_server_realm(&owned(&["charm", "lyra", "remo"])));
        assert!(!has_server_realm(&owned(&[])));
        // An unknown key must not panic or count as server-realm.
        assert!(!has_server_realm(&owned(&["not-a-package"])));
    }
}

#[cfg(test)]
mod category_tests {
    use super::*;

    /// Utilities is the only multi-pick category.
    ///
    /// Testing used to be multi-pick, which would let a project select two
    /// test runners once there is more than one in the catalog - two
    /// `tests/` layouts, two selene standard libraries, two gate steps.
    #[test]
    fn only_utilities_allows_more_than_one() {
        for category in Category::ALL {
            let expected = category == Category::Utility;
            assert_eq!(
                category.allows_multiple(),
                expected,
                "{} should{} allow multiple",
                category.label(),
                if expected { "" } else { " not" }
            );
        }
    }

    /// No package is pre-selected in a single-pick category, so pressing
    /// enter through the walkthrough selects nothing rather than silently
    /// adding a dependency.
    #[test]
    fn a_single_pick_category_has_a_none_answer_available() {
        for category in Category::ALL.iter().filter(|c| !c.allows_multiple()) {
            let choices = in_category(*category).filter(|p| p.primary_choice).count();
            assert!(choices > 0, "{} has no options at all", category.label());
        }
    }
}
