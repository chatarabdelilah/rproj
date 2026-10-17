# Existing-project composition worker

Private protocol 1 extends the matching bundled CLI with `composition-context`
and `composition-job`; public commands, project formats, inspection JSON and
desktop preference schemas are unchanged. Existing worker verbs are unchanged.
Native controls follow in the third implementation PR; this document records
worker behavior, not real-window acceptance.

## Capture and preparation

Context captures the canonical project, name, exact record source, graph and
relevant file identities. Missing/malformed records and unsupported recorded
choices remain explicit problems; nothing is adopted or normalized. Unknown
TOML metadata remains in the original record. Context requires no global setup
completion flag or configured new-project root/template.

Preparation is an owned serialized operation. It checks the captured context,
validates additive choices and uses the shared executor's temporary projection
to produce actual planned file actions/reasons and external operations. Project
and machine configuration stay unchanged. Missing prerequisite executables give
the existing `rproj setup` handoff and Recheck instruction; no installer or
credential UI is introduced. Changed inputs require explicit reload, not merging.

Relevant inputs include absent targets, declaration/configuration files, lock
resolution input, asset directory presence and the existing Blender scene.
Source trees and installed package trees are not copied or rewritten by the
planner. The directory creation identity detects replacement of the captured
root where the filesystem exposes creation time. Source fingerprints detect
accidental edits; they are not an adversarial authentication mechanism.

## Apply and cancellation

Apply rederives and compares the reviewed plan before the first write, checks
prerequisites and reuses the existing Watch mutation guard. Same-project Watch
blocks mutation; another project's Watch is never stopped or retargeted. Existing
tool pins stay intact; only missing selected tool pins are added. Download/trust
operations are visible in Review. Required tool failure stops execution.

An operation-owned temporary directory carries a bounded request and atomically
replaced report (2 MiB each). The report correlates protocol, operation ID and
project, and records NotStarted, MayHaveBegun, Completed or Unknown. This is an
existing directory, so no directory-claim assertion is made. MayHaveBegun is
written before staging starts. `rproj.toml` is atomically committed last, after
required file/tool steps succeed. Other files may remain partially changed;
there is no project-wide rollback promise.

Human stdout/stderr feeds bounded Activity output; successful tool output is
retained too. Logs never establish success. Worker exit and its structured
report must agree. Missing, malformed, mismatched or contradictory reports yield
an uncertain Apply outcome with unknown file state. Preparation never requests
project writes and cannot report a mutation phase.

Cancellation remains pending until the existing owned process-tree runner has
terminated descendants and drained both streams. A valid completed report
survives cancellation racing completion. Interrupted reports preserve a known
mutation boundary when possible; otherwise file state is unknown. Failure to
confirm teardown remains an error, distinct from uncertain file state after
confirmed process teardown. Session drafts and reports are not preferences.

## Automated evidence

Eleven Windows worker fixtures cover read-only preparation, successful structured
Apply and stdout/stderr, stale drafts/reviews, partial tool failure, absent and
malformed reports, descendant cancellation, cancellation before launch,
unsupported workers, malformed/missing/unsupported records, bounded/non-UTF-8
damaged records and cancellation racing a completed report. Fixtures use
disposable Unicode/spaced paths, isolated Watch storage and a Rokit stub. Two
core protocol tests cover correlation, invalid mutation states and size bounds.
These are automated process tests, not actual external-tool or native-window
acceptance. Native preview launches and memory-stability acceptance remain paused.
