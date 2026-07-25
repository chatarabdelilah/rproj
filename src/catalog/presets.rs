pub struct Preset {
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// Package keys from `wally_packages::PACKAGES`. Companions are still
    /// applied automatically on top of this list, same as guided mode.
    pub packages: &'static [&'static str],
}

/// A starting set of curated bundles. Expanding this list is ongoing content
/// work, not something this planning/implementation pass tries to be exhaustive about.
pub const PRESETS: &[Preset] = &[
    Preset {
        key: "react-reflex-lyra",
        label: "React + Reflex + Lyra",
        description: "The most common littensy-style stack: React for UI, Reflex for state, Lyra for data/profiles",
        packages: &["react", "reflex", "lyra", "testez", "janitor", "promise"],
    },
    Preset {
        key: "vide-charm-minimal",
        label: "Vide + Charm, minimal",
        description: "Lightweight setup: Vide for UI, Charm for cross-boundary state, ProfileStore for data",
        packages: &["vide", "charm", "videCharm", "profilestore", "janitor"],
    },
    Preset {
        key: "fusion-charm",
        label: "Fusion + Charm",
        description: "Fusion for UI (with its own local state), Charm for state that needs to sync client/server",
        packages: &["fusion", "charm", "profilestore", "janitor", "promise"],
    },
];

pub fn find(key: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.key == key)
}
