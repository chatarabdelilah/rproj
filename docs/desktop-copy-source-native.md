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

### Native review

The Soft workbench now offers Copy source from Overview and a captured-owner
Resume action. Collection runs independently of the worker and serialized
mutations. A single active collector and latest queued request bound background
work; replacement/closure abandons obsolete reads at collector checkpoints.
Browsing and closing a project retain the review. Explicit workflow navigation
hides it without discarding it. Replacement defaults to Keep review.

Copy submits the retained exact payload through GPUI's clipboard API. The API
does not report OS acceptance, so feedback says **Copy requested**. Refresh keeps
the prior snapshot until success; a failed refresh labels that older snapshot.
Partial collections expose skipped paths/reasons and **Copy included files**.
Empty, missing-src and failed collections without a snapshot cannot be copied.

Display preparation runs off-thread, annotating controls and newline endings
without changing payload bytes. It refuses over 100,000 rows or 16 MiB annotated
text. Lines longer than 4,096 Unicode scalar values use labeled continuation
rows; nothing is truncated. File, skip and preview lists render their viewport.
The file/preview panes stack below 600 logical pixels of main content. The outer
page scrolls at short heights, including with expanded Activity. Keyboard file
selection navigates the complete preview without changing the copied files.

Activity adds captured-owner/count/error metadata without flushing or retagging
a concurrent operation's buffered output. Source actions do not replace mutation
outcomes or cancellation controls. Successful preparation respects manual
collapse; Activity height and existing close protections remain unchanged.

Eleven new checks cover annotations/display bounds, exact snapshot clipboard
requests, failed refresh, latest queued requests and stale results, empty/partial
states, worker-independent background completion, Watch/draft/operation ownership,
default-Keep dismissal/focus, stale replacement/copy, both theme/layout branches,
file navigation, collapse and abandonment on destruction. The full headless suite
passes 162 tests; desktop Clippy passes. Initial test compilation exposed a glob
import of GPUI's test macro and incorrect fixture Watch fields; these were fixed.
The first targeted run exposed an unobservable skip-list group and a short-list
scroll assertion; the accessible group and long fixture now pass. The first full
suite hit two sandbox filesystem denials; the unchanged suite passed outside
that restriction. CLI Copy's four tests and all 12 shared collector tests also
pass on this branch. Formatting and portable release packaging pass, including
extracted version, worker/self-check and inspection checks. Packaging retains
the existing warning about incomplete notices for three legacy dependencies;
this is not a new acceptance pass for those notices. CodeRabbit 0.7.6 reviewed
all 20 files with zero findings. Final evidence-only edits were inspected
directly. Actual reviewed-head/main CI and cleanup are recorded on the delivery
PR after verification.

No native preview was launched. Real geometry, assistive technology, OS clipboard
acceptance and native memory stability remain unverified. Earlier wireframes and
their sample-only evidence are unchanged. No version or protocol change.
