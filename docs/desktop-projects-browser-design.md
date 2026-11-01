# Projects browser proposal

Design scope approved November 1, 2026, against main `f5ada11`. The
[interactive sample](prototypes/projects-browser.html) starts with Skyline
open and watched. Lantern is selected for preview. Its Rojo file is readable
and its rproj record is malformed. **Open project** is the only action in the
preview that changes the active workspace.

**Browse projects…** opens from the compact project switcher or the empty
workspace. Back returns to the originating section or empty view and restores
focus. The switcher keeps recent folders, including ones outside the projects
root. Browsing lists recognized direct children of the configured folder and
the recognized launch directory. Sample discovery skips linked children and
ignores nested projects. Canonical fixture paths identify rows, including two
projects named Relay. A project discovered in both places gets one row with
both origins.

The preview identifies the folder, full path, discovery source, rproj record,
Rojo file and diagnostics. It uses adjacent panes starting at 600 logical pixels
of main-content width and stacks below that threshold. Search matches names
and paths. Returning to the browser restores the query, selected path and list
position. Open activates Overview and updates recents. Watch, the retained
creation draft and prior check output keep their original owners.

Refreshing keeps usable rows on screen, coalesces repeated requests and ignores
obsolete scan and preview results. If the selected folder disappears, the user
can refresh without losing the active project. Empty results, no search matches,
a missing or unreadable root, skipped links and a readable project with a
malformed record each have a distinct explanation.

All paths, diagnostics, operations and workspace contents in this proposal
are local sample data. The terminal's ten project actions stay outside the
preview. Native implementation planning follows design review.
