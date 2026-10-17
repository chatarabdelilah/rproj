# Existing-project composition design

Approved scope, October 17, 2026; baseline main `c5e257c`. The next deliverable
is a separate interactive Soft wireframe for review, not native controls.
Development continues while native preview launches and memory-stability
acceptance remain paused. No Windows troubleshooting or VM setup is included.

## Product boundary

Overview and Dependencies offer **Edit packages & capabilities…**. The flow is
**Dependencies → Capabilities → Review**, with the captured project identity
and full path visible. Existing package choices, capabilities, implementations
and dependency workflow are locked. New additions may be revised. Existing file
exclusions remain unchanged; removal, migration, version updates, optional-file
editing, saved-setup management and project adoption are outside this pass.

Recorded choices, manual Wally declarations, resolutions and file presence are
different facts. Manual dependencies remain visible without being imported into
the graph. Unsupported recorded composition or an absent/malformed record blocks
Apply without hiding the project or silently normalizing its data.

Review describes additions, derived tools/packages, affected files and reasons,
and external operations. It explains generated configuration refresh, possible
record reformatting, downloads and newly requested Blender scenes. This is an
impact review, not an exact file-diff or rollback promise.

## Interaction contract

One composition draft retains its original project across navigation. Resume
identifies that project. Replacing, discarding or reloading a dirty draft defaults
to Keep draft. A stale source invalidates the prepared review; reload requires
explicitly discarding the old draft rather than silently merging.

Apply requires explicit confirmation naming the project. Watch on that project
blocks Apply; Stop remains a separate, acknowledged action. Another project's
Watch is unaffected. Captured source identity is checked again at confirmation.

Simulated output and outcomes remain associated with their originating project.
Cancellation remains pending until the simulated worker finishes teardown.
Failure/cancellation/uncertain outcomes identify possible partial changes without
claiming rollback. Normal close during Apply defaults to Keep open; Cancel changes
and close names the project, warns about partial changes and waits for confirmed
teardown. Unconfirmed teardown keeps the simulated window open.

## Implementation boundary to resolve after design review

The terminal editor currently fixes packages and capability keys, omits workflow
editing and invokes the existing composition executor. That executor checks the
record source, acquires the Watch mutation guard, updates wiring/tools/Wally,
runs Upgrade and refreshes dependency/sourcemap output. Returned failures attempt
guarded restoration of the original record, not all project files.

The native proposal additionally locks existing implementations and preserves
existing exclusions. The terminal draft currently invalidates optional-file
choices when packages/capabilities change. Native implementation must explicitly
reconcile these differences; this wireframe does not change terminal behavior or
establish that a matching preparation/report worker already exists.

Catalog-derived file impacts also describe the proposed experience, not proof
that the current executor materializes every catalog artifact. In particular,
native planning must audit newly selected formatting and asset-pipeline files
against actual generator coverage rather than treating the catalog as an
executable file-change plan. Preparation, preservation, stale-input guards and
truthful partial outcomes need a separate implementation design.

## Delivery and validation

The prototype and validation artifacts are conversation-local. Earlier working
wireframes remain unchanged. All projects, paths, declarations and outcomes use
labeled sample data; no filesystem, worker, Watch or installer action is real.
This pass changes no Rust, public API, protocol, configuration schema or release.

## Local validation, October 17

The separate `rproj-project-composition-wireframe.html` conversation fragment
uses the approved Soft shell and the working New Project catalog fixture,
checked against current core catalog strings. Seven source hashes freeze the
core catalogs/graph, terminal draft/executor and Watch policy at baseline
`c5e257c`; the earlier New Project fragment's hash is unchanged.

All 36 local simulation/DOM/source checks and 18 Light/Dark palette comparisons
pass. Coverage includes fixed choices/exclusions, Wally/None, capability
requirements, manual declarations, invalid records, source/Watch changes during
confirmation, serial operations, Stop acknowledgement, cancellation and close,
retained project ownership/results, bounded output/filter states, panel height,
focus references, optional helper failures, escaped text and malformed saved
state. JavaScript parses successfully. The initial interface remains useful
before optional host integration; initialization failures have an in-context
error instead of replacing the whole workbench with Activity output.

The prototype and `target/composition-design/` checks are local review artifacts,
not shipped product code. Responsive CSS covers wide/narrow panes, long paths,
wrapping controls and coarse input. Actual geometry, browser modality/keyboard
traversal, inline host integration and Lucide rendering remain **unverified**:
DOM simulation and palette calculations do not establish those observations.
No new supported inline render-verification route was exposed by the available
tool metadata; retired browser/native routes were not retried. No browser,
native preview, external worker or installer was launched for this pass.

Native implementation requires owner review of step order, locked choices,
impact wording, Watch blocking and draft/recovery/close behavior, followed by a
separate implementation plan. No design-acceptance claim is made here.
