# rproj 0.20.4 — Official Wally types and safe CI upgrades

Unpublished candidate prepared October 7, 2026. Public alpha.

## Changes

Generated Wally CI now uses the official Rokit-installed wally-package-types
release instead of building a patched commit with Cargo. Official 1.7.0 includes
the generic-default ordering and `const` parser fixes that required the old
workaround. Package installation, sourcemap selection and shared/server/dev
package arguments retain their behavior.

Before upgrading managed Wally CI, rproj requires the official stable tool pinned
at 1.7.0 or newer in the project's `rokit.toml`. Older, missing, malformed or
unverifiable pins stop the upgrade before any file changes, including with
`--yes`. Upgrade explains installation/update and retry; it never rewrites tool
pins. Changes to the manifest during confirmation also refuse the upgrade.
Projects without managed Wally CI bypass this check.

For an older project, run these commands from its directory, verify the manifest
records an official stable version at least 1.7.0, then retry:

```powershell
rokit update wally-package-types
rproj upgrade
```

If the tool is absent, use `rokit add wally-package-types` first. A malformed
manifest must be repaired before retrying. Machine-wide manifests, historical
patched caches and existing projects are not automatically changed. New-project
tool selection continues through Rokit's existing version resolution.

This patch retains [0.20.3's Git recovery and existing limits](release-notes-0.20.3.md).
Rust dependencies and runtime/test source are unchanged from reviewed PR #90.

## Verification and limits

The isolated build reports `rproj 0.20.4`. All 488 locally available ordinary
locked tests passed, alongside strict all-target Clippy and formatting. Two
machine-setup cancellation checks are blocked locally by the owner's active
background Watch; it remains running. Complete 490-test Windows stable/Rust 1.89
CI is required. The 25 ignored checks remain separate; the named official 1.7.0
Windows generic-default/`const` compatibility test passed explicitly and serially.

The [release audit](release-audit.md) and candidate PR record clean locked
packaging, local CodeRabbit review and exact-head/main CI as those gates finish.
The focused Ubuntu CI job runs the same official-binary compatibility check.

First-time Windows installation/retry, authenticated Open Cloud, full fresh-Linux
generated-project execution, UI Labs Studio stories and Scribe Studio playtesting
remain unverified. The focused Linux type-tool test does not close full project
acceptance. Hyper-V/VM provisioning remains deferred; this patch does not claim
beta readiness.

The owner alone publishes with `cargo publish --locked`. Registry/archive identity
verification, the annotated tag and GitHub alpha prerelease follow successful
publication.
