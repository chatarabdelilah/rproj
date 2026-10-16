# Guided desktop New Project

Approved direction, October 16, 2026: New Project first, guided steps, and an
explanation/handoff to existing setup when machine setup is missing.
This is a conversation prototype for review, not native creation or file output.

## Entry and navigation

Use the accepted Soft shell. Add New project… to the compact project menu and
the empty-project view. Creation occupies main content; project sections,
appearance and global Watch remain available. A session-only draft survives
browsing existing projects. The menu offers Resume project creation. Explicit
Cancel creation protects a changed draft with a default-Keep confirmation.

| Step | Content |
| --- | --- |
| Project | Name, full read-only destination under the configured root, current template and machine-setup state. |
| Dependencies | Guided/Expert/saved starting point; Wally/None; compatible package choices with search and descriptions. |
| Capabilities | Existing capabilities and concrete implementations, including explicit testing/backend compatibility. |
| Files | Derived tree with reasons; optional exclusions only, visibly locked required entries. No template editor. |
| Review | Choices, packages/tools/files, side effects and optional new named setup; explicit simulated Create confirmation. |

Guided is the default. Back/Next and filtering retain unrelated work. Changing
dependency strategy explains cleared packages and recomputed file choices before
commit; incompatible testing stays visible until explicitly resolved. Saved
compositions preserve concrete implementations/exclusions. Invalid or unsupported
saved data is shown as an error, never silently normalized.

## Simulation contract

Use existing repository catalog entries and representative graph fixtures as the
behavioral source. The sample catalog is a labeled, frozen repository snapshot;
the simulated derivation is representative, not a replacement for Rust graph
execution or a current-version recommendation. No external package research or network operation
is required. Keep links to the catalog's documentation destinations.

Machine setup missing shows `rproj setup` and a simulated recheck, with no native
installer, credential entry or automatic provisioning. Name/device/path validation,
destination collision, invalid template and saved-setup collision block Create.
Saving a composition uses a new name; there is no replacement or manager UI.

Simulated Create captures its originating destination and immutable reviewed
choices, expands Activity and keeps output available across navigation. Cancel
stays pending until the simulated worker outcome. Failure/cancellation after work
starts can leave incomplete files: show the path and require inspection or a new
destination, not overwrite, delete or rollback. Success offers Open project;
it never starts or switches Watch.

Watch and its sample owner remain global throughout draft editing, project
navigation and simulated creation. Scenario controls sit outside product chrome.
All sample operations are labeled; no project files, preferences or machine state
are read or changed by the conversation prototype.

## Validation and review

Validate syntax, scoped selectors, unique identifiers, accessibility references,
keyboard focus, dialog/menu dismissal, draft retention, filtering, saved replay,
graph invalidation, required-file protection, captured results and cancellation.
Check success, missing setup, invalid template, unsupported setup, destination
collision and partial failure/cancellation scenarios.

Use responsive wide/narrow Soft layouts and readable Light/Dark/System modes.
Reuse the working preview's product palette and controller conventions. Keep a
useful static initial form; show initialization errors in context. Do not add a
ResizeObserver/ancestor-sizing dependency or rely on optional host helpers.
One supported render route may verify actual interaction/geometry; unavailable
rendering remains unverified. Do not retry retired native or failed browser routes.

The editable conversation fragment lives in the task-owned visualization folder,
named `rproj-new-project-wireframe.html`; validation artifacts are under
`target/new-project-design/`. Older working proposals remain unchanged.
The review selects navigation, wording, step order, recovery and file-review
behavior before native work is planned. No Rust/public API/schema/release change
belongs to this pass. [Workflow parity](desktop-workflow-parity.md) records gaps.

### Local validation, October 16

All 32 simulation/source/DOM checks pass using Node 25.2.1 and jsdom 26.1.0.
The full authored controller runs against the fragment, with deterministic timers
and a dialog-state adapter. Coverage includes all five steps/scenarios, invalid
names/destinations, saved replay, companion packages, capability/file invalidation,
required entries, explicit confirmation, navigation/draft retention, cancellation
before/after destination claim, failure, old events and bounded output. DOM checks
cover initialization, selectors/IDs/labels/ARIA references, input focus retention,
menu Escape, confirmation dismissal, both theme selections and width controls.
Host-state echo preserves the session draft; unavailable optional helpers do not
break initialization. A deliberate startup error leaves the initial form visible.

JavaScript syntax and source snapshot hashes pass. The 18 selected text/surface
pairs meet 4.5:1 contrast in the two Soft palettes. The fragment is below 1 MB,
with no network runtime, ancestor sizing or ResizeObserver dependency. Required
and optional file expectations were checked against the existing Rust catalog.
This does not establish equivalence with all Rust graph/execution behavior.

The local preview runtime has no browser executable available. Actual rendering,
wide/narrow geometry, host Lucide replacement, browser keyboard traversal and
native dialog modality remain **unverified**. DOM adapters are not browser
acceptance. No browser was installed and retired/failed preview routes were not
retried. No native acceptance is claimed by this design pass.

Validation scripts, frozen catalog/source hashes and JSON results are retained
under ignored `target/new-project-design/`; `node target/new-project-design/validate.cjs`
reruns this local session's checks. The prototype is conversation content outside
the repository, not part of the package or a shipped feature.

### Bounded design review

1. Follow Next through Dependencies and Capabilities; review whether the choices
   and their explanations belong in those steps.
2. Inspect Files and Review, including excluded optional entries and setup save.
3. Use a missing-setup or failure scenario; review the handoff/recovery wording.
4. Try Narrow and Light/Dark, then approve the flow or name a specific change.

Only simulated Create runs here. Native creation remains behind this review.
