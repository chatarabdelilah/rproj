# rproj 0.13.1

Alpha patch release: safer interactive tool configuration.

## Changes

- `rproj configure` now keeps existing values outside its guided choices unless you explicitly choose to replace them.
- Leaving every setting unchanged no longer rewrites the configuration file.
- TOML changes are checked against the intended parsed configuration before writing. rproj refuses malformed or unusual layouts that its legacy line writer cannot safely update.
- Added two unit and six PTY regressions for custom values, structured JSON values, handwritten TOML, explicit replacement, concurrent malformed edits, and semantic merge safety.

## Compatibility

There are no project schema, generated-file, dependency, or CLI-argument changes. Existing configurations remain valid. A valid but unusual TOML layout may now be refused rather than rewritten; manually edit that file, or use a conventional layout, to make a guided change. This guard applies to `rproj configure`; the existing upgrade/scaffolding writers are unchanged.

After publication, install or update with `cargo install rproj --version 0.13.1 --locked`.

## Verification And Limits

The candidate includes 309 ordinary passing tests and 19 explicitly ignored tests (328 discovered). Formatting, clippy, locked packaging, reviewed-head CI, and merged-main CI are required release gates. Studio/Rojo/provisioning live checks were not rerun for this patch because its code neither invokes nor changes those paths; their 0.13.0 evidence remains historical.

This remains alpha software. Fresh-Windows provisioning and live Open Cloud execution have not been verified. The TOML safeguard does not add file locking or crash-atomic writes, and it deliberately refuses unsafe layouts instead of trying to repair them.
