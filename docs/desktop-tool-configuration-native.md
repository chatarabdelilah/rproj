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

Private worker and native controls are subsequent PRs. This shared checkpoint
does not claim native implementation, rendering, memory stability, Windows
real-window/scaling acceptance or macOS/Linux desktop acceptance.
