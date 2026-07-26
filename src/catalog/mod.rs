pub mod place_template;
pub mod quality_checks;
pub mod tool_catalog;
pub mod tool_settings;
pub mod wally_packages;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Maintenance {
    Active,
    CommunityStable,
    Legacy,
}

impl Maintenance {
    /// Full explanation, used by `rproj info <key>`.
    pub fn badge(&self) -> &'static str {
        match self {
            Maintenance::Active => "actively maintained",
            Maintenance::CommunityStable => "community-stable",
            Maintenance::Legacy => "legacy, avoid for new projects",
        }
    }

    /// One word, used inline in pickers so option lines don't run long -
    /// `rproj info <key>` shows the full badge instead.
    pub fn short_badge(&self) -> &'static str {
        match self {
            Maintenance::Active => "active",
            Maintenance::CommunityStable => "stable",
            Maintenance::Legacy => "legacy",
        }
    }
}
