# rproj - UX Direction

This describes the current interaction contract and subsequent direction. Historical milestones are in [the roadmap](plan.md); implementation details belong in [architecture](architecture.md).

## Product Principle

**rproj asks for decisions about a project, derives the required configuration, and shows what it will do.**

Users choose a dependency strategy, packages, and capabilities. Tools, pins, and required artifacts follow. Optional artifacts remain adjustable where removing them does not invalidate an earlier decision.

Choices need plain-language explanations. Unavailable options should explain prerequisites rather than disappear or silently change the project.

## Interface Boundary

Ratatui is the selected interactive interface. There is no planned Tauri application or separate desktop frontend.

The current interface is hybrid:

- Bare `rproj` opens the Ratatui workspace hub in an interactive terminal.
- Catalog browsing and global-template editing stay within full-screen interfaces.
- Hub-driven New Project uses Ratatui for composition and review. Setup, direct `rproj new`, and tool configuration retain inquire prompts.
- Home, Projects, Catalog, creation questions, and Template Explorer borrow one terminal session. There is no alternate-screen teardown between these internal views.
- The hub suspends the terminal session before invoking existing command implementations, presents their result, and waits for Enter before returning to the selected project or Home. It keeps the selected action and refreshes workspace context.
- Direct commands remain available for automation. Redirected welcome and Catalog output remain plain.
- External tools continue doing their own work. Navigable results can be considered without embedding replacement tools.

T2 (0.14.0) extends hub-driven creation with persistent Home navigation and Catalog clarity. It does not replace the external toolchain or migrate Machine Setup to Ratatui.

## Project Decisions

The graph records intent and derives coherent tools and artifacts:

```text
Project
|-- Dependency strategy: Wally, Git submodules, or none
|-- Packages: code the project depends on
|-- Capabilities: workflows the project supports
|   `-- Compatible implementation, required tools, and artifacts
`-- Reviewable summary
```

Guided and expert interaction serve different experience levels without changing the meaning of a choice. Saved setups reuse decisions.

Changing an upstream choice invalidates only incompatible downstream choices. It must not silently select another runner or discard unrelated preferences.

| Capability | Implementations | Selection |
| --- | --- | --- |
| Linting | Selene | No redundant implementation prompt |
| Formatting | StyLua | No redundant implementation prompt |
| Type checking | luau-lsp | No redundant implementation prompt |
| Quality gate | Lute | No redundant implementation prompt |
| Continuous integration | GitHub Actions | No redundant implementation prompt |
| Asset pipeline | Asphalt, Tungsten | Choose an implementation |
| Testing | Jest Roblox, TestEZ | Wally offers both alphabetically without a recommendation; other workflows use TestEZ |

Testing stays optional. TestEZ remains the internal compatibility fallback for older configurations; display order must not change a stored runner.

## Workspace Hub

T3 (0.15.0) makes Home machine-level: Projects, New Project, Edit Project Template, Machine Setup, Catalog. Project selection is session-only. Home still reports machine setup, saved setups, global template state, and cached updates.

Projects discovers only immediate, non-linked children of the configured projects root and the recognized launch directory. Missing roots and malformed projects have explanatory warnings; browsing never creates directories or invokes tools. Filtering matches names and paths. F5 refreshes on one read-only worker; stale results are discarded. Enter opens the selected project's actions; Back preserves the browser state.

Wide terminals show actions and details side by side; narrow terminals switch focus between stacked panes. Minimum-size screens retain Help and Exit. Esc and Ctrl+C exit cleanly.

The Catalog excludes Place Template, which belongs to the dedicated Explorer. It groups Packages by category and Tools by installation type. VS Code contains Extensions and Themes & Icons. Groups sort alphabetically; search covers the current group and descendants. Existing package examples, metadata, and named lookups remain available.

Enter opens a group or focuses details. Tab changes panes; arrows/Page Up/Page Down/Home/End navigate or scroll. Esc unwinds detail focus and the navigation stack, restoring selection/filter/scroll. Ctrl+C leaves Catalog for Home, or exits standalone Catalog. Named lookups and redirected listings remain plain.

Configure Tools, Upgrade, Watch, Test, and Copy Source live on the project screen, with unavailable reasons. Each action revalidates its selected directory, runs outside the alternate screen, and returns to the same project after acknowledgement. Esc returns to Projects; Ctrl+C returns Home. Successful creation opens the new project and selects it in the refreshed browser. Setup and cancelled/failed creation keep Home recovery. Watch remains foreground-only and awaits the active child before returning.

## Template Explorer

`rproj configure project` edits the machine-wide template for future projects, not existing project files.

The Explorer provides structural instance edits, a typed Inspector, settings, undo/redo, and built-in Advanced JSON for uncommon values. No external editor is launched.

rproj owns the project name, DataModel root class, conventional source mounts, and dependency/testing mount positions. Static nodes and settings remain editable around those boundaries. Ownership errors identify conflicting paths instead of silently overwriting them.

Ctrl+S requires structural checks and the complete Rojo validation matrix before atomic replacement, then stays in the editor. Selection, mode, and bounded history remain. Only successful persistence updates the saved baseline; undo after saving becomes dirty. JSON repair follows the same rule. Errors are scrollable and retain the draft.

Esc retains local Back behavior; leaving Explorer and Ctrl+C protect unsaved changes since the last save. Reset requires confirmation, removes only the custom template, and returns to the caller. Standalone editing owns its terminal; Home editing borrows Home's terminal.

Model-file conversion/import is not part of this interface. Rojo's model support remains available in ordinary projects; this does not expand global-template filesystem-path permissions.

## Summary And Preservation

The summary renders the graph and explains why files are created. Users can revise decisions before creation. Files needed for selected workflows must not be offered as independent removals.

Configure and upgrade preserve unrelated user-owned content and refuse data they cannot safely interpret. Ownership and merge behavior are artifact-specific; do not claim every configuration is universally mergeable.

The 0.13.1 `configure` fix keeps existing values outside the guided catalog behind a default-No replacement confirmation. Existing answers that remain unchanged are not written; missing settings still receive prompted defaults. Unsafe TOML layouts produce a refusal with manual-edit guidance, not a damaged file. This safety correction retains inquire; further Ratatui configuration screens are not implemented by this change.

Errors identify the operation and an actionable recovery step. A missing executable is not a lint failure; a test failure is not permission to switch runners.

## Ratatui Creation

See the bounded [project-creation plan](ratatui-project-creation.md) for scope and
verification. [Diagnostic logs](diagnostic-logs.md) record semantic
choices/actions without turning the interface into a keystroke recording.

The hub creation flow uses the existing graph:

1. Enter the name and retain the current machine-setup boundary.
2. Choose dependencies, packages, and capabilities through guided or expert interaction.
3. Review the derived tree/files and revise individual choices without losing unrelated work.
4. Confirm before filesystem or provisioning changes; restore the terminal before existing execution logic runs.

Require generated-output parity for unchanged choices, saved-setup parity, Unicode/responsive tests, cancellation coverage, and unchanged direct command behavior.

Additional project types, a Studio plugin, embedded quality tools, and a desktop GUI are not release prerequisites. The roadmap records their scope decisions.
