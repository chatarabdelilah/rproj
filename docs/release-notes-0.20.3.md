# rproj 0.20.3 — Missing-Git creation guidance

Published October 7, 2026. Public alpha.

## Changes

When Git is unavailable during project initialization, rproj now explains how
to recover: install Git from its official page, ensure it is on PATH, open a
new terminal and check `git --version`. Inspect the directory left by the
failed attempt, then retry creation in a new destination; existing directories
remain protected from overwriting.

The original OS error remains in the error chain. Permission failures,
nonzero Git command failures and absent working directories keep their
diagnostics. Projects that already have `.git` still skip initialization.

No project migration is required. Dependencies, successful Git initialization
and the rest of project generation retain their existing behavior. This patch
retains [0.20.2's Watch improvements and limits](release-notes-0.20.2.md).

## Verification and limits

The regression exercises the confirmed project-creation executor in a child
process whose PATH contains only an empty temporary directory. It proves Git
is unavailable, checks the recovery message and preserves configuration,
existing fixture files and the parent process's PATH. This check failed before
the fix and passes afterward. Git is never uninstalled from the host.

The candidate passed 485 ordinary locked Windows tests, strict all-target
Clippy, formatting and clean locked packaging. One applicable ignored New
Project check passed serially with Git 2.52.0.windows.1, Rokit 1.2.0 and Rojo
7.7.0, confirming creation and saved-composition replay/cancellation on the
already configured PC. Its temporary fixtures were removed and machine
configuration stayed unchanged. The 24 ignored tests remain separate; only
that named live check was run explicitly for this candidate.

[The release audit](release-audit.md) records package inspection and release
gates. [Release PR #88](https://github.com/chatarabdelilah/rproj/pull/88) passed
final CodeRabbit review with zero findings after correcting one minor notes
state mismatch. Exact final-head and merged-main Windows stable, Rust 1.89 and
package CI passed on their first attempts.

Successful installation/retry on fresh Windows, authenticated Open Cloud,
fresh Linux generated-project execution, UI Labs Studio stories and Scribe
Studio playtesting remain unverified. Ignored checks are separate acceptance
evidence, not ordinary passes. Hyper-V/VM provisioning remains deferred; this
patch does not claim beta readiness.

The owner-published archive's SHA-256 matches crates.io checksum
`f2403a0d54155e7ec3c37dee9a9f99d9adfee66f0a242cdc2d98f4bfb3ea013c`.
Its manifest and lockfile are 0.20.3, and its clean Git identity matches reviewed
release commit `2932862af2cfc7a7670e54da06e133c799a53592`. The annotated `v0.20.3`
tag and [GitHub alpha prerelease](https://github.com/chatarabdelilah/rproj/releases/tag/v0.20.3)
point to that same immutable commit.
