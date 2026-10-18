# Private Upgrade worker

The matching bundled CLI exposes private `upgrade-support`, `upgrade-context` and
`upgrade-job` operations. Protocol version 1 is an internal desktop handoff, not
a public command or a persisted project/preference format. Unsupported workers
disable Upgrade review; other workbench actions retain their existing protocol.

Context identifies the canonical directory and its identity. Preparation runs in
an owned worker off the UI thread and returns the shared captured review. Every
read and missing target is rechecked before returning it. Preparation never writes
the project or machine configuration. No completed global setup flag is required.
The existing Wally CI pin check applies only where relevant; Upgrade downloads or
installs nothing. Oversized reviews fail clearly before writes, directing users
to the terminal; neither diffs nor replacement contents are silently truncated.

Apply uses the same authoritative checked executor as terminal Upgrade. It
revalidates the root, inputs, reviewed generated output and Watch ownership.
Changed inputs require explicit Refresh review, not an automatic replan-and-apply.
Writes may begin before replacement because staging can create parent folders.
Per-file replacement preserves existing permissions; no project-wide rollback is
promised. A replacement failure reports the earlier updated-file count.

Requests/reports use operation-owned temporary directories and atomic replacement,
with the existing 2 MiB per-message limit. Reports correlate protocol, operation
and project and distinguish NotStarted, MayHaveBegun, Completed and Unknown writes.
Human stdout/stderr is bounded Activity output, never a success protocol. Missing,
invalid or mismatched terminal reports cannot establish success or unchanged
files. Confirmed terminal success wins a late cancellation request.

Cancellation remains pending until the owned tree exits and output streams drain.
Failure before spawning is recoverable with NotStarted writes. Failure to confirm
owned teardown remains a distinct error that prevents closing; an uncertain file
report after confirmed teardown does not imply living descendants.

## Automated evidence

Nine Windows behavior fixtures plus their subprocess driver cover read-only
preparation/success, stale/missing targets and tampered contents, absent/malformed/
mismatched reports, unsupported workers, malformed records, Watch conflicts,
post-write failure, cancellation/descendant cleanup, late confirmed completion,
pre-cancellation/missing workers and oversized preparation. Paths contain spaces
and Unicode. Watch fixtures hold disposable owner locks, not real Watch engines;
the post-write report-failure fixture modifies a completed worker outcome to test
report interpretation. A separate executor fixture injects a failure after one
actual replacement and verifies remaining-change recovery and record preservation.
Two new core protocol tests cover correlation and invalid state combinations.

Native controls and real-window acceptance are not established here. Native
previews remain paused; the memory-stability issue remains unresolved. No version
bump or publication belongs to this worker milestone.
