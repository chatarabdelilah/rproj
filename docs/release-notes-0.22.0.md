# rproj 0.22.0 — Review Upgrade file changes before applying

Published October 8, 2026. Public alpha.

## Changes

Interactive `rproj upgrade` and Home's **Upgrade Project** now show actual
file diffs in Ratatui. Select a changed file to read its reason and scroll
the additions/removals. New files appear as additions; skipped-file warnings
remain visible. CRLF and missing final newlines are marked, terminal controls
are escaped, and long lines can be panned horizontally.

**A** opens one Apply-all confirmation. **Enter means No** and returns to
review; **Y** explicitly approves all reviewed changes. Esc closes a dialog,
then cancels Upgrade; Ctrl+C returns Home or exits standalone Upgrade. Review
keeps selection, focus and per-file scrolling through Help and resize.
Below 60 x 16, Help/cancellation remain available and Apply is blocked,
including an already-open confirmation. Home borrows its terminal during
review and restores ordinary output before execution and acknowledgement.

`rproj upgrade --yes` remains plain and applies without the viewer, with the
same safety checks. Empty plans bypass the viewer. Redirected Upgrade with
pending changes requires `--yes`; refusal writes nothing. The exact captured
plan reaches the existing conflict checks and staged writer, without replanning
or applying an independently chosen subset of files.

This release also fixes a confirmed preservation defect in published
0.21.2: Selene merges could change unrelated multiline TOML string content
that resembled managed settings. Both managed-setting and package-exclusion
passes now share Configure's semantic guard, including recreated configuration.
Unsafe merges refuse before confirmation or writes, name `selene.toml`, and
give manual-edit/retry guidance. Safely mergeable files continue to upgrade.
Custom exclusions, lint choices, comments, unrelated values and user source
remain covered by preservation regressions.

No dependencies, external-tool boundaries, project schema, managed fields or
tool pins change. Version 0.22.0 is a minor alpha because the interactive
review and approval behavior intentionally changes; it is not beta readiness.

## Verification and limits

Implementation [PR #102](https://github.com/chatarabdelilah/rproj/pull/102) and
[PR #103](https://github.com/chatarabdelilah/rproj/pull/103) passed complete
local checks, CodeRabbit review and all four required CI jobs on their reviewed
heads and merged main. The viewer suite passed 503 ordinary tests with 23
prerequisite-dependent tests ignored, including all 34 Upgrade integration
tests. Tests cover default-No snapshots, explicit approval, Wally/TestEZ/None,
unsafe merge refusal, staging and external-edit conflicts, one-run/no-op
behavior, resize/Help recovery, selected-project execution, Home borrowing and
raw-mode restoration. Source, tests, dependencies and CI are unchanged from
the reviewed viewer implementation in this release preparation.

Release verification and exact review/CI evidence are recorded in
[the release audit](release-audit.md) and
[release PR #104](https://github.com/chatarabdelilah/rproj/pull/104).
The owner published from clean commit
`9dfc1a4cc7f13ac94839b84d601adc72bf00c927`. The official archive's checksum
and Git identity match that reviewed release, including all 97 source/test files.

The binary reports `rproj 0.22.0`. Formatting, the ordinary locked suite
(503 passed, 23 ignored), strict all-target Clippy, clean committed packaging,
both local reviews, and all four required reviewed-head/merged-main CI jobs passed.

Fresh-Windows installation/retry, authenticated Open Cloud, full fresh-Linux
project execution and manual Studio acceptance remain unverified. VM
provisioning stays deferred. Ignored tests are not counted as passes.
