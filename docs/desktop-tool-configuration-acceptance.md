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
