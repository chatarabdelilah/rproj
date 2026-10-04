# rproj 0.19.1 — Template Explorer recovery and file protection

Release candidate prepared October 4, 2026. Public alpha; not yet published.

## Changes

- Shrinking an edited Template Explorer below its 60 x 16 editing minimum
  keeps the discard confirmation visible and usable. Enter/Y discards;
  Esc/N retains the draft. Editing, paste, save and reset remain blocked
  while the terminal is too small.
- Both standalone and Home template editing now refuse save/reset when
  another process changes, creates or deletes the template after opening
  or the last successful save. Refusal retains the draft and existing file
  or absence. Reopen Template Explorer to load the current file.
- Save checks the snapshot before and after Rojo validation and after staging.
  Successful saves refresh the snapshot; failed validation or replacement
  retains the previous snapshot and draft for retry. Template replacement
  uses the shared permission-preserving writer.

## Existing projects

These fixes affect the machine-wide template used by future projects. Existing
project files and generated output are unchanged; no upgrade or migration is
required for this patch. UI Labs remains available at 2.4.2.

External-change checks are optimistic, not a filesystem lock. A writer can
still race between the final check and replacement/removal. Reopening loads
the current file; it does not merge an older in-memory draft automatically.

## Verification and limits

The candidate passed 449 ordinary locked Windows tests, formatting and clippy.
Its 22 ignored tests are not ordinary passes; explicit installed-tool runs
passed all 14 live workflows, four real-Rojo template regressions and the
installed Jest credential-refusal regression. Clean locked packaging passed,
including its extracted-package build and inspected 101-file archive. The
release PR and [release audit](release-audit.md) record final CodeRabbit review
and exact-head/main CI before owner publication.

Authenticated Open Cloud execution, fresh Windows provisioning, fresh Linux
generated-project execution, UI Labs Studio stories and Scribe Studio playtesting
remain unverified. Windows symlink privileges are unavailable on this audit
machine; Unix permission-mode checks require Unix. Ignored tests are not passes.
