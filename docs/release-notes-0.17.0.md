# rproj 0.17.0 - T5: Ratatui Machine Setup

Alpha release. Published archive and annotated tag verified at `9ee8d5167175c7bacb58a61e9317b4fda5d443ea`.

## Changes

- Machine Setup opens a review-first Ratatui screen with searchable selection categories, including separate VS Code extensions and themes/icons. Existing empty selections, unknown keys, and inactive parent-dependent choices are preserved.
- Apply requires explicit default-No confirmation. Opening or cancelling does not probe tools, install software, save configuration, or create directories. Deselecting does not uninstall anything; the projects folder remains read-only.
- Sequential installation progress, item results, output, and recovery/manual instructions remain inside the TUI. Home shares one terminal session; standalone setup and first-use/reconfigured creation use the same flow.
- Stop confirmation waits for the active item and prevents later items and configuration saving. Completed installations remain. Completed runs with individual failures clearly report warnings; bootstrap and persistence failures remain failures.
- External installers, project tool pins, configuration schema, project workflows, and named project-tool setup remain unchanged. No dependency was added.

## Limits

Installation output is bounded and terminal controls are stripped. Native installer/UAC dialogs can appear separately, and commands that require terminal input must be completed manually. Cancelling cannot force-stop a stuck installer or roll back installed software. Machine configuration stages a complete document before atomic replacement; its schema is unchanged.

Tests use injected configuration and harmless subprocesses. Fresh-machine winget/Studio/Blender/VS Code installation and vendor dialogs are not claimed as verified; they require a separately approved environment. No unrelated live integrations were rerun.

## Release Ownership

The owner runs `cargo publish --locked` only after the agent completes review, merging, and main CI. The annotated tag and GitHub alpha prerelease are created only after the published archive is verified.
