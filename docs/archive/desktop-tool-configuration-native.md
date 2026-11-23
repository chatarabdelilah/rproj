# Native Configure Tools implementation

Approved October 20 against `3ccade4`. Delivery uses three sequential reviewed
PRs: shared model/execution, private worker, then native controls. The published
baseline remains 0.22.0. Native launches and real-window acceptance remain paused.

## Shared settings and execution

The four-target catalog and typed, UI-independent drafts live in core. CLI
re-exports preserve existing callers and generators. Filesystem reads, existing
TOML/JSON writers and replacement remain in the CLI. Missing settings differ from
explicit null/custom values; only explicit, correctly typed choices are staged.
No-op and reverted drafts preserve bytes and create no files.

Read-only preparation captures the canonical project, directory/file identities,
missing paths, original bytes and exact proposed output. Windows identity uses
volume/file indices; Unix uses device/inode. Links and Windows reparse points
are refused before canonicalization and at revalidation. A reviewed target cannot
be redirected through request-supplied replacement paths.

The shared executor holds the Watch mutation guard, compares against authoritative
preparation, records possible mutation before staging, and rechecks sources before
replacement. Missing `.vscode` is claimed exclusively. Staging preserves existing
permissions and writes a complete sibling file before atomic replacement. These
are optimistic checks; unrelated external writers do not share a filesystem lock.
Failure can leave a newly created directory, so no project-wide rollback is promised.

Every edited value must survive the writer. A legacy field removed because its
replacement exists blocks preparation and explains how to revert that choice.
Unrelated choices remain editable. Modernizing the catalog is deferred. JSON
formatting/cleanup remains visible in Review; after successful saving the baseline
is parsed from the actual saved output, including removed legacy values.

## Evidence and remaining delivery

Regression fixtures cover the existing writer bytes before extraction, all four
targets without setup/tools/records, custom/null values, exact integers, stale
sources and missing parents, identical-byte file replacement, replaced directories,
Windows junctions, forged reviews, legacy-key refusal/recovery, malformed/unsafe
inputs, Watch conflicts, locked/read-only files and failed replacement recovery.
Final command results and review/CI identities belong to the delivery PR.

Shared execution merged in PR #128 at `95bf7b2`. Reviewed head `b4b116d`
(CI `39348443855`) and merged main (CI `39386051451`) passed all seven executed
jobs on their first attempts. Local CodeRabbit reported zero final findings.
The merged local/remote branches are removed.

## Private settings worker

Versioned support, Context, Prepare and Save operations use the matching bundled
CLI. Context accepts a regular project directory without a record, setup flag or
installed tool. It loads the selected catalog target only. Preparation remains
read-only; Save reuses the shared checked executor and Watch guard.

Requests and atomic reports retain the operation, canonical project and target.
The 2 MiB per-message limit applies before execution and to complete responses;
oversized reviews fail rather than truncating changes. Explicit JSON null survives
IPC as a present custom value, distinct from an absent setting. Worker-provided
output is re-derived authoritatively before writing.

Save has no caller cancellation token. The owned Windows job must finish and
drain its streams before interpreting the report. Pre-spawn failure reports no
writes; missing, invalid or mismatched completion reports report uncertainty.
Unconfirmed teardown remains a distinct error that must block closing and further
mutations. Successful reports carry the exact saved bytes and parsed baseline.
Logs remain output, never evidence of successful saving.

Worker tests exercise Unicode/spaced paths, absent records/tools/setup, explicit
null, stale/tampered requests, Watch ownership, malformed/missing/mismatched
reports, contradictory process exit, size bounds and descendant cleanup. A
completion-loss fixture deliberately replaces a successful report after actual
saving and confirms that this becomes uncertain. A separate integration test uses
the real bundled CLI for read-only preparation in a disposable directory. Save
fixtures isolate Watch storage through the test bridge.

Private worker PR #129 merged at `e680b77`. Reviewed head `16e7858`
(CI `39388513452`) and merged main (CI `39443772804`) passed all seven executed
jobs on their first attempts; merged branches are removed.

## Native controls — third PR

Tools now offers Configure tool settings for recorded and unrecorded folders.
The four catalog targets use explicit typed choices, exact signed-integer input,
per-field Revert and default-Keep consent for unfamiliar values. Current values,
suggested defaults and pending edits stay separate. The worker supplies blocked
field explanations by probing the existing writer against captured bytes, without
filesystem writes or a second legacy-key policy. This extends the unreleased
private settings protocol to version 2; older workers disable only this editor.

A session-only draft retains its original owner across browsing and closure.
Creation/composition drafts and the Upgrade review remain independent. Preparation
runs off-thread; editing invalidates Review. Missing/empty files, actual writer
formatting and cleanup appear in the complete wrapped diff. Settings and Upgrade
share viewport rendering, retaining all rows while measuring visible rows only.
Targets stack below 600 logical pixels of main-content width. Long paths remain
horizontally accessible; fields scroll at the minimum window size.

Named Save confirmation defaults to Keep reviewing and rechecks review/Watch
identity. Same-project Watch blocks Save, with Stop separate; other Watch ownership
is unchanged. Save has no Cancel action. Captured Activity output, height, filtering
and manual collapse remain intact. Success advances the parsed baseline; failure
keeps edits; uncertainty requires explicit reload. Close after save waits for
confirmed completion and then protects every remaining dirty draft. Unconfirmed
teardown blocks closing and mutations. Dismissals restore workbench/flow focus.

Local validation: 676 CLI/core tests passed, 24 ignored; 83 headless desktop tests
passed. Formatting, strict CLI/core/desktop Clippy, fresh archive verification and portable
non-window smoke checks pass; final local CodeRabbit reports zero findings.
Actual reviewed-head/main CI identities are recorded on the delivery PR. No native previews were launched. Headless evidence
does not establish native memory stability, real-window rendering/scaling,
assistive-technology acceptance, or macOS/Linux desktop acceptance.
