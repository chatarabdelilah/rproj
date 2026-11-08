# Copy Source review design

Approved design scope, November 8, 2026, against main `64948d9`. This milestone
delivers a separate Soft conversation proposal, not native implementation.
Published version remains 0.22.0; native previews remain paused.

## Settled behavior

- Overview offers **Copy source…**. Review captures the project and recursive
  `src` scope, included/skipped paths and reasons, UTF-8 bytes and complete text.
  File navigation does not change the payload. Panes sit adjacent at 600 logical
  pixels of main-content width and stack below it.
- Copy submits the displayed snapshot. Explicit Refresh replaces it only after
  successful preparation. A failed refresh labels the previous snapshot and
  leaves that snapshot available. Partial collection uses **Copy included files**.
- One read-only review survives browsing and project closure. Replacement
  defaults to **Keep review**. Existing drafts, operations and Watch ownership
  remain independent. Copy completion never triggers another copy.
- The prototype uses sample fixtures and a local clipboard sink only. Activity
  contains owner/count/error metadata, never source contents. Missing helpers
  and icons do not prevent a useful initial screen.

## Payload authority and boundaries

`src/commands/copy.rs::run_with` supplies the existing framing: each readable
UTF-8 file contributes `// --- relative/path --- ` followed by LF, its original
contents, and two LF separators. Headers include the trailing space. Empty files
and non-Luau text count. No extension filtering or per-file inclusion choices.
Fixture order is explicit; the terminal currently does not promise sorted order.

The proposal exposes traversal/read failures instead of claiming complete copy.
Linked paths and oversized preparation have labeled refusal fixtures; these
are prospective native behavior, not claims about current CLI safeguards.
No arbitrary roots, export, editing, installation, upload or real clipboard use.
Display-only control/newline annotations never enter the copied text.

Implementation remains subject to owner review. No Rust, public API, protocol,
configuration, preference or release changes belong to this pass.

## Delivery evidence

See [acceptance](desktop-copy-source-acceptance.md) for tests, limitations and a
bounded review. Earlier wireframes remain unchanged.
