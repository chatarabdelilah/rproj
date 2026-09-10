# rproj 0.15.0 - T3: Projects Browser

Published September 10, 2026 as an alpha release. The crates.io archive and annotated tag identify commit `b1c1664273bea2fce9bb0f4b954f554cc96f9dfb`.

## Changes

- Home now offers Projects, New Project, Edit Project Template, Machine Setup, and Catalog.
- Projects discovers direct, non-linked project folders under the configured root, plus a recognized launch directory. Filter names/paths, refresh with F5, and inspect warnings without writing configuration or creating folders.
- Configure Tools, Upgrade, Watch, Test, and Copy Source run against the selected project. Output is acknowledged before returning to that project. Back preserves browser state; Ctrl+C returns Home.
- Successful creation opens the new project's action screen and selects it in the refreshed Projects list.
- Removed the redundant Place Template section from Catalog and flat `rproj info`; the template data and Explorer are unchanged.

## Compatibility And Limits

Direct CLI commands still use the exact current directory. No dependency, persistent registry, configuration-schema change, project deletion, arbitrary folder browser, or background Watch is introduced. Browsing is shallow and may report filesystem access errors. Project selection is session-only.

Machine Setup still uses existing prompts; its Ratatui migration is T5. Saved Setup management is T4. Model import remains permanently dropped.

## Verification

330 ordinary tests and three applicable live creation checks passed. Nineteen prerequisite-dependent tests remain ignored in the ordinary suite. Formatting, clippy, locked packaging, local CodeRabbit review, and reviewed-head/main Windows stable and Rust 1.89 CI passed. See the [release audit](https://github.com/chatarabdelilah/rproj/blob/main/docs/release-audit.md) for evidence and remaining Open Cloud/fresh-machine limitations. Owner publication and archive/tag/release alignment are verified.
