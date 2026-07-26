pub mod tool_catalog;
pub mod wally_packages;

// `Legacy` isn't attached to any catalog entry yet (Mantle/ProfileService were
// dropped rather than kept as legacy options - see the plan) but the badge
// system is designed to carry it the moment a future entry needs it.
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Maintenance {
    Active,
    CommunityStable,
    Legacy,
}

impl Maintenance {
    pub fn badge(&self) -> &'static str {
        match self {
            Maintenance::Active => "actively maintained",
            Maintenance::CommunityStable => "community-stable",
            Maintenance::Legacy => "legacy, avoid for new projects",
        }
    }
}
