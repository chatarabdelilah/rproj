# Native Project Tool Setup implementation

Approved November 17, 2026 against clean main `bd1782a`. Published version
remains 0.22.0. The approved Soft design is Tool → Review → Results, one
captured project/tool per review, with wait-for-completion closing.

## Delivery sequence

1. Shared read-only planning and checked single-tool execution, preserving
   existing pins/files and reporting required tool failures accurately.
2. Versioned private worker/client operations, bounded atomic reports and
   owned process-tree completion without a cancellation action.
3. Native Soft controls, retained ownership and default-Keep confirmations,
   Activity integration and close-after-completion protections.

Existing CLI ancestor discovery remains available; native requests capture an
explicit project. Setup requires a Rojo project file and actual prerequisites,
not an rproj record or a completed global setup flag. Shared batch setup
semantics remain outside the single-tool failure correction.

Preparation captures relevant existing and missing paths without writes.
Execution revalidates sources, prerequisites and Watch ownership. Existing
pins/configuration stay intact; missing text configuration uses the existing
generators. External trust/cache/download operations and partial effects must
be reported explicitly. No rollback is promised.

## Evidence

The pre-extraction CLI fixtures reproduced success after failed tool addition
and unnecessary external calls for an existing custom pin. The original
missing-manifest/config fixture passed. The initial sandboxed run was blocked
by Windows access denial before reaching setup; its results were not counted
as defect reproductions. Running the disposable suite with access to the Watch
mutex reproduced both defects. The new checked executor passes all four CLI
fixtures, including misleading failure output, missing result pins, failed
trust/init and unexpected changes to other pins. Generator text remains exact.

