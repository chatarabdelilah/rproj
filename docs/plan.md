# rproj - Release Roadmap

Updated September 5, 2026. This describes current priorities, not every idea considered during development. [Architecture](architecture.md) describes implementation; [UX](ux-redesign.md) defines the interface; [Releasing](releasing.md) defines publication gates.

## Direction

**Make it straightforward to set up and maintain an ordinary Roblox/Luau project on Windows.**

rproj connects existing tools, explains choices, derives coherent configuration, and helps users recover when setup fails. Generated projects must remain usable without rproj.

- Ratatui remains the interactive interface: the workspace hub, Catalog, and Template Explorer.
- Direct commands remain available for automation and normal subprocess output.
- Rojo owns model loading, builds, synchronization, and sourcemaps.
- Selene owns linting, StyLua owns formatting, and luau-lsp owns type analysis.
- Rokit, Wally, Git, Lute, and test runners remain external tools with project configuration and pins.
- A library dependency must serve an approved feature. Availability alone is not a reason to embed a tool.

## Current State

The published baseline is **v0.12.1, public alpha**. **v0.12.2** is the release-hardening candidate, with [audit evidence and limitations](release-audit.md). Public availability does not mean every integration or configuration contract is ready for a stable release.

The existing product includes machine setup, configurable project generation, saved compositions, upgrades, tool configuration, the global Template Explorer, workspace hub and Catalog, watching, source copying, and optional TestEZ/Jest Roblox testing.

The unfinished v0.13.0 model-import experiment is **shelved and outside the release baseline**. Work is preserved on `codex/m7-model-import`; it must not be merged as part of release hardening. Its version bump does not establish a release commitment.

## Scope Removed

| Former proposal | Decision |
| --- | --- |
| M7: broad library migration | Cancelled. There is no goal to move the toolchain into rproj. |
| Embedded Selene or StyLua | Removed. Working external tools do not need internal replacements. |
| Full Moon | Removed. No approved standalone Lua/Luau parsing requirement exists. |
| M6 / M7a + M6a: static model-to-template conversion | Shelved, not a release prerequisite. Conversion and data-preservation obligations are unnecessary for the current product. |
| Tauri GUI and GUI-driven core-library extraction | Dropped, not deferred. Ratatui is the selected interface direction. |
| R4: additional project types | Uncommitted backlog, requiring a concrete use case and working build targets. |
| M8: rproj Studio plugin | Uncommitted backlog. No new plugin is needed to release the existing product. |

Rojo already accepts `.rbxm` and `.rbxmx` models and filesystem mounts. The shelved importer offered a different convenience: converting supported models into self-contained, editable global-template JSON. That does not justify delaying release. This decision does not expand rproj's current global-template `$path` rules. See Rojo's [sync details](https://rojo.space/docs/v7/sync-details/) and [project format](https://rojo.space/docs/v7/project-format/).

## Next: Release Hardening

No new feature milestone is required first. Audit the baseline, fix concrete defects, and record evidence for workflows users already have.

### 1. Establish The Baseline

- Run formatting, ordinary tests, clippy, and locked packaging on a clean checkout.
- Verify Windows stable and Rust 1.89 CI, tag/release/package alignment, and outstanding CodeRabbit findings.
- Review ignored integration tests and architecture's manual-verification checklist. Historical passes are not evidence for the current candidate.
- Record checks as passed, failed, or blocked with commands, versions, and prerequisites. Missing Studio or Open Cloud access is not a passing test.

### 2. Verify Existing Workflows

| Area | Required evidence |
| --- | --- |
| Machine setup | Deliberate installation prompts, useful missing-tool errors, and safe reruns. Use an explicitly provisioned test environment; ordinary tests must not install applications. |
| Creation | Coherent minimal, Wally, and submodule output and pins; saved setups and revisions preserve choices. |
| Template Explorer | Editing, validation, reset, repair, cancellation, and terminal restoration preserve the last valid template. |
| Daily use | Dependency recovery in Watch, correct runner dispatch, and documented Copy/Catalog behavior. |
| Upgrade/configure | Review and cancellation work; managed fields update without damaging source or unrelated configuration. |
| Checks/CI | Clean fixtures pass; deliberate lint/format/type/test failures fail. Missing Jest credentials fail clearly. Record live Studio/Open Cloud evidence separately. |
| TUI/CLI | Wide, narrow, and small-terminal behavior, Unicode input, Ctrl+C, and plain redirected output. |

### 3. Fix And Document

Fix blockers in small reviewed changes with regression tests. Prioritize data loss, broken generated projects, misleading success, terminal damage, and installation/recovery failures over cosmetics.

Bring README instructions, architecture claims, test inventory, and troubleshooting into agreement with verified behavior. Do not mark a gap closed merely because an unexecuted test exists.

### 4. Ship The Verified Candidate

Choose the version after the changes are known: a patch for compatible fixes, a minor alpha release for intentional behavior changes. A roadmap edit alone does not require Cargo publication.

Pass the [release checklist](releasing.md), review CodeRabbit, merge, verify main CI, and remove merged branches. The owner runs `cargo publish --locked`; tags and the GitHub prerelease follow confirmed crates.io publication.

## After Hardening: Continue Ratatui

The next feature candidate is **project creation inside Ratatui**: choose dependencies, packages, and capabilities, revise the summary, and confirm creation without switching between unrelated prompt styles.

This is not a release prerequisite. Scope it after the audit, using the existing project graph and execution logic, not another application framework. Preserve guided/expert behavior, saved compositions, generated output for unchanged choices, and direct commands.

Further configuration or upgrade screens should address observed friction. A full-screen wrapper around every long-running subprocess is not a goal by itself.

## Release Stages

- **Alpha, now:** usable public releases with explicit limitations and potentially changing configuration contracts.
- **Beta:** current end-to-end evidence for critical workflows, no known release blockers, and defined command/configuration contracts intended for stabilization.
- **Stable 1.0:** beta usage supports those contracts and upgrade/recovery behavior is dependable. Feature count or an arbitrary date does not establish readiness.

The old remaining-days total is retired because it assumed features no longer planned. Estimate individual approved changes after their scope and test requirements are understood.

## Delivered History

Legacy IDs explain earlier discussions; they no longer determine the sequence.

| Delivered work | Releases |
| --- | --- |
| M1-M3b: artifacts, catalog additions, per-tool setup, coherent requirements | v0.3.0-v0.4.0 |
| R1-R2: capabilities and persisted project graph | v0.5.0-v0.6.0 |
| R2b-R3: catalog refresh, obsolete-tool removal, capability information | v0.7.0-v0.8.0 |
| M5-M5b: validated global template and built-in Explorer | v0.9.0-v0.10.2 |
| T1: shared Ratatui foundation, workspace hub, Catalog | v0.11.0 |
| M4: Jest Roblox as a TestEZ peer | v0.12.0 |

**Active sequence: release audit -> targeted fixes -> verified release -> evaluate the next Ratatui workflow.**
