# Release-Hardening Audit

## October 4: Saved Setups browser resize recovery

Work started from clean main `30e2b7b` (PR #58), with no open PRs.
The existing Saved Setups rendering regression now types a filter over 31
matching disposable setups and one excluded setup, selects `sample29`, and
scrolls both its list and composition details. One app and TestBackend resize
through 120 x 30, 280 x 70, 80 x 24, 60 x 16, 40 x 10 and back.

The test checks the selected row and highlight within the list pane, selected
document, query, focus and detail scroll; Help closes to the exact prior buffer
at every size. Restored composition cells match the original buffer, scrolling
still works, and Actions/Back retains the selected setup and browser scroll.
Existing storage-error and editor Help checks remain covered. List offsets may
adjust with viewport size. This is TestBackend coverage, not a new PTY check.

Formatting, diff checks, clippy with warnings denied and all 453 ordinary tests
passed; 22 prerequisite-dependent tests remain ignored. The full suite ran with
normal Windows permissions. CodeRabbit CLI 0.7.6 connected after explicit owner
approval but returned a 12-minute quota cooldown; no local review result is
claimed. PR review and CI evidence will be recorded before closeout.
No runtime, dependency or version change is involved; published 0.19.1 remains
unchanged, and the test uses only temporary setup storage.

## October 4: Projects PTY resize recovery

Work started from clean main `429b4bd` (PR #57), with no open PRs.
An ordinary regression in `tests/hub.rs` runs the real binary in a PTY.
Its uniquely named temporary launch project supplies a long package summary;
filtering by its full name excludes unrelated projects without changing the
machine configuration or provisioning tools. Diagnostic logging is disabled.

The regression focuses and scrolls details at 120 x 30, resizes through
80 x 24, 60 x 16 and 40 x 10, and opens/closes Help at each small size.
It checks fresh resize output, restores the exact scrolled screen at 120 x 30,
then verifies Home/End scrolling, opening the filtered project, backing out
and returning Home with Ctrl+C before a clean exit. It does not certify
non-default browser selection or a scrolled project list; those remain covered
by the separate TestBackend regression below.

The new PTY test passed ten repetitions. Formatting, clippy with warnings
denied and all 453 ordinary tests passed; 22 prerequisite-dependent tests
remain ignored (475 Windows tests discovered). An initial restricted-token
full run failed existing Windows DACL preservation and Bash checks; rerunning
with normal permissions passed. No runtime, dependency or version change is
involved; published 0.19.1 remains unchanged. PR review and CI evidence are
recorded on the PR before closeout.

## October 4: Projects resize recovery coverage

Work started from clean main `11dcfb6` (PR #56), with no open PRs and successful
merged-main CI `37180824242`. The existing Projects rendering regression now
uses one app and TestBackend through 120 x 30, 80 x 24, 60 x 16, 40 x 10 and
back. Thirty matching temporary projects plus one excluded project exercise a
typed filter, non-default selection and a scrolled list. Injected discovery
warnings provide enough detail text to verify actual keyboard scrolling.

The regression checks filter and selected-path preservation, selected-row text
and highlight inside the list pane at usable sizes, detail focus/scroll, Help
open/close at every size, identical
restored detail cells and working Home/End scrolling after recovery. List offsets
may adjust to keep the selected row visible. Existing disabled-action reasons
and the 280 x 70 rendering check remain covered. No runtime change, provisioning,
dependency or version bump is involved; published 0.19.1 remains unchanged.

Local formatting, `cargo test --locked` (452 passed, 22 prerequisite-dependent
tests ignored), clippy with warnings denied and diff checks passed. The test
inventory remains 474 Windows tests (400 unit and 74 integration). CodeRabbit's
initial review identified a weak row-visibility assertion: the selected name
also appeared in the detail path. The test now checks the list pane and its
selection highlight. Final review and reviewed-head/main CI will be recorded
on the PR before closeout.
This is TestBackend coverage, not a real-terminal Projects resize test;
ignored live-tool tests are not counted as passes.

## October 4: Catalog resize recovery coverage

Work started from clean main `1af843e` (PR #55), with no open PRs. Two new
ordinary unit regressions resize the same Catalog and TestBackend through
120 x 30, 80 x 24, 60 x 16, 40 x 10 and back. They preserve a typed filter,
non-default selected entry, detail scroll and rendered detail contents;
verify a long list keeps its selected row visible and remains scrolled;
and exercise Help, Back history, keyboard and mouse scrolling after recovery.
List offsets may adjust with viewport size while retaining selection visibility.

A third ordinary regression runs standalone `rproj info` in the shared real
terminal harness, filters to reactRoblox, scrolls to its caveats, shrinks through
the same sizes, opens/closes Help while undersized, and restores the exact
scrolled screen. Ctrl+Home/End still work afterward and Esc exits successfully.
It waits for the complete filter and fresh resize output, uses a temporary
working directory and disables diagnostic logging. No runtime or dependency
change, machine provisioning or Cargo version bump is involved.

Local formatting, `cargo test --locked` (452 passed, 22 prerequisite-dependent
tests ignored), clippy with warnings denied and diff checks passed. There are
474 discovered Windows tests: 400 unit and 74 integration. The new PTY regression
also passed ten consecutive repetitions. CodeRabbit reported zero findings.
Ignored live-tool tests were not rerun and are not counted as passes. Final
reviewed-head/main CI and branch cleanup evidence will be recorded on the PR.
Published 0.19.1 and its tag remain unchanged.

## October 4: 0.19.1 publication verified

The owner published 0.19.1 to crates.io at `2026-10-04T00:38:42.3801Z`.
The version is current and non-yanked. The downloaded archive matches registry
SHA-256 `be193ef19a8277f78d788bb9494834d91ba3146d7d7a6daf1d32e7f854cdb210`.
Its `.cargo_vcs_info.json` identifies clean reviewed release commit
`957a0aa67fa679dca4194e6fd3c739594c409dc6`, exactly the main from
[release PR #53](https://github.com/chatarabdelilah/rproj/pull/53).
Both packaged Cargo files identify 0.19.1 and the manifest retains Rust 1.89.
No publication was performed by the agent.

PR #53's final head `61397d1` passed CI `37165190198`; merged release commit
`957a0aa` passed CI `37165309630`. Both passed Windows stable, Rust 1.89 and
locked packaging. Their trees matched exactly and the completed local/remote
release branch was removed. Badge freshness was skipped and is not a pass.
Final whole-branch CodeRabbit review reported zero findings after its service
cooldown; the corrected iterative README finding is documented below.

After archive verification, annotated tag `v0.19.1` was created at `957a0aa`.
Remote tag object `fd9083834e6a194824ea6c3e22e0a095ea154968` peels to the
same commit. The matching [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.19.1)
was published at `2026-10-04T00:41:43Z`, is not a draft and remains marked as
a prerelease. Its body uses the dedicated 0.19.1 release notes.

Publication closeout initially contained only documentation. PR #54's first
CI run (`37165858860`, head `13be429`) passed stable and packaging but failed
the Rust 1.89 terminal resize regression with invalid JSON, not a save timeout.
The fixture waited for an address substring before resizing; ConPTY can deliver
paste as individual keys, leaving the trailing comma pending when the terminal
shrinks. The test now waits for the entire inserted field including its comma.
This changes test synchronization only; the published runtime, dependencies,
Cargo version and tag remain unchanged. Formatting, all 449 ordinary tests
(22 explicitly ignored) and clippy with warnings denied passed locally.
The corrected resize regression also passed ten consecutive local repetitions;
iterative CodeRabbit review reported zero findings.
Final whole-branch CodeRabbit review and exact-head/main CI are required for the expanded
closeout; their final evidence and completed branch cleanup are recorded on
PR #54. The successful release-candidate live gates were not repeated.
Publication closes release alignment; the acceptance limits below remain
unverified. No subsequent release candidate is active.

## October 4: 0.19.1 patch candidate preparation

Preparation started from clean main `32b4673` (PR #52), with successful
merged-main CI `37162300958` and no open PRs. Registry verification found
0.19.0 current and non-yanked; 0.19.1 was not present. The selected patch
contains PR #51's small-terminal exit fix and PR #52's external-edit save/reset
protection, plus PR #50's terminal recovery regressions. It changes no
generated-project schema and requires no project migration.

`codex/release-0-19-1` aligns Cargo.toml and the root Cargo.lock package entry
to 0.19.1 without changing dependency resolution. Dedicated candidate notes
describe the behavior and optimistic conflict-detection limit.

Candidate local gates passed on Windows with Cargo 1.94.0:

- Formatting, `cargo test --locked` (449 ordinary passes, 22 ignored), clippy
  with warnings denied and diff checks. Metadata confirms 0.19.1 and Rust 1.89.
- `RPROJ_TEST_TIMEOUT=180 cargo test --locked --test live -- --ignored --test-threads=1 --nocapture`:
  all 14 passed in 138.19 seconds, including actual negative-gate diagnostics,
  setup replay/refusal, Watch recovery and local Jest pass/fail.
- `cargo test --locked --bin rproj real_rojo -- --ignored --test-threads=1 --nocapture`:
  all four passed in 9.97 seconds, including the terminal save/refusal/repair
  boundary and ten generated template validation variants.
- With the actual installed Jest Roblox CLI 0.4.1 executable selected,
  `open_cloud_missing_credentials_report_names_and_preserve_exit_code` passed
  in 4.35 seconds. The first invocation incorrectly selected Rokit's dispatch
  shim; copying that shim into the fixture caused OS error 50 and exit 1.
  Selecting the installed executable corrected the audit invocation; no runtime
  fix was needed. Synthetic partial credentials exercise 14 refusals.

Installed tools included Rojo 7.7.0, Wally 0.3.2, Rokit 1.2.0, Git for Windows
2.52.0, Selene 0.31.0 and StyLua 2.5.2. Before/after checks confirmed unchanged
machine configuration, template, global Rokit manifest and existing project/setup
names; temporary fixture cleanup completed. The live scaffold refreshed
JestRobloxRunner rather than preserving its prior bytes. The resulting plugin
matches upstream v0.4.4 asset SHA-256
`7fa3e89d172ed8dc76bbbc57f6f36082818d41c4451a26217edae6c475bcabd8`.
This normal scaffold side effect is not an unchanged-plugin claim.

Headroom processed the approved Cargo version diff (zero tokens saved);
diagnostic logs stayed local. Iterative CodeRabbit found one minor README
overstatement; it now explains the optimistic conflict-detection limit.
Clean candidate `01c76f1` passed `cargo package --locked`: 101 files,
1.3 MiB (327.8 KiB compressed), with a successful extracted-package build.
The inspected archive excludes `docs/`, `.github/` and `.codegraph/`; both
packaged Cargo files identify 0.19.1, the manifest retains Rust 1.89 and its
Git identity matches that clean commit. Subsequent documentation-only evidence
updates do not alter runtime, dependencies, README or the packaged file set.

Final branch review and exact-head/main CI will be recorded on the release PR
before owner publication. Authenticated Open Cloud execution, fresh Windows
provisioning, fresh Linux generated-project execution, UI Labs Studio stories,
Scribe Studio playtesting and unavailable Windows symlink privileges remain
unverified. Unix permission-mode checks were not run on Windows.
At preparation closeout, owner publication and archive/tag/GitHub alignment
were pending; the publication section above records their completion.

## October 4: Template Explorer external-edit protection

Work started from clean main `3dc9e7e` (PR #51), with successful merged-main
CI `37154970455` and no open PRs. A disposable terminal regression reproduced
the defect: another process replaced the template after the editor opened,
but Ctrl+S overwrote that edit and reported success.

Both standalone and Home entry points now load the draft and persistence
baseline from the same `project_template::EditSession` read. Save and reset
refuse changed contents, external creation or deletion. Save checks before
Rojo validation and after permission-preserving sibling staging; the snapshot
advances only after successful persistence. Refusal retains the draft and
names reopening Template Explorer as the recovery action. This is optimistic
conflict detection, not a file lock: external writes between the final check
and replacement/removal remain possible.

Five new storage regressions cover external change/create/delete refusal for
save/reset, validation-time changes without write artifacts, successive saves
and reset/recreation, validation refusal/repair and non-file read refusal.
Two terminal regressions verify external save/reset refusal, bytes or absence
preservation, dirty-draft retention, retry after restoration of the original
snapshot, and raw-mode restoration. The child harness uses the same session
and production editor with disposable paths; no global template, machine
configuration, projects, plugins or authentication are changed.

Local locked gates passed: 449 ordinary Windows tests, with 22 ignored
prerequisite-dependent cases, formatting, clippy with warnings denied and
diff checks. There are 471 discovered tests (398 unit/73 integration), with
40 project-editor tests. An explicit installed-Rojo 7.7.0 run of
`pty_real_rojo_save_and_rejection_preserve_last_valid_template` passed in
5.31 seconds on the final source, exercising valid save, upstream refusal and corrected-draft
save through all ten generated variants. Other ignored live-tool tests were
not rerun and are not counted as passes.

Headroom processed the approved source diff and saved 47 tokens; diagnostic
logs stayed local. Iterative CodeRabbit review reported zero findings. Final
branch review and exact-head/main CI closeout are recorded on the change's PR.
This runtime protection and PR #51's resize fix await the next selected
release candidate; Cargo versions, published 0.19.0 and its tag are unchanged.

## October 3: Template Explorer resize recovery

Verification started from clean main `6ab9ef3` (PR #50). A disposable real
terminal regression reproduced an exit trap: shrinking an edited Explorer to
40 x 10 hid its discard confirmation and blocked its confirmation keys.
The editor now renders Exit confirmation below the 60 x 16 editing minimum
and accepts Enter/Y to discard or Esc/N to retain the draft. Help and Ctrl+C
remain available; editing, paste, save, reset and other confirmation actions
remain blocked at these sizes.

Three new ordinary terminal regressions verify edited-draft discard with
byte-identical persistence, JSON draft preservation through 80 x 24, 60 x 16,
40 x 10 and back, blocked small-screen typing/paste/save/reset, refusal of a
pending reset, and raw-mode restoration after exit. The PTY harness replays
recorded resize boundaries at their actual dimensions. Its child runs the
production editor with a disposable template path; no global template,
configuration, project, plugin or authentication state is changed.

Local locked gates passed: 442 ordinary Windows tests, with 22
prerequisite-dependent tests ignored, formatting and clippy with warnings
denied. There are 464 discovered tests (391 unit/73 integration), including
38 project-editor tests. Ignored live-tool tests were not rerun for this
terminal-only change and are not counted as passes. Headroom processed the
approved source diff and saved 33 tokens; diagnostic logs stayed local.
Iterative CodeRabbit review reported zero findings. Whole-branch review and
exact-head/main CI closeout are recorded on the change's PR.
This runtime fix is unreleased; the published 0.19.0 baseline and its tag
remain unchanged. Prepare a new version when the next candidate is selected.

## October 3: Template Explorer terminal recovery regressions

Verification started from clean main `729beb5` after the 0.19.0 publication
closeout. The new code is confined to the existing `#[cfg(test)]` editor
harness; runtime behavior, dependencies and Cargo versions are unchanged.

Four new ordinary terminal regressions exercise first-edit cancellation with
an existing or absent template, saving then discarding a later draft with
Esc/Ctrl+C, invalid-JSON refusal followed by repair, and a real Windows sharing
violation followed by retry. They preserve original bytes or file absence,
unknown fields at the editor/persistence boundary and unrelated settings,
confirm the draft survives a failed save, verify temporary-file cleanup,
reopen saved content and assert raw mode is disabled after the editor returns.

The child harness runs the production editor loop and existing atomic template
writer with an injected disposable path; it does not change the user's global
template, machine configuration, projects, plugins or authentication. Basic
Home/editor handoff remains covered by the existing hub integration tests;
these new cases exercise the isolated editor and save boundary.

With installed Rojo 7.7.0,
`cargo test --locked --bin rproj project_editor::app::tests::pty_real_rojo_save_and_rejection_preserve_last_valid_template -- --ignored --exact --nocapture`
passed in 5.38 seconds. Through the real terminal loop, valid edits passed all
ten generated sourcemap/build variants; invalid `servePlaceIds` was refused
before persistence; repairing the draft passed validation and saved. The last
valid file remained byte-identical during refusal. This installed-tool test
remains ignored without its prerequisite and is not an ordinary-CI pass.

Two fixture assumptions were corrected during verification: Rojo rejects an
invented unknown top-level field, so the live fixture uses supported fields;
ConPTY may deliver pasted characters separately, so the live repair uses
ordinary editing keys rather than assuming one Undo removes the whole paste.
Unknown-field preservation is tested separately without claiming upstream Rojo
acceptance. No production defect was reproduced.

Local gates passed: 439 ordinary Windows tests with 22 prerequisite-dependent
tests ignored, formatting, clippy with warnings denied and `git diff --check`.
Headroom processed the approved source diff and saved 16 tokens; diagnostic
logs stayed local. Iterative CodeRabbit review reported zero findings. The
whole-branch review of `d1651ab` found only a stale aggregate test count in
architecture; it is corrected to 461 discovered (388 unit/73 integration),
439 ordinary and 22 ignored. The documentation-only correction was inspected
directly without repeating successful runtime/live gates. Exact-head/main CI
closeout is recorded on [PR #50](https://github.com/chatarabdelilah/rproj/pull/50),
including final-head/main Git identities, CI runs and branch cleanup. No new
release candidate or publication is needed.

## October 3: 0.19.0 publication verified

The owner published 0.19.0 to crates.io at `2026-10-03T16:55:48.275821Z`.
The version is not yanked and crates.io reports it as current. The downloaded
archive's SHA-256 matches registry checksum
`f7711708ad0669085feedbe5f2ae7194c6e59a10eed53b36863ae253162047f8`.
Its `.cargo_vcs_info.json` identifies
`68fa47929fc9deea81a3a8447e01db9f8cd71301`, exactly the clean, reviewed release
commit from [PR #48](https://github.com/chatarabdelilah/rproj/pull/48).
Both packaged Cargo files identify 0.19.0. No publication was performed by
the agent.

PR #48's final head `fbd8052` passed CI `37135709267`; merged release commit
`68fa479` passed CI `37135909071`. Both passed Windows stable, Rust 1.89 and
locked packaging. Their trees matched exactly and the completed local/remote
release branch was removed. Badge freshness was skipped and is not a pass.

After archive verification, annotated tag `v0.19.0` was created at `68fa479`.
Remote tag object `64ca544de4f1880e24f60561f776acee9647ad1c` peels to that same
commit. The matching [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.19.0)
was published at `2026-10-03T17:00:43Z`, is not a draft, and remains marked
as a prerelease. Its body uses the dedicated 0.19.0 release notes.

The documentation closeout changes no runtime, dependencies or version.
Publication closes release alignment; the acceptance gaps listed below remain
unverified. No subsequent release candidate is active.

## October 3: 0.19.0 alpha candidate preparation

Preparation started from clean main `a47d818` (PR #47), whose merged-main
CI `37108848149` passed. The owner approved preparing the next release.
`codex/release-0-19-0` aligns Cargo.toml and the root Cargo.lock entry to
0.19.0 without changing dependency resolution. Dedicated release notes cover
the merged changes in PRs #38–#47 and their remaining alpha limitations.

Candidate local gates passed on Windows with Cargo 1.94.0: 435 ordinary tests
passed and 21 prerequisite-dependent tests were ignored; formatting, clippy
with warnings denied, locked offline metadata and `git diff --check` passed.
The lockfile diff changes only the root package version.

Applicable ignored checks were then executed explicitly on the candidate:

- `RPROJ_TEST_TIMEOUT=180 cargo test --locked --test live -- --ignored --test-threads=1 --nocapture`:
  all 14 passed together in 141.82 seconds, including actual negative-gate
  diagnostics, setup replay/refusal, Watch recovery and local Jest pass/fail.
- `cargo test --locked --bin rproj real_rojo -- --ignored --test-threads=1 --nocapture`:
  all three passed in 4.93 seconds, covering ten built-in/guided validation
  variants and rejection of an invalid property.
- With `RPROJ_LIVE_JEST_CLI` selecting installed Jest Roblox CLI 0.4.1,
  `cargo test --locked --test test_workflow open_cloud_missing_credentials_report_names_and_preserve_exit_code -- --ignored --exact --test-threads=1 --nocapture`:
  passed in 5.01 seconds, exercising 14 refusals with synthetic partial values.

Installed tools included Rojo 7.7.0, Wally 0.3.2, Rokit 1.2.0, Git for Windows
2.52.0, Selene 0.31.0 and StyLua 2.5.2. Before/after comparisons confirmed
unchanged machine configuration, Jest Studio plugin bytes, existing project
names and saved setup names. Disposable fixture cleanup completed.
Headroom processed the approved Cargo version diff (zero tokens saved);
diagnostic logs stayed local.

Clean candidate `4c34500` passed `cargo package --locked`: 101 files,
1.3 MiB (324.0 KiB compressed), followed by a successful extracted-package build.
The file list excludes documentation, development metadata and user secrets.
Both packaged Cargo files use 0.19.0, retain Rust 1.89, and the package Git
identity matches the clean commit. Subsequent documentation-only updates do
not alter the tested source, dependencies or packaged file set.

The final `cr review --agent --base main` review of `41f1b18` reported zero
findings across all six changed files. The iterative review's pending-gate
advisory confused this candidate with prior PR #47 evidence; the sections now
explicitly distinguish them. Later documentation-only evidence updates are
inspected directly. Exact-head and merged-main CI must both pass and be recorded
on [release PR #48](https://github.com/chatarabdelilah/rproj/pull/48) before
owner publication. That PR's closeout records the final candidate Git identity,
both CI runs, tree preservation and completed branch cleanup.

At preparation closeout, owner publication and tag/GitHub release alignment
were pending; the publication section above records their subsequent completion.
Authenticated Open Cloud execution, fresh Windows
provisioning, fresh Linux generated-project execution, UI Labs Studio stories,
Scribe Studio playtesting and unavailable Windows symlink privileges remain
unverified; Unix permission-mode checks were not run on Windows.

## October 3: Open Cloud missing-credential regressions

The audit began on clean main `2bc6462` (PR #46), with merged-main CI
`37083248511` passing and no open PRs. Generated cloud CI already refused
missing credentials; local `rproj test` delegated credential resolution to
Jest Roblox. No production correction was needed.

The new ordinary regression executes the credential guard extracted from
generated CI with Git Bash, followed by an offline success marker. All seven
incomplete combinations are tested with absent and empty variables: each exits
1, names the required fields and prevents the marker from running. All-present
synthetic values reach the marker without launching a runner. No API-key value
is printed. This is an executed shell check, not only a workflow string check.

With `RPROJ_LIVE_JEST_CLI` pointing to installed Jest Roblox CLI 0.4.1,
`cargo test --locked --test test_workflow open_cloud_missing_credentials_report_names_and_preserve_exit_code -- --ignored --exact --test-threads=1 --nocapture`
passed in 4.10 seconds. It drives `rproj test` for all seven incomplete
credential combinations using standard variables and then `JEST_`-prefixed
variables: 14 refusals, each naming exactly the missing fields with environment
guidance and preserving runner exit code 2. Each case keeps the production
project, test source and installed package link byte-identical. Preparation
uses fixture tools; the existing package-reuse regression still passes.

The real-runner fixture clears all six credential variables only in each child
environment, supplies synthetic values for present fields, never supplies a
complete set, and sets the cloud base URL to loopback. It uses disposable
directories and does not provision applications or change user authentication,
projects, machine configuration or Studio plugins. This test remains ignored
without its explicit installed-runner prerequisite. Source excerpts were sent
to Headroom under the owner's existing approval; diagnostics stayed local.

This establishes missing-credential error handling, not authenticated cloud
execution or upload success. Fresh-machine provisioning and Windows symlink
execution also remain separate gaps. No runtime/dependency/version change,
publication, tag or release preparation. Review and CI evidence belong to the
implementing PR.

Local gates passed: 435 ordinary Windows tests, with 21 prerequisite-dependent
tests ignored; formatting, clippy with warnings denied and `git diff --check`
also passed. The new ordinary shell test requires Bash from the existing Git
for Windows installation. The restricted sandbox token could not create Bash's
signal pipe; the test passed under the normal host token used by this suite's
Windows permission regressions.

CodeRabbit identified an additional Git Bash installation layout; discovery now
accepts both `bin/bash.exe` and `usr/bin/bash.exe` beside Git's ancestor paths.
Its advisory to ignore or skip a missing Bash is not adopted: Windows CI provides
Git Bash, this guard should be exercised by ordinary CI, and unavailable tooling
must remain a reported prerequisite failure. The requirement is documented.

[PR #47](https://github.com/chatarabdelilah/rproj/pull/47) merged at `a47d818`.
Final CodeRabbit review of head `e1e40df` reported zero findings. PR CI
`37108674698` and merged-main CI `37108848149` passed Windows stable,
Rust 1.89 and locked packaging. The merged tree matched the reviewed head;
the completed local and remote branch was removed.

## October 3: Template Explorer validation with real Rojo

Clean main `845512a` (PR #45) passed
`cargo test --locked --bin rproj real_rojo -- --ignored --test-threads=1 --nocapture`
with installed Rojo 7.7.0: **three passed, zero failed, zero ignored**, in
4.24 seconds. This explicitly executes three prerequisite-dependent tests
that remain ignored in ordinary CI; it does not change the ordinary test count.

- The built-in template passed sourcemap generation and binary builds for all
  ten validation variants: plain, plain with tests, four Wally combinations,
  two submodule combinations, and two Jest combinations.
- Guided CFrame, UDim2, UDim, and Rect attribute values, plus guided GUI
  Size/SliceCenter/Padding properties, passed the same real-Rojo validation.
- An invalid Part Anchored value was refused with a Rojo rejection error.

These tests use disposable validation workspaces, and no matching temporary
workspace remained afterward. They do not save the user's global template or
modify existing projects, provision tools, or use cloud credentials. This
verifies the validation boundary, not a new interactive edit/save/cancel session.
Open Cloud, fresh-machine provisioning, and Windows symlink-privilege execution
remain separate gaps. No runtime, dependency, version, publication or tag change.

[PR #46](https://github.com/chatarabdelilah/rproj/pull/46) merged at `2bc6462`.
Its documentation diff was inspected directly. PR head `0c4a027` passed CI
`37083008971`; merged-main CI `37083248511` passed Windows stable, Rust 1.89,
and locked packaging. The merged tree matched the reviewed head and completed
local/remote branches were removed.

## October 2: combined live verification and negative-gate diagnostics

After the GitHub API quota reset, clean main `144c35a` (PR #44) passed
`cargo test --locked --test live -- --ignored --test-threads=1 --nocapture`:
**14 passed, zero failed, zero ignored**, in 138.22 seconds, with
`RPROJ_TEST_TIMEOUT=180`. This closes the previous combined-run gap for project
creation, cancellation/revision, saved setup replay/refusal, Wally/submodule
builds and checks, dependency recovery in Watch, and local Jest pass/fail.
Jest's three starter specs passed; its deliberate failure returned 1 with
two passing/one failing test and preserved the installed package files.

The captured negative-gate output exposed a fixture weakness despite that green
run: all three added defects followed the starter module's terminal `return`,
so syntax errors caused the failures instead of the intended diagnostics.
The fixture now inserts defects before `return`, preserving the strict-mode
header, requires each intended diagnostic (`TypeError:`, Selene's
`undefined_variable`, and StyLua's diff), and rejects syntax/parse errors.
It still requires a nonzero gate exit and a green gate after each restoration.
The undefined global may also fail type analysis; this is not a claim that
exactly one tool rejects each defect.

Adding diagnostic assertions first reproduced the old fixture's failure in
9.99 seconds. After correcting placement, the focused live test
`the_generated_gate_rejects_bad_code_one_step_at_a_time` passed in 17.08 seconds.
The full 14-test result above belongs to main `144c35a`; the focused result
verifies the only changed live test. No production behavior changed.

Local ordinary verification passed: `cargo test --locked --quiet` ran 434
passing tests with 20 prerequisite-dependent tests ignored. Formatting,
clippy with warnings denied, and `git diff --check` passed. Review and final-head
CI evidence belong to the implementing PR.

All temporary project/setup fixtures were removed after these runs. Machine
configuration stayed byte-identical; the existing project list and shared Jest
Studio plugin hash were unchanged. Existing tools, Studio and caches were used;
no machine applications, authentication or user projects were changed. Headroom
was used for authorized source compression; diagnostic logs stayed local.
Open Cloud, fresh-machine provisioning, and Windows symlink-privilege execution
remain separate gaps. No version bump, publication, tag or release is prepared.

[PR #45](https://github.com/chatarabdelilah/rproj/pull/45) merged at `845512a`.
CodeRabbit reported zero findings on reviewed head `5c92dbe`. PR CI
`37058950254` and merged-main CI `37059254161` passed Windows stable, Rust 1.89,
and locked packaging. The merged tree matched the reviewed head; completed
local and remote branches were removed.

## October 2: live workflow audit and repeated test preparation

The audit began on clean main `79f2dab` (PR #43), with merged-main CI
`36821611764` passing and no open PRs. This is unreleased source verification,
not a new package candidate. Installed prerequisites were Rokit 1.2.0, Rojo
7.7.0, Wally 0.3.2, Git 2.52.0.windows.1, Studio, and its Jest runner plugin.
Local Rust/Cargo were 1.94.0; Jest CLI 0.4.1 was available in tool storage.

The first serial live run was stopped after a Home test timed out: its helper
still expected Composition immediately after naming a project. The helper now
waits for Review and explicitly selects Start point. All three affected Home
tests passed in 19.98 seconds. A subsequent full run completed with 12 passing
and two failing tests in 101.90 seconds:

- The setup-refusal fixture used nested setup names, now forbidden by the
  existing single-name contract. It now reserves unique input/output filenames
  directly in the setup directory and retains byte-preservation/cleanup checks.
- The live Jest test failed before runner startup because `sync_for_test`
  reprocessed links already rewritten by `wally-package-types`. Complete Wally
  trees now reuse their existing links and types, regenerating only the sourcemap.
  Missing/stale dependencies retain the full install/sourcemap/retype path.

Focused verification after those fixes:

| Check | October 2 result |
| --- | --- |
| `cargo test --locked --test live invalid_saved_setups_refuse_before_reconfiguration_or_creation -- --ignored --exact --test-threads=1 --nocapture` | Passed all four refusal cases in 8.55 seconds. |
| `cargo test --locked --test live jest_starter_specs_pass_and_report_failure -- --ignored --exact --test-threads=1 --nocapture` | Passed in 26.99 seconds: three starter specs succeeded, then the deliberate failure returned 1 with two passing/one failing test. Both runs preserved every installed package file byte-for-byte. |
| Ordinary fixture CLI regression | Failed before the runtime fix; passed afterward. TestEZ and Jest each test complete-tree reuse twice, plus missing-package recovery followed by reuse. Fixture executables record ordering without network/provisioning. |
| `cargo test --locked --test live -- --ignored --test-threads=1 --nocapture` | Final combined attempt completed with nine passing/five failing tests in 79.31 seconds. GitHub rate-limit exhaustion prevented required Jest and saved-replay tool pins. Wally/submodule clean-gate and gate-failure checks also failed in this run; a complete rerun remains required. This is not a passing live suite. |
| Local ordinary gates | 434 ordinary Windows tests passed; 20 prerequisite-dependent tests ignored. Formatting and clippy with warnings denied passed. |

Live runs used `RPROJ_TEST_TIMEOUT=180` and serial execution. All owned project
and setup fixtures were removed, including the stopped run's fixtures; machine
configuration stayed byte-identical. Normal Jest scaffolding refreshed the
existing shared Studio runner plugin, changing its hash. Rokit/Wally caches and
trust state are shared. No machine applications were provisioned. Authentication
and existing user projects/setups were unchanged. The live gate helper
now prints captured tool output on failure rather than only an exit code.

CodeRabbit raised the case of complete bare links after a manual `wally install`.
The package-preserving test behavior is retained deliberately: runtime execution
does not require exported types, and type restoration belongs to the full
scaffolding/Watch sync. This boundary is documented and the ordinary CLI
regression covers both complete typed and bare links without mutating them.
Final CodeRabbit review on `9ddd80e` reported only a stale architecture test
total; it was corrected and the documentation-only diff was inspected directly.

The combined rerun after rate-limit reset is recorded above. Open Cloud,
fresh-machine provisioning, and Windows symlink-privilege evidence remain outside
this audit. No version bump, publication, tag, or release is prepared. Review,
package, and CI evidence belong to the implementing PR.

[PR #44](https://github.com/chatarabdelilah/rproj/pull/44) merged at `144c35a`.
Final CodeRabbit review covered runtime/tests on `9ddd80e`; the final head
`f46575c` corrected only documentation. Final-head CI `36991045623` and
merged-main CI `36991461179` passed. Locked packaging passed and completed
branches were removed.

## Unreleased: safe Jest refresh saves

Jest's production mount repair, generated test project, and merged runner config
now use the shared staging/permission helper. Replacement no longer removes or
uses existing `.rproj-old` files. Read-only and symbolic-link targets are refused;
failed replacement keeps its target intact and cleans up the temporary sibling.
Writes remain sequential, so an earlier successful production repair can remain
when a later target fails. There is no external-writer lock or crash guarantee.

The backup regression failed against the old writer and passed after the fix.
Focused Windows tests cover backup preservation and all three output targets'
read-only refusal and locked replacement failure, cleanup, and successful retry.
Shared permission regressions cover Windows DACL preservation; Unix mode and
Windows symlink execution retain the limitations recorded below. No dependency
or version change, machine provisioning, or live Studio/Open Cloud run.

October 1 local verification passed 433 ordinary Windows tests, with 20
prerequisite-dependent tests ignored, formatting, and clippy with warnings denied.
Review and CI evidence belong to the implementing PR.

[PR #43](https://github.com/chatarabdelilah/rproj/pull/43) merged at `79f2dab`.
CodeRabbit reported zero findings on `e847165`; reviewed-head CI `36821401624`
and merged-main CI `36821611764` passed. Completed branches were removed.

## Unreleased: staged upgrade replacements

Upgrade prepares and syncs every planned replacement before replacing any
target, using the shared staging/permission helper also used by Configure.
Preparation failure preserves target contents; newly created parent directories
may remain. The whole snapshot set is rechecked after staging. A failed
replacement preserves its target, cleans up temporary files, reports the number
of earlier targets saved, and asks the user to rerun after fixing the cause.
Earlier successful replacements are not rolled back. The separate additive
metadata merges remain outside this replacement boundary.

October 1 local verification passed 430 ordinary Windows tests, with 20
prerequisite-dependent tests ignored, formatting, and clippy with warnings denied.
The later read-only failure regression failed before the fix and passed after.
Actual Windows lock failures cover the first and second targets, cleanup,
progress reporting, and successful reruns. An injected partial staging write
preserves the original; Configure's read-only failure retains pending changes.
Existing Windows DACL regressions pass after moving the permission helper.
The Unix mode regression remains unexecuted locally.

CodeRabbit identified symbolic-link replacement as a behavior regression;
preparation now refuses linked targets without replacing the link or shared
configuration. The explicitly invoked Windows symlink regression was blocked
at fixture creation by OS error 1314 (missing privilege), not passed. It is
ignored on Windows by default and ordinary on Unix; no machine setting changed.

No dependency or version change; no provisioning or Studio/Open Cloud run.
Review and CI evidence belong to the implementing PR.

[PR #42](https://github.com/chatarabdelilah/rproj/pull/42) merged at `1e87f39`.
Final CodeRabbit review reported zero findings on `5c4405e`; reviewed-head CI
`36803293173` and merged-main CI `36803533721` passed. Completed branches were
removed.

## Unreleased: upgrade confirmation conflicts

Upgrade now retains the exact snapshots used for planning and merging. Recorded
rewrite targets and inputs, including the composition record and production
Rojo document, are checked after confirmation and before the first write. File
edit/create/delete conflicts refuse the reviewed plan without writes. Non-missing
read errors also fail safely. Cancellation preserves every fixture file and
directory; accepted upgrades retain custom settings and user-owned files.

September 30 local verification passed 426 ordinary Windows tests, with 19
prerequisite-dependent tests ignored, formatting, and clippy with warnings denied.
The stale-save regression failed before the fix and passed afterward. All 15
upgrade terminal regressions pass without network or tool provisioning.
Conflict detection is optimistic; later writes are not a multi-file transaction.
The separate `.gitignore`, `.luaurc`, and `tests/.luaurc` merge helpers read
current files after the planned writes; their targets are not snapshotted.
No version bump or publication is prepared. Review and CI evidence belong to
the implementing PR.

[PR #41](https://github.com/chatarabdelilah/rproj/pull/41) merged at `d720134`.
CodeRabbit reported no runtime findings; its documentation clarification was
corrected and directly inspected. Reviewed-head CI `36763458103` and merged-main
CI `36763842366` passed. Completed branches were removed.

## Unreleased: shared configuration saving

CLI configuration and Ratatui now share one `EditSession`: a single loaded
snapshot, current values, pending changes, checked merging, and staged file
replacement. Direct CLI saves now reject external edits (including valid
TOML/JSON, file creation, and deletion). No-op bytes, unknown/unsupported values,
and cancellation remain protected. Failed saves retain pending changes and the
original baseline for retry. Conflict detection is optimistic; this is not a
cross-process lock or a guarantee of crash durability.

September 30 local verification passed 421 ordinary tests on Windows, with 19
prerequisite-dependent tests ignored, formatting, and clippy with warnings denied.
New coverage includes both formats' no-op/reverted changes, external file
creation/deletion/replacement, failed replacement cleanup and retry, invalid
merges, missing projects, and real CLI cancellation/conflicts. Read-only and
locked destinations retain the original file and pending changes. Replacement
preserves Windows DACL entries and inheritance protection; the safe wrapper
adds one Windows-only dependency without introducing unsafe repository code.
A Unix-mode preservation regression was added but not exercised locally.
No machine provisioning or Studio/Open Cloud run was needed or performed.

No version bump or publication is prepared. Review and CI evidence belong to
the implementing PR.

[PR #40](https://github.com/chatarabdelilah/rproj/pull/40) merged at `efe6615`.
Final local CodeRabbit review reported zero findings on `096b004`; reviewed-head
CI `36751976197` and merged-main CI `36760836260` passed. Completed branches
were removed.

## Unreleased: shared creation choices and validation

Direct CLI and Ratatui creation now use the capability catalog's shared rules
for workflow-compatible implementation choices, ordering, prerequisites, and
Jest backend routing. Picker ordering does not change legacy TestEZ defaults.
Direct prompts retain skip-with-explanation behavior; Ratatui retains correction
before graph mutation. Unknown saved values retain their existing behavior.

New catalog and screen tests cover all workflows, implementations, both Jest
backends, missing prerequisites, and unknown saved values. A real terminal test
exercises direct prompts without provisioning tools or creating project files.
This is a behavior-preserving refactor; no version bump or publication is prepared.

September 30 local verification passed 410 ordinary tests, with 19
prerequisite-dependent tests deliberately ignored, plus formatting and clippy
with warnings denied. Live Studio/Open Cloud and fresh-machine provisioning
were not rerun for this refactor.

[PR #39](https://github.com/chatarabdelilah/rproj/pull/39) merged at `fa8e6be`.
Local CodeRabbit review reported zero findings; reviewed-head CI `36668710496`
and merged-main CI `36668894684` passed. Completed branches were removed.

## Unreleased: UI Labs package

The optional `uiLabs` utility uses `pepeeltoro41/ui-labs@2.4.2` and can be
selected alongside any UI framework. Catalog details include a controls example
and explain that the Studio plugin is installed separately. Git submodules use
the utility repository's `src` module, not the plugin repository.

The Wally index confirms version 2.4.2 is shared with no dependencies. Upstream
tag `v2.4.2` supplies the source layout and control API; its checked-in
`wally.toml` still says 2.4.1, so the registry is the version authority.
Sources: [registry](https://github.com/UpliftGames/wally-index/blob/main/pepeeltoro41/ui-labs),
[utility source](https://github.com/PepeElToro41/ui-labs-utils/tree/v2.4.2).
Studio story execution remains unverified. This addition is not yet published.

[PR #38](https://github.com/chatarabdelilah/rproj/pull/38) merged at `eb09aef`.
Local CodeRabbit review reported zero findings. Reviewed-head CI `36667964166`
and merged-main CI `36668163520` passed Windows stable, Rust 1.89, and packaging.
The completed feature branches were removed.

September 30 local verification passed: 404 ordinary tests (19 deliberately
ignored), formatting, clippy with warnings denied, locked packaging with
`--allow-dirty` for the reviewed source, and `cargo run --locked -- info uiLabs`.
The ordinary catalog checks cover guide imports, unique keys, dependency closure,
and package selection. No Studio plugin installation or story playtest was run.

## 0.18.1 published

Scope is the post-0.18.0 work from [PR #33](https://github.com/chatarabdelilah/rproj/pull/33),
[PR #34](https://github.com/chatarabdelilah/rproj/pull/34), and
[PR #35](https://github.com/chatarabdelilah/rproj/pull/35): compatible existing
Jest mounts, explicit setup choices, Scribe Studio, forced trusted global Rokit
adds, additive project composition, Catalog mouse scrolling, TestEZ companion
configuration, explicit StarterPlayer classes, and reliable Test/Watch Wally
synchronization. No dependency was added.

[Release notes](release-notes-0.18.1.md) record user-visible behavior and limits.
The owner published from clean commit
`9d136812d5d33d358b67a36e503db47466fc0760` on September 29, 2026. The
non-yanked crates.io archive checksum is
`e2080b8fef645191fdb7e6c0df5c33b11530a0678080a5efdac067bab285c7d3`;
its embedded Git identity, annotated tag, and
[GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.18.1)
match that commit.

Local candidate gates passed 404 ordinary tests with 19 prerequisite-dependent
tests ignored, formatting, clippy with warnings denied, and locked packaging
(96 files, 1.2 MiB uncompressed, 310.1 KiB compressed). The final clean package
also embedded `9d136812d5d33d358b67a36e503db47466fc0760`. Reviewed-head CI run
`36590433561` and merged-main CI run `36590794320` passed Windows stable,
Rust 1.89, and package jobs. Evidence is recorded in
[PR #36](https://github.com/chatarabdelilah/rproj/pull/36). Fresh Windows
provisioning and a Scribe Studio playtest remain unverified.

## 0.18.1: project composition maintenance and reliable test/watch coexistence

New Project starts at Review and retains Start point access to Guided, Expert,
and saved setups. Projects can add packages and capabilities through a Ratatui
editor for valid `rproj.toml` projects. Existing choices remain selected;
dependency-workflow changes and removals stay outside this bounded editor.
Wally updates preserve unrelated entries and comments, generated configuration,
test mounts, test folders, tool pins, and sourcemaps are refreshed, and an
external `rproj.toml` edit blocks saving.

Test execution reuses a complete selected Wally package tree, avoiding the
directory-replacement window that crashed a concurrent Rojo sourcemap watcher.
Missing aliases still run the existing Wally recovery path. TestEZ companion
configuration is always generated with TestEZ. Catalog details accept mouse-wheel
scrolling while keyboard scrolling remains available. Generated StarterPlayer
and StarterPlayerScripts nodes carry explicit class names.

The project editor is additive: stop an active Watch before applying package
additions. It may reformat `rproj.toml`; Wally entries outside rproj's catalog are
left untouched. On September 29, 2026, the locked ordinary suite passed 404 tests
with 19 prerequisite-dependent tests ignored. Formatting and clippy with warnings
denied passed, and the installed real Rojo validated every built-in template
variant. CodeRabbit identified ten applicable recovery, manifest-preservation,
and freshness findings; all were corrected, and the final complete-diff review
reported zero findings. CI evidence belongs to the implementing PR.

## 0.18.1: explicit choices, Scribe Studio, and Rokit setup trust

Fresh Machine Setup and New Project optional selections start unchecked;
saved choices are preserved. Removed catalog selection-default fields and
redundant initial-selection state. Scribe Studio uses the existing manual
Studio-plugin flow. Confirmed machine tool installation uses
`rokit add --global --force`, including for pinned sources; project-local pins
keep their existing behavior. A setup rerun reinstalls selected global tools.

The [duplication audit](duplication-audit.md) records measured baseline size,
shared execution already present, remaining duplicated policy/persistence,
and the recommended bounded consolidation. It is not a claim of a completed
Inquire migration. Tests, review, and CI evidence are recorded in the PR.
No live machine provisioning or Scribe playtest was performed.

## 0.18.0 published

Scope is the merged work from [PR #30](https://github.com/chatarabdelilah/rproj/pull/30)
and [PR #31](https://github.com/chatarabdelilah/rproj/pull/31), plus aligned manifest
versions and release documentation. No dependency or runtime behavior changed
during version preparation.

[Release notes](release-notes-0.18.0.md) include the DevPackages import migration,
Local Studio/Open Cloud selection, credentials, and verification limits. The owner
published from clean commit `ba1319b5ec0a681ffac0d773ae0fb893b21c0692` on
September 21, 2026. The non-yanked crates.io archive checksum is
`0882efd53f10ade38d6b46515a699b77ca972769383903c25a1f0e33ed27df23`;
its embedded Git identity, annotated tag, and
[GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.18.0)
match that commit.

Candidate verification results and reviewed-head/main CI are recorded in
[PR #32](https://github.com/chatarabdelilah/rproj/pull/32). Live Open Cloud,
fresh Linux generated-project execution, and fresh Windows provisioning remain
unverified; ignored tests are not counted as passes.

## 0.18.0: Jest execution selection and Ratatui refinements

The Jest picker now asks for Local Studio or Open Cloud. The choice is retained
in project records and saved setups, drives runner configuration, and determines
whether generated CI contains cloud tests. Local creation alone provisions the
Studio runner plugin. This supersedes PR #30's repository-variable switch.

Catalog rows contain names only; group explanations appear in Overview and
entry explanations remain in Details. Enter opens groups, never focuses an
entry; Page Up/Down and Ctrl+Home/End scroll details without a focus change.
Projects > Configure Tools stays in the shared Ratatui terminal, reviews changes,
preserves unrelated/unsupported settings, protects cancellation, checks external
edits, and atomically saves the selected tool's configuration. Direct CLI configure
retains its prompt interface.

Local verification: 388 ordinary tests passed, 19 prerequisite-dependent tests ignored; formatting and clippy passed. The PTY regression saves a tool setting and returns to the project with one terminal enter/leave and no suspension. Unit coverage checks settings preservation, cancellation, external edits, malformed files, JSON merging, four terminal sizes, Jest backend persistence, and both generated CI variants. The isolated installed Jest Roblox 0.3.24/Studio regression also passed eight package examples after real Wally installation and Rojo sourcemap regeneration. CodeRabbit reported one documentation clarification about atomic visibility versus crash durability; it was corrected. Reviewed-head/main CI are recorded in the implementing PR. Open
Cloud execution is not claimed without credentials; no applications are installed
by ordinary tests.

## 0.18.0: generated project CI and ignore coverage

Jest projects now expose `ReplicatedStorage.devPackages` in the ordinary
sourcemap as well as the Jest project. Disk casing remains `DevPackages`.
The test project omits Lighting and enables LoadStringEnabled. PR #30 initially generated CI that
opted into Open Cloud with `JEST_OPEN_CLOUD=true` (superseded above); enabled cloud tests still failed
on missing credentials. Ignore rules cover generated place files, place locks,
coverage, and local environment files while preserving `.env.example`.
See [the source review and migration notes](project-template-review.md).

Local verification: 378 ordinary tests passed, 19 ignored; formatting and clippy
with warnings denied passed. The isolated real-Wally/Rojo/Jest regression passed
using the installed pinned Jest Roblox 0.3.24 executable, including ordinary
sourcemap development-package resolution and eight Studio package examples.
The initial attempt through the Rokit shim failed because the temporary fixture
has no Jest tool manifest; direct use of the existing pinned binary resolved that
fixture prerequisite without installing applications. Open Cloud execution and
fresh Linux generated-project execution remain unverified. CodeRabbit review
and reviewed-head/merged-main CI evidence are recorded in the change's PR.

## T5: 0.17.0 Published

Owner publication is verified. The non-yanked crates.io archive checksum is `c95a02febad76d5956b2446dbf492ea78ddb888d248e34a5a0737fa266ef13a2`; its Git identity, annotated tag, and [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.17.0) match clean commit `9ee8d5167175c7bacb58a61e9317b4fda5d443ea`. [Final reviewed-head CI](https://github.com/chatarabdelilah/rproj/actions/runs/35499542790) and [merged-main CI](https://github.com/chatarabdelilah/rproj/actions/runs/35499712898) passed Windows stable, Rust 1.89, and packaging. Badge freshness was intentionally skipped. All three merged implementation/fix branches were deleted locally and remotely.

A subsequent stable-Windows CI run timed out in the Catalog diagnostic-log PTY test. That test now waits for its filter to render and uses the documented direct Catalog exit instead of an unsynchronized Enter/Esc sequence. All eight diagnostics tests and formatting passed locally; this is test-only and does not change Catalog navigation.

Implementation merged through [PR #26](https://github.com/chatarabdelilah/rproj/pull/26), followed by test-only synchronization corrections.

Merged-main CI exposed a repeated-Watch PTY synchronization race. A checkpoint before launch was insufficient: ConPTY repainted the previous normal-screen acknowledgement when the next watcher left the alternate screen. The failed CI snapshot showed that watcher still running while the test expected Project actions. Each fixture launch now has a unique readiness marker, and completion waits start only after that marker is visible. Ten consecutive runs of all nine hub tests passed locally. This changes test synchronization only, not Watch runtime behavior. [PR #28](https://github.com/chatarabdelilah/rproj/pull/28) records the correction and final-head/main CI gates; owner publication followed successful checks.

- Local locked ordinary suite: **375 passed, 19 ignored**, 394 discovered (334 unit and 60 integration tests). Fixture PTY coverage exercises review/category cancellation, default-No confirmation, success and Back/Exit, worker panic, fatal failure, save failure, and stop-after-active-child without configuration saving or a subsequent item. Home cancellation remains in one terminal session.
- Real fixture subprocesses exercise concurrent stdout/stderr draining, Unicode/control-sequence handling, failed spawning, nonzero exits, and cooperative stopping. All roots/configuration writes are injected temporary fixtures; no applications are installed. Global-config replacement is staged, synced, and tested for preserving the old file when Windows denies replacement.
- Formatting, clippy, and locked packaging passed (94 packaged files, 1.1 MiB uncompressed, 294.8 KiB compressed after review fixes). CodeRabbit identified four applicable findings: completion lost after Back/Exit, CRLF doubling, mismatched Rokit trust guidance, and stale architecture documentation. All were corrected with regression coverage; follow-up module-map indentation was also corrected. Final local gates passed after runtime fixes. [Reviewed runtime CI](https://github.com/chatarabdelilah/rproj/actions/runs/35457787313) passed Windows stable, Rust 1.89, and packaging at `df102d0`. [PR #26](https://github.com/chatarabdelilah/rproj/pull/26) records final-head checks and release closeout; final merged-main verification and owner publication are recorded above.
- Missing live acceptance: fresh Windows provisioning, winget/vendor/UAC dialogs, real VS Code/Blender installation, and manual plugin/account linking were not exercised. Existing installers and project paths remain, but fixture parity does not establish fresh-machine installation success. No unrelated ignored live integrations were rerun.
- Limits: stop waits for the active item; no rollback or force-kill. A stalled installer may require external intervention. Display/output parsing are bounded with explicit truncation/failure behavior. Completed attempts record selection intent even when individual installations need repair; warnings are not proof of readiness.

## T4: 0.16.0 Published

Owner publication is verified. Crates.io reports non-yanked `0.16.0`; its archive SHA256 is `8031eb010130589d0b9ab74233845aabc53557e9d799f54d372ad04ee0373c3d`, matching the registry checksum. `.cargo_vcs_info.json` identifies clean commit `d54a9a874d8f23f5a57ebd573b6f1cd806506000`. The annotated `v0.16.0` tag and [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.16.0) use that commit. [Main CI](https://github.com/chatarabdelilah/rproj/actions/runs/35426957745) passed, and the fully merged implementation branch was deleted locally and remotely.

- Local ordinary locked suite: 351 passed, 19 ignored (370 discovered: 312 unit and 58 integration tests). A stale Catalog navigation index was corrected after inserting the new Home destination.
- Storage checks cover byte preservation, unknown fields, unsupported choices, invalid Jest repair, conflicts, collisions, name restrictions, Windows junction rejection, locked replacement failure, duplication, deletion, and rename partial failure. Fixtures use temporary storage.
- Manager checks cover saved baselines, default-No discard/deletion, failed saves retaining drafts, browser Back state, four terminal sizes, and repeated-save PTY restoration. Home PTY coverage verifies the manager borrows the same terminal. Creation controls retain provenance and unknown exclusions in setup mode.
- Formatting and clippy passed. The saved-setup replay integration passed for Wally and Git submodules (one serial test, 22.45 seconds), including direct replacement semantics and source-byte preservation during replay. Unrelated live integrations are not rerun for this milestone.
- Locked packaging passed at runtime commit `f85e14d`: 89 files, 1.1 MiB uncompressed, 284.6 KiB compressed; archive compilation passed. CodeRabbit CLI 0.7.6 completed the committed branch's base-main review with zero reported findings. A preliminary working-tree review also reported none; the committed review was necessary to include new source files after staging. Subsequent evidence-only edits do not change runtime or dependencies. [PR #24](https://github.com/chatarabdelilah/rproj/pull/24) records the reviewed commit, CI run links, and merged-main verification; both Windows toolchains and packaging must pass before owner publication.

Limits: unsupported composition choices conservatively disable guided editing; file management remains available. Changed composition saves may drop comments/reformat TOML. External-writer checks are best effort, and rename is not a multi-file transaction. T5 remains Machine Setup; model import remains permanently dropped.

## T3: 0.15.0 Published (September 10, 2026)

Owner publication is verified: crates.io's archive SHA256 is `2dc75bbd10e4e11180995ce267f2b5a7f96b8eb60802fe1adf4e10ce50424a45`; `.cargo_vcs_info.json` records clean commit `b1c1664273bea2fce9bb0f4b954f554cc96f9dfb`. The annotated `v0.15.0` tag and [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.15.0) match that identity. Version 0.15.0 is not yanked.

- Scope: Projects browser, selected-path command dispatch, creation handoff, and removal of Catalog Place Template presentation. No dependency or configuration-schema change; model import stays dropped.
- Local ordinary locked suite: 330 passed, 19 ignored (349 discovered: 292 unit and 57 integration tests). Coverage includes junction exclusion, malformed projects, missing roots, stale scans, Back state, four-size rendering, deleted targets, and A/B path isolation with fake tools and a fake clipboard sink.
- Eight Home PTY checks cover terminal continuity, selected-project actions, repeated Watch interruption, watcher failure, cancelled provisioning, and redirected output. Three live creation checks passed serially using installed tools: cancellation, concurrent-destination refusal, and successful creation/saved-setup handoff. Their scratch projects are removed; no machine applications were installed.
- Formatting, clippy, and locked packaging passed. Packaging at `c57221c` contained 86 files (1.0 MiB uncompressed, 272.1 KiB compressed) and compiled successfully from its archive. Local CodeRabbit CLI 0.7.6 completed the base-main review on September 10 with zero findings. [PR #22](https://github.com/chatarabdelilah/rproj/pull/22) records reviewed-head and merged-main Windows stable/Rust 1.89/package CI; publication is gated on both runs passing.
- Existing Open Cloud and fresh-machine provisioning limitations remain. Ordinary tests do not modify the real clipboard or provision machine applications.
- Reviewed head `2eb6b49` passed CI run `34426275526`; merged main `b1c1664` passed CI run `34426498712`. Both runs passed Windows stable, Rust 1.89, and package verification. The merged feature branch was deleted locally and remotely. Owner publication, annotated tag, and GitHub prerelease are complete; documentation-only alignment needs no further Cargo publication.

Previous baseline (September 9, 2026): **0.14.0**, alpha, with persistent Home, template save continuity, and Catalog clarity. Its crates.io archive records `f86a7bd06d0f5b637a41a0e0ca05792be66c42e5`, matching the annotated tag and [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.14.0). SHA256: `b3469e9cf85ae5f1a88f6ec6aaf5065ce05301cd52172414b5adafc34974b95e`. Model import and embedded quality tools are permanently dropped. See [release notes](release-notes-0.14.0.md).

The published package and annotated `v0.12.2` tag correspond to commit `f71bf4e`; the [GitHub release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.12.2) is a prerelease. The automated Jest regression was merged afterward in [PR #7](https://github.com/chatarabdelilah/rproj/pull/7), at `a855a2f`. It is present on main, not in the published 0.12.2 archive; no runtime code or version changed in that PR.

## Confirmed Defects And Fixes

1. **Project creation touched the destination before confirmation.** Escape could leave an empty directory, and explicit Cancel recursively removed it even if another process had added files. Creation now happens after confirmation, uses an exclusive directory creation, and never deletes the destination on cancellation. Unit tests cover an existing directory and missing parents; a live regression covers another writer creating a sentinel while the prompt is open, for both Create and Cancel.
2. **Some Inspector values did not match Rojo's serialization.** UDim/UDim2/Rect now use explicit nested representations with integer offsets; CFrame attributes use position/orientation fields. Reopening values flattens components for the existing input controls. Unit round trips and a real-Rojo validation test cover the fix. NaN/infinity and fractional/out-of-range offsets are rejected before changing the draft.
3. **Live tests were stale and unsafe to rerun against an occupied project root.** They selected TestEZ in the old package picker, assumed VS Code was selected, manually parsed the wrong global-config path, and deleted fixed-name directories. The harness now follows current capability/runner prompts, reads actual TOML via the configured directory API, checks provisioning up front, and uses unique scratch names without removing pre-existing directories.
4. **Jest Roblox projects could not run their selected test runner.** Rokit 1.2 rejected untrusted project-local sources, the generated runner invoked `jest-roblox` while Rokit installs `jest-roblox-cli`, and `test.projects` used DataModel-path strings that 0.3.24 interprets as configuration-file paths. rproj now trusts selected sources before every add, invokes the installed executable, and emits inline project entries with filesystem include globs.

## Evidence

### 0.14.0 T2 Published

Published at `f86a7bd` after PR #20 and owner publication; archive, annotated tag,
and alpha release are aligned. Scope: persistent Home, template save
baselines, foreground cancellation, Catalog hierarchy/readability, and bundled
package guidance. No new catalog packages or machine applications are installed.

September 9 local evidence:

- The ordinary locked suite passed: 320 passed, 19 explicitly ignored,
  339 discovered (282 unit and 57 integration). Formatting, clippy with warnings
  denied, and diff whitespace checks passed. Reviewed-head/main CI and package
  identities are recorded below when complete.
- Windows PTY fixtures passed repeated Watch interruption, unexpected Watch
  failure, Home re-entry, and interrupted Rokit restoration without starting Lute.
  An exclusively held child file proves the child has exited before Home returns.
- Template PTYs passed repeated saves without closing, confirmed reset, and
  malformed JSON repair. Unit tests cover saved-baseline undo/redo and atomic
  failure. A large paste exposed per-character full-screen redraw cost; queued
  input now redraws at a bounded interval. Float round-trip testing exposed
  serde_json's default parser rounding; float_roundtrip now preserves those values.
- All three real-Rojo template/editor checks passed with Rojo 7.7.0. All three
  live Home-creation regressions passed in 29.51 seconds: cancellation,
  concurrent-destination refusal, and confirmed creation/saved-setup replay.
  Initial sandbox denials were not counted as passes; the successful run used
  unique temporary projects and a unique saved setup outside the sandbox.
- Eight Catalog package snippets executed without errors through installed
  Jest Roblox CLI 0.3.24 and Studio, following Wally installation and sourcemap
  retyping. The fixture needed its missing source directories restored; its
  original count incorrectly assumed starter specs despite requesting none.
  The corrected complete live stack check passed in 12.83 seconds.
  [Example evidence](catalog-examples.md) separates source review from runtime coverage.

Locked packaging passed at `8389643`: 85 files, 1.0 MiB uncompressed and 263.9 KiB
compressed; the crate built from its archive. The package includes the new source
modules and PTY fixture, while retaining the existing documentation/CI exclusions.
After review fixes, locked packaging passed again at `4889a48`: the same 85 files,
1.0 MiB uncompressed and 264.2 KiB compressed, compiled from the archive.

Local CodeRabbit review identified recovery and cancellation improvements:
creation now names the retained destination on failure/cancellation, Blender's
temporary script has RAII cleanup, and VS Code/Blender discovery preserves
cancellation instead of a misleading missing-tool error. VS Code extension
cancellation is propagated, and README states the help/version logging exception.
The ordinary suite and clippy passed again after these runtime fixes. The full
local review completed with 11 findings; the narrow six-file follow-up completed
with zero findings. [Release PR #20](https://github.com/chatarabdelilah/rproj/pull/20)
records reviewed-head CI, any remote review follow-up, and merged-main verification
before the owner is asked to publish. Documentation-only evidence updates do not
change the tested runtime or dependency graph.

Remote CodeRabbit review at `13ffed0` added an explicit Jest JSON formatter and a
90-second failing watchdog for the PTY child fixture. Both were applied: all
eight Home PTYs passed, and all eight Catalog snippets passed again through
Studio in 37.88 seconds. Its prose-wrapping edge case was reproduced and fixed
with assertions in the existing Catalog test; the four Catalog tests pass.
The audit header date was corrected. Windows stable, Rust 1.89, and package CI
passed at both `13ffed0` and final correction commit `2ec83ba`, then on merged
main `f86a7bd`. The owner's `.codex/` and `.serena/` ignore rules keep local agent
configuration outside Git and Cargo archives; no such configuration is shipped.

Review decisions: do not force `process::exit` on a second interrupt because it
skips terminal restoration and child waiting. Do not record machine setup as
completed after cancellation. Do not relax the immediate child-lock assertion:
the subprocess wait must already have completed before acknowledgement. Catalog
Ctrl+C intentionally returns to Home (standalone Catalog exits); its separate
Quit/Back outcomes express that caller-dependent behavior. Acknowledgement remains
the documented Enter action: the suggested interrupted-read shortcut did not
work in a Windows PTY and was reverted, not counted as a passing Ctrl+C check.

Remaining limits: no fresh-machine installation/cancellation certification,
no Open Cloud run, and no full UI/data/replication lifecycle execution for every
package. Commands ignoring Ctrl+C can delay return until they finish; no forced
termination or detached supervisor is added.

### Unreleased Configuration Preservation

Three new regressions first reproduced destructive behavior in 0.13.0: an unlisted choice was reset to its catalog default, a structured JSON value was replaced with a boolean, and accepting handwritten TOML produced duplicate keys/tables. The fix omits unchanged answers, keeps unsupported values by default, and validates a proposed TOML merge against the expected parsed document before writing. Explicit replacement remains available. This does not introduce file locking or crash-atomic writes, and the legacy TOML writer may refuse unusual valid layouts rather than rewrite them unsafely. Shared upgrade/scaffolding writers are unchanged.

September 8: all 309 ordinary tests passed (19 explicitly ignored, 328 discovered). Eight regressions were added: two unit tests and six PTY tests. An initial new replacement test used the heading-wait helper for an inline confirmation; correcting that test synchronization produced the passing full run. Formatting, clippy, reviewed-head CI, and main CI passed on the fix PR. The 0.13.1 release candidate packaged 80 files, 965.4 KiB uncompressed / 249.0 KiB compressed; [release PR #17](https://github.com/chatarabdelilah/rproj/pull/17) and [merged-main CI](https://github.com/chatarabdelilah/rproj/actions/runs/34183475162) passed. CodeRabbit required manual review for this OSS release PR; that review found no issues. Live Studio/Rojo/provisioning checks are not rerun because their execution paths are unchanged; the dated 0.13.0 evidence below remains historical, not a new pass.

### Released Evidence

| Check | Result |
| --- | --- |
| 0.13.0 candidate local gates | September 8: 301 ordinary tests passed (19 explicitly ignored); formatting and clippy passed. All 14 serial live regressions passed in 183.89 seconds. The three real-Rojo checks passed in 5.22 seconds. Verified installed Rokit 1.2.0, Rojo 7.7.0, and Wally 0.3.2; the Jest runner was exercised through its project-local pin, not a global shim in this repository. Packaged verification and exact reviewed-head/main CI are recorded on the release PR before publication. |
| Creation merge/main CI | PR #14 merged at `a90037f`, tree-identical to reviewed `ef1d5df`. [Main CI](https://github.com/chatarabdelilah/rproj/actions/runs/34162680901) passed Windows stable, Rust 1.89, and package builds. Both CodeRabbit findings were resolved; the merged feature branch was deleted. |
| Creation source verification | 301 ordinary tests passed; 19 explicitly ignored; 320 discovered (273 unit + 47 integration). Formatting, clippy, and `cargo package --locked` passed (80 files). The three real-Rojo template/Inspector checks passed. Windows stable, Rust 1.89, and package CI passed on `ea07979`; [PR #14](https://github.com/chatarabdelilah/rproj/pull/14) records subsequent reviewed-head and merge/main verification. |
| Creation CodeRabbit review | Both actionable findings were verified and fixed: pasting into name/setup modals now preserves the Review selection (regression assertions added), and architecture live-test counts/breakdowns now match 14 live tests. Unfinished capability choices also have explicit back-navigation regressions. |
| Creation CI correction | The first Windows stable/1.89 run exposed an existing hub-name test that depended on this machine's saved setup. The fixture now explicitly marks setup ready; the separate disabled-action test covers the unconfigured state. No machine provisioning was added to CI. |
| Creation live regression | September 7: all 14 serial live tests passed in 125.57 seconds, including hub cancellation, real confirmation, concurrent destination refusal, named setup save/replay, direct saved-setup replay/refusal, Wally/submodules, generated quality gates, and Jest starter success/deliberate failure. The initial hub test used the wrong prompt label; corrected to the existing `Project folder name` before the passing run. Only unique temporary fixtures were removed. |
| Ordinary suite with saved-setup refusal regression | 273 passed; 16 deliberately ignored; 289 discovered (253 unit + 36 integration) |
| Diagnostic-logger source suite | 286 passed; 16 deliberately ignored; 302 discovered (258 unit + 44 integration). Five logger unit tests and eight integration tests cover unique bounded logs, redaction/control escaping, command outcomes, accepted settings, TUI navigation without text capture, opaque runner arguments, opt-out, and non-fatal logging failures. |
| Live regression rerun with diagnostic logging | Passed September 7: saved-setup refusal/replay together in 28.53 seconds; Jest starter specs passed, then the deliberate failure returned 1 with two passed / one failed in 28.93 seconds. Existing unique temporary fixtures were removed by the harness. |
| Formatting and clippy | Passed locally |
| Existing live project suite | Seven passed in 38.39 seconds on September 5 after harness corrections |
| Concurrent-destination live regression | Passed for both cancellation and confirmation |
| Existing real-Rojo template checks | Passed with Rojo 7.7.0 |
| Compound Inspector output | Passed real-Rojo validation across the template matrix |
| Live Jest Roblox execution | `jest-roblox-cli` 0.3.24, Studio CLI backend: three generated starter specs passed in 11.58 seconds on September 6 |
| Automated live Jest regression, PR #7 | Reviewed test passed in 26.84 seconds on September 6: scaffolded Wally + Jest in a unique temporary directory, verified project-local pins and three passing starter specs, then broke the shared spec and verified exit code 1, the assertion failure, and two passing / one failing test. Temporary project removed. |
| Automated saved-setup replay | Passed in 20.07 seconds on September 6: real `new --save-setup` then `new --like` for both Wally and Git submodules, with charm/promise, lint/format, and a dropped `.gitignore`. Verified exact graphs, no repeated choices, installed package links, selected tool pins, generated/omitted files, unchanged saved setup, and cleanup. |
| Automated saved-setup refusal | Passed in 7.78 seconds on September 6: missing setup, malformed TOML, and Jest with no manager or Git submodules each exited 1 with a case-specific error before explicit `--reconfigure`. No input was sent; no destination/parent was created; machine config, source fixtures, and an existing output-setup sentinel remained byte-identical. Temporary directories removed. |
| Upstream badge check | Passed September 6; advisory to review Matter's Active badge (last reported push December 31, 2024); not proof that the project is abandoned |
| Previous CodeRabbit findings, PR #4 | Both addressed in baseline commit 4737c84; plugin identity and ServerPackages exclusion regression present |
| Published 0.12.2 package | cargo package --locked passed at `f71bf4e`; 73 files, 852.5 KiB uncompressed; crates.io reports 0.12.2 |
| PR CI and CodeRabbit | Historical 0.12.1 review: [PR #6](https://github.com/chatarabdelilah/rproj/pull/6). Jest regression: [PR #7](https://github.com/chatarabdelilah/rproj/pull/7), CodeRabbit reported no actionable findings at `2496a42`; Windows stable, Rust 1.89, and package checks passed. [Post-merge main CI](https://github.com/chatarabdelilah/rproj/actions/runs/34038222655) passed at `a855a2f`. |

Reproduction commands:

```powershell
cargo fmt --all --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
$env:RPROJ_TEST_TIMEOUT = '180'
cargo test --locked --test live jest_starter_specs_pass_and_report_failure -- --ignored --test-threads=1 --nocapture
cargo test --locked --test live saved_setup_replays_workflow_packages_capabilities_and_dropped_files -- --ignored --test-threads=1 --nocapture
cargo test --locked --test live invalid_saved_setups_refuse_before_reconfiguration_or_creation -- --ignored --test-threads=1 --nocapture
cargo test --locked --test live -- --ignored --test-threads=1
cargo test --locked steps::rojo::tests:: -- --ignored --test-threads=1
cargo test --locked guided_compound_values_pass_real_rojo_validation -- --ignored
cargo test --locked badges_do_not_contradict_upstream -- --ignored --nocapture
cargo package --locked
```

The live project suite uses unique directories under the configured projects root and shares real Rokit/Wally caches. The Jest regression requires Studio and JestRobloxRunner to be installed; normal scaffolding refreshes that plugin. Only its temporary project is isolated and removed. Run serially on an explicitly provisioned machine, not as an unattended installer on a fresh host. The regression remains ignored by ordinary CI and must be invoked explicitly.

## Remaining Limits

The refusal regression is also ignored because it reads real machine configuration and uses the configured projects/setup roots. It owns unique fixture directories and never answers provisioning prompts; a newly created empty setup directory may remain. A saved global template can invoke Rojo validation before setup refusal. This is not fresh-machine provisioning evidence.

The saved-setup regression uses the real configured projects root and setup directory, reserving a unique setup filename before the CLI writes it. Both projects and the reserved setup are removed, including partial runs; a newly created empty setup directory may remain. Existing setups and machine configuration are not changed. Rokit/Wally caches and trust state remain shared; this test requires provisioned tools and network access, runs serially, and does not install Studio plugins. Like the Jest regression, it is ignored by ordinary CI and does not change the published 0.12.2 package.

- **Jest evidence covers the local Studio CLI backend.** The automated starter-spec pass/fail gap is closed by PR #7. This does not verify Open Cloud, an attached Studio session, or a fresh Windows installation. In T2, the ignored `real_jest_stack_installs_validates_retypes_and_executes` test now executes eight Catalog snippets instead of only no-test success; the live regression separately verifies starter execution through `rproj test`.
- **Open Cloud execution was not performed.** No audit universe/place and credentials were supplied. Generated preflight and workflow text are tested, but that is not equivalent to a current hosted run.
- **Fresh Windows provisioning was not performed.** winget/editor/Studio/plugin installation changes the machine. Existing unit coverage and inspection are not a substitute for a clean-machine acceptance test.
- **Saved-setup evidence is bounded.** Replay covers valid Wally and Git-submodule compositions. Refusal covers missing/malformed records and Jest without Wally, before explicit reconfiguration or creation. It does not cover every hand-edited/legacy setup or unknown-package fallback. Clipboard contents, machine provisioning/recovery, and every template keyboard path also remain gaps. No claim of beta/1.0 readiness is made.
- **Template validity remains Rojo-authoritative.** This patch repairs specific existing compound controls, not every possible explicit/future property representation. Unsupported advanced values and old malformed drafts are not automatically normalized.

## Handoff

Keep Ratatui and all external tool boundaries. The model-import experiment was permanently retired at the owner's request on September 9, 2026. The local branch and importer-only build artifacts were deleted; no remote branch, separate worktree, or stash existed. No importer code was merged. Historical commits are not rewritten.

Releases 0.13.0, 0.13.1, and 0.14.0 are aligned and complete. The agent handles review findings, merge, post-merge CI, merged-branch cleanup, and release alignment; the owner alone runs `cargo publish --locked` when a new package is ready. Test-only and documentation-only follow-ups do not require publication or moving an existing release tag.

Saved-setup replay/refusal, diagnostic logging (PR #11), shared confirmed execution (PR #12), and Ratatui creation (PR #14) shipped in 0.13.0 through release PR #15. [Merged-main CI](https://github.com/chatarabdelilah/rproj/actions/runs/34181012236) passed. Configuration preservation shipped in 0.13.1 through PRs #16 and #17. T2 shipped in 0.14.0 through [PR #20](https://github.com/chatarabdelilah/rproj/pull/20), with [merged-main CI](https://github.com/chatarabdelilah/rproj/actions/runs/34317249700) passing at `f86a7bd`. The owner published; archive identity, annotated tag, and alpha release are aligned. Completed release branches are deleted. Do not republish or move shipped tags. Remaining alpha audit gaps stay tracked.

## Post-Handoff Review: September 7, 2026

Reviewed the task history and changes from v0.12.1 through main at `fe8f4f4`.

- **Release identity:** crates.io 0.12.2 is not yanked; its archive records
  `f71bf4e103735a532b2e8de53cbbd560f5c2a3d3`, matching the annotated v0.12.2
  tag and GitHub alpha release. Do not undo or retag that publication.
- **Process correction:** 0.12.2 was pushed directly to main and Cargo-published
  by the agent after a short "publish 0.12.2" request. That bypassed the intended
  PR workflow and owner-operated publication handoff. Root `AGENTS.md` now makes
  the ownership rule and new-task startup requirements explicit.
- **Change assessment:** the Jest executable/configuration/trust fixes address
  observed failures. PRs #7-#10 add regression coverage and documentation;
  PR #11 adds the requested logger; PR #12 extracts existing confirmed execution.
  No runtime blocker was found in this review. No rollback is indicated.
- **Independent verification:** 286 ordinary tests passed, 16 deliberately
  ignored; formatting and clippy passed locally. GitHub records successful
  main CI for every post-0.12.1 merge, including
  [fe8f4f4](https://github.com/chatarabdelilah/rproj/actions/runs/34101827132).
  CodeRabbit's documentation findings in PRs #8/#10 are addressed; logger and
  execution-boundary reviews report no actionable findings. The existing live
  test evidence is historical, not a new live run during this review.
- **Remaining caution:** logs contain local paths and selected names, have no
  automatic retention cleanup, and are not a complete screen transcript. Review
  them before sharing. Open Cloud and fresh-machine acceptance remain unverified.

Update September 8: **0.13.0 is published and aligned** after
[release PR #15](https://github.com/chatarabdelilah/rproj/pull/15) and successful
merged-main CI. The owner performed publication; the agent verified the archive
identity before creating the annotated tag and GitHub alpha prerelease. The next
configuration-preservation fix is separate and unreleased.
