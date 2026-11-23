# Configure Tools proposal acceptance

October 19, 2026. This records a conversation prototype, not native acceptance.
The owner approved the [scope and behavior](desktop-tool-configuration-design.md):
four existing targets, one captured target draft, Settings → Review, and waiting
for Save rather than cancelling it on close. Visual design approval is pending.

## Confirmed local evidence

`node target/tool-configuration-design/validate.cjs` passes **38 checks** and
**28 numeric Light/Dark palette comparisons**. Validation artifacts are ignored
and thread-local; the prototype is conversation response content, not product code.

- Catalog extraction includes all four targets and 33 fields, with source hashes
  and unchanged earlier-wireframe hashes. JavaScript syntax and useful static/runtime
  startup content pass with absent or throwing optional helpers. State restoration
  never replays Save and rejects unknown/prototype-property project identifiers.
- Pure transition checks cover unset/default/revert, exact i64 validation, locked
  unfamiliar values/null, captured ownership, shared JSON-file preservation,
  source/review/Watch invalidation, acknowledged Stop, failures, stale results,
  baseline advancement, and source changes during Save. The sample before/after
  diff retains controls, CRLF, missing newline and line numbers. Fixture merge
  checks are not a replacement for tests of the actual Rust writers.
- DOM checks cover all targets, refusal/loading states, named default-Keep dialogs,
  focus restoration, switcher Escape and picker cancellation, retained review,
  close-after-save, other dirty drafts, output states/filtering/height, appearance,
  long paths, unique identifiers and accessibility references. Initial failures
  exposed Keep-button clicks overwriting their originating focus target; the
  corrected implementation passes both Save and close-dialog focus regressions.
- Source checks confirm the responsive 600-pixel pane breakpoint, narrow stacking,
  wrapping, labeled native controls, visible focus and text-safe DOM insertion.
  Text/disabled/diff colors meet 4.5:1 and the selected focus comparison meets 3:1
  in both declared themes. Numeric comparisons do not establish screenshot quality.

## Unverified and deferred

No supported inline inspection route is exposed by the available tools. Actual
rendering, geometry, browser Tab/Enter/Space activation, native modal/assistive
behavior and host icon rendering remain unverified. No retired browser/native
automation or native preview launch was attempted.

Real filesystem saves, actual writer/parser behavior, worker execution, teardown
and external tools are not exercised by this simulation. Native memory stability,
Windows real-window/scaling/System-appearance acceptance and macOS/Linux desktop
acceptance remain unresolved or unverified. This milestone performs no OS/VM
work, publication, installer work, version bump or runtime/API/schema changes.

## Bounded design review

1. Change Skyline's StyLua column width, choose Review changes, inspect its diff
   and dismiss Save with Keep reviewing (30 seconds).
2. Load Unfamiliar current value and verify Keep existing; browse Workshop and
   Resume to check captured ownership (30 seconds).
3. Load Slow Save / close handling, review/save, then compare Keep open and
   Close after save (30 seconds).

Owner review is the next gate. Exact documentation-PR head/main CI and merged
branch cleanup are recorded on its PR, without treating them as rendering passes.


## Native automated evidence — October 21

The approved design is implemented through shared execution, private worker and
native controls; see [implementation](desktop-tool-configuration-native.md).
The earlier prototype evidence above remains a separate historical design gate.
The full CLI/core suite passes 676 tests (24 prerequisite-dependent tests ignored).
All 83 desktop tests pass without opening native windows. Settings-specific tests
cover all four catalog targets, typed inputs/integer bounds, unfamiliar values,
locked legacy fields, wide/narrow geometry, real Tab/Space activation, default-Keep
confirmations, canonical captured ownership, retained drafts, stale events,
Watch conflicts, responsive navigation during Save, parsed saved baselines,
recoverable pre-spawn failure, uncertainty and teardown failure, uncancellable
Save, cross-draft close protection, and completion while a dialog remains open.
Long wrapped diffs retain 900 rows with viewport rendering in Light/Dark;
existing Activity/filtering/height and inspection-recovery tests remain passing.

Initial attempts exposed test imports shadowing Rust's test macro and a fixed
scroll distance insufficient for the narrow viewport. Imports and geometry-based
scrolling were corrected. A keyboard assertion assumed mouse activation had
focused a button; it now follows and verifies actual Tab focus before Space.
A UTF-8 editing error truncated the new view; it was restored from inspected
source before the full suite passed. A sandbox-denied temporary preferences
replacement passed with required test access. These were not native preview runs.
Real-window, assistive-technology, memory-stability and non-Windows desktop
acceptance remain unverified; native launches remain paused.

Local CodeRabbit found an overbroad preparation-error reload requirement. Cancelled
preparation, pre-spawn failures and recoverable writer refusals now retain an
editable draft; stale capture failures carry explicit Reload from file guidance.
Worker and headless regressions cover both paths. Its separate conditional concern
about Save pre-spawn errors was already handled: the client returns Failed with
NotStarted, and only unconfirmed teardown propagates as an error. The existing
missing-executable worker test and new headless recovery test verify this boundary.

The full terminal rerun caught a diagnostic regression in the stale-source wrapper:
it hid the original deleted-project explanation. The wrapper now includes both
the underlying reason and reload guidance. Final local review reported zero
findings; formatting, strict CLI/core/desktop Clippy and archive/non-window portable
checks passed on the delivered source. Existing three legacy dependency
notice gaps remain tracked separately; they are not new acceptance passes.
