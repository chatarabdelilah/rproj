# rproj - Release Roadmap

Updated September 9, 2026. This describes current priorities, not every idea considered during development. [Architecture](architecture.md) describes implementation; [UX](ux-redesign.md) defines the interface; [Releasing](releasing.md) defines publication gates.

## Direction

**Make it straightforward to set up and maintain an ordinary Roblox/Luau project on Windows.**

rproj connects existing tools, explains choices, derives coherent configuration, and helps users recover when setup fails. Generated projects must remain usable without rproj.

- Ratatui remains the interactive interface: the workspace hub, project creation, Catalog, and Template Explorer.
- Direct commands remain available for automation and normal subprocess output.
- Rojo owns model loading, builds, synchronization, and sourcemaps.
- Selene owns linting, StyLua owns formatting, and luau-lsp owns type analysis.
- Rokit, Wally, Git, Lute, and test runners remain external tools with project configuration and pins.
- A library dependency must serve an approved feature. Availability alone is not a reason to embed a tool.

## Current State

The published baseline is **v0.14.0, public alpha**: persistent Home, template saves that stay open, and a grouped Catalog with offline examples. Its crates.io archive, annotated tag, and [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.14.0) agree on `f86a7bd`. Public availability does not mean every integration or configuration contract is ready for a stable release.

The automated live Jest regression is merged on main in [PR #7](https://github.com/chatarabdelilah/rproj/pull/7), after the 0.12.2 publication. It verifies three passing generated starter specs and a deliberate assertion failure through `rproj test`. Review and post-merge CI passed; this test-only change requires no package release.

The existing product includes machine setup, configurable project generation, saved compositions, upgrades, tool configuration, the global Template Explorer, workspace hub and Catalog, watching, source copying, and optional TestEZ/Jest Roblox testing.

The unfinished model-import experiment is **permanently dropped** by the owner's September 9, 2026 decision. Its unmerged local branch and importer-specific build artifacts were removed; no importer implementation or DOM/serializer dependencies entered main. It is not a future milestone.

## Scope Removed

| Former proposal | Decision |
| --- | --- |
| M7: broad library migration | Cancelled. There is no goal to move the toolchain into rproj. |
| Embedded Selene or StyLua | Removed. Working external tools do not need internal replacements. |
| Full Moon | Removed. No approved standalone Lua/Luau parsing requirement exists. |
| M6 / M7a + M6a: static model-to-template conversion | Permanently dropped. The owner requested deletion of the experiment and its branch; it will not be reintroduced. |
| Tauri GUI and GUI-driven core-library extraction | Dropped, not deferred. Ratatui is the selected interface direction. |
| R4: additional project types | Uncommitted backlog, requiring a concrete use case and working build targets. |
| M8: rproj Studio plugin | Uncommitted backlog. No new plugin is needed to release the existing product. |

Rojo already accepts `.rbxm` and `.rbxmx` models and filesystem mounts. rproj will not add a competing model conversion/import layer. This decision does not expand rproj's current global-template `$path` rules. See Rojo's [sync details](https://rojo.space/docs/v7/sync-details/) and [project format](https://rojo.space/docs/v7/project-format/).

## Release Hardening Baseline

No new feature milestone is required first. Audit the baseline, fix concrete defects, and record evidence for workflows users already have.

Automated **saved-setup replay** now verifies both Wally and Git submodules: saved choices, generated files and tool pins, no repeated choice prompts, an unchanged source setup, and temporary-file cleanup. Refusal checks now cover missing/malformed setups and Jest without Wally, including explicit `--reconfigure`, with no project creation or fixture/config mutation. Local Jest pass/fail execution is covered; Open Cloud and fresh-machine provisioning remain separate gaps.

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

## Shipped: T2 Home Navigation and Catalog Clarity

Published **0.14.0 alpha** makes Home persistent, keeps Template Explorer open after successful saves, and groups the Catalog with offline package examples and scrollable details. Internal views borrow one terminal session; existing prompts and foreground processes run outside the alternate screen and return after acknowledgement. Direct commands and project formats remain unchanged.

[PR #20](https://github.com/chatarabdelilah/rproj/pull/20) is merged at `f86a7bd`. CodeRabbit findings were addressed, reviewed-head and merged-main Windows stable/Rust 1.89/package CI passed, and the completed feature branch was deleted. Owner publication and archive/tag/alpha-release alignment are verified. [Release notes](release-notes-0.14.0.md) and [audit evidence](release-audit.md) record the scope and remaining limits.

The bounded [project-creation implementation plan](ratatui-project-creation.md)
defines the hub-driven first slice, shared execution boundary, and acceptance
checks. The [local diagnostic logger](diagnostic-logs.md) is merged in PR #11,
and the shared confirmed-execution boundary is merged in PR #12. Both are
included in published 0.13.0. Creation screens are implemented in
[PR #14](https://github.com/chatarabdelilah/rproj/pull/14), which records review and CI evidence.

The implemented feature is **project creation inside Ratatui**: choose dependencies, packages, and capabilities, revise the summary, and confirm creation without switching between unrelated prompt styles.

Release PR #15 is merged at `50f2e34`; its reviewed-head and main CI passed, the owner published 0.13.0, and release alignment is verified. Open Cloud and fresh-machine checks remain documented limitations rather than inferred passes.

The completed 0.13.1 change fixes confirmed `configure` preservation failures: unlisted or unsupported existing values remain unless explicitly replaced, unchanged settings avoid writes, and TOML merges that fail parsing or alter unrelated values are refused. No new UI dependency or external-tool migration is involved. Focused prompt/merge regressions and ordinary CI cover this scope; Studio provisioning is unrelated and remains historical evidence.

After T2, prioritize one separate workflow based on use: project browsing under the configured projects root, Ratatui Machine Setup, foreground/background Watch lifecycle, catalog package additions, or a bounded structural audit. None is part of 0.14.0; no new dependency or broad rewrite is approved by this list.

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
| Project confirmation safety and Template Explorer compound values | v0.12.1 |
| Jest Roblox provisioning, executable name, and generated config fixes | v0.12.2 |
| Local diagnostic logging and hub-driven Ratatui project creation | v0.13.0 |
| Interactive configuration preservation | v0.13.1 |
| T2: persistent Home, template save continuity, grouped Catalog and examples | v0.14.0 |

**Active sequence: observe real workflows -> select one bounded improvement -> review, release, and align it.**