Rokit [Add](https://github.com/rojo-rbx/rokit/blob/main/src/cli/add.rs) documents
that `--force` also overwrites existing pins. This path therefore invokes
[Trust](https://github.com/rojo-rbx/rokit/blob/main/src/cli/trust.rs) separately,
revalidates the review, then adds without force using an explicit executable
alias. Existing pins never invoke an installer. Source/output checks are
optimistic checks, not a transaction against arbitrary external writers.

Shared implementation was delivered from `codex/project-tool-setup-shared`. The
first full CLI/core run passed 830 tests with 25 prerequisite-dependent cases ignored;
165 headless desktop tests passed. Strict CLI/core and desktop Clippy, formatting,
portable archive/static-runtime checks and matching-CLI smoke checks passed.
The archive preserves notices for 590 packages; three existing legacy archive
notice-completeness gaps remain. Native previews, real installation and real-window acceptance remain
paused/unverified because native memory stability is unresolved.

CodeRabbit 0.7.6 reviewed eleven files and raised two questions. Upstream
[Rojo accepts JSONC](https://github.com/rojo-rbx/rojo/blob/master/src/json.rs),
so the newly introduced strict JSON parse was removed: tool setup captures and
preserves the readable project file, but does not certify its contents or require
Rojo installation. A comments/trailing-comma regression covers this boundary.
`Reporter::run` already rejects nonzero exits; an additional fake-Rokit case now
covers failure after a valid pin was written, without losing the failed outcome.
The final CodeRabbit rerun reviewed the same eleven files with zero findings.
Focused reruns passed four CLI fixtures and thirteen setup unit tests (one
Windows symlink-prerequisite case ignored); strict CLI/core Clippy passed again.
Shared [PR #156](https://github.com/chatarabdelilah/rproj/pull/156) merged at
`c402613`. Reviewed head `c282515` passed all seven executed jobs in
[head CI 44010750269](https://github.com/chatarabdelilah/rproj/actions/runs/44010750269);
merged main passed all seven in
[main CI 44011471802](https://github.com/chatarabdelilah/rproj/actions/runs/44011471802).
Both were first attempts; badge freshness was intentionally skipped. The final
portable archive was rebuilt after the JSONC correction and passed again.
The merged local/remote branch is removed. Private worker implementation now
continues on `codex/project-tool-setup-worker`; native controls remain pending.

Each PR requires applicable formatting/tests/Clippy, portable packaging checks,
local CodeRabbit review, final diff inspection, reviewed-head and merged-main
CI, and merged-branch cleanup. No version bump, publication or installation.

## Private worker

The private `project-tool-support`/`project-tool-job` handoff adds preparation
and confirmed execution through the matching bundled CLI. Preparation supplies
the captured project/tool context, prerequisites and complete reviewed effects;
it creates no project or machine configuration. Unsupported workers leave other
verbs unchanged. Requests and atomic reports retain the 2 MiB limit.

The client shares the existing serialized-operation guard and owned Windows
process-tree execution. Setup exposes no cancellation token; the client waits
for confirmed exit and pipe drain before interpreting the report. Reports must
match operation/project/tool identity, reviewed step labels, expected step
states, effects and exit status. Logs cannot establish success. Missing or
contradictory reports yield uncertainty; failures before spawn remain
recoverable. Teardown errors retain their existing typed close-blocking signal.

Known tool failures retain individual completed/failed/skipped file/tool results
and possible effects. Any failure outside those confirmed step results after an
effect callback becomes uncertain. Retry requires a renewed review; no rollback
is promised. Worker tests use compiled disposable stand-ins and isolated Watch
state, never real installations or native windows. The full CLI/core run passed
846 tests (25 prerequisite-dependent cases ignored), including twelve worker
tests and three new protocol checks. All 165 headless desktop tests passed;
strict CLI/core/desktop Clippy and formatting passed. Portable packaging and
matching-worker smoke checks passed without launching a native window.

The first worker CodeRabbit review raised three findings. The process-cleanup
test now requires a successful `tasklist` query before accepting its observation.
The two questions were checked against existing boundaries: all errors after
owned spawn carry `OwnedTeardownUnconfirmed`, including wait/drain errors, and
the worker test module is explicitly Windows-only. Neither requires a runtime
change. The final CodeRabbit review covered eleven files with zero findings.
The assertion initially needed rustfmt; after formatting, all twelve worker
tests and strict CLI/core Clippy passed again. [Worker PR #157](https://github.com/chatarabdelilah/rproj/pull/157)
merged at `e176f73`. Reviewed head `bbdb7d4` passed all seven executed jobs in
[head CI 44069122534](https://github.com/chatarabdelilah/rproj/actions/runs/44069122534);
merged main passed all seven in
[main CI 44069844940](https://github.com/chatarabdelilah/rproj/actions/runs/44069844940).
Both were first attempts; badge freshness was intentionally skipped. The merged
tree matched the reviewed branch, which was removed locally and remotely.

## Native controls

`codex/project-tool-setup-native` adds Tool → Review → Results to Tools, beside
Configure tool settings. The switcher can resume the captured review after
project navigation or closure. Catalog search distinguishes project executables,
Blender scene creation and guidance-only machine entries. Machine Setup is an
explicit handoff. Review displays the checked planner's ordered effects and
complete new text-file contents; existing pins/files and binary scenes retain
their separate explanations.

One session-only review owns its project/tool throughout preparation and setup.
Replacement and confirmation default to Keep; Watch ownership is checked again
at confirmation and in the executor. Setup shares the serialized operation lane,
has no cancellation action, and waits for owned completion. Results retain
individual tool/file outcomes; retry requires explicit review. Close after setup
waits for confirmed completion and then honors other dirty drafts. Reopening the
close decision withdraws an earlier close request while the new choice is pending.

All 176 headless desktop tests and strict desktop Clippy pass. Eleven new checks
cover layouts/themes, retained ownership, unavailable workers, prerequisite
errors, stale events/confirmations, Activity output/height, Watch guards and
close/completion races. The existing all-draft navigation audit now includes
Project Tool Setup. Early compile attempts exposed incorrect assumptions about
pinned GPUI imports/test APIs; these were corrected against existing controls.
The first layout checks needed visible filtered choices and explicit scrolling
to below-fold content. Focus observation must precede `track_focus`. The
disabled-action check exercises activation refusal because the pinned test
snapshot does not expose that button's disabled accessibility flag.

The November 17 validation attempt after the last close-race edits did not run:
automatic approval review exhausted its usage allowance. Validation resumed on
November 18. The full CLI/core suite passed 846 tests with 25 ignored prerequisites;
the nested Watch fixture driver's result is not double-counted. Strict CLI/core
Clippy, formatting and portable packaging/static-runtime/matching-worker smoke
checks passed. The archive preserves 590 dependency notice sets; the same three
legacy notice-completeness gaps remain. CodeRabbit reviewed all 22 changed files
and raised one question about the global teardown guard. The guard in
`workbench/upgrade/controller.rs` already includes `project_tool.teardown_uncertain`;
`uncertain_completion_and_teardown_keep_closing_blocked` verifies that it blocks
closing. This refutes the concern without a runtime change. The final evidence-only
documentation update was inspected directly. [Native PR #158](https://github.com/chatarabdelilah/rproj/pull/158)
is the delivery ledger for actual reviewed-head and merged-main CI, attempts and
branch cleanup. Initial commit `776a8e8` preceded this evidence-link update;
its CI does not substitute for final-head verification.
No native preview, real installer or platform-browser acceptance is claimed.
