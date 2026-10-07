# rproj 0.21.1 — Complete upgrade review

Published October 7, 2026. Public alpha.

## Changes

`rproj upgrade` now previews every planned write before one confirmation.
`.gitignore`, root `.luaurc`, and TestEZ's `tests/.luaurc` join the same plan,
file-conflict checks, and staged replacements as other maintained files.
Previously, upgrade could report already up to date and then write housekeeping
files without offering confirmation.

Housekeeping-only repairs now require confirmation. Declining, Esc, and Ctrl+C
before application leave the project unchanged. An empty plan writes nothing
and does not prompt. `--yes` still shows the complete plan and skips prompting.
Unparseable `.luaurc` files are explicitly reported and preserved, including
documents with comments; skipped files are not claimed to be up to date.

Existing custom ignore entries, aliases, globals, and configured language mode
are preserved. Creation and upgrade share pure content planners with the same
merge rules. CLI flags, confirmation defaults, tool pins, file eligibility, and
project records retain their existing behavior; no migration is required.

The patch retains [0.21.0's Wally-or-None workflows](release-notes-0.21.0.md)
and [the official Wally tool requirement](release-notes-0.20.4.md).

## Verification and limits

Implementation [PR #96](https://github.com/chatarabdelilah/rproj/pull/96) merged
at `9eea7eb912e291ef1ebf48d6a979dc053e13b68e`. Final-head and merged-main CI
passed Windows stable, Rust 1.89, official Wally compatibility on Linux, and
packaging. The implementation passed 28 upgrade tests, 485 ordinary tests with
23 ignored (nested child summaries excluded), formatting, strict Clippy, clean
locked packaging, and both local CodeRabbit reviews with zero findings.

Seven regressions cover review/cancellation, interactive and unattended saves,
custom values and byte-preserving reruns, edit/delete/create conflicts,
unreadable targets, malformed documents, and later staging failure. Preparation
failure preserves target contents but may leave empty parent directories;
successful earlier replacements are not rolled back if a later replacement
fails. Recovery instructions report saved files and explain how to retry.

The release binary reports 0.21.1. Formatting, 485 ordinary tests with 23
ignored, and strict all-target Clippy pass. Clean locked packaging builds its
105-file archive; all 95 source/test files match the checkout, and Git identity
and manifest/root-lockfile versions are aligned. Both local CodeRabbit reviews
report zero findings, including all seven files in the final branch review.
Release [PR #97](https://github.com/chatarabdelilah/rproj/pull/97) passed all four
required jobs on its final reviewed head and merged main; [the release audit](release-audit.md)
records the run identities and package inspection. Source, tests, dependencies and CI are
unchanged from the reviewed implementation; only the package version and release
documentation change.

Fresh-Windows installation/retry, authenticated Open Cloud, full fresh-Linux
project execution, UI Labs Studio stories and Scribe Studio playtesting remain
unverified. VM provisioning remains deferred. This is a public alpha release,
not a claim of beta readiness.

The owner published 0.21.1 after all release gates passed. The official archive
contains 105 files, all byte-identical to a clean locked package from release
commit `f9d300ff2539033852d51926488b05bf3840cb29`. Its clean Git identity,
both manifests, and root lockfile match that commit and version. Registry checksum:
`7d58dab4175ddfd4c4b0fd2eca32976c66bdc01df5f2b769e5bae74c388b1031`.
The annotated `v0.21.1` tag and
[GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.21.1)
identify that commit. Earlier published tags remain unchanged.
