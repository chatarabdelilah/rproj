# Native Machine Setup implementation

Approved October 24, 2026 against main `a56533a`. The owner approved three
sequential reviewed PRs: shared planning/execution, a private cooperative-Stop
worker, then the Soft Selections → Review → Results controls. Version 0.22.0
remains unchanged. Native previews and real installation acceptance stay paused.

## Shared planning and checked execution — first PR

The authoritative 36-entry catalog, six categories, selection rules, ordered
queue and typed results now live in core. CLI re-exports retain terminal callers.
Fixtures established before extraction freeze catalog descriptions/sources/docs,
all-selected execution order and saved unknown/inactive choice preservation.

Preparation captures configuration bytes, missing paths and file/directory
identities without creating directories. The projects folder is read-only;
unfamiliar recorded choices are retained without executing them. Saved choices
are separate from detected presence. Inconclusive detection says Not verified.

Execution holds a machine-setup lock and the existing global Watch mutation
guard. The authoritative configuration and reviewed queue are revalidated before
external operations; the projects-folder identity is checked again at its step.
Rokit/folder failures are fatal; other failures/manual steps retain warning
results. Existing installers and tool boundaries remain in the CLI.

Stop finishes the current item, skips remaining items and is checked before
persistence. Completed runs, including warnings, atomically save selections after
revalidating the configuration. Installation outcomes and configuration-save
failure are separate. Once persistence is confirmed, a late Stop cannot rewrite
completion. Completed installations can remain after any failure or Stop; there
is no rollback promise or protection against arbitrary external writers beyond
the documented identity/content checks.

The terminal consumes the checked worker's saved baseline; it no longer writes
configuration after interpreting a possibly late Stop request. Existing terminal
interaction tests remain in the regression suite. R explicitly reloads saved
selections with a default-No confirmation, allowing a fresh review after a stale
or completed run without silently rebasing edits. Opening the screen performs
no installation probes; detected presence remains Not verified until checked.

## Validation and delivery

Tests use disposable paths and fake installers. No native preview or real
installation has been launched. Validation and exact reviewed-head/merged-main
CI identities are recorded below. Native controls remain the third PR.

Initial attempts: the extraction script had a syntax error before any writes;
the corrected extraction preserved all three fixtures. Early test adaptation
missed a typed completion event; it was corrected. Restricted-sandbox directory
replacement and atomic saves returned Access denied; all 30 initial setup tests
passed with the required filesystem access. Clippy found the catalog Maintenance
re-export was only needed by test code; it is retained under `cfg(test)`.
The full CLI/core regression suite and all 83 headless desktop tests then passed.
Final CLI/core validation passes 702 tests, with 24 prerequisite-dependent tests
ignored; the desktop suite passes 83 headless tests. Strict CLI/core and desktop
Clippy pass. Additional tests cover Watch blocking, linked paths, the Stop/save
boundary, explicit reload/retry, and thread-local Ctrl+C deferral during an item.

The first complete CodeRabbit review found two issues: unused synchronous
detection delayed terminal startup, and a stale review had no in-session reload.
Both are fixed; final local review reports zero findings across all 23 changed
files. One reload PTY assertion initially checked a footer still visible beneath
the confirmation; it now waits for actual dialog dismissal. An intermediate
Clippy run required moving a test module after production items. Subsequent
full tests and strict Clippy passed. Fresh final crate archives and the corrected portable package pass their build
and non-window smoke checks. Three pre-existing dependency-notice gaps remain.
Exact reviewed-head and merged-main CI identities belong to the delivery PR.

PR #132 merged at `921f0ca550728d5f19d8eeb6369e66d605714335`. Reviewed head
`16539987ce51a1ea02a11dbea9dc46cee3c0f772` (run `40145034639`) and merged
main (run `40145395391`) passed all seven executed CI jobs on their first
attempts. Badge freshness was intentionally skipped. The merged tree matches
the reviewed tree; both merged branches were removed.

## Private cooperative worker — second PR

Versioned `setup-support` and `setup-job` operations expose read-only context,
preparation and checked Apply through the matching CLI. Public commands and
configuration formats remain unchanged. Preparation verifies the actual machine
configuration and prerequisites off the UI thread; it creates no destination.
Apply revalidates those inputs and acquires the existing any-Watch mutation guard.

An operation-owned directory holds bounded, atomic request/progress/result and
Stop messages. Reports identify the machine, operation, ordered item states,
possible effects, persistence and final outcome. Unknown or contradictory
completion is uncertain, regardless of stdout. The 2 MiB limit remains intact.
Successful saved contents are checked against the reviewed selections.

Stop has a separate cooperative token. The worker acknowledges it while an item
is running, then finishes that item, skips the remainder and avoids saving.
Confirmed persistence wins a late Stop race. The Windows owner waits on one
retained job-completion future and drains both streams; it never converts normal
Stop into process termination. Unconfirmed teardown retains operation storage
and is a distinct error for the native close/mutation guard. External services
or elevated installers outside the owned job remain unverified.

Initial worker validation exposed transient Windows Access denied while sampled
reports were replaced. A bounded retry applies only to transient IPC sharing
errors; a held-reader regression covers it. A separate Watch test compared
verbatim-prefixed and ordinary Windows fixture paths; canonicalizing the test
driver root corrected the fixture without weakening production identity checks.
The first formatting attempt also preceded creation of its test module; the
missing fixture was added before validation. No failures were marked as passes.

