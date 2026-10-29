# Project Template proposal acceptance

October 29, 2026. This records the separate conversation prototype described in
[the design](desktop-project-template-design.md), not native implementation or
real-window acceptance. Owner review of this specific proposal is pending.

## Confirmed local evidence

`node target/project-template-design/validate.cjs` passes **57 checks** and
**26 numeric Light/Dark contrast comparisons**. `node --check` passes for the
model and controller; the assembled inline script parses independently.
The new fragment is `rproj-project-template-wireframe.html`. The ignored local
validation directory retains model/controller sources, the build/validation
scripts, earlier-fragment hashes and the JSON results. All six earlier working
wireframes retain their original SHA-256 hashes.

- Fixtures distinguish the built-in source, custom override, empty/malformed
  text, unfamiliar fields, linked/unreadable paths and missing/rejecting Rojo.
  The built-in fixture includes the current three managed mounts and Lighting /
  ColorCorrection defaults. Representative metadata is explicitly a sample.
- Model checks cover shared Explorer/JSON state, typed values, tree operations,
  protected identities/mounts, reserved names, path restrictions, incompatible
  property retention, undo/redo, no-op byte preservation, source invalidation,
  Save/Reset baselines and retained machine ownership across project navigation.
  Sequential operation identifiers reject stale results. Watch stays independent.
- DOM checks cover useful startup without host helpers/icons, change/review/save,
  advanced-text repair, default-Keep confirmation focus, menu Escape, folder
  cancellation, dirty reload/replacement, reset removal diff and built-in result,
  stale confirmation refusal, Activity search/height and completion/close races.
  Unique identifiers and accessibility references resolve in the tested screens.
  Host-restored state affects appearance only, never drafts or operations.
- Diff checks cover old/new line numbers, three-line context, all changed rows,
  escaped controls, CRLF and missing-final-newline annotations, and a 2,200-row
  change without truncation. Reset reviews deletion of the override, not an
  invented write of the built-in document. Changed saves show formatting effects.
- Source checks confirm stacking below 600 main-content pixels, shell reflow,
  wrapped paths/diffs and visible focus. Tested text pairs exceed 4.5:1; accent
  contrast also exceeds the 3:1 focus minimum. Activity resizing uses an accessible
  height slider in this proposal. These are source/numeric checks, not measured
  browser geometry or native drag acceptance.

The initial 51 checks passed. Direct source review added five regressions and
corrected operation-fixture replacement, tree-selection focus, uncertainty's
pending-close state and in-dialog stale-action explanations. The expanded run
passed 55/56: dismissal of a stale Save dialog returned to Project Template
instead of Reload because the fallback trigger was selected too early. After
correcting that ordering, all 56 checks passed. A final source review also guarded
structured Undo/Redo against discarding unapplied Advanced JSON; its added
regression brings the final run to 57 passing checks. No failed run is counted
as a pass.

## Unverified and deferred

Available tools expose no supported inline rendering-inspection route. Actual
wide/narrow geometry, System appearance, host icons, browser keyboard activation,
modal focus trapping and assistive-technology behavior remain unverified. JSDOM
uses dialog stubs and cannot establish these behaviors. No retired automation
route or native preview was attempted.

The sample class/property catalog is intentionally incomplete; numeric sample
controls use exact JavaScript-safe integers. Unknown data remains available in
Advanced JSON. Structural checks and eight generation-variant/Rojo outcomes are
simulated, not evidence of production validator equivalence. Source identities,
Save/Reset, termination, Watch and independent workflow drafts are local models;
no actual file operations, tool executions, atomicity or rollback are claimed.

Native memory stability is unresolved. Windows real-window/scaling and macOS/Linux
desktop acceptance remain paused or unverified. This milestone changes no Rust,
API, worker protocol, project/configuration format or release. Published version
0.22.0 remains unchanged. Native implementation requires subsequent design approval.

## Bounded owner review

1. In Lighting, change Brightness from 2.5 to 3 and choose Set value. Choose Review
   changes, then Simulation controls → Finish active operation. Inspect the diff
   and open Save template, then Keep reviewing (60 seconds).
2. Open Advanced JSON, type invalid text and choose Apply to draft. Check the
   repair message; Reload from file → Discard and continue restores the fixture.
   Choose Restore built-in template and inspect its separate removal review and
   default-Keep confirmation (60 seconds).

Choose whether Explorer / Advanced JSON, the mandatory review and reset treatment
are ready for native implementation planning. No native launch is needed.
