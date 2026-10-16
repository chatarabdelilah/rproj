# Desktop workflow parity

Baseline: October 16, 2026, main `9910df1`, published version 0.22.0.
This inventory distinguishes a working desktop slice from replacement of the
whole terminal product. Ratatui remains available; direct commands remain permanent.

## Inventory

“Present” describes an implemented surface, not complete workflow acceptance.
The evidence below keeps automated coverage separate from real-window observations.

| Workflow / entry | CLI and Ratatui behavior | Desktop coverage / gap | Reusable boundary and next action |
| --- | --- | --- | --- |
| Bare `rproj` / Home | Interactive Home; redirected invocation prints plain help. Projects, New Project, Saved Setups, Edit Project Template, Machine Setup, Catalog. | Partial: project workbench, not a machine-level Home replacement. | CLI dispatch and hub action inventory. Preserve terminal behavior. |
| `rproj app [path]` | Explicitly launches matching desktop/CLI distribution. | Present: existing-project workbench and recent-project menu. | Core desktop preferences and explicit worker discovery. Keep portable launch coverage. |
| `rproj inspect [path] [--json]` | Read-only, versioned project snapshot. | Present: Overview, Dependencies, Tools; actual declarations/resolutions/presence remain distinct. | Core inspection and catalog. Keep unknown/malformed/offline cases visible. |
| Projects browser | Discovers configured-root projects and launch directory; ten project actions. | Partial: Open/recent/Close; no equivalent discovery browser. | CLI discovery is not yet a core service. Plan discovery separately from creation. |
| `rproj new <name>` / New Project | Wally/None, guided/expert/saved composition, capabilities, files, named setup save, exclusive destination, execution. Direct command may provision; Home requires recorded machine setup. | Missing. Next design: guided creation, with existing-setup handoff. | Graph/catalog rules, preparation and `execute_confirmed`; no terminal draft widgets in core. |
| `rproj setup` / Machine Setup | Review selections, explicit Apply, sequential installs, pending Stop and recovery; completed installations can remain. | Missing. New Project explains the prerequisite; it does not install machine applications. | Machine setup model/worker plus external installers. Separate native design. |
| `rproj setup <tool>` | Project tool setup, configuration and manual instructions. | Missing; Tools currently distinguishes pins from detection only. | Existing tool catalog/install/config boundaries. A detected executable is not proof of setup. |
| `rproj configure [tool]` / Configure Tools | Guided configuration and guarded saves preserving unrelated content. | Missing; inspection is not an editor. | Configuration sessions and artifact-specific preservation rules. |
| Project Composition | Ratatui edit of recorded packages/capabilities; reviewed save and generated wiring. | Missing. | Creation composition rules plus guarded project-composition apply; not arbitrary TOML rewriting. |
| `rproj configure project` / Edit Project Template | Template Explorer, structured/JSON editing, validation, history, guarded save/reset. | Missing. Creation only previews the current template and generated tree. | Template policy and validation; editor UI remains separate. |
| `rproj upgrade [--yes]` / Upgrade | Captured file-diff plan, explicit Apply-all, conflict checks; plain noninteractive execution with `--yes`. | Missing. | Existing planner, semantic guards, snapshots and staged writer. Not a package/tool updater. |
| `rproj watch` / foreground Watch | Dependency preparation and foreground Rojo sourcemap loop. | No foreground equivalent; desktop uses persistent Watch deliberately. | Keep direct foreground CLI for scripts/terminal users. |
| `watch start/status/logs/stop` / project Watch actions | One Windows background owner, global status/logs, acknowledged Stop. | Present: global owner, output, Start/Stop and confirmed switching. | Core operation bridge + CLI supervisor. Preserve detached ownership and external Rojo. |
| Checks | Existing configured Lute check script; not a new public `rproj check` command. | Present: availability reasons, background execution, filtering and cancellation. | Worker Check operation; preserve project-owned script and external tools. |
| `rproj test [args...]` / Test | Selected runner/backend, dependency restoration, prerequisite handling and CLI argument passthrough. | Partial: existing default Test action; no arbitrary argument editor. | Worker Test operation. Real Roblox/cloud prerequisite acceptance is separate. |
| `rproj copy` / Copy Source | Copy src files with relative-path headers. | Missing. | Existing explicit-path source collection; platform clipboard boundary. |
| `rproj info [key]` / Catalog | Full package/tool/capability catalog and explanations. | Partial: known dependency details; no complete catalog browser. | Core package catalog currently shares only part of CLI catalog content. |
| Saved Setups | Browse/edit/rename/duplicate/delete with no-clobber and stale-source checks; creation can load/save a composition. | Missing manager. Creation wireframe includes replay and new-name save only. | Saved document validation/preservation plus graph; no native manager in this milestone. |

## Evidence and limits

The baseline has 566 passing CLI/core tests, 21 desktop tests and 24 ignored
prerequisite-dependent cases. These are baseline evidence, not tests rerun by
this documentation/prototype pass. Core tests cover snapshots; CLI creation,
hub, saved-setup and upgrade suites cover existing behavior; GPUI tests cover
navigation, focus, operation ownership, layouts and cancellation.

The owner confirmed Soft themes/System changes, 100%/125% scaling, keyboard/menu
focus, responsive checks, Activity clipping and retained resized height.
The replacement Watch preview passed native Start → Watching/Succeeded with Stop
enabled, confirmed Switch → destination Watching/Succeeded, and Stop → Stopped.
Native Close-project/Watch persistence and confirmation Escape were not part of
that final three-step observation; automated coverage is not manual acceptance.

[PR #111](https://github.com/chatarabdelilah/rproj/pull/111) is merged.
[Final-head CI](https://github.com/chatarabdelilah/rproj/actions/runs/38841289738)
and [merged-main CI](https://github.com/chatarabdelilah/rproj/actions/runs/38841859391)
passed all seven executed jobs. The badge job was skipped intentionally.
macOS/Linux core CI is not desktop-platform acceptance. Fresh-machine setup,
real Roblox runners, authenticated Open Cloud and public-distribution legacy
notice completeness remain separate checks.

## Incremental creation extraction, after design approval

1. Move interface-independent graph/catalog choice and plan derivation behind
   core services, retaining existing generated-output fixtures as the oracle.
2. Share explicit-context preparation: configured root, machine/config/template
   state, saved composition validation and destination availability.
3. Keep confirmed execution behind the existing CLI/external-tool boundary;
   define its desktop request/result contract in a separate implementation plan.
4. Build desktop controls over that shared behavior. Ratatui keeps its own
   input/picker/focus state and continues using the same authoritative rules.

No new public API or wire schema is defined here. There is no whole-codebase
rewrite, configuration migration, replacement installer or stable-SDK promise.
See [New Project design](desktop-new-project-design.md) for this review milestone.
