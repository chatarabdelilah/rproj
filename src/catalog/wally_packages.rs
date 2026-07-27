use super::Maintenance;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    StateManagement,
    Ui,
    DataProfile,
    Testing,
    Utility,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::StateManagement => "State management",
            Category::Ui => "UI",
            Category::DataProfile => "Data & profiles",
            Category::Testing => "Testing",
            Category::Utility => "Utilities",
        }
    }

    pub const ALL: [Category; 5] = [
        Category::StateManagement,
        Category::Ui,
        Category::DataProfile,
        Category::Testing,
        Category::Utility,
    ];

    /// Whether more than one pick makes sense in this category. State
    /// management/UI/data-profile are architecturally exclusive choices (you
    /// don't run two UI frameworks at once), so those stay single-select;
    /// testing and utility libraries are additive toolboxes where wanting
    /// several at once (janitor + promise + greentea, say) is normal.
    pub fn allows_multiple(&self) -> bool {
        matches!(self, Category::Testing | Category::Utility)
    }
}

/// Where a package's source sits inside its cloned repo, for the
/// git-submodule workflow. Verified against each upstream repo's own
/// `default.project.json` rather than assumed - most are a plain `src` (or
/// `lib`), but monorepos like littensy/charm and littensy/ripple publish
/// several packages from one repo, so those point at a specific subpackage
/// folder instead of the repo root.
#[derive(Clone, Copy)]
pub struct Submodule {
    /// Folder name under `modules/submodules/` that this package's repo is
    /// cloned into. Packages sharing a repo share this value, so the repo
    /// is only cloned once. Chosen explicitly rather than derived from the
    /// clone URL's last segment, which would produce names like
    /// `roblox-lua-promise` and inconsistent casing (`Janitor`, `Fusion`).
    pub dir: &'static str,
    /// Path to the requirable source *within* `dir` - the folder holding
    /// the package's `init.luau` (or a single file, for one-file packages).
    pub path: &'static str,
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
///
/// Applies to the Wally workflow only. A git submodule is just a checkout;
/// nothing enforces a realm, and the whole `modules/` tree is mounted in one
/// place regardless.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Realm {
    Shared,
    Server,
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
    /// Git clone URL, used by the git-submodule package workflow instead of
    /// Wally. Some entries share the same repo (e.g. charm/charmSync/
    /// reactCharm/videCharm all live in littensy/charm's `packages/`
    /// directory as a monorepo) - the submodule workflow dedupes by this
    /// URL and clones it once, since Wally's per-subpackage publishing has
    /// no equivalent for a raw git checkout.
    pub git_repo: &'static str,
    /// The instance name this package is mounted under, both in
    /// `modules/submodules/` and as the generated `modules/<name>.luau`
    /// link file. Canonical upstream casing (`Charm`, `CharmSync`, `gt`),
    /// not the catalog key - this is the name written in user code, and
    /// for the monorepo packages it is load-bearing, see `submodule`.
    pub module_name: &'static str,
    /// Where this package's real Luau source lives inside its cloned repo,
    /// for the git-submodule workflow only (Wally resolves its own package
    /// layout and never looks at this). `None` means upstream only ships a
    /// working module through an npm/pnpm install step (react-lua's
    /// `require("@pkg/...")` aliases resolve through `node_modules`, which
    /// a bare git clone never populates), so it can't be vendored as a raw
    /// submodule at all - `pick_package_workflow` falls back to Wally when
    /// one of these is selected.
    pub submodule: Option<Submodule>,
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
    // --- UI ---
    PackageSpec {
        key: "react",
        source: "jsdotlua/react@17.2.1",
        realm: Realm::Shared,
        git_repo: "https://github.com/jsdotlua/react-lua",
        module_name: "React",
        submodule: None,
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
        submodule: None,
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
        submodule: Some(Submodule { dir: "vide", path: "src" }),
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
        submodule: Some(Submodule { dir: "fusion", path: "src" }),
        description: "Reactive UI library with state management built in",
        maintenance: Maintenance::Active,
        category: Category::Ui,
        docs_url: "https://elttob.uk/Fusion/",
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
        submodule: Some(Submodule { dir: "reflex", path: "src" }),
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
        submodule: Some(Submodule { dir: "react-reflex", path: "src" }),
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
        submodule: Some(Submodule { dir: "charm", path: "packages/charm/src" }),
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
        submodule: Some(Submodule { dir: "charm", path: "packages/charm-sync/src" }),
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
        submodule: None,
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
        submodule: Some(Submodule { dir: "charm", path: "packages/vide-charm/src" }),
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
        submodule: Some(Submodule { dir: "lyra", path: "src" }),
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
        submodule: Some(Submodule { dir: "profilestore", path: "ProfileStore.luau" }),
        description: "DataStore session-locking wrapper - the successor to ProfileService, recommended for new projects",
        maintenance: Maintenance::Active,
        category: Category::DataProfile,
        docs_url: "https://madstudioroblox.github.io/ProfileStore/",
        primary_choice: true,
    },
    // --- Testing ---
    PackageSpec {
        key: "testez",
        source: "roblox/testez@0.4.1",
        realm: Realm::Shared,
        git_repo: "https://github.com/Roblox/testez",
        module_name: "TestEZ",
        submodule: Some(Submodule { dir: "testez", path: "src" }),
        description: "Roblox's own BDD-style unit testing framework - archived by Roblox in Sept 2024, no longer receiving updates upstream, but still the most common Wally-installable test framework in existing projects",
        maintenance: Maintenance::Legacy,
        category: Category::Testing,
        docs_url: "https://roblox.github.io/testez/",
        primary_choice: true,
    },
    // --- Utilities ---
    PackageSpec {
        key: "janitor",
        source: "howmanysmall/janitor@1.18.3",
        realm: Realm::Shared,
        git_repo: "https://github.com/howmanysmall/Janitor",
        module_name: "Janitor",
        submodule: Some(Submodule { dir: "janitor", path: "src" }),
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
        submodule: Some(Submodule { dir: "ripple", path: "packages/ripple/src" }),
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
        submodule: None,
        description: "React bindings for Ripple's animation primitives",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://github.com/littensy/ripple",
        primary_choice: false,
    },
    PackageSpec {
        key: "videRipple",
        source: "littensy/vide-ripple@0.10.2",
        realm: Realm::Shared,
        git_repo: "https://github.com/littensy/ripple",
        module_name: "VideRipple",
        submodule: Some(Submodule { dir: "ripple", path: "packages/vide-ripple/src" }),
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
        submodule: Some(Submodule { dir: "remo", path: "src" }),
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
        submodule: Some(Submodule { dir: "promise", path: "lib" }),
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
        submodule: Some(Submodule { dir: "greentea", path: "src" }),
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
        submodule: Some(Submodule { dir: "t", path: "lib" }),
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
        submodule: Some(Submodule { dir: "sift", path: "src" }),
        description: "Immutable data utility library for tables/arrays (Llama-style helpers) - no longer actively maintained upstream, but stable and widely used",
        maintenance: Maintenance::CommunityStable,
        category: Category::Utility,
        docs_url: "https://cxmeel.github.io/sift",
        primary_choice: true,
    },
];

impl PackageSpec {
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
    keys.into_iter().filter_map(|k| find(k)).any(|p| p.realm == Realm::Server)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A server-realm package listed under `[dependencies]` doesn't land in
    /// the wrong folder - wally refuses to resolve it and the install fails,
    /// aborting the whole scaffold. This is what happened to every selection
    /// containing ProfileStore.
    #[test]
    fn profilestore_is_the_server_realm_package() {
        let profilestore = find("profilestore").expect("profilestore is in the catalog");
        assert!(profilestore.realm == Realm::Server, "ProfileStore is published server-realm");

        // Verified by installing all 22 catalog packages together: the
        // install succeeds only with ProfileStore under
        // `[server-dependencies]` and everything else under `[dependencies]`.
        for spec in PACKAGES.iter().filter(|p| p.key != "profilestore") {
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