The first CodeRabbit review found that stdout/stderr disconnection could panic
the active worker. Logging now ignores pipe write failures; reports remain the
authority. An additional failure-path test exposed Windows blocking pipe reads
delaying runtime destruction after failed Stop delivery. The error path now
shuts down the runtime in the background, retains IPC storage and reports
unconfirmed teardown without killing the installer. The finite fake installer
then finishes normally and writes its terminal report. Its first test compile
used a nonexistent logging enum name; that fixture typo was corrected. Clippy
also required boxing the larger progress-event payload.

Final local validation passes 720 CLI/core tests (24 prerequisite-dependent
tests ignored), all 83 headless desktop tests, formatting and strict CLI/core/
desktop Clippy. Crate archive verification and portable version/inspection smoke
checks pass; three pre-existing notice gaps remain. The final source diff was
inspected directly. CodeRabbit's second review has no code findings; its single
minor documentation finding incorrectly identifies the creation PRs #113–115 as
Machine Setup delivery. Git history confirms #132 is shared Machine Setup, so
that finding is dismissed. Exact reviewed-head and merged-main CI belong to
this PR's delivery record. Native controls and real-window acceptance remain
pending; no preview or real installer is used in this PR.

Real installers/UAC, native rendering, memory stability, assistive technology,
and macOS/Linux desktop acceptance remain unverified. Installer services outside
an owned process tree must not be described as owned or safely terminated.

PR #133 merged at `a26b91666fdc0f29bf2189dbdab60e13b43473c1`. Reviewed head
`3d5e4e5fe78919e8a89eb4a230966c7eb7ff3a30` (run `40226382180`) and merged
main (run `40226736261`) passed all seven executed CI jobs on their first
attempts. Badge freshness was intentionally skipped. Merged and reviewed trees
match; both merged branches were removed.

## Native Soft controls — third PR

The app-level Machine Setup action and New Project prerequisite shortcut open
Selections → Review → Results. All six catalog categories support search,
details, sources and documentation. Saved selections, pending choices and
unverified installation presence remain distinct. The projects folder is
read-only. Unknown choices are retained without execution; inactive add-ons or
extensions remain selected but excluded until their parent app is selected.
List/detail panes stack below 600 logical pixels of main-content width.

The machine draft is session-only and independent of project browsing, closure
and other drafts. Review shows the authoritative ordered queue and effects.
Apply defaults to Keep reviewing and rechecks review identity and any observed
Watch. Stop Watch is separate. Unsupported workers disable setup operations
with an explanation while other workbench functions remain available.

Machine Setup owns its Activity output and typed results. Progress uses bounded
delivery; the reliable final report still expands Activity for a new failure
whose intermediate progress was dropped. Normal progress and success preserve
manual collapse and panel height. Installation outcome and selection persistence
are separate; stopped/fatal runs do not claim saved selections. Explicit retry
reloads configuration and its read-only root while retaining proposed software,
then prepares a new review; another explicit Apply is always required. Changed
unknown saved choices block that retry until saved selections are reloaded.

Stop has its own confirmed action and cooperative token, never the generic
process-killing cancellation path. Close defaults to Keep open; Stop and close
waits for the active item, owned descendants and streams before checking other
dirty drafts. Completion while a dialog is open cannot overwrite a saved result
with a late Stop. Unconfirmed teardown blocks closing and further mutations.

Automated fixtures launch no native windows or real installers. Initial test
compilation exposed an imported test macro conflict and incorrect test helper
imports; explicit imports fixed both. A resize test tried clicking the clipped
part of a scrollable control; centering it in the viewport fixed the fixture.
Clippy required boxing the terminal report event. A new category test used a
borrowed identifier where GPUI requires an owned/static identifier; it now uses
SharedString. Keyboard tests explicitly establish Tab focus before activation.
These failed attempts are not acceptance passes. The first CodeRabbit review
reported zero findings; final reviewed-head/main CI belongs to the delivery PR.

Final automated behavior checks pass **100 headless desktop tests** and **720
CLI/core tests**, with **24 prerequisite-dependent tests ignored**. Formatting
and strict desktop/CLI/core Clippy pass. Headless checks cover all categories,
keyboard selection and search retention, minimum/default layouts, long Unicode
roots and expanded Activity in Light/Dark, retry read failure without losing
choices, new failures in reliable results, and completion while Stop is open.
These checks do not establish native rendering or real-installer acceptance.

Direct final inspection also identified a recovery edge: after a retry's context
read failed, retained proposed choices lacked a baseline for the dirty check.
Such choices now remain dirty and protected by the close/discard confirmation.
The regression clears the old context before injecting that read failure.
Pre-spawn failure is described as not started rather than uncertain installation.
Fresh crate archives and portable version/inspection smoke checks pass; three
pre-existing dependency-notice gaps remain. No version or publication changes.
Final CodeRabbit review reports zero findings across all 18 changed files.
The portable archive was rebuilt after the recovery fix and its non-window
version/inspection smoke checks passed again. Final diff inspection and formatting
passed; reviewed-head/main CI identities will be retained in the delivery PR.
