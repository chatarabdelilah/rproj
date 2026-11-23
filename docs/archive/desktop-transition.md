# Desktop transition

Approved October 8, 2026. This supersedes the Ratatui-only direction, not the
retirement of Tauri, model import, or embedded replacements for external tools.
The published baseline is 0.22.0; implementation starts from `17b7ef4`.

## Implementation checkpoint (October 16)

The native Soft workbench and Watch recovery are merged through
[PR #111](https://github.com/chatarabdelilah/rproj/pull/111), `9910df1`, with reviewed-head
and merged-main CI passing. The owner confirmed real Start/Switch/Stop; remaining
manual coverage is distinguished in [Soft acceptance](soft-workbench-acceptance.md).

Next, the owner selected guided New Project with an explanation/handoff to existing
machine setup. The [parity inventory](desktop-workflow-parity.md) records the gaps;
the [creation design](desktop-new-project-design.md) records the approved flow.
Shared composition and the headless worker merged in PRs #113 and #114. The native
five-step controls now use them; [creation acceptance](desktop-creation-acceptance.md)
records automated coverage and pending Windows observations. Existing-project
acceptance is separate from full workflow
parity, desktop distribution and macOS/Linux acceptance.

### Approved native design (October 15)

The owner approved the section sidebar, compact project switcher, and Activity
that opens when needed, then selected Soft as shown in the working comparison:
warm surfaces, roomier spacing, rounded controls, platform typography, labeled
Lucide icons, and restrained blue emphasis. This baseline merged in
[PR #107](https://github.com/chatarabdelilah/rproj/pull/107) at `24595f6`.
Operation origins stay visible across project navigation; Watch switching confirms
both project paths. The latest inline
comparison works for both proposals; older conversation previews are superseded.
This approves the design direction, not native acceptance or a public release.

The native implementation now uses GPUI Kit's project menu and alert dialog,
the four sidebar sections, registered Soft palettes, and the existing resizable
Activity panel. Desktop-private run IDs reject stale progress/results and retain
project labels; output labels survive bounded-history eviction. Watch confirms
both paths and rejects stale selection/ownership before using the existing
Stop-acknowledgement worker path. See [Soft acceptance](soft-workbench-acceptance.md)
for automated evidence, the isolated preview, and pending real-window checks.

### Historical prototype evidence (October 14)

The owner clarified that technical prototype work should continue, while the
current UI must not become the release design by default. Keep its navigation,
layout, and styling provisional. Wireframes and visual mockups still require
separate review; technical acceptance is not design acceptance. The owner
confirmed opening and resizing, then clarified that text and buttons are visible.
The initial missing-icons report was resolved on October 13: a targeted check
confirmed the Open project folder icon in both Light and Dark themes. Text/buttons
are readable in both themes. Tab/Shift+Tab focus stays visible between Overview
and Dependencies, and Enter/Space activate the expected section. Enter opens the
folder picker; Esc cancels while preserving the project and restoring Tab
navigation. The owner confirmed readable, unclipped controls and correct clicks
at 125% Windows Scale and after restoring 100%. On October 14, the owner confirmed
that a five-second Check stays responsive during theme changes, finishes, and
supports filtering and restoring its output. Do not restart the stalled native
capture/activation route.

A five-second disposable Check now passes through the shipped CLI worker with
installed Lute 1.0.0, preserving the existing Watch state, files, and permissions.
Together with the owner's native check, this completes the initial Windows GPUI
feasibility gate. The [audit](release-audit.md) records the exact evidence and
limits. Wireframe/design review is the next product milestone; full workflow
parity and public-release acceptance remain separate gates.

Work continues on `codex/desktop-alpha` in
[PR #106](https://github.com/chatarabdelilah/rproj/pull/106); no release candidate, publication, or
tag is prepared. The CLI/core workspace, versioned inspection, manifest-based
Watch summary, Blender preflight/repair, and CLI worker protocol are implemented.
The ordinary CLI/core suite passed 563 tests (536 CLI, 27 core), with 24 live or
prerequisite-dependent tests ignored. Core coverage includes
bounded configuration reads, diagnostic redaction, both output streams, and
Windows descendant cancellation. A test-build bridge now verifies Check/Test
dispatch, both output streams, exact failure codes, and Watch-lock refusal/recovery
with disposable tools and storage. The shipped Check worker now also has the
installed-Lute and native evidence above; real Test/Roblox runner acceptance
remains separate. Both CLI/core archives built successfully with
the final dependency graph using locked offline packaging in a fresh target
directory. Repeated packaging at the unchanged development version had reused an
older local-registry core source; CI now isolates that registry by run identity.

Read-only inspection of `scarywormtower` found 13 manifest aliases, including
manual `greentea`, `scribe`, and `uiLabs` additions, separately from 11 saved
composition keys. No project files were modified. Blender fixture coverage and
an explicitly executed installed-Blender temporary-scene test passed.

GPUI Kit 0.7.1 compiles with workspace-local Rust 1.99, renders a Windows window,
and exposes keyboard focus after adding an explicit focus owner. System Rust
defaults remain unchanged. The initial Windows feasibility gate is complete;
broader workflow acceptance remains pending. All seven executed PR CI jobs passed on
`00784b3`, including the portable build and extracted-copy checks. That portable
preview is the source of the owner's opening/resizing evidence. Complete
CodeRabbit review at `87dc635` found only a stale checkpoint statement, corrected
by direct documentation review; it reported no further code findings. Updated-head
CI and merge evidence are tracked in PR #106; the native gate no longer blocks
review of this unreleased prototype. An
initial portable ZIP passed extracted version, worker identity, read-only
inspection, and static-runtime dependency checks. See
[the framework record](desktop-feasibility.md). The complete uncommitted CodeRabbit
review found one inspection-recovery bug, corrected before final branch review.
Eight desktop tests now pass, covering recovery, keyboard navigation,
unavailable actions, preference preservation, catalog failure visibility, and
bounded faithful output. Watch-output requests are serialized and reject stale
results after project, worker, or Watch changes. The earlier partial tracked-file pass does not count as
complete review.

## Product and release boundary

Desktop is the future primary human interface: a native project workbench,
guided by default, without AI chat. Keep CLI commands for people and automation.
Maintain Ratatui until desktop replacements have verified workflow parity.
Use the same repository and coordinated alpha versions for `rproj`, `rproj-core`,
and `rproj-desktop`; the core is initially a shared product library, not a stable SDK.

First public desktop alpha: Windows, existing rproj and ordinary Rojo projects,
inspection, Watch, existing checks/tests, and logs. Do not automatically adopt
projects or rewrite configuration on open. Ship a portable ZIP with matching
desktop/CLI binaries and license notices, runnable without Rust. Bare `rproj`
keeps its terminal behavior. `rproj app` is explicit. Owner-only Cargo publication
and publication-before-tag rules still apply; no version bump is needed to start.

## Implementation sequence

1. Fix Watch's manifest/saved-selection discrepancy and Blender's stale machine
   selection gate. A requested 3D artifact must not disappear because Blender was
   installed outside rproj. Preflight before writes; explicit `rproj setup blender`
   repairs a missing scene and preserves an existing one. Never alter a user's
   project merely to test the fix.
2. Review wireframes, then validate a real GPUI Kit window with a small shared-core
   slice. Gate on obtainable released dependencies, Windows build/launch,
   responsiveness, keyboard/focus, resizing/scaling, and package launch. Pin a
   release and lockfile. A material failure stops broad GUI extraction: record
   evidence and revisit the framework rather than silently substituting one.
3. Build read-only project inspection and `rproj inspect [path] --json` over the
   same versioned snapshot: declarations, lock resolutions, package presence,
   tool pins/detection, diagnostics, and action availability. All Wally realms,
   aliases, and unfamiliar packages remain visible. Malformed is not empty;
   lockfile presence is not installation proof. No subprocess or network on inspect.
4. Add desktop project/recent navigation, package details from existing catalog
   content, Watch controls, checks/tests, and bounded logs. Operations use explicit
   paths, structured state/results, progress, and cancellation off the UI thread.
   Preserve current external execution and file safeguards; never scrape human
   output for state. Serialize conflicting work and retain one active Watch.
5. Verify CLI/core/desktop parity and the portable package, update review/CI
   boundaries, and deliver reviewed PRs with checks on reviewed head and merged main.

## Watch and operation contract

Watch prepares dependencies and maintains the Rojo sourcemap. It is not an
automatic updater or Studio synchronization service. Keep one active background
project and leave it running when the desktop closes. Show its project globally;
Stop-and-switch waits for acknowledgement. Unresponsive/failed/cancelled states
must not be reported as successful. The desktop uses an explicitly located,
matching CLI worker rather than treating its GUI executable as the worker.

Manual manifest edits refresh inspection without silent installation or upgrades.
Checks use the existing `lute run check` when configured. Tests retain their
existing selected runner/backend and prerequisites. Existing environment-based
credentials keep working; snapshots and diagnostics never expose secret values.

## Design

Wireframes precede native implementation. Use a quiet sidebar workspace with a
dominant project view and resizable activity area. Start with system/light/dark
appearance, readable platform typography, visible labels, keyboard access,
contextual help, and a wordmark. Logo design does not block the functional gate.

Use [GPUI Kit's coding and design guidance](https://github.com/longbridge/gpui-kit/tree/main/skills)
when implementing GPUI. [Lucide](https://lucide.dev) supplies command icons;
[Charmed Icons](https://github.com/littensy/charmed-icons) is the file/folder icon
trial candidate, with copyright/license notices retained for bundled assets.
[Skill Icons](https://github.com/tandpfun/skill-icons) remains a branding reference.
Figma is optional; shadcn/ui and ReUI are design references, not native Rust components.

## Acceptance evidence

- Inspection: added/removed/unknown/aliased dependencies, all realms, missing or
  stale lockfiles, absent package files, invalid inputs, plain Rojo, and no writes.
- Blender: installed without saved selection, unavailable executable, explicit
  output exclusion, saved compositions, existing-scene preservation, repair.
  The reported `scarywormtower` project requests 3D and drops nothing but has no
  Blender folder; Blender 5.0 is installed and global app selections are empty.
  Its historical creation path is not proven; reproduce with isolated fixtures.
- Operations: both output pipes, failures, cancellation, no conflicting work,
  Watch persistence, duplicate starts, switching acknowledgement, stale owners.
- Desktop: real-window keyboard/focus, themes, long paths, scale/resize, large
  lists/logs, offline inspection, empty/error/permission states, packaged launch.
- CI: Windows stable/MSRV for applicable packages, portable core Linux/macOS,
  desktop build/interaction checks, package-from-tarball verification, and
  applicable live tests. Ignored/missing-prerequisite checks are not passes.

## Ordered follow-up planning

1. Creation, machine setup, configuration editors, saved compositions and template
   trees; retire Ratatui only after these workflows and recovery paths are verified.
2. Richer maintained catalog examples; Spark vs native Input Action System;
   Scribe/Studio and independent ProfileStore guidance; optional
   [Bedrock](https://github.com/christopher-buss/bedrock) and
   [Roblox-BlenderBridge](https://github.com/TylerAtStarboard/Roblox-BlenderBridge).
3. Cached update discovery, reviewed tool/package updates, validation, an
   installation-aware rproj updater, and OS-backed credentials. Keep ordinary
   tool configuration in its own project files. Selene's local Roblox std is an
   explicit reproducibility option, not a mandatory generated file.
4. Windows installer, independently accepted macOS/Linux desktop distributions,
   platform setup/Watch support, final branding, and additional theme controls.

Do not add plugins just to accumulate tooling. CodeRabbit remains focused on
behavior, configuration preservation, and execution safety; measure CI before
removing or restructuring checks. GitHub/Figma integrations are optional.
