use super::*;

fn page(kind: Kind, key: &str) -> Guide {
    guide(&EntryId::new(kind, key)).unwrap()
}

#[test]
fn all_six_inventories_resolve_with_unique_typed_identity_and_sorted_names() {
    let inventory = entries();
    assert_eq!(
        inventory.len(),
        wally_packages::PACKAGES.len()
            + tool_catalog::all_setup_entries().count()
            + capabilities::CAPABILITIES.len()
            + artifacts::ARTIFACTS.len()
            + tool_settings::CONFIGURABLE_TOOLS.len()
            + tool_usage::TOPICS.len()
    );
    let identities: BTreeSet<_> = inventory.iter().map(|e| e.id.clone()).collect();
    assert_eq!(identities.len(), inventory.len());
    for kind in Kind::ALL {
        assert!(inventory.iter().any(|e| e.id.kind == kind));
    }
    let ordering: Vec<_> = inventory
        .iter()
        .map(|e| (e.id.kind, e.name.to_lowercase(), e.id.key.clone()))
        .collect();
    let mut sorted = ordering.clone();
    sorted.sort();
    assert_eq!(ordering, sorted);
    for entry in inventory {
        let g = guide(&entry.id).unwrap();
        assert_eq!(g.entry, entry);
        assert!(!g.sections.is_empty());
        for link in g.links {
            assert!(link.url.starts_with("https://") || link.url.starts_with("http://"));
            assert!(!link.label.is_empty());
        }
        assert!(
            g.related
                .iter()
                .all(|id| id != &entry.id && identities.contains(id))
        );
        assert_eq!(
            g.related.iter().collect::<BTreeSet<_>>().len(),
            g.related.len()
        );
    }
    assert!(guide(&EntryId::new(Kind::Package, "unfamiliar/library")).is_none());
}

#[test]
fn overlapping_keys_resolve_by_kind_not_legacy_lookup_precedence() {
    let tool = page(Kind::Tool, "stylua");
    let settings = page(Kind::Settings, "stylua");
    assert_eq!(tool.entry.name, settings.entry.name);
    assert_ne!(tool.sections, settings.sections);
    assert!(
        settings
            .sections
            .iter()
            .any(|s| s.heading == "column_width" && s.text.contains("120"))
    );
    assert!(
        tool.sections
            .iter()
            .any(|s| s.heading == "Provider" && s.text.contains("JohnnyMorganz/StyLua"))
    );
    assert_ne!(
        page(Kind::Topic, "ci").sections,
        page(Kind::Capability, "ci").sections
    );
}

#[test]
fn package_examples_and_legacy_projection_share_exact_imports_and_caveats() {
    let legacy = package_entries().unwrap();
    for p in wally_packages::PACKAGES {
        let g = page(Kind::Package, p.key);
        let usage = package_usage::find(p.key).unwrap();
        let old = legacy.iter().find(|e| e.key == p.key).unwrap();
        assert_eq!(g.examples.len(), 1);
        assert_eq!(g.examples[0].code, old.example);
        assert_eq!(old.example, package_usage::example(&usage));
        assert_eq!(old.caveats, usage.caveats);
        assert!(g.links.iter().any(|l| l.url == p.docs_url));
        assert!(g.sections.iter().any(|s| s.text == usage.caveats));
    }
    assert!(
        page(Kind::Package, "profilestore").examples[0]
            .code
            .contains("ServerScriptService\").serverPackages.profilestore")
    );
    assert!(
        page(Kind::Package, "jest-globals").examples[0]
            .code
            .contains("devPackages.JestGlobals")
    );
    for key in ["jest", "jest-globals"] {
        assert!(
            page(Kind::Package, key).examples[0]
                .code
                .ends_with("    end)\nend)")
        );
    }
    assert!(
        page(Kind::Package, "testez")
            .sections
            .iter()
            .any(|s| s.text.contains("no-manager projects"))
    );
}

