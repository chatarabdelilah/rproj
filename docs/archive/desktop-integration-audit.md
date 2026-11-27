# Desktop parity and integration audit

**November 23 decision.** The owner closed F1 and lifted the pause on F4.
Proceed with ordinary Windows real-window acceptance. Reopen F1 only on
recurrence. The November 10 findings below retain their original wording.
[Incident record](desktop-memory-investigation.md).

Approved November 10, 2026, against clean main `c5c4f87`. Audit first: reconcile
all workflow coverage, then focus on navigation/ownership, operations/events,
Watch, and close/focus interactions. Runtime fixes and new features require a
separate plan. Published version remains 0.22.0. Native previews stay paused.

November 18 follow-up: the separately approved [Project Tool Setup implementation](desktop-project-tool-setup-native.md)
addresses F2's native functionality gap. Shared execution/worker PRs #156–157
have verified delivery; native controls have headless evidence and their own
delivery ledger. The matrix below retains the November 10 audit's observations.

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
| Bare rproj / Home | A, G, D | [CLI Home](../../src/commands/hub.rs), [hub tests](../../tests/hub.rs), [desktop shell](../../crates/rproj-desktop/src/workbench.rs); all six destinations reachable; aggregate readiness/update summary absent | Destination deliveries below; default CLI launch is intentional |
| rproj app | A | [Explicit launcher and inline tests](../../src/commands/app.rs); portable smoke checks | [Soft/Watch #111](https://github.com/chatarabdelilah/rproj/pull/111); version/self-check do not open a window |
| inspect | A, H | [Core inspection](../../crates/rproj-core/src/inspection.rs), [inspection tests](../../tests/inspection.rs), [shell tests](../../crates/rproj-desktop/src/workbench/ui_tests.rs) | #111; historical Overview/Dependencies navigation only |
| Projects browser | A | [Browser](../../crates/rproj-desktop/src/workbench/projects.rs), [preview/Open tests](../../crates/rproj-desktop/src/workbench/projects/tests.rs), [discovery](../../crates/rproj-core/src/project_discovery.rs) | [#146](https://github.com/chatarabdelilah/rproj/pull/146), [evidence](desktop-projects-browser-native.md) |
| New Project | A, H | [Shared creation](../../crates/rproj-core/src/creation/mod.rs), [native state](../../crates/rproj-desktop/src/workbench/creation.rs), [controls/close tests](../../crates/rproj-desktop/src/workbench/ui_tests.rs) | [#115](https://github.com/chatarabdelilah/rproj/pull/115); October 16–17 [acceptance](desktop-creation-acceptance.md) uses disposable storage/tool stubs |
| Machine Setup | A | [Checked executor](../../src/commands/machine_setup/checked.rs), [native tests](../../crates/rproj-desktop/src/workbench/setup/ui_tests.rs) | [#134](https://github.com/chatarabdelilah/rproj/pull/134), [evidence](desktop-machine-setup-native.md); real installers unverified |
| setup tool | G | [setup_tool](../../src/commands/setup.rs), [terminal tests](../../tests/setup.rs); no corresponding native entry/worker operation | CLI only; F2 below |
| Configure Tools | A | [Checked settings](../../src/commands/configure/checked.rs), [native tests](../../crates/rproj-desktop/src/workbench/settings/ui_tests.rs) | [#130](https://github.com/chatarabdelilah/rproj/pull/130), [acceptance](desktop-tool-configuration-acceptance.md) |
| Composition | A | [Shared draft](../../crates/rproj-core/src/creation/composition.rs), [executor](../../src/commands/project_composition.rs), [native tests](../../crates/rproj-desktop/src/workbench/composition/ui_tests.rs) | [#122](https://github.com/chatarabdelilah/rproj/pull/122), [acceptance](desktop-composition-acceptance.md) |
| Project Template | A | [Terminal editor](../../src/commands/project_template.rs), [native controller](../../crates/rproj-desktop/src/workbench/project_template/controller.rs), [tests](../../crates/rproj-desktop/src/workbench/project_template/ui_tests.rs) | [#143](https://github.com/chatarabdelilah/rproj/pull/143), [evidence](desktop-project-template-native.md) |
| Upgrade | A | [Terminal regressions](../../src/commands/upgrade/regression_tests.rs), [native model](../../crates/rproj-desktop/src/workbench/upgrade.rs), [tests](../../crates/rproj-desktop/src/workbench/upgrade/ui_tests.rs) | [#126](https://github.com/chatarabdelilah/rproj/pull/126), [acceptance](desktop-upgrade-acceptance.md) |
| Foreground Watch | D | [CLI Watch](../../src/commands/watch.rs), [worker fixtures](../../tests/workbench.rs); desktop deliberately uses detached Watch | [#111](https://github.com/chatarabdelilah/rproj/pull/111); no duplicate foreground native mode required |
| Persistent Watch | A, H | [Operation bridge](../../crates/rproj-core/src/operations.rs), [desktop tests](../../crates/rproj-desktop/src/workbench/ui_tests.rs), [worker fixtures](../../tests/workbench.rs) | #111; owner confirmed Start → Switch → Stop, not complete close/Escape matrix |
| Checks | A, H | [Worker dispatch](../../src/workbench.rs), [Activity/control tests](../../crates/rproj-desktop/src/workbench/ui_tests.rs) | #111; historical responsive probe/filter observation; no public check command implied |
| Test | A, G | [CLI arguments](../../src/cli.rs), [terminal action](../../src/commands/projects.rs), [worker](../../src/workbench.rs), [fixtures](../../tests/test_workflow.rs) | Existing default Test; no native custom arguments, matching Ratatui; real runner prerequisites unverified |
| Copy Source | A | [Shared collector](../../crates/rproj-core/src/source_copy.rs), [native tests](../../crates/rproj-desktop/src/workbench/source_copy/tests.rs) | [#152](https://github.com/chatarabdelilah/rproj/pull/152), [evidence](desktop-copy-source-native.md); OS clipboard unverified |
| Catalog | A | [Bundled guides](../../crates/rproj-core/src/catalog/browser.rs), [native tests](../../crates/rproj-desktop/src/workbench/library/tests.rs), [terminal compatibility](../../tests/catalog_compatibility.rs) | [#149](https://github.com/chatarabdelilah/rproj/pull/149), [evidence](desktop-catalog-native.md); OS clipboard/browser unverified |
| Saved Setups | A | [Terminal manager](../../src/commands/saved_setups.rs), [native controller](../../crates/rproj-desktop/src/workbench/saved_setups/controller.rs), [tests](../../crates/rproj-desktop/src/workbench/saved_setups/ui_tests.rs) | [#138](https://github.com/chatarabdelilah/rproj/pull/138), [evidence](desktop-saved-setups-native.md) |

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

New [audit tests](../../crates/rproj-desktop/src/workbench/audit_tests.rs) reuse
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

November 18 follow-up to F3: the owner selected an exact-argument native design
with captured session ownership and protected unsubmitted edits. The separate
[proposal](desktop-test-arguments-design.md) and [acceptance](desktop-test-arguments-acceptance.md)
do not yet change native Test execution. An aggregate Home summary remains a
separate optional decision. The historical findings below retain their audit scope.

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

### First-run readiness follow-up

On November 23, the owner verified the real-window workflow in the Windows
first-install VM using the [desktop build from `b742a89`](https://github.com/chatarabdelilah/rproj/actions/runs/45129687301).
With the draft named `FirstRunAcceptance`, New Project reported missing setup
and offered Machine Setup above Recheck. The review listed only the missing
projects folder. Apply reported it installed and the selections saved.
From Overview, Resume project creation reopened the draft. Next reached
Dependencies, and Back showed the original name. This run ended in the wizard
before project generation.

The subsequent Guided Wally creation attempts on that build exposed three
prerequisites after the worker claimed a destination. The first attempt
could not launch Git, the second Wally, and the third Rojo. Wally 0.3.2
ran by full path after Machine Setup installed it. The third attempt used
a desktop session launched with Rokit's bin directory on PATH.
None of the three projects completed. The Windows worker regression covers
missing Rojo during review and its disappearance between review and creation.

The owner repeated the Guided Wally workflow with zero packages or tool pins
using the [Windows build from `b1efa54`](https://github.com/chatarabdelilah/rproj/actions/runs/45165874527).
Review listed both missing commands, `rojo` and `wally-package-types`, and
disabled Create project. PowerShell's `Test-Path` returned `False` for the
`FirstRunPreflight` destination. After setup, creation succeeded at that same
destination. Activity reported writing `default.project.json`, generating
`sourcemap.json`, and updating `.gitignore`. The owner clicked Open project
and confirmed that Overview showed `FirstRunPreflight` as the active project.

On the same Windows build, the owner clicked **Start Watch** and observed the
status change from **Stopped** to **Running**. Clicking **Stop Watch** returned
the status to **Stopped**. Overview still showed `FirstRunPreflight` as active.

On November 24, the owner started Watch again on build `b1efa54` and added
a uniquely named Luau module under `FirstRunPreflight/src`. A PowerShell
search found the new module name in `sourcemap.json` while Watch was running.
The owner then clicked **Stop** and confirmed the status returned to **Stopped**.

The owner restarted Watch and renamed the same test module. A PowerShell check
confirmed that `sourcemap.json` contained the new name and no longer contained
the original name. After the owner stopped Watch, the status returned to **Stopped**.

With Watch running again, the owner moved the renamed module from
`FirstRunPreflight/src` to the VM's temporary folder. PowerShell parsed the
updated `sourcemap.json` and confirmed that the module name was absent.
The owner then stopped Watch.

The owner started Watch for `FirstRunPreflight` and closed Desktop.
With Desktop closed, `rproj watch status` reported that Sourcemap Watch was
running for that project. On reopening, Desktop showed **Watching — FirstRunPreflight**
without another start. Clicking **Stop** returned the status to **Stopped**.

The owner started Watch again for `FirstRunPreflight`, left Desktop open,
and ran `rproj watch stop` in PowerShell. The CLI identified the project
and reported that Watch would stop after the active recovery command. Desktop
changed to **Stopped** without a click or manual refresh.

With Desktop still open, the owner ran `rproj watch start` from
`FirstRunPreflight`. The CLI reported **Preparing** for that project, and
Desktop changed to **Watching** without a manual refresh. After the owner
clicked **Stop** in Desktop, `rproj watch status` reported **Stopped** for
`FirstRunPreflight` and confirmed that Watch's owned processes had exited.

On November 23, inspection of the [New Project view](../../crates/rproj-desktop/src/workbench/creation/view.rs)
at `24ddeb9` found that an unconfigured machine gets a Machine Setup button on
the name-and-destination step. The same message offers `rproj setup` and Recheck
for terminal users. Opening Machine Setup hides the creation view. A successful
setup result requests fresh creation context, and reopening New Project reuses
the existing draft. See the [setup controller](../../crates/rproj-desktop/src/workbench/setup/controller.rs)
and [creation controller](../../crates/rproj-desktop/src/workbench/creation/controller.rs).

The desktop session test covers blocking review with missing setup. The CLI
worker fixture clears recorded readiness and checks that preparation gives
recovery guidance and leaves the destination absent. A separate headless
[interaction test](../../crates/rproj-desktop/src/workbench/setup/ui_tests.rs)
types a project name, clicks Machine Setup, supplies a successful setup report
and refreshed creation context, then clicks Overview → Resume project creation.
It checks the original destination and advances to Dependencies. Installer
execution, worker transport and real-window usability are outside this fixture.

### November 24 — Apply additions and Run checks on Windows

On `rproj-Windows-FirstInstall`, the owner retried adding Selene and Lute to
`FirstRunPreflight`, a Wally project with zero dependencies. The earlier Apply
had written the check script and Selene configuration, then failed when Rojo
could not resolve the missing `Packages` directory.

With portable build `db3bd98`, **Apply additions** succeeded on the same project.
Activity showed that Wally downloaded zero packages, Rojo generated
`sourcemap.json`, and `wally-package-types` processed `Packages`. Desktop then
reported that it had saved `rproj.toml`. [Fix and regression test in PR #203](https://github.com/chatarabdelilah/rproj/pull/203).

After reopening the project, the owner selected **Checks → Run checks**.
Activity reported **Succeeded** with 0 errors, 0 warnings, and 0 parse errors.
**Run tests** was disabled because the project had no test runner configured.

### November 24 — Copy Source on the Windows clipboard

In the same VM, the owner clicked **Copy source** for `FirstRunPreflight`.
Desktop reported **Copy requested**, with 3 included files, 0 skipped paths
and 505 UTF-8 bytes. The owner then pasted the source into chat, confirming
clipboard delivery. The paste contained the path header and Luau source for
`src/client/hello.client.luau`, `src/server/hello.server.luau` and
`src/shared/hello.luau` in preview order. The preview's `[LF]` annotations
were absent from the paste. The byte count came from Desktop.

### November 24 — Catalog documentation in the Windows browser

In `rproj-Windows-FirstInstall`, the owner opened the Rojo entry in **Catalog**
and clicked **Documentation**. The owner confirmed that `rojo.space` opened
directly in the VM's browser.

### November 25 — CharmSync Catalog example on the Windows clipboard

In `rproj-Windows-FirstInstall`, the owner clicked **Copy example** on the
**CharmSync** Catalog entry and pasted the multiline Luau example into chat.
The paste included both `CharmSync.client.addSignals({ coins = coins })` and
the teardown call `CharmSync.client.removeSignals("coins")`, confirming
clipboard delivery for this entry.

### November 25 — Catalog keyboard navigation on Windows

In `rproj-Windows-FirstInstall`, the owner pressed **Tab** once with **Copy example**
focused on the **CharmSync** entry. The focus indicator moved to **Charm · Package**
under **Related guidance** while the detail heading still read **CharmSync**.
Pressing **Enter** opened the related entry and changed the detail heading to **Charm**.
The owner then clicked **Back** once with the mouse and the **CharmSync** entry reappeared.

### November 26 — Catalog search on Windows

With **Packages** selected in the Windows VM's Catalog, the owner typed
`CharmSync` into **Search all catalog sections**. The results showed only
**CharmSync**. Clearing the search restored the full **Packages** list.

Searching for `zzzz-no-match` showed **0 entries** and the message “No entries
match these filters. The selected guide remains available.” The previously
selected guide stayed visible in the detail pane.

### November 26 — Catalog at the minimum window size on Windows

After clearing the search, the owner resized Desktop to its minimum window
size in the Windows VM. The selected Catalog guide stayed readable and
scrollable in the detail pane.

At **150% Windows display scaling**, the selected Catalog guide stayed
readable at the minimum window size. Scrolling brought lower content in the
guide into view.

On November 27, at **150% Windows display scaling**, the owner pressed **Tab**
with **Copy example** focused on the **CharmSync** entry. The focus indicator
was visible on **Charm · Package**. Pressing **Enter** opened the **Charm** guide.
In a separate check at the same scaling, pressing **Shift+Tab** from
**Charm · Package** moved visible focus back to **Copy example**.

### November 27 — Reviewed Rokit tool update on Windows

In the Windows VM, **Check for updates** reported **Current** for Lute and
Selene in `FirstRunPreflight`. The owner copied the project to
`UpdateReviewAcceptance` and set the copy's Selene pin in `rokit.toml` to
`0.31.0`. Desktop found `0.32.0` as a newer stable release.

**Prepare review** succeeded and showed the exact Selene pin change from
`0.31.0` to `0.32.0`, the shared Rokit cache and shim effects, and the existing
trust requirement. The confirmation named the copy's folder and target version.
Pressing **Enter** activated **Keep reviewing** and returned to the review
without starting Apply.

The owner reopened the dialog and explicitly clicked **Apply update**.
Activity reported success for `UpdateReviewAcceptance` and verification of the
installed Selene executable. Desktop showed the new `0.32.0` pin and marked
Selene **Unchecked**. A fresh **Check for updates** returned it to **Current**.

### November 10 audit checks

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
