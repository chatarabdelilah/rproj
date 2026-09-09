# rproj 0.14.0 - Home Navigation and Catalog Clarity

Alpha release, T2. Existing project formats and direct command arguments are unchanged.

- Home is persistent: internal full-screen views share its terminal, and commands return after their result is acknowledged. Selection is retained and workspace context refreshes.
- Template Ctrl+S validates and atomically saves without closing. Save baselines, undo history, repair mode, and failed-write recovery protect the draft. Long save errors are scrollable.
- Catalog groups packages and tools, separates VS Code themes/icons, preserves Back state, and adds scrollable offline guidance/examples for every existing package.
- Home-launched foreground commands handle cancellation without closing rproj. Watch awaits its child and distinguishes interruption from an unexpected failure. Interrupted provisioning cannot start subsequent installs or save completion.
- Diagnostic logs record transitions/outcomes without raw navigation keystrokes or template contents. Home shows log paths with failures, not routine navigation.

No new packages, background Watch, projects browser, Ratatui Machine Setup, importer, embedded quality tools, or GUI framework are included.

## Verification and Limits

See [the release audit](release-audit.md) and [Catalog example evidence](catalog-examples.md). Automated Windows PTYs cover shared terminal ownership, repeated saves/repair/reset, cancelled prompts, Watch interruption/failure, and stopping before the test runner after interrupted restoration. Real Rojo checks retain final format authority.

Only already-installed tools are used for live checks. Full fresh-machine provisioning, Open Cloud credentials, and every UI package's complete runtime lifecycle are not validated by this release. Package examples are concise starting points, not complete persistence, replication, or security implementations. Submodule commits can differ from catalogued Wally versions.

The owner publishes with `cargo publish --locked` after reviewed-head/main CI. The annotated tag and GitHub alpha prerelease follow verified crates.io publication.
