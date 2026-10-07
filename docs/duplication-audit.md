# Duplication audit

Baseline: `a13e4dbd1add18b9f9bc04093c3764328444aae4` (PR #33).
Scope: Inquire/Ratatui creation and configuration, Machine Setup, catalogs,
and their execution boundaries. CodeGraph supplied the symbol/caller paths;
focused reads filled omitted source ranges. Headroom was used for exploration
and test-output storage/compression; its configured proxy was unreachable.

## What the line count includes

The baseline has **30,903 physical lines across tracked Rust files**. This
counts comments, blank lines, embedded templates, inline tests, and data.
It excludes Cargo dependencies, build output, Markdown, and `.codegraph`.
Reproduce the total by summing `splitlines()` on `git show <baseline>:<path>`
for paths returned by `git ls-files '*.rs'` at that commit.

| Area | Physical Rust lines |
| --- | ---: |
| `src/commands` | 10,252 |
| `src/catalog` | 5,671 |
| `src/steps` | 5,008 |
| `src/project_editor` | 3,299 |
| `tests` | 2,949 |
| Top-level `src` | 2,641 |
| `src/tui` | 565 |
| `src/config` | 518 |

The `tests` row is only the integration-test tree. Unit tests and dedicated
test modules under `src` are included in their owning areas. These totals are
not executable-line counts or measured duplicate-line counts. No comparable
Wally checkout/count was used, so this audit does not substantiate a 5,000-line
comparison or a promise to reduce rproj to that size.

## Findings and consolidation order

### 1. Creation duplicates selection policy, not the scaffold executor

[`new::pick_capabilities`](../src/commands/new.rs) and
[`creation::Draft::open/accept`](../src/commands/creation/model.rs) independently
filter capability implementations, hide the Open Cloud implementation from the
runner picker, sort choices, and route Jest to its backend choice. They also
handle missing requirements differently: direct prompts skip an unsupported
selection with a message; the Ratatui draft asks for correction.

**September 30 consolidation:** `Capability::implementation_choices` now owns
workflow filtering, runner sorting, and the separate Open Cloud choice;
`missing_requirements` and `needs_jest_backend` supply shared validation and
backend routing. Both callers use these rules. Compatibility defaults remain
independent of picker ordering. Direct prompts still skip unmet requirements;
Ratatui still asks for correction before changing the graph.

Catalog and Ratatui parity tests were added before replacing the callers.
They now cover Wally/None, each capability and implementation, both Jest
backends, and saved unknown values. A terminal regression exercises the direct
prompts for both Jest backends, TestEZ-only workflows, and missing gate refusal.
The configuration persistence boundary is also consolidated below.

Both interfaces already reuse `ProjectGraph`, `offerable_package`,
`add_companions`, `apply_derived_packages`, `prepare_project`, and
`execute_confirmed`. Project writes and package installation are not two
independent implementations. Moving these helpers to new files alone would
not reduce duplication.

### 2. Configure now shares its edit session and persistence

**September 30 consolidation:** [`EditSession`](../src/commands/configure/session.rs)
owns the tool, one loaded snapshot, current values, pending changes, checked
merge, and save. Direct prompts and Ratatui delegate to it. The JSON parser and
value merger are shared with VS Code scaffolding/upgrade, but configuration
merges use the loaded snapshot rather than rereading the file during merging.

Both interfaces reject external content changes, including file creation and
deletion. Saving stages and syncs a temporary file beside the destination,
rechecks the baseline, then replaces it. Failed saves retain the baseline and
pending changes. No-op saves preserve exact bytes; cancellation writes nothing.
Conflict detection is optimistic, not a lock on arbitrary external writers.
The stronger Ratatui behavior now also protects direct CLI configuration.

### 3. Machine Setup is already one UI and worker

[`provision::run`](../src/commands/provision.rs) delegates to Ratatui Machine
Setup; it is not a second Inquire installer. `setup <tool>` is a distinct
project-local operation. The remaining overlap in
[`toolchain`](../src/steps/toolchain.rs) is Rokit error classification between
streamed Machine Setup reporting and project batch tallies.

This change fixes Machine Setup's confirmed global add with `--force`.
Project adds still preserve existing project pins. Share result classification
only if that code changes again; do not combine the operations in a way that
forces project pins or loses streamed status reporting.

### 4. Default-selection machinery was unnecessary and is removed

Fresh setup and project pickers now start empty. Saved sets and the project's
graph are the only sources of checked choices. Removed the two catalog
`default_selected` fields, their per-entry values, first-run default merging,
and the creation draft's separate `capabilities_answered` flag. Direct CLI
creation follows the same empty-default policy; catalog descriptions agree.
Required packages derived from explicit choices and reviewed generated files
remain intact. This is a concrete simplification, not a broad UI rewrite.

### 5. Catalog data and tests account for size without being duplicate runtime

Catalog entries, usage examples, templates, and settings declarations explain
much of `src/catalog`. Existing `catalog_view` already supplies shared detail
content to plain output and Ratatui. Moving that data to JSON/TOML would
mostly relocate lines and introduce parsing/schema work.

The 1,903-line project-template editor is a separate tree/JSON editing feature,
not a second new-project wizard. Its navigation, undo, property validation,
and tests cannot be removed by consolidating Inquire choices. A smaller
secondary cleanup is a common test-fixture compiler: projects and hub tests
currently build the same fake tool independently. Preserve behavioral tests;
do not target test deletion as a size goal.

## Scribe Studio addition

Added `scribe-studio` to the existing manual Studio-plugin catalog. Selection
is optional and initially unchecked; Machine Setup reports the install link
as a manual step. This uses the existing installer dispatch and detail model.
No Scribe library, game configuration, new installer, or plugin detection
mechanism is added.

Sources: [Creator Store](https://create.roblox.com/store/asset/113609038046646/Scribe-Studio),
[official plugin guide](https://scribe.ericplane.dev/studio-plugin/), and
[upstream repository](https://github.com/ericplane/Scribe).
Rokit behavior was checked against installed 1.2.0 help and
[`add` source at v1.2.0](https://github.com/rojo-rbx/rokit/blob/v1.2.0/src/cli/add.rs).
`add --force` bypasses its trust prompt and reinstalls/replaces the global pin;
`install --force` is not a trust bypass.

## Verification scope

Ordinary tests cover fresh guided/expert choices across dependency strategies,
explicit selection retention, saved machine choices, and Scribe's manual
dispatch. Ignored live creation helpers were updated to explicitly choose
their required capabilities. No machine-wide tool or Studio plugin was
installed during this audit. Live provisioning and Scribe execution are not
claimed as verified.
