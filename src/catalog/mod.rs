pub mod place_template;
pub mod quality_checks;
pub mod tool_catalog;
pub mod tool_settings;
pub mod tool_usage;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Duplicate keys make `find` return whichever came first and the
    /// other unreachable - silently, since both still appear in pickers.
    #[test]
    fn every_catalog_key_is_unique() {
        let mut keys: Vec<&str> = tool_catalog::all_setup_entries().map(|e| e.key).collect();
        keys.extend(wally_packages::PACKAGES.iter().map(|p| p.key));
        let mut seen = std::collections::BTreeSet::new();
        for key in keys {
            assert!(seen.insert(key), "duplicate catalog key `{key}`");
        }
    }

    /// `rproj info` groups tools by family through FAMILY_ORDER; a family
    /// missing from that list means its tools never print.
    #[test]
    fn every_tool_family_appears_in_the_display_order() {
        for entry in tool_catalog::all_setup_entries() {
            assert!(
                tool_catalog::FAMILY_ORDER.contains(&entry.family),
                "`{}` has family `{}`, which FAMILY_ORDER omits - it would never be listed",
                entry.key,
                entry.family
            );
        }
    }

    /// Companions must name packages that exist, or a guided selection
    /// silently drops them.
    #[test]
    fn companion_rules_reference_real_packages() {
        for package in wally_packages::PACKAGES {
            for has_ui in [true, false] {
                for companion in wally_packages::companions_for(package.key, |_| has_ui) {
                    assert!(
                        wally_packages::find(companion).is_some(),
                        "`{}` pulls in unknown companion `{companion}`",
                        package.key
                    );
                }
            }
        }
    }

    /// Docs links are shown to the user and baked into generated link
    /// files, so an empty or malformed one is visible breakage.
    #[test]
    fn every_entry_has_a_plausible_docs_url() {
        for entry in tool_catalog::all_setup_entries() {
            assert!(entry.docs_url.starts_with("https://"), "{}: {}", entry.key, entry.docs_url);
            assert!(!entry.description.is_empty(), "{} has no description", entry.key);
        }
        for package in wally_packages::PACKAGES {
            assert!(package.docs_url.starts_with("https://"), "{}: {}", package.key, package.docs_url);
            assert!(!package.description.is_empty(), "{} has no description", package.key);
        }
    }

    /// A wally source must be `scope/name@version` - `PackageSpec::author`
    /// and `version` parse it, and wally.toml is written from it verbatim.
    #[test]
    fn every_package_source_is_a_valid_wally_coordinate() {
        for package in wally_packages::PACKAGES {
            let source = package.source;
            assert!(source.contains('/'), "{}: `{source}` has no scope", package.key);
            assert!(source.contains('@'), "{}: `{source}` has no version", package.key);
            assert!(!package.author().is_empty(), "{}: empty author", package.key);
            assert!(!package.version().is_empty(), "{}: empty version", package.key);
        }
    }

    /// Usage notes are keyed by catalog key; one that matches nothing is
    /// unreachable through `rproj info`.
    #[test]
    fn usage_notes_point_at_real_entries_or_topics() {
        for usage in tool_usage::USAGE {
            let known = tool_catalog::find(usage.key).is_some()
                || wally_packages::find(usage.key).is_some()
                || usage.key == "rokit";
            assert!(known, "usage note `{}` matches no catalog entry", usage.key);
        }
    }
}