#[test]
fn settings_include_every_supported_value_and_destination_without_project_state() {
    for target in tool_settings::CONFIGURABLE_TOOLS {
        let g = page(Kind::Settings, target.key);
        let destination = match target.target {
            tool_settings::ConfigTarget::ProjectToml { filename } => filename,
            tool_settings::ConfigTarget::VsCodeSettings => ".vscode/settings.json",
        };
        assert!(
            g.sections
                .iter()
                .any(|s| s.heading == "Destination" && s.text == destination)
        );
        for setting in target.settings {
            let s = g
                .sections
                .iter()
                .find(|s| s.heading == setting.display_key())
                .unwrap();
            assert!(s.text.contains(setting.description));
            assert!(s.text.contains(&setting.kind.default_value().to_string()));
            if let tool_settings::SettingKind::Choice { options, .. } = &setting.kind {
                for option in *options {
                    assert!(s.text.contains(option.value));
                    assert!(s.text.contains(option.explanation));
                }
            }
        }
        assert!(g.examples.is_empty());
    }
}

#[test]
fn related_guides_follow_authoritative_edges_and_explicit_scribe_links() {
    let scribe = page(Kind::Package, "scribe");
    assert!(
        scribe
            .related
            .contains(&EntryId::new(Kind::Tool, "scribe-studio"))
    );
    assert!(
        scribe
            .related
            .contains(&EntryId::new(Kind::Package, "profilestore"))
    );
    let plugin = page(Kind::Tool, "scribe-studio");
    assert!(
        plugin
            .sections
            .iter()
            .any(|s| s.text.contains("does not add the Scribe library"))
    );
    assert!(
        plugin
            .links
            .iter()
            .any(|l| l.url.contains("113609038046646"))
    );
    assert!(plugin.examples.is_empty());
    let typed = page(Kind::Capability, "typecheck");
    assert!(
        typed
            .related
            .contains(&EntryId::new(Kind::Tool, "luau-lsp-cli"))
    );
    let ci = page(Kind::Capability, "ci");
    assert!(ci.related.contains(&EntryId::new(Kind::Capability, "gate")));
    let format = page(Kind::Capability, "format");
    assert!(
        format
            .related
            .contains(&EntryId::new(Kind::File, ".gitattributes"))
    );
    assert!(
        page(Kind::Package, "reactReflex")
            .related
            .contains(&EntryId::new(Kind::Package, "react"))
    );
    assert!(
        page(Kind::Tool, "rojo")
            .related
            .iter()
            .any(|e| e.key == "rojo-plugin")
    );
}

#[test]
fn capability_implementations_requirements_and_file_reasons_are_complete() {
    for c in capabilities::CAPABILITIES {
        let g = page(Kind::Capability, c.key);
        for i in c.implementations {
            let s = g.sections.iter().find(|s| s.heading == i.display).unwrap();
            for key in i.tools.iter().chain(i.packages).chain(i.artifacts) {
                assert!(s.text.contains(key));
            }
        }
        for requirement in c.requires {
            assert!(g.sections.iter().any(|s| s.text.contains(requirement)));
        }
    }
    for f in artifacts::ARTIFACTS {
        let g = page(Kind::File, f.key);
        assert!(g.sections.iter().any(|s| s.text == file_reason(f)));
        for r in f.also_requires {
            assert!(g.sections.iter().any(|s| s.text.contains(&r.describe())));
        }
    }
}

#[test]
fn search_covers_case_names_keys_descriptions_and_categories() {
    let scribe = page(Kind::Package, "scribe").entry;
    for query in [" SCRIBE ", "replicated", &scribe.category, "Packages", ""] {
        assert!(scribe.matches(query));
    }
    assert!(!scribe.matches("no matching text"));
    assert!(
        page(Kind::Tool, "scribe-studio")
            .entry
            .matches("studio plugin")
    );
}
