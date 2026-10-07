# rproj 0.21.2 — Upgrade settles in one run

Prepared October 7, 2026. Unpublished public-alpha patch candidate.

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
are unchanged. This candidate retains the complete upgrade review shipped in
[0.21.1](release-notes-0.21.1.md).

## Verification and limits

The implementation [PR #99](https://github.com/chatarabdelilah/rproj/pull/99)
passed all four required jobs on its final head and merged main. The ordinary
suite passed 486 tests with 23 prerequisite-dependent tests ignored, including
all 29 Upgrade tests. Formatting, strict all-target Clippy, clean locked
packaging and local CodeRabbit reviews completed; two minor documentation
findings were corrected and inspected directly. Candidate-specific preparation
evidence is recorded in [the release audit](release-audit.md).

The 0.21.2 candidate also passed formatting, 486 ordinary tests with 23 ignored,
strict all-target Clippy and precommit locked package verification. Final local
branch review, clean archive inspection and release PR/main CI remain delivery
gates; their commit and run identities are recorded in the release PR.

Fresh-Windows installation/retry, authenticated Open Cloud, full fresh-Linux
project execution and manual Studio checks remain unverified. VM provisioning
stays deferred. This public-alpha patch is not a claim of beta readiness.

The repository owner publishes with `cargo publish --locked` after release
verification. No 0.21.2 tag or GitHub release is created before crates.io accepts
the package and its archive Git identity is verified.
