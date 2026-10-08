# rproj 0.21.2 — Upgrade settles in one run

Published October 8, 2026. Public alpha.

## Changes

When `rproj upgrade` recreates a missing Wally `selene.toml`, it now uses the
same managed-field merge as subsequent runs before presenting the review.
Previously, the next upgrade offered another confirmation solely to remove
spacing from the exclusion array. After applying the repair, the next run now
reports already up to date and leaves every project file unchanged.

Interactive approval and `--yes`, Wally with and without TestEZ, and None are
covered by a new regression. Custom exclusions, unrelated configuration,
confirmation/cancellation, and file protection retain their existing behavior.
Shared Configure rendering, public interfaces, project formats and tool pins
are unchanged. This release retains the complete upgrade review shipped in
[0.21.1](release-notes-0.21.1.md).

## Verification and limits

The implementation [PR #99](https://github.com/chatarabdelilah/rproj/pull/99)
passed all four required jobs on its final head and merged main. The ordinary
suite passed 486 tests with 23 prerequisite-dependent tests ignored, including
all 29 Upgrade tests. Formatting, strict all-target Clippy, clean locked
packaging and local CodeRabbit reviews completed; two minor documentation
findings were corrected and inspected directly. Release-preparation
evidence is recorded in [the release audit](release-audit.md).

The 0.21.2 release preparation also passed formatting, 486 ordinary tests with
23 ignored, strict all-target Clippy and clean locked package verification.
Both local CodeRabbit reviews reported zero findings; the final review covered
all seven release files. Release [PR #100](https://github.com/chatarabdelilah/rproj/pull/100)
passed all four required jobs on the reviewed head and merged main. Its complete
trees matched, and the release branch was removed after main verification.

Fresh-Windows installation/retry, authenticated Open Cloud, full fresh-Linux
project execution and manual Studio checks remain unverified. VM provisioning
stays deferred. This public-alpha patch is not a claim of beta readiness.

The owner published 0.21.2 after release verification. The official archive's
105 files are byte-identical to the clean locked rebuild at
`287b7b60de9247ac85ce7b575a31c547bb9fec3c`. Its clean Git identity, both manifests
and root lockfile identify that commit and version. Registry checksum:
`c067c17f5f92a963246f13efc6724d07028b5ace87fd932b069f054b8200481c`.
The annotated `v0.21.2` tag and
[GitHub alpha prerelease](https://github.com/chatarabdelilah/rproj/releases/tag/v0.21.2)
identify the verified published commit. Earlier shipped tags remain unchanged.
