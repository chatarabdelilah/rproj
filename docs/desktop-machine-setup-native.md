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
CI identities will be recorded when this first PR is delivered. Worker and
native controls are not yet implemented by this branch.

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

Real installers/UAC, native rendering, memory stability, assistive technology,
and macOS/Linux desktop acceptance remain unverified. Installer services outside
an owned process tree must not be described as owned or safely terminated.
