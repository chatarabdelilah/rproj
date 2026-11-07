# Catalog browser proposal

Scope approved November 7, 2026, against main `25c59b2`. This is a separate
Soft conversation prototype, not a native implementation. The approved defaults
are a searchable library and read-only guides: documentation, copyable examples
and related entries. Project edits and installation stay in existing workflows.

Catalog is an app-level action and works without an open project. Recognized
dependency details offer **View in Catalog**. Six sections follow the terminal
inventory: Packages, Tools, Capabilities, Generated files, Configurable settings
and Topics. Search includes names, keys, descriptions and category labels, across
the library by default. Entry identity includes its kind, so StyLua's tool and
settings entries cannot collide.

The initial sample selects Scribe. The related ProfileStore library and Scribe
Studio plugin have their own guides. Package examples include imports, placement,
caveats and official documentation links. Other guides show relevant requirements,
providers, file reasons or settings destinations, without empty package sections.
These are curated sample entries from the repository catalog, not a complete
inventory or a live upstream/version check.

## Fixture sources

| Sample | Repository source | Distinction |
| --- | --- | --- |
| Scribe / ProfileStore | `src/catalog/package_usage.rs`, core `catalog/wally_packages.rs` | Shared schema versus server-only profiles; complete sample imports, limited lifecycle examples. |
| Scribe Studio | `src/catalog/tool_usage.rs`, core `catalog/tool_catalog.rs` | Manual Creator Store install, `StudioHook = true`; no detection or implicit Scribe dependency. |
| StyLua tool / settings / generated file | Core tool, settings and artifact catalogs | Executable/provider, configuration fields, and generation reason are separate entry kinds. |
| Formatting capability | Core `catalog/capabilities.rs` | Derives StyLua and its configuration; browsing does not enable it. |
| CI topic | `src/catalog/tool_usage.rs` | Explains project checks without installing or executing anything. |

Adjacent result/guide panes start at 600 logical pixels of main-content width;
below that they stack. Query, selection and navigation position survive leaving
Catalog. Related-entry Back restores the originating guide, filters and focus.
No-match results and an unavailable guide are distinct. Unknown dependencies keep
their inspection facts instead of receiving guessed catalog guidance.

The compact switcher, four project sections, appearance, Activity and Watch remain
visible. Catalog navigation never changes their owners or any retained draft.
Sample workspace inspection separates recorded composition, manual declarations,
resolved versions and installed files. None of those values is inferred from
catalog metadata. Native implementation planning follows owner design approval.
