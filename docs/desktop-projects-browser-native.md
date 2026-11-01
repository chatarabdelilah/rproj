# Native Projects browser

Approved November 1 against clean main `3f3e723`. The owner selected the
[preview-and-open design](desktop-projects-browser-design.md) for implementation.
The earlier wireframe review is a design decision. Its manual rendering and
real-window acceptance remain unverified.

## Shared discovery

The terminal and desktop use the same read-only scanner. Each row has a
canonical path, a folder name and an origin (projects folder, launch folder
or both). Direct children qualify by the existing rproj or default Rojo file
marker, even when the file is malformed. Linked children are skipped with a
warning. A recognized launch folder remains available when the configured
folder is unavailable. Sorting, terminal navigation and actions stay compatible.

The private `projects-context` query reads the machine configuration through
the matching CLI and returns a versioned projects-folder path. Relative settings resolve against the
original launch directory, never against the worker executable’s directory. Each
refresh resolves it again. The query needs neither machine setup nor external
tools and does not create the directory. Filesystem discovery and inspection
run outside the UI thread. Recent folders belong to the switcher.

## Native handoff

The Projects browser overlays the originating workbench view. Its selection,
preview, query and list position belong to this session. Only a successful
explicit Open commits a new project, fresh snapshot, Overview and recent entry.
The checked handoff preserves the active workspace on errors or stale results.
Watch and running operations retain their owners.

Delivery uses two reviewed PRs, first the scanner and context query, then the
native controls. Native preview and memory-stability acceptance remain paused.

Shared-branch verification passed 790 CLI/core tests with 24 prerequisite-based
tests ignored, strict Clippy, formatting and fresh joint package verification.
The first local review identified the worker-directory ambiguity for relative
configuration. A fixture now changes the root between requests and verifies
relative resolution against the launch directory. The second review had no
findings.

## Native controls and automated acceptance

PR #145 merged the shared scanner at `b7dc664`. Reviewed head `d79dad4`
(CI `41291128540`) and merged main (CI `41291544581`) passed all seven
executed jobs on their first attempts. The trees match and the merged branch
was removed.

Browse projects… is available in the project switcher and empty workspace.
Back returns to the originating view and restores keyboard focus. Filtering
matches folder names and full paths. A virtualized result list keeps its scroll
handle across navigation. The list and detail are adjacent at 600 logical pixels
of main-content width and stack below it. The detail identifies the full path,
discovery source, record and Rojo file status and inspection diagnostics.

Scan, preview and explicit Open have independent request identities. Refreshes
coalesce, and selecting quickly queues only the latest preview. A failed Open
cannot be hidden by an older preview. Changing the query while the same project
is still selected preserves its current detail. Open rechecks the canonical
directory and shared marker rule, then installs the fresh snapshot, Overview
and recent entry together. A folder replaced by a junction is refused. Watch,
Activity, operations and other drafts retain their original owners.

The first native build caught missing GPUI trait imports and a focus accessor
argument. The headless control tests then caught delayed focus restoration and
a skipped recent entry when preferences had not loaded. Back now focuses the
origin immediately, and session recents merge with preferences when they arrive.
A full-suite run exposed an empty-view layout regression that hid New Project
at minimum size. The two entry actions now share a wrapping row. CodeRabbit's
first native review identified unnecessary preview reads while filtering, covered
by a regression that checks the same selection does not restart inspection.

Headless evidence is separate from real-window acceptance. Native memory
stability remains unresolved and Windows previews remain paused. macOS/Linux
desktop acceptance is unverified. Published version is 0.22.0.
