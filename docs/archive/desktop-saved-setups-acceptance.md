# Saved Setups proposal acceptance

October 27, 2026. This records a conversation prototype, not native acceptance.
[Design scope](desktop-saved-setups-design.md) uses the approved Soft shell and
review-first editor. Owner review of this specific flow is pending.

## Confirmed local evidence

`node target/saved-setups-design/validate.cjs` passes **47 checks** and
**34 numeric Light/Dark contrast comparisons**. `node --check` passes for the
model and controller; the assembled inline script also parses independently.
The separate fragment is `rproj-saved-setups-wireframe.html`; its ignored local
validation directory contains source snapshots, earlier-file hashes and JSON/log
results. No existing conversation wireframe was changed.

- Model checks cover review-first/no-op/revert behavior, original-byte management,
  metadata/provenance/exclusion retention, companion and capability requirements,
  required files, Wally/None testing decisions, missing machine context, unsafe and
  unsupported sources, Unicode names/collisions and source races. Captured ownership,
  explicit Save/Discard before reuse/management, independent creation protection,
  serialization, Watch independence, stale completions, failure/partial/uncertain
  outcomes and close races are exercised.
- DOM checks cover a useful initial browser without host helpers/icons, typed
  choices, full paths, complete sample diffs, default-Keep focus, switcher Escape,
  folder-picker cancellation, retained draft navigation, Activity filtering/height,
  no mutation-cancellation action, unique identifiers and resolving accessibility
  references. Restored host state is presentation-only and cannot replay operations.
- Source checks establish responsive stacking at 600 content pixels, shell reflow,
  wrapped text, visible focus and no network access. Text/accent/error comparisons
  meet 4.5:1 on tested surfaces; accent contrast also exceeds the 3:1 focus minimum.
  These are numeric/source checks, not measured browser layout or visual acceptance.

The initial 45-check run passed. During direct interaction-source review, an
additional stale-dialog regression found that dismissal attempted to return focus
to a now-disabled Rename trigger. The fix preserves focus through dialog updates
and uses Reload when the original trigger becomes disabled. An additional empty-
storage test also verifies that New Project resumes an existing creation draft.
The expanded run initially passed 46/47, then passed all 47 after the focus fix.
No native preview, real setup write or external tool execution was performed.

## Unverified and deferred

Available tools expose no supported inline rendering-inspection route. Actual
wide/narrow geometry, host icons, System appearance, browser keyboard activation,
native dialog focus trapping and assistive-technology behavior remain unverified.
JSDOM uses dialog stubs and is not evidence for those behaviors. No retired
automation route was retried.

Document writing, source identity checks, operations, completion, Watch and process
termination are simulated. The sample TOML writer demonstrates effects for frozen
fixtures; it does not establish production writer equivalence or atomicity.
This design-only record predates the separate [native implementation](desktop-saved-setups-native.md).
Its automated evidence is recorded there. Native memory stability is unresolved;
Windows real-window/scaling and macOS/Linux desktop acceptance remain paused or
unverified. No Rust/API/worker/configuration format/version/publication changes
belong to this milestone. Version 0.22.0 is unchanged.

## Bounded owner review

1. Edit Skyline, change one capability, return to Review and expand the diff.
   Open Save, then choose Keep reviewing (45 seconds).
2. Return to Saved Setups and choose Use in New Project. Check the Keep/Save/
   Discard decision, then explicitly discard and inspect the Project-step handoff
   (30 seconds).
3. Try Rename with Simulation controls set to partial Rename; confirm the two
   names and reload guidance after Deliver simulated result (30 seconds).

## Delivery record

Documentation-only delivery uses a `codex/` branch and direct diff review, as
required by the repository agreement for documentation changes. No local CodeRabbit
runtime review is claimed. Actual reviewed-head and merged-main CI, merge identity
and branch cleanup are recorded in the PR and local continuation record when they
complete; this file does not pre-claim those results.
