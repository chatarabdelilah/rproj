# rproj 0.21.1 — Complete upgrade review

Candidate prepared October 7, 2026. Public alpha. Owner publication is pending.

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

The candidate binary reports 0.21.1. Formatting, 485 ordinary tests with 23
ignored, and strict all-target Clippy pass. Clean locked packaging builds its
105-file archive; all 95 source/test files match the checkout, and Git identity
and manifest/root-lockfile versions are aligned. Both local CodeRabbit reviews
report zero findings, including all seven files in the final branch review.
Release-preparation reviewed-head and merged-main CI must pass before owner
publication; [the release audit](release-audit.md)
records their outcomes and package inspection. Source, tests, dependencies and CI are
unchanged from the reviewed implementation; only the package version and release
documentation change.

Fresh-Windows installation/retry, authenticated Open Cloud, full fresh-Linux
project execution, UI Labs Studio stories and Scribe Studio playtesting remain
unverified. VM provisioning remains deferred. This is a public alpha candidate,
not a claim of beta readiness. The owner publishes from clean main after all
candidate gates pass; the tag and GitHub prerelease follow registry confirmation.
