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
the matching CLI and returns a versioned absolute projects-folder path. Each
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
