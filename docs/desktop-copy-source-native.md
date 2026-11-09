# Native Copy Source implementation

Approved November 9, 2026, against clean main `7ee7310`. Deliver in two sequential
reviewed PRs: shared bounded collection/CLI reporting, then native Soft controls.
Published version remains 0.22.0. No native previews or real clipboard acceptance.

## Approved contract

Review captured recursive `src` contents before explicitly copying them. Refresh
never silently changes an existing snapshot on failure. Partial copies name skips.
Shared limits are 8 MiB including framing and 10,000 visited entries. Terminal
copy remains noninteractive and reports bytes and partial results accurately.
Desktop preparation is independent of the CLI worker, setup, Watch and mutations.
One background lane rejects stale results; retained reviews add no close prompt.
Native display is limited to 100,000 rows and 16 MiB annotated text, never truncated.

## Evidence

### Shared collection

Exact framing tests passed against the original CLI before extraction. Core now
collects with file ranges, captured canonical ownership and explicit skips; the
CLI retains noninteractive copy and reports included/skipped counts and UTF-8
bytes. Clipboard access occurs only after successful nonempty collection.
Existing same-file/walkdir dependencies are reused without new resolved packages.
Root and source handles detect observable replacement; linked descendants are
skipped, including tested Windows junctions. Ancestors outside the selected
project are canonicalized (including system temporary-directory aliases).

The original CLI/core suite passed 638 CLI tests with 24 prerequisite-dependent
ignores. Its core portion exposed a noncanonical-path unit-fixture mistake after
the containment check was tightened. The corrected full core run passed 175 tests
(152 unit, 21 inspection, two discovery). Twelve collector tests cover framing,
empty/partial results, byte/entry limits, bounded reads, abandonment, directories,
file changes and junction ownership. Earlier sandbox attempts denied disposable
directory rename/junction creation; these passed unchanged outside the sandbox
after the fixture's slash-separated junction argument was corrected. No native
window was launched. Additional final checks and PR identities follow on delivery.

The final 12 collector tests, 151 headless desktop tests, CLI/core and desktop
Clippy, formatting and fresh joint core/CLI archive verification passed.
CodeRabbit 0.7.6 reviewed all nine changed files with no findings. The final
evidence-only documentation update was inspected directly. Head/main CI identities
and branch cleanup are recorded on the shared delivery PR after verification.

Read checks are not an atomic filesystem snapshot or protection against arbitrary
external writers. A blocked OS read is not forcibly interrupted. Source contents
never enter diagnostics; only path/count/reason metadata does.
