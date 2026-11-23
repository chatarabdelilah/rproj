# Headless desktop creation worker

The approved New Project flow uses shared composition and an explicitly located,
version-matched CLI. The native controls are a separate delivery. Existing
Check/Test/Watch/catalog verbs and public CLI commands retain their behavior.

## Private handoff

`creation-context` returns protocol-1 context: configured absolute projects root,
machine readiness, template state, saved compositions and source snapshots.
Malformed/unsupported setups remain visible, with no replay normalization. Missing
storage is distinct from unreadable storage. Native parse diagnostics omit raw
TOML values. Source bytes remain transient and are not desktop preferences or logs.

`creation-job <directory>` reads a typed Prepare or Execute request from an
operation-owned temporary directory. Requests and atomic reports are bounded to
2 MiB and correlated by operation identity. Human stdout/stderr remain output;
they are never parsed for success. Context responses and saved-setup snapshots
also have explicit size limits; oversized data is an error, not a truncated list.
These are internal distribution interfaces, not a stable SDK or configuration
schema. A worker without creation support leaves existing workbench features usable.

## Preparation and execution

Preparation checks recorded machine setup, names, supported composition, template,
destination and new setup-name availability. It compares relevant config/template
and selected-setup bytes, derives the review plan and validates custom templates
through existing Rojo code in temporary workspaces. It creates no project folder
and changes no user configuration. A relative configured root is an actionable
error rather than being resolved against a worker's arbitrary current directory.

Execution repeats validation and compares the entire reviewed plan. It reuses the
existing executor: Blender preflight precedes exclusive directory claim; Git,
scaffolding, tool/dependency operations and optional new setup save follow the
reviewed choices. Native saves use server-derived storage paths; terminal entry
points retain their existing behavior. New setup saves never replace existing
content, including a collision arriving after review or directory claim.

The report records Claiming before exclusive creation and Claimed after it. A
nonexistent destination is compared with Watch through its existing canonical
ancestor without creating directories. The creation guard leaves unrelated Watch
ownership/session data unchanged and does not weaken the existing-operation guard.

Windows preparation/execution share the existing operation mutex and owned Job
Object. Cancellation remains pending until process-tree teardown and output drain.
Preparation cancellation has a typed outcome. Execution retains its destination,
result and claim state. A missing/mismatched report after cancellation means
unknown ownership and possible partial files. A zero exit without a trustworthy,
consistent terminal report cannot mean success. Teardown errors remain errors so
the later close guard can keep the app open. No rollback or overwrite is attempted.

## Evidence and acceptance boundary

The CLI/core suite passes 589 tests with 24 prerequisite-dependent tests ignored;
strict Clippy passes. Focused coverage includes no-write preparation, stale
config/template/selected-setup input, Unicode/spaced paths, directory reuse,
late setup-name collisions, malformed reports, unsupported workers, cancellation
before/after claim and descendant-pipe teardown. A responsive Watch fixture retains
its owner lock, Watching state and exact session bytes during creation guarding.

Worker tests inject storage into the production helpers in a test-build child.
They do not rely on Windows profile environment overrides or touch owner storage.
A fixture Git executable supplies success/failure/cancellation gates while the
real scaffold/executor writes disposable files. These prove worker behavior, not
real Git/Wally/Roblox or native-window acceptance. Desktop controls, manual
creation/close review and release/distribution acceptance remain separate gates.

No version bump, publication, project-file migration or desktop-preference change.
