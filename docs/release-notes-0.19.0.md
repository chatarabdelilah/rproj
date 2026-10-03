# rproj 0.19.0 — Safer project maintenance and UI Labs

Public-alpha candidate; not yet published.

## Changes

- UI Labs 2.4.2 is available in the package catalog, including Wally and submodule
  setup guidance: `uiLabs = "pepeeltoro41/ui-labs@2.4.2"`.
- Direct and Ratatui project creation share capability choices and validation.
  Configure uses shared edit sessions that preserve unknown settings, leave
  unchanged files byte-identical, and retain the draft when saving fails.
- Configure refuses externally changed files. Upgrade checks the reviewed file
  snapshots again before replacing them and stages planned replacements first.
  Existing file permissions are preserved; read-only files and symlink targets
  are refused. Failed replacements retain the target and clean temporary files.
- Jest project preparation uses the same save boundary and preserves existing
  `.rproj-old` backups. Compatible production `devPackages` mounts remain intact.
- Test preparation preserves complete Wally package trees, including typed and
  bare installed aliases, and refreshes the sourcemap. Missing or stale package
  trees still use normal installation and type recovery.

## Existing projects

After installing this version, run `rproj upgrade` to apply applicable project
updates. UI Labs is optional; adding it to an existing composition does not
remove other packages. Stop an active Watch before saving package additions.

A complete manually installed Wally tree can retain bare aliases during Test;
full scaffolding and Watch own type generation. Configuration saves and upgrades
use optimistic freshness checks, not file locks. An upgrade or Jest preparation
can retain earlier successful writes if a later file fails; the affected target
remains intact and the error supports retry. These are not crash-atomic,
multi-file transactions.

## Verification and limits

The candidate passed 435 ordinary Windows tests, formatting and clippy. Its
21 ignored tests were not counted as passes; explicit installed-tool runs passed
all 14 live workflow tests, three real-Rojo validation tests, and the installed
Jest credential-refusal regression. Locked packaging and its extracted-package
build passed with an inspected 101-file archive. Final review and CI are pending;
[the release audit](release-audit.md) records the commands and evidence.

This release adds the Windows-only `windows-permissions` 0.2.4 dependency for
permission-preserving saves. Local validation runs on Windows; authenticated
Open Cloud execution, fresh Windows provisioning, fresh Linux generated-project
execution, a UI Labs Studio story, and a Scribe Studio playtest remain unverified.
The Windows symlink regression requires privileges unavailable on the audit
machine; Unix permission-mode checks require Unix. Ignored tests are not passes.
