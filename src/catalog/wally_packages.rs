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

pub struct PackageSpec {
    /// Short identifier used in rproj.toml and on the CLI (e.g. `rproj info reflex`).
    pub key: &'static str,
    /// Wally dependency line value, e.g. "littensy/reflex@4.3.1".
    pub source: &'static str,
    /// Git clone URL, used by the git-submodule package workflow instead of
    /// Wally. Some entries share the same repo (e.g. charm/charmSync/
    /// reactCharm/videCharm all live in littensy/charm's `packages/`
    /// directory as a monorepo) - the submodule workflow dedupes by this
    /// URL and clones it once, since Wally's per-subpackage publishing has
    /// no equivalent for a raw git checkout.
    pub git_repo: &'static str,
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
        git_repo: "https://github.com/jsdotlua/react-lua",
        description: "Roact-style declarative UI library, a Luau port of React",
        maintenance: Maintenance::Active,
        category: Category::Ui,
        docs_url: "https://jsdotlua.github.io/react-lua/",
        primary_choice: true,
    },
    PackageSpec {
        key: "reactRoblox",
        source: "jsdotlua/react-roblox@17.2.1",
        git_repo: "https://github.com/jsdotlua/react-lua",
        description: "React's Roblox renderer - required alongside react to mount anything",
        maintenance: Maintenance::Active,
        category: Category::Ui,
        docs_url: "https://jsdotlua.github.io/react-lua/",
        primary_choice: false,
    },
    PackageSpec {
        key: "vide",
        source: "centau/vide@0.4.1",
        git_repo: "https://github.com/centau/vide",
        description: "Lightweight reactive UI + state library built for Luau",
        maintenance: Maintenance::Active,
        category: Category::Ui,
        docs_url: "https://centau.github.io/vide/",
        primary_choice: true,
    },
    PackageSpec {
        key: "fusion",
        source: "elttob/fusion@0.3.0",
        git_repo: "https://github.com/dphfox/Fusion",
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
        git_repo: "https://github.com/littensy/reflex",
        description: "Redux-inspired predictable state container",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://littensy.github.io/reflex/",
        primary_choice: true,
    },
    PackageSpec {
        key: "reactReflex",
        source: "littensy/react-reflex@0.3.6",
        git_repo: "https://github.com/littensy/react-reflex",
        description: "React bindings for Reflex",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://littensy.github.io/reflex/",
        primary_choice: false,
    },
    PackageSpec {
        key: "charm",
        source: "littensy/charm@0.11.0",
        git_repo: "https://github.com/littensy/charm",
        description: "Atom-based state management, inspired by Jotai/Nanostores",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://github.com/littensy/charm",
        primary_choice: true,
    },
    PackageSpec {
        key: "charmSync",
        source: "littensy/charm-sync@0.4.0",
        git_repo: "https://github.com/littensy/charm",
        description: "Client/server atom synchronization for Charm",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://github.com/littensy/charm",
        primary_choice: false,
    },
    PackageSpec {
        key: "reactCharm",
        source: "littensy/react-charm@0.4.0",
        git_repo: "https://github.com/littensy/charm",
        description: "React bindings for Charm",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://github.com/littensy/charm",
        primary_choice: false,
    },
    PackageSpec {
        key: "videCharm",
        source: "littensy/vide-charm@0.4.0",
        git_repo: "https://github.com/littensy/charm",
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
        git_repo: "https://github.com/paradoxum-games/lyra",
        description: "Full game framework with a built-in player-data/profile layer",
        maintenance: Maintenance::Active,
        category: Category::DataProfile,
        docs_url: "https://paradoxum-games.github.io/lyra/",
        primary_choice: true,
    },
    PackageSpec {
        key: "profilestore",
        source: "lm-loleris/profilestore@1.0.3",
        git_repo: "https://github.com/MadStudioRoblox/ProfileStore",
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
        git_repo: "https://github.com/Roblox/testez",
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
        git_repo: "https://github.com/howmanysmall/Janitor",
        description: "Cleanup/connection-management utility (a faster, typed Maid)",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://howmanysmall.github.io/Janitor/",
        primary_choice: true,
    },
    PackageSpec {
        key: "ripple",
        source: "littensy/ripple@0.10.2",
        git_repo: "https://github.com/littensy/ripple",
        description: "Spring/tween-based animation library for Roblox UI, inspired by react-spring",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://github.com/littensy/ripple",
        primary_choice: true,
    },
    PackageSpec {
        key: "reactRipple",
        source: "littensy/react-ripple@3.0.1",
        git_repo: "https://github.com/littensy/ripple",
        description: "React bindings for Ripple's animation primitives",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://github.com/littensy/ripple",
        primary_choice: false,
    },
    PackageSpec {
        key: "videRipple",
        source: "littensy/vide-ripple@0.10.2",
        git_repo: "https://github.com/littensy/ripple",
        description: "Vide bindings for Ripple's animation primitives",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://github.com/littensy/ripple",
        primary_choice: false,
    },
    PackageSpec {
        key: "remo",
        source: "littensy/remo@1.5.3",
        git_repo: "https://github.com/littensy/remo",
        description: "Type-safe remote event/networking wrapper",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://github.com/littensy/remo",
        primary_choice: true,
    },
    PackageSpec {
        key: "promise",
        source: "evaera/promise@4.0.0",
        git_repo: "https://github.com/evaera/roblox-lua-promise",
        description: "Promise/A+-style async utility for Luau",
        maintenance: Maintenance::CommunityStable,
        category: Category::Utility,
        docs_url: "https://eryn.io/roblox-lua-promise/",
        primary_choice: true,
    },
    PackageSpec {
        key: "greentea",
        source: "corecii/greentea@0.4.11",
        git_repo: "https://github.com/corecii/greentea",
        description: "Runtime type-checking utility",
        maintenance: Maintenance::CommunityStable,
        category: Category::Utility,
        docs_url: "https://github.com/corecii/greentea",
        primary_choice: true,
    },
    PackageSpec {
        key: "t",
        source: "osyrisrblx/t@3.1.1",
        git_repo: "https://github.com/osyrisrblx/t",
        description: "Runtime type checker - validates values (e.g. RemoteEvent payloads) against type definitions",
        maintenance: Maintenance::CommunityStable,
        category: Category::Utility,
        docs_url: "https://github.com/osyrisrblx/t",
        primary_choice: true,
    },
    PackageSpec {
        key: "sift",
        source: "csqrl/sift@0.0.11",
        git_repo: "https://github.com/csqrl/sift",
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

    /// The folder name to use under `Modules/` for the git-submodule package
    /// workflow, derived from `git_repo`'s last path segment (e.g. "charm"
    /// out of "https://github.com/littensy/charm").
    pub fn repo_folder_name(&self) -> &'static str {
        self.git_repo.rsplit('/').next().unwrap_or(self.git_repo)
    }
}

pub fn find(key: &str) -> Option<&'static PackageSpec> {
    PACKAGES.iter().find(|p| p.key == key)
}

pub fn in_category(category: Category) -> impl Iterator<Item = &'static PackageSpec> {
    PACKAGES.iter().filter(move |p| p.category == category)
}
