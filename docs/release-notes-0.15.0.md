# rproj 0.15.0 - T3: Projects Browser

This is an alpha release. Publication is performed by the repository owner.

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

See [release audit](release-audit.md) for local tests, live prerequisites, review, and CI evidence. The owner runs `cargo publish --locked` only after the release candidate is reviewed and merged. Tag and GitHub alpha release follow published-archive verification.
