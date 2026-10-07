# rproj 0.21.0 — Wally or None

Published October 7, 2026. Public alpha.

## Breaking change

Git-submodule dependencies are fully retired. Direct prompts, New Project,
saved-setup editing, summaries and Catalog guidance offer Wally or None.
Persisted workflow values are only `wally` and `none`.

Old `git-submodules` project records and saved setups are unsupported and fail
through configuration-error handling. There are no aliases, automatic conversion,
legacy support or migration tools. Earlier alpha projects remain the owner's
responsibility. This release does not delete or convert existing repositories,
custom templates, `.gitmodules` files or dependency folders.

The submodule generator, Git add/update helpers, Watch/Test restoration,
source-layout metadata, manual dependency closure, modules artifact/mount,
special exclusions and generated CI submodule checkout setting are removed.
Upstream repository links, canonical module names and Wally aliases remain.

Ordinary Git initialization and missing-Git recovery are preserved. Wally retains
package selection, dependency resolution, realms, development packages and
testing. None retains dependency-free creation and optional TestEZ. Custom
template fields, including an owner-controlled `modules` node, remain preserved.
Rojo template validation covers the eight remaining variants.

## Test correction

The existing Saved Setup resize regression now waits for the complete expected
composition pane after scrolling, resize restoration and returning from Actions.
This fixes a CI-observed partial-redraw assertion without changing product
behavior or weakening file, selection, filter, scroll or terminal checks.

## Verification and limits

The implementation is merged in [PR #93](https://github.com/chatarabdelilah/rproj/pull/93)
at `63519bf432de2528e150aba2d183d695fa248807`. Its reviewed-head and main CI passed
all four required jobs. Each Windows toolchain passed 479 ordinary tests, with
23 ignored tests tracked separately. Serial installed-tool checks passed for
all eight Rojo variants, Wally creation/gate, deliberate gate failures, None
creation through Home, saved Wally replay and unsupported saved-record refusal.

The isolated release-candidate binary reports `rproj 0.21.0`. Formatting, all
479 ordinary locked tests, strict all-target Clippy, the serial installed-tool
checks above and official 1.7.0 Windows compatibility passed again for the
candidate. The 23 ignored checks remain separate; no ordinary checks were excluded.
Clean locked packaging passed with 105 files and matching Git identity. Local
CodeRabbit iteration and final branch reviews both completed with zero findings
across all seven release files. Release-preparation head/main CI and delivery evidence are
recorded in the [release audit](release-audit.md) and
[release-preparation PR #94](https://github.com/chatarabdelilah/rproj/pull/94).
The official-Wally 1.7.0 Linux compatibility check remains intact. It verifies
generic-default and `const` compatibility, separately from full project acceptance.
The [0.20.4 Wally pin requirement](release-notes-0.20.4.md) is unchanged; older
Wally projects still have an explicit tool-update path.

Fresh-Windows installation/retry, authenticated Open Cloud, full fresh-Linux
project execution, UI Labs Studio stories and Scribe Studio playtesting remain
unverified. VM provisioning remains deferred. This is a public alpha release,
not a claim of beta readiness.

The owner published 0.21.0 after all candidate gates passed. The official archive
contains 105 files, all identical to a clean package from release commit
`07dcf17fb8a841c2f842a3753f58ad10507f99e1`; its clean Git identity and manifest/
root-lockfile versions match. Registry checksum:
`40c69158d8c771dc62d3a9eda6d171f78f1a1d199bcf27e478e43b6dbc50c8d8`.
The annotated `v0.21.0` tag and
[GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.21.0)
identify that commit. Published 0.20.4 and earlier tags remain unchanged.
