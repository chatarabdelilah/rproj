# rproj - Release Roadmap

Updated October 7, 2026. This describes current priorities, not every idea considered during development. [Architecture](architecture.md) describes implementation; [UX](ux-redesign.md) defines the interface; [Releasing](releasing.md) defines publication gates.

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

The published baseline is **v0.21.0, public alpha**, removing Git-submodule dependencies so new projects and saved compositions use Wally or None. It retains the official Wally tool requirement, missing-Git guidance, background Watch acknowledgment improvements, missing-Cargo and missing-WinGet recovery, persistent Watch, Template Explorer recovery and file protection, UI Labs 2.4.2, shared creation/configuration behavior, safer saves and Wally package preservation during tests. Crates.io publication, archive identity, annotated tag and GitHub alpha release are verified at `07dcf17`. Cargo manifest and root lockfile are aligned at 0.21.0; there is no active release candidate. See [published notes](release-notes-0.21.0.md) and [audit evidence](release-audit.md) for verification and limits.

**Shipped in 0.21.0: fully retire Git-submodule dependencies.**
New projects and saved compositions support only Wally and None. Old
`git-submodules` records are intentionally rejected through configuration errors,
without aliases, automatic conversion or migration tooling. The owner retains
responsibility for older alpha projects. Ordinary Git initialization and missing-Git
recovery remain; existing projects, templates, `.gitmodules` files and dependency
directories are not deleted or rewritten. Previous shipped tags stay unchanged.
Implementation and delivery evidence are tracked in
[PR #93](https://github.com/chatarabdelilah/rproj/pull/93). Serial installed-tool
checks passed for all eight Rojo variants, Wally and None creation, gates and
saved Wally replay. The complete ordinary inventory is now 479 passed/23 ignored;
fresh-machine and authenticated Open Cloud acceptance remain separate gaps.

**Shipped in 0.20.4: retire the Wally source-build workaround.** Official wally-package-types 1.7.0 contains the generic-default and `const` parser fixes previously supplied by a patched build. Generated CI uses the Rokit-installed release. Upgrade refuses managed Wally CI when the official stable project pin is older than 1.7.0, absent or unverifiable; it explains an explicit update and leaves every file unchanged. Tool pins remain owner-controlled, and changes during confirmation refuse the upgrade. Isolated official-binary compatibility checks passed on Windows and a focused Ubuntu CI runner, separately from fresh-machine/project acceptance. Historical machine caches remain untouched. [PR #90](https://github.com/chatarabdelilah/rproj/pull/90) implements the change; [PR #91](https://github.com/chatarabdelilah/rproj/pull/91) prepared that release. Previous shipped tags remain immutable.

The automated live Jest regression is merged on main in [PR #7](https://github.com/chatarabdelilah/rproj/pull/7), after the 0.12.2 publication. It verifies three passing generated starter specs and a deliberate assertion failure through `rproj test`. Review and post-merge CI passed; this test-only change requires no package release.

The existing product includes machine setup, configurable project generation, saved compositions, upgrades, tool configuration, the global Template Explorer, workspace hub and Catalog, watching, source copying, and optional TestEZ/Jest Roblox testing.

The unfinished model-import experiment is **permanently dropped** by the owner's September 9, 2026 decision. Its unmerged local branch and importer-specific build artifacts were removed; no importer implementation or DOM/serializer dependencies entered main. It is not a future milestone.

The October 6 MSRV CI failure exposed a Projects assertion that compared
soft-wrap serialization instead of physical rows. Projects and Catalog now
share a complete physical-row snapshot wait; a deterministic regression
accepts wrap-metadata changes while rejecting text moved between rows. This
repairs the test harness without changing Projects behavior or the release.

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

The [duplication audit](duplication-audit.md) records shared creation execution,
capability choices/validation, and configuration edit sessions for Inquire and
Ratatui. Each interface retains its interaction and error recovery. Configuration
saving preserves unknown values and no-op bytes, supports cancellation, and
rejects external edits. Upgrade also retains its planned rewrite/input snapshots
and refuses external changes before writing. Planned replacements are staged
together, preserve permissions, and report recovery instructions on failure.
Jest's test/watch refreshes also use the shared save helper, preserving existing
backups and failed targets while allowing retries after file restrictions clear.
Continue the existing-workflow verification below;
smaller remaining duplication is documented rather than a new rewrite milestone.
Fresh optional choices now start unchecked; Scribe Studio is an optional
manual plugin entry and confirmed global Rokit adds use `--force`.

No new feature milestone is required first. Audit the baseline, fix concrete defects, and record evidence for workflows users already have.

Automated **saved-setup replay** now verifies Wally: saved choices, generated files and tool pins, no repeated choice prompts, an unchanged source setup, and temporary-file cleanup. Refusal checks now cover missing/malformed setups and retired workflow values, including explicit `--reconfigure`, with no project creation or fixture/config mutation. Local Jest pass/fail execution is covered; Open Cloud and fresh-machine provisioning remain separate gaps.

The October 2 live audit corrected stale Home navigation and saved-setup fixture
names, and reproduced a test-preparation defect: retyping an already processed
Wally tree could fail before the Jest runner. Testing now preserves a complete
package tree while regenerating its sourcemap; missing packages still take the
install/retype path. Focused live Jest pass/fail and setup-refusal checks passed.
After the GitHub API quota reset, all 14 live tests passed together on main
`144c35a`. Their captured output also exposed malformed negative-gate fixtures:
defects after a terminal return produced syntax errors. The corrected fixture
passed separately, now requiring actual type/lint/format diagnostics and rejecting
parse errors. See the dated audit for exact revisions and results; Open Cloud
and fresh-machine provisioning remain separate gaps.

October 3 verification ran the three ignored real-Rojo template tests on main
`845512a`. Built-in templates and guided compound values passed sourcemap/build
validation across all ten generated variants; an invalid property was refused.
This verifies Template Explorer's validation boundary, not a new interactive
editing session. See the dated audit for the command and remaining limits.

Template Explorer's isolated terminal harness now covers edited-draft
discard, byte/absence preservation, invalid-JSON recovery, and Windows locked
replacement retry. An explicit installed-Rojo run also exercises valid save,
upstream refusal and corrected-draft recovery across generated variants.
The tests use disposable persistence paths and check raw-mode restoration;
they do not modify the user's global template or expand Rojo's accepted fields.
See the dated audit for boundaries and evidence.

Terminal resize coverage now preserves an active JSON draft through narrow,
minimum and undersized screens. It reproduced and fixed an Exit confirmation
trap below 60 x 16: the discard dialog remains visible and usable while edits,
save and reset stay blocked. This fix shipped in 0.19.1.

Catalog resize regressions now cover search, non-default selection, list and
detail scrolling, Help and Back through wide, narrow, minimum and undersized
screens. The standalone terminal test restores the same scrolled details and
verifies scrolling still works afterward. This is test-only coverage; no new
release is needed. See the October 4 audit for evidence.

Projects now has a TestBackend resize regression preserving a typed filter,
non-default selection, scrolled details and Help through the same sizes.
It verifies restored detail contents and continued keyboard scrolling;
real-terminal coverage now also preserves a uniquely filtered launch project,
scrolled details and Help through those sizes, then verifies keyboard scrolling,
opening the selected project and returning Home. This is test-only
coverage and requires no new release. See the October 4 audit for evidence.

Saved Setups now has real-terminal resize coverage alongside its TestBackend
regression. Disposable setups exercise a typed filter, non-default selection,
scrolled list and composition details, Help through narrow/minimum/undersized
screens, restored detail contents, continued scrolling and Actions/Back.
Fixture bytes and raw-mode restoration are checked. This is test-only coverage;
published 0.19.1 remains unchanged. See the October 4 audit for evidence.

Saved Setup editor resize regressions now preserve a filtered capability
revision, checked choices, detail focus/scroll and Help, then save after recovery.
Undersized default-No and confirmed discard cover both Back and Home; discard
after a second revision retains the last successful save. The isolated terminal
driver mirrors Home's input gate and checks blocked small-screen edits/saves,
fixture bytes and raw-mode restoration. Home dispatch and other composition
steps remain outside this bounded coverage. No new release is needed.

New Project now has one TestBackend resize regression preserving a filtered,
checked package revision, detail focus/scroll and Help through 120 x 30,
80 x 24, 60 x 16, 40 x 10 and recovery. It restores the same screen, verifies
continued scrolling, applies the revision and reopens the retained package
choice. An isolated PTY regression now exercises production input gating,
blocked undersized edits/acceptance/paste, restored physical rows, continued
scrolling and accepted-choice reopening before cancellation. It checks
fixture bytes and raw-mode restoration without preparation, Home dispatch or
project execution. This is test-only coverage; no new release is needed.

A second TestBackend regression now follows a capability revision through
Capabilities, test implementation and Jest execution. Each screen preserves its
filter, selection, checked choices and detail focus/scroll through the same
resize sequence and Help. Cancellation restores the reviewed TestEZ composition;
acceptance retains Open Cloud on reopening. An isolated PTY now drives these
three screens through the same sizes using production input gating. It verifies
blocked undersized input, complete resized screens, continued scrolling,
whole-graph cancellation and retained execution choice, with unchanged fixture
bytes and terminal restoration. Home dispatch, preparation, confirmed creation
and hosted execution remain separate checks.

Strategy revisions now have TestBackend resize recovery for leaving Wally
through None and Testing repair. Filters, selection, detail focus/scroll, warning and graph
survive Help and the same resize sizes. Escape restores the complete reviewed
Open Cloud composition; acceptance retains TestEZ or disabled Testing while
preserving lint. No additional terminal test is planned for the retired strategy.

Template Explorer now captures the file it opens and refuses save/reset after
external changes, creation or deletion. Disposable storage and terminal tests
verify external bytes/absence and the draft survive refusal; a fresh snapshot
is recorded after successful saves. Installed-Rojo save/refusal/repair also
passed. This protection likewise shipped in 0.19.1. See the October 4
audit for evidence and the optimistic conflict-detection limit.

Open Cloud missing-credential handling now has executed regression coverage:
the generated CI guard rejects absent/empty credentials before the runner step,
and installed Jest 0.4.1 through `rproj test` names missing fields and preserves
exit code 2 for standard and prefixed environment variables. Synthetic partial
credentials and a loopback endpoint are used; authenticated cloud execution
remains unverified. See the October 3 audit.

### 1. Establish The Baseline

- Run formatting, ordinary tests, clippy, and locked packaging on a clean checkout.
- Verify Windows stable and Rust 1.89 CI, tag/release/package alignment, and outstanding CodeRabbit findings.
- Review ignored integration tests and architecture's manual-verification checklist. Historical passes are not evidence for the current candidate.
- Record checks as passed, failed, or blocked with commands, versions, and prerequisites. Missing Studio or Open Cloud access is not a passing test.

### 2. Verify Existing Workflows

| Area | Required evidence |
| --- | --- |
| Machine setup | Deliberate installation prompts, useful missing-tool errors, and safe reruns. Use an explicitly provisioned test environment; ordinary tests must not install applications. |
| Creation | Coherent minimal and Wally output and pins; saved setups and revisions preserve choices. |
| Template Explorer | Editing, validation, reset, repair, cancellation, and terminal restoration preserve the last valid template. |
| Daily use | Dependency recovery in Watch, correct runner dispatch, and documented Copy/Catalog behavior. |
| Upgrade/configure | Review and cancellation work; managed fields update without damaging source or unrelated configuration. |
| Checks/CI | Clean fixtures pass; deliberate lint/format/type/test failures fail. Open Cloud projects fail clearly when credentials are missing; Local Studio projects omit CI tests. Record live Studio/Open Cloud evidence separately. |
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

## Shipped: T3 Projects Browser (0.15.0)

T3 introduced Projects, New Project, Edit Project Template, Machine Setup, and Catalog on Home. Projects is read-only shallow discovery plus explicit-path action dispatch; it is not a registry or filesystem manager. The redundant Catalog Place Template section was removed. PR #22 passed local review and reviewed-head/main CI, and owner publication was verified on September 10, 2026. Its merged feature branch was deleted. T4 subsequently added Saved Setups to Home; see the release audit for evidence.

## Next Milestones

Complete upgrade review is implemented in [PR #96](https://github.com/chatarabdelilah/rproj/pull/96):
housekeeping changes now join the same preview, confirmation, snapshots, and
staged replacements as other generated files. Cancellation writes nothing and
an empty plan is a no-op; unparseable `.luaurc` files remain explicitly skipped.
All 28 upgrade tests, the ordinary suite (485 passed, 23 ignored), formatting,
strict Clippy, clean locked packaging, and both CodeRabbit reviews (zero
findings) pass, along with implementation-head CI. Final reviewed-head/main
verification is tracked in the PR and release audit; this runtime change is
unreleased after published 0.21.0.

**Shipped in 0.20.0 alpha: persistent background Watch v1 on Windows.** It supports
one project per user, manual startup, sourcemap Watch after rproj closes,
status/logs/stop controls, owned process-tree cleanup, and cooperative recovery
cancellation. It adds no Rojo Serve, sign-in task, automatic restart, or service.
Implementation and acceptance are recorded in [the release audit](release-audit.md);
owner publication, archive identity, annotated tag and GitHub alpha release
are verified at `f3cdea8`. [Release notes](release-notes-0.20.0.md) record limits.
The fresh-Windows environment inventory is complete: the current PC is already
configured, and no spare Windows machine or ready local VM is available. The
owner deferred Hyper-V/VM provisioning on October 6; do not resume it on an
ordinary request to continue. Fresh-machine acceptance remains unverified.
Authenticated Open Cloud execution likewise requires a suitable environment
and credentials.

Continue existing-workflow hardening without host provisioning. A bounded
missing-Cargo regression reproduced an unactionable Rokit bootstrap spawn error;
the correction names the Rust prerequisite and PATH recovery, while preserving
permission and Cargo build failures. Missing WinGet now similarly points to
App Installer, PATH verification and a new-terminal retry. Both use shared
NotFound-only recovery; installer output, hash diagnostics and permission
failures retain their existing handling. These fixes do not establish
fresh-machine acceptance.
**0.20.1 is published**, packaging these two diagnostic fixes and the Projects
physical-row assertion repair. Archive identity, immutable annotated tag and
GitHub alpha release match clean release commit `07634cc`.
See [release notes](release-notes-0.20.1.md) and [release preparation](releasing.md).
Template Explorer's PTY driver now isolates libtest's parallel timer output
from its active screen and waits for complete JSON text/cursor acknowledgments.
A deliberately slow paste reproduced the child's 60-second warning inside a
JSON row; the isolated child completed the same probe in 72.46 seconds. The
final helper retains one intact paste and all file-protection assertions;
timeouts and ordinary-suite parallelism are unchanged. This is a test-only
repair, requiring no new package release. Local validation is recorded in the
release audit; required CodeRabbit review and CI remain release-independent gates.
The editor repair merged in [PR #82](https://github.com/chatarabdelilah/rproj/pull/82)
with clean local review and first-attempt head/main CI. The October 7 follow-up
inventoried all nine unit-test PTY launch sites. Five already isolated libtest's
timer output; the remaining four now use the same single-thread child flag.
Existing Machine Setup, capability-prompt and Saved Setup behavior checks pass.
This closes the selected child-output audit; no new workflow matrix or package
release is required. Review and CI evidence belong in the follow-up PR.

The Watch investigation reproduced two acknowledgement failures: a valid delayed
reply exceeded the shared 300 ms status budget, and an aborted reply escaped as a
fatal supervisor error. State-changing acknowledgements now allow two seconds
for durable state writes; status, connection and send deadlines remain 300 ms.
Reply delivery failures no longer terminate the owned engine. Controlled
regressions and both installed-tool Watch acceptances pass; the historic CI
host's precise timing remains unmeasured, with phase-specific errors now available.
See the release audit for evidence and limits.

The Watch runtime repair merged in [PR #84](https://github.com/chatarabdelilah/rproj/pull/84)
at `481e33b`, with first-attempt final-head and main CI passing. The owner published
0.20.2 alpha after [PR #85](https://github.com/chatarabdelilah/rproj/pull/85)'s verified
release gates; registry/archive identity, annotated tag and GitHub alpha release
match `eaa0447`. Published versions stay immutable.

The bounded missing-Git creation audit reproduced a guidance defect: the
confirmed creation executor returned only `failed to spawn git: program not found`
and left an empty destination. The repair adds the official Git installation
link, PATH/new-terminal verification and a safe retry in a new destination.
The original OS error and permission/command failures remain intact; an absent
working directory is not misreported as missing Git. A disposable project and
child-only PATH regression passed after failing on the original behavior, with
parent PATH, configuration and existing fixture files unchanged. Existing Git
repositories still bypass initialization. See the release audit for scope and
evidence; the repair shipped in 0.20.3 alpha.

The repair merged in [PR #87](https://github.com/chatarabdelilah/rproj/pull/87) at
`bdd2ab6`, after zero findings in both local CodeRabbit reviews and first-attempt
final-head/main CI. The owner published 0.20.3 after [PR #88](https://github.com/chatarabdelilah/rproj/pull/88)'s
release gates passed; registry/archive identity, annotated tag and GitHub alpha
release match `2932862`. The Wally repair shipped in 0.20.4 after [PR #91](https://github.com/chatarabdelilah/rproj/pull/91)'s
release gates passed; registry/archive identity, annotated tag and GitHub alpha
release match `1a04668`. Git-submodule retirement shipped in 0.21.0 after
[PR #94](https://github.com/chatarabdelilah/rproj/pull/94)'s release gates passed;
registry/archive identity, annotated tag and GitHub alpha release match `07dcf17`.
No additional code audit or
feature is automatically queued. The next planning decision is to prioritize the
remaining workflow acceptance gaps using the coverage table above, current
evidence and available environments. Machine-wide provisioning remains deferred;
this targeted check does not close fresh-Windows installation acceptance.

The October 5 bounded New Project
confirmation/execution audit found no actionable defect and passed the three
existing live Home creation checks plus four ordinary boundary checks. That
audit is complete; see [the release audit](release-audit.md). The recent resize
coverage sequence is also complete for its selected scope. Additional matrices
are not an automatic development queue.

Current work is release hardening driven by concrete defects and acceptance
requirements. Authenticated Open Cloud execution and fresh-Windows provisioning
remain unverified and require suitable test environments. Beta requires current
critical-workflow evidence and contracts intended for stabilization; no beta or
1.0 date is committed.

Package additions and further bounded code audits remain **uncommitted backlog**.
Select them for a concrete use case or observed defect. The completed creation
audit does not close the broader code-audit backlog.

T4 shipped as **0.16.0 alpha** in [PR #24](https://github.com/chatarabdelilah/rproj/pull/24). The published archive, annotated tag, and GitHub prerelease identify `d54a9a8`. Existing saved setups now have their own Home manager; changes affect future reuse. New setup creation remains part of New Project. T5 also shipped as 0.17.0.

1. **T5 - Ratatui Machine Setup (0.17.0 shipped):** review-first selection, in-TUI progress/results, and cooperative stopping after the active item. The published archive, annotated tag, and GitHub alpha prerelease identify `9ee8d51`. [PR #28](https://github.com/chatarabdelilah/rproj/pull/28) records final-head and merged-main verification.
2. **Background Watch v1 (0.20.0 shipped):** archive, annotated tag and GitHub alpha release identify `f3cdea8`. Fresh-Windows and authenticated Open Cloud acceptance remain unverified. Package additions and further audits remain separate backlog. Model import remains permanently dropped.

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
| T3: Projects browser, selected-path commands, creation handoff | v0.15.0 |
| T4: Saved Setup manager, preservation-aware editing and file operations | v0.16.0 |

**Active sequence: observe real workflows -> select one bounded improvement -> review, release, and align it.**
