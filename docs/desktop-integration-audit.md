# Desktop parity and integration audit

Approved November 10, 2026, against clean main `c5c4f87`. Audit first: reconcile
all workflow coverage, then focus on navigation/ownership, operations/events,
Watch, and close/focus interactions. Runtime fixes and new features require a
separate plan. Published version remains 0.22.0. Native previews stay paused.

## Workflow evidence matrix

The [parity inventory](desktop-workflow-parity.md) describes behavior and deliberate
differences. **A** means implemented with automated evidence rerun by this audit;
**H** means limited historical owner observations, not renewed acceptance;
**G** means a functionality/presentation gap; **D** means a deliberate interface
difference. All rows retain unverified real-window/platform gates. Delivery
links identify merged PRs with recorded reviewed-head/main CI. Their merge state
and delivery ledgers were checked during this audit.

| Inventory row | Class | Current implementation and automated evidence | Delivery / historical scope |
| --- | --- | --- | --- |
| Bare rproj / Home | A, G, D | [CLI Home](../src/commands/hub.rs), [hub tests](../tests/hub.rs), [desktop shell](../crates/rproj-desktop/src/workbench.rs); all six destinations reachable; aggregate readiness/update summary absent | Destination deliveries below; default CLI launch is intentional |
| rproj app | A | [Explicit launcher and inline tests](../src/commands/app.rs); portable smoke checks | [Soft/Watch #111](https://github.com/chatarabdelilah/rproj/pull/111); version/self-check do not open a window |
| inspect | A, H | [Core inspection](../crates/rproj-core/src/inspection.rs), [inspection tests](../tests/inspection.rs), [shell tests](../crates/rproj-desktop/src/workbench/ui_tests.rs) | #111; historical Overview/Dependencies navigation only |
| Projects browser | A | [Browser](../crates/rproj-desktop/src/workbench/projects.rs), [preview/Open tests](../crates/rproj-desktop/src/workbench/projects/tests.rs), [discovery](../crates/rproj-core/src/project_discovery.rs) | [#146](https://github.com/chatarabdelilah/rproj/pull/146), [evidence](desktop-projects-browser-native.md) |
| New Project | A, H | [Shared creation](../crates/rproj-core/src/creation/mod.rs), [native state](../crates/rproj-desktop/src/workbench/creation.rs), [controls/close tests](../crates/rproj-desktop/src/workbench/ui_tests.rs) | [#115](https://github.com/chatarabdelilah/rproj/pull/115); October 16–17 [acceptance](desktop-creation-acceptance.md) uses disposable storage/tool stubs |
| Machine Setup | A | [Checked executor](../src/commands/machine_setup/checked.rs), [native tests](../crates/rproj-desktop/src/workbench/setup/ui_tests.rs) | [#134](https://github.com/chatarabdelilah/rproj/pull/134), [evidence](desktop-machine-setup-native.md); real installers unverified |
| setup tool | G | [setup_tool](../src/commands/setup.rs), [terminal tests](../tests/setup.rs); no corresponding native entry/worker operation | CLI only; F2 below |
| Configure Tools | A | [Checked settings](../src/commands/configure/checked.rs), [native tests](../crates/rproj-desktop/src/workbench/settings/ui_tests.rs) | [#130](https://github.com/chatarabdelilah/rproj/pull/130), [acceptance](desktop-tool-configuration-acceptance.md) |
| Composition | A | [Shared draft](../crates/rproj-core/src/creation/composition.rs), [executor](../src/commands/project_composition.rs), [native tests](../crates/rproj-desktop/src/workbench/composition/ui_tests.rs) | [#122](https://github.com/chatarabdelilah/rproj/pull/122), [acceptance](desktop-composition-acceptance.md) |
| Project Template | A | [Terminal editor](../src/commands/project_template.rs), [native controller](../crates/rproj-desktop/src/workbench/project_template/controller.rs), [tests](../crates/rproj-desktop/src/workbench/project_template/ui_tests.rs) | [#143](https://github.com/chatarabdelilah/rproj/pull/143), [evidence](desktop-project-template-native.md) |
| Upgrade | A | [Terminal regressions](../src/commands/upgrade/regression_tests.rs), [native model](../crates/rproj-desktop/src/workbench/upgrade.rs), [tests](../crates/rproj-desktop/src/workbench/upgrade/ui_tests.rs) | [#126](https://github.com/chatarabdelilah/rproj/pull/126), [acceptance](desktop-upgrade-acceptance.md) |
| Foreground Watch | D | [CLI Watch](../src/commands/watch.rs), [worker fixtures](../tests/workbench.rs); desktop deliberately uses detached Watch | [#111](https://github.com/chatarabdelilah/rproj/pull/111); no duplicate foreground native mode required |
| Persistent Watch | A, H | [Operation bridge](../crates/rproj-core/src/operations.rs), [desktop tests](../crates/rproj-desktop/src/workbench/ui_tests.rs), [worker fixtures](../tests/workbench.rs) | #111; owner confirmed Start → Switch → Stop, not complete close/Escape matrix |
| Checks | A, H | [Worker dispatch](../src/workbench.rs), [Activity/control tests](../crates/rproj-desktop/src/workbench/ui_tests.rs) | #111; historical responsive probe/filter observation; no public check command implied |
| Test | A, G | [CLI arguments](../src/cli.rs), [terminal action](../src/commands/projects.rs), [worker](../src/workbench.rs), [fixtures](../tests/test_workflow.rs) | Existing default Test; no native custom arguments, matching Ratatui; real runner prerequisites unverified |
| Copy Source | A | [Shared collector](../crates/rproj-core/src/source_copy.rs), [native tests](../crates/rproj-desktop/src/workbench/source_copy/tests.rs) | [#152](https://github.com/chatarabdelilah/rproj/pull/152), [evidence](desktop-copy-source-native.md); OS clipboard unverified |
| Catalog | A | [Bundled guides](../crates/rproj-core/src/catalog/browser.rs), [native tests](../crates/rproj-desktop/src/workbench/library/tests.rs), [terminal compatibility](../tests/catalog_compatibility.rs) | [#149](https://github.com/chatarabdelilah/rproj/pull/149), [evidence](desktop-catalog-native.md); OS clipboard/browser unverified |
| Saved Setups | A | [Terminal manager](../src/commands/saved_setups.rs), [native controller](../crates/rproj-desktop/src/workbench/saved_setups/controller.rs), [tests](../crates/rproj-desktop/src/workbench/saved_setups/ui_tests.rs) | [#138](https://github.com/chatarabdelilah/rproj/pull/138), [evidence](desktop-saved-setups-native.md) |

Opening unrecorded folders for native settings, or collecting source without a
worker/record, is intentional. Ratatui's Projects menu first checks project
identity. Different entry prerequisites do not mean desktop must adopt that gate.

## Integration outcomes

All named checks below passed in the fresh full desktop run. They use simulated
events and isolated files, not an exhaustive cross-product of every state.

| Boundary | Reused evidence | New evidence and outcome |
| --- | --- | --- |
| Navigation/ownership | Catalog `catalog_preserves_project_watch_activity_and_independent_drafts_during_operations`; Projects `preview_and_fresh_open_keep_watch_drafts_activity_and_the_original_operation` and `removed_preview_and_stale_open_never_replace_the_workspace`; per-workflow retention tests | `audit_all_retained_sessions_survive_navigation_during_checks`: six dirty drafts plus Upgrade/Copy Source survive nine navigation routes, project switching/closure and a synthetic Check; exactly one workflow visible. Pass at default/minimum sizes in both Light and Dark. |
| Operations/events | Operation identity model, per-worker malformed/mismatched report fixtures, per-workflow busy/teardown guards; Copy Source independent background lane/latest-request tests | `audit_unavailable_worker_actions_and_stale_events_preserve_original_operation`: unavailable-worker actions preserve Check; wrong-ID, duplicate and post-completion events cannot overwrite owner/result/output. Pass. This workerless test alone does not prove live-worker serialization; existing owned-worker fixtures supply that evidence. |
| Watch | `watch_confirmation_restores_focus_and_rejects_changed_selection`; composition/settings/Upgrade stale-confirmation guards; setup any-Watch guard; worker Stop-before-switch and failed-Stop fixtures | All six drafts retain Watch's actual owner in new navigation/close checks. Catalog, Copy Source, Saved Setups and Template remain independent. Pass; no new real Watch run. |
| Close/focus | Creation/composition cancel/teardown tests; settings/template/saved completion-in-dialog tests; `setup_stop_and_close_wait_without_killing_and_honor_other_drafts`; uncertainty guards | `audit_six_dirty_drafts_require_individual_close_decisions`: one close decision cannot discard six drafts; Escape preserves remaining drafts; final default Enter keeps creation and Watch remains running. Pass. |
| Activity/layout | `activity_tracks_origin_collapse_filter_cancel_and_late_events`, `minimum_activity_height_contains_long_failure_and_watch_text`, `activity_resize_height_survives_collapse_reopen`; bounded-log and per-workflow focus/diff/layout tests | New navigation test preserves manual collapse and 290-pixel session height; event test retains captured searchable output. Pass. Existing default/minimum/theme tests supply detailed geometry evidence. |

New [audit tests](../crates/rproj-desktop/src/workbench/audit_tests.rs) reuse
constructors through `cfg(test)` helpers. Navigation calls the real controller
routes and renders headless frames; it does not claim every sidebar item was
mouse-clicked or its scroll geometry accepted. No production API/state changed.

Close evidence also includes `creation_close_cancellation_waits_and_teardown_error_keeps_window`,
`cancelled_composition_closes_only_after_result_and_other_draft_confirmation`,
`settings_completion_during_close_dialog_preserves_other_drafts`,
`saved_close_completion_race_defaults_keep_then_protects_creation_draft`, and
`uncertain_completion_while_close_dialog_is_open_preserves_all_drafts`.
Cancellable operations wait for teardown; saves wait for completion; Machine
Setup waits after the active item. Uncertain file results and unconfirmed process
teardown remain distinct, not interchangeable reasons to close or retry.

## Ranked follow-ups

No additional runtime defect was reproduced. These five items distinguish
observed symptoms, missing functionality and unverified gates. No failing product
reproduction was hidden or merged as an ignored test.

| Rank / finding | Expected versus observed; reproduction/evidence | Scope and next action |
| --- | --- | --- |
| F1 — high, unresolved stability symptom | Native previews should coexist with other apps. Owner reported system memory exhaustion/display failures associated with native windows; a leak/cause is not established. [Investigation](desktop-memory-investigation.md); not reproduced in this audit. | Keep previews paused. Future isolation needs a separately approved bounded plan; no host troubleshooting or VM authorization here. |
| F2 — next development, native functionality gap | Inspect `setup_tool` in src/commands/setup.rs and native Tools/worker dispatch: CLI can pin/setup a project tool and create selected configuration/Blender assets; native has no equivalent. Global Machine Setup and four-target Configure Tools do not replace it. Do not execute setup to reproduce an inventory gap. | Design captured-project tool setup review, prerequisites, partial failure and same-project Watch protection before implementation. |
| F3 — medium, optional parity differences | CLI `test [args...]` accepts arguments; Ratatui and native Test use `[]`. Home has readiness/update hints; desktop exposes all six destinations without a combined Home. Inspect src/cli.rs, src/commands/projects.rs, src/workbench.rs and hub. | Decide whether arguments or a machine summary merit native UI. Do not duplicate foreground Watch to make inventories identical. |
| F4 — acceptance gap | Headless controls/exact clipboard-browser requests pass; GPU/window, OS clipboard/browser, accessibility, scaling and macOS/Linux desktop integration remain unverified. October observations predate later controls. | Maintain real-window/platform gates. Resume bounded acceptance only after the stability pause is resolved. |
| F5 — prerequisite/distribution gap | Fixtures do not prove real Blender/Rojo/Studio/Open Cloud/installers/UAC. Twenty-four prerequisite cases remain ignored; portable notices retain three legacy completeness warnings. | Run documented prerequisite checks only when available/authorized; resolve notice completeness before public-distribution claims. Do not install apps or substitute user data to pass an audit. |

## Validation ledger

- `cargo test --locked -p rproj -p rproj-core --all-features`: **813 passed**, **24 ignored**, no failures (638 CLI, 175 core; subprocess output not counted twice). Ignored cases require real tools/runners, provisioned machines, symlink privilege or badge-network access.
- `cargo +1.99.0 test --locked -p rproj-desktop`: **165 passed**, none ignored. Three new checks supplement 162 existing tests. No native preview launches.
- Formatting and strict CLI/core all-target/all-feature Clippy and desktop all-target Clippy passed. One test-fixture `field_reassign_with_default` diagnostic was corrected without changing assertions/runtime code. Local documentation links and whitespace checks passed.
- `pwsh -NoProfile -File scripts/package-desktop.ps1 -Toolchain 1.99.0`: portable build and non-GUI version/self-check/inspection verification passed. Notices for 590 packages were preserved; three legacy completeness warnings remain separate from build success.
- Local CodeRabbit 0.7.6 reviewed all 18 files with zero findings; final diff inspected directly. The evidence-only review-result update was inspected separately. Exact reviewed-head/main CI and branch cleanup belong to the delivery PR ledger. No version bump, publication, native window, real clipboard/browser request or installation.

Intermediate harness attempts used wrong/private API names, then assumed an
offscreen control could be clicked and its scroll container was an observed
test element. Those assumptions were rejected; all navigation now invokes the
same controller routes with the original retention/visibility assertions. A
later event-test compile corrected assumptions that Run implemented Debug and
output yielded borrowed strings. Final full tests passed. These were harness
errors, not product reproductions. Incremental-cache hard-link warnings fell
back to copying. All attempts were headless.

The delivery PR's final body records exact head/main SHAs, CI runs/attempts and
cleanup, avoiding a commit solely to name its own hash.
