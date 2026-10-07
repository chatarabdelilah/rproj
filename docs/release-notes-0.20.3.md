# rproj 0.20.3 — Missing-Git creation guidance

Unpublished candidate prepared October 7, 2026. Public alpha.

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

Candidate release checks are in progress. [The release audit](release-audit.md)
distinguishes completed checks from pending gates; final review and exact-head/
main CI must complete before owner publication.

Successful installation/retry on fresh Windows, authenticated Open Cloud,
fresh Linux generated-project execution, UI Labs Studio stories and Scribe
Studio playtesting remain unverified. Ignored checks are separate acceptance
evidence, not ordinary passes. Hyper-V/VM provisioning remains deferred; this
patch does not claim beta readiness.

The owner alone publishes with `cargo publish --locked`. Registry/archive
identity verification, the annotated tag and GitHub alpha prerelease follow
successful publication.
