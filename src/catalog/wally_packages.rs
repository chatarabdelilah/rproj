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
}

pub struct PackageSpec {
    /// Short identifier used in rproj.toml and on the CLI (e.g. `rproj info reflex`).
    pub key: &'static str,
    /// Wally dependency line value, e.g. "littensy/reflex@4.3.1".
    pub source: &'static str,
    pub description: &'static str,
    pub maintenance: Maintenance,
    pub category: Category,
    pub docs_url: &'static str,
    /// Shown as a guided-mode / preset choice. Companions (bridge/renderer
    /// packages that only make sense alongside a primary pick) are not
    /// offered on their own in guided mode - they ride along automatically,
    /// see `companions_for`. The expert flat checklist always shows every
    /// entry regardless of this flag.
    pub primary_choice: bool,
}

/// Packages that get pulled in automatically alongside a primary pick
/// (e.g. picking `react` also needs `react-roblox` to actually render).
/// Guided mode and presets apply this; expert mode lists everything
/// individually so an experienced dev can opt out of a companion.
pub fn companions_for(key: &str) -> &'static [&'static str] {
    match key {
        "react" => &["reactRoblox"],
        "reflex" => &["reactReflex"],
        "ripple" => &["reactRipple"],
        "charm" => &["charmSync", "reactCharm"],
        _ => &[],
    }
}

pub const PACKAGES: &[PackageSpec] = &[
    // --- UI ---
    PackageSpec {
        key: "react",
        source: "jsdotlua/react@17.2.1",
        description: "Roact-style declarative UI library, a Luau port of React",
        maintenance: Maintenance::Active,
        category: Category::Ui,
        docs_url: "https://jsdotlua.github.io/react-lua/",
        primary_choice: true,
    },
    PackageSpec {
        key: "reactRoblox",
        source: "jsdotlua/react-roblox@17.2.1",
        description: "React's Roblox renderer - required alongside react to mount anything",
        maintenance: Maintenance::Active,
        category: Category::Ui,
        docs_url: "https://jsdotlua.github.io/react-lua/",
        primary_choice: false,
    },
    PackageSpec {
        key: "vide",
        source: "centau/vide@0.4.1",
        description: "Lightweight reactive UI + state library built for Luau",
        maintenance: Maintenance::Active,
        category: Category::Ui,
        docs_url: "https://centau.github.io/vide/",
        primary_choice: true,
    },
    PackageSpec {
        key: "fusion",
        source: "elttob/fusion@0.3.0",
        description: "Reactive UI library with state management built in",
        maintenance: Maintenance::Active,
        category: Category::Ui,
        docs_url: "https://elttob.uk/Fusion/",
        primary_choice: true,
    },
    // --- State management ---
    PackageSpec {
        key: "reflex",
        source: "littensy/reflex@4.3.1",
        description: "Redux-inspired predictable state container",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://littensy.github.io/reflex/",
        primary_choice: true,
    },
    PackageSpec {
        key: "reactReflex",
        source: "littensy/react-reflex@0.3.6",
        description: "React bindings for Reflex",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://littensy.github.io/reflex/",
        primary_choice: false,
    },
    PackageSpec {
        key: "ripple",
        source: "littensy/ripple@0.10.2",
        description: "Fine-grained reactive state library",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://ripple.littens.dev/",
        primary_choice: true,
    },
    PackageSpec {
        key: "reactRipple",
        source: "littensy/react-ripple@3.0.1",
        description: "React bindings for Ripple",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://ripple.littens.dev/",
        primary_choice: false,
    },
    PackageSpec {
        key: "charm",
        source: "littensy/charm@0.11.0",
        description: "Atom-based state management, inspired by Jotai/Nanostores",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://charm.littens.dev/",
        primary_choice: true,
    },
    PackageSpec {
        key: "charmSync",
        source: "littensy/charm-sync@0.4.0",
        description: "Client/server atom synchronization for Charm",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://charm.littens.dev/",
        primary_choice: false,
    },
    PackageSpec {
        key: "reactCharm",
        source: "littensy/react-charm@0.4.0",
        description: "React bindings for Charm",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://charm.littens.dev/",
        primary_choice: false,
    },
    PackageSpec {
        key: "videCharm",
        source: "littensy/vide-charm@0.4.0",
        description: "Bridge between Vide and Charm, for using Charm atoms in Vide UI",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://charm.littens.dev/",
        primary_choice: false,
    },
    PackageSpec {
        key: "videRipple",
        source: "littensy/vide-ripple@0.10.2",
        description: "Bridge between Vide and Ripple, for using Ripple stores in Vide UI",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://ripple.littens.dev/",
        primary_choice: false,
    },
    PackageSpec {
        key: "remo",
        source: "littensy/remo@1.5.3",
        description: "Type-safe remote event/networking wrapper",
        maintenance: Maintenance::Active,
        category: Category::StateManagement,
        docs_url: "https://littensy.github.io/remo/",
        primary_choice: true,
    },
    // --- Data & profiles ---
    PackageSpec {
        key: "lyra",
        source: "paradoxum-games/lyra@0.6.0",
        description: "Full game framework with a built-in player-data/profile layer",
        maintenance: Maintenance::Active,
        category: Category::DataProfile,
        docs_url: "https://paradoxum-games.github.io/lyra/",
        primary_choice: true,
    },
    PackageSpec {
        key: "profilestore",
        source: "lm-loleris/profilestore@1.0.3",
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
        description: "Roblox's own unit testing framework",
        maintenance: Maintenance::CommunityStable,
        category: Category::Testing,
        docs_url: "https://roblox.github.io/testez/",
        primary_choice: true,
    },
    // --- Utilities ---
    PackageSpec {
        key: "janitor",
        source: "howmanysmall/janitor@1.18.3",
        description: "Cleanup/connection-management utility (a faster, typed Maid)",
        maintenance: Maintenance::Active,
        category: Category::Utility,
        docs_url: "https://howmanysmall.github.io/Janitor/",
        primary_choice: true,
    },
    PackageSpec {
        key: "promise",
        source: "evaera/promise@4.0.0",
        description: "Promise/A+-style async utility for Luau",
        maintenance: Maintenance::CommunityStable,
        category: Category::Utility,
        docs_url: "https://eryn.io/roblox-lua-promise/",
        primary_choice: true,
    },
    PackageSpec {
        key: "greentea",
        source: "corecii/greentea@0.4.11",
        description: "Runtime type-checking utility",
        maintenance: Maintenance::CommunityStable,
        category: Category::Utility,
        docs_url: "https://github.com/corecii/greentea",
        primary_choice: true,
    },
];

pub fn find(key: &str) -> Option<&'static PackageSpec> {
    PACKAGES.iter().find(|p| p.key == key)
}

pub fn in_category(category: Category) -> impl Iterator<Item = &'static PackageSpec> {
    PACKAGES.iter().filter(move |p| p.category == category)
}
