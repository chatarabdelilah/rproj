# Machine Setup proposal acceptance

October 23, 2026. This is evidence for a conversation prototype, not native
acceptance. [Approved scope](desktop-machine-setup-design.md) covers the Soft
Selections → Review → Results flow. Owner visual-design review is pending.

## Confirmed local evidence

`node target/machine-setup-design/validate.cjs` passes **41 checks** and
**24 numeric Light/Dark contrast comparisons**. JavaScript syntax passes separately.
The conversation fragment and ignored validation artifacts are thread-local;
this documentation PR contains no product/runtime changes.

- Fixtures extract all 36 catalog entries and retain source hashes. Hash checks
  confirm every earlier conversation HTML file is unchanged. Static/runtime
  startup is useful without host state or icons, including throwing helpers.
  Host restoration never replays installation; echoes cannot reset active edits.
- Model checks cover fresh/returning configuration, unknown keys, inactive
  dependencies, executor order, stale review and confirmation, any-Watch blocking,
  acknowledged Stop, serialized operations, machine ownership, retained drafts,
  completion/warnings, fatal/nonfatal failures, selection-save failure, late events,
  and uncertain termination. A configuration change during installation refuses
  selection saving and requires reload. Retry reviews the whole queue explicitly.
- DOM checks cover six categories, search, details/docs, New Project shortcut,
  switcher dismissal/picker cancellation, default-Keep confirmations, focus return,
  close races, Activity filtering/height, theme/layout selection and Unicode paths.
  Identifiers are unique and accessibility references resolve. The first run found
  one failure: a simulation-control click overwrote the dialog's originating focus
  target. Preserving that target fixes the regression; the final run has no failures.
- Source checks confirm responsive pane/shell reflow, wrapped paths/output,
  labeled native controls and text-safe DOM updates. Palette comparisons meet
  4.5:1 for text and 3:1 for the tested focus/selection pair. Manual-only warning
  completion respects Activity collapse; new failures expand it. These checks
  establish state/CSS intent, not measured rendering geometry.

## Unverified and deferred

No supported inline rendering-inspection route is exposed by available tools.
Actual wide/narrow geometry, browser keyboard activation, native dialog trapping,
assistive technology, host icons and System appearance remain unverified. No
retired automation route or native preview was attempted.

Installation/detection, downloads, UAC dialogs, external tools, actual filesystem
changes, cancellation and process teardown are simulated. They are not acceptance
of a future worker or executor. Native memory stability remains unresolved; Windows
real-window/scaling and macOS/Linux desktop acceptance remain paused or unverified.
No machine/VM changes, installation, publication, version bump or Rust/API/protocol/
persisted-format changes belong to this milestone. Version 0.22.0 is unchanged.

## Bounded owner review

1. Select an optional app, open Review and inspect the queue. Open Apply, then
   choose Keep reviewing (30 seconds).
2. Use Simulation controls → Watch conflict; Review stays available, while Apply
   waits for the separate Stop Watch acknowledgement (30 seconds).
3. Apply a normal sample run, then Close sample → Stop and close. Observe waiting
   for the current item and the retained partial-results warning (30 seconds).

The documentation PR records final reviewed-head/main CI and branch cleanup.
Those checks do not establish rendering or native acceptance. Native implementation
planning follows owner design approval.

## Native implementation — shared execution checkpoint

The design is approved. The first runtime PR extracts the catalog and selection
model, adds captured checked execution and guards, and preserves terminal behavior.
702 CLI/core tests pass (24 ignored); all 83 headless desktop tests pass.
The [implementation record](desktop-machine-setup-native.md) records review fixes,
failed attempts and remaining delivery. These disposable fake-installer checks
do not establish real installation, process-tree or native-window acceptance.
No native previews or real installers were launched.
