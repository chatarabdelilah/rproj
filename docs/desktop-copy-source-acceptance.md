# Copy Source wireframe acceptance

The owner approved the [design scope](desktop-copy-source-design.md), not native
implementation, on November 8 against main `64948d9`. The separate
[prototype](prototypes/copy-source.html) contains only labeled sample data.

## Automated evidence

All 23 [Node/DOM checks](prototypes/copy-source.test.cjs) pass with jsdom 26.1.0,
including 24 Light/Dark contrast comparisons above 4.5:1.
Reproduce from the repository root:

```powershell
node docs/prototypes/copy-source.test.cjs target/new-project-design/dom-runtime/node_modules/jsdom
```

The final argument can name another existing jsdom 26.1.0 module directory;
omit it when Node resolves jsdom normally. No package dependency was added.

- Independent JSON fixtures establish exact headers (including trailing space),
  original contents, separators, empty/non-Luau files and UTF-8 byte counts.
  Control/newline annotations and line numbers stay out of copied text.
- Review ownership survives project browsing/closure. Explicit replacement,
  default-Keep focus, menu dismissal, folder-picker cancellation and independent
  draft/operation close guards are simulated. File navigation changes no payload.
- Copy uses the captured snapshot after source changes. Refresh replaces it only
  on success; failure labels the previous snapshot. Tests cover all refusal
  fixtures, recovery, partial labels and late preparation/clipboard results.
- Activity keeps bounded owner/count/error metadata without source contents.
  Filtering, stored height and manual collapse survive navigation/completion.
  The separate simulated Stop acknowledges without changing the review.
- Literal initial content is useful without JavaScript. Runtime works without
  icons or host helpers; only appearance is optionally persisted. Accessibility
  references, unique IDs, syntax, 600-pixel main-content/550-pixel shell rules,
  wrapping and coarse-pointer targets are checked in source/DOM.

The clipboard is an in-memory test sink. No real project files were collected,
no tools executed by the prototype, and no OS clipboard calls were made.
The linked-path and size-limit cases are labeled refusal simulations, not
implemented CLI guarantees or a chosen native size budget.

The initial staged whitespace check flagged the intentional trailing spaces in
literal startup headers. Encoding those three spaces as HTML character references
preserves their displayed text and lets the whitespace check pass. The existing
Catalog prototype suite also passed all 16 tests; its source is unchanged.

The first 22 tests passed. A further focus probe found that a clipboard completion
replaced file buttons and moved focus to the document body. Status-only updates
now retain the preview DOM; successful refresh restores a surviving file control
or falls back to Refresh. The added regression test passes. CodeRabbit 0.7.6
reviewed all eight files with no findings both before and after that direct-review
correction. The final evidence-only documentation update was inspected directly.
The tracked fragment and inline copy are byte-identical; all nine earlier
conversation fragments retain their SHA-256 hashes.

No supported inline rendering-inspection route was available. Actual geometry,
clipping, keyboard activation, assistive technology and real clipboard integration
remain unverified. DOM focus and CSS checks are not real-window evidence. Native
previews and memory-stability acceptance remain paused.

## Bounded owner review

1. Select the empty file, then **Copy source**. Change sample source and copy
   again; only **Refresh** should change the preview (30 seconds).
2. Under Sample scenarios, choose **Some files skipped**, then **Load fresh
   sample review**. Check the warnings and **Copy included files** (30 seconds).
3. Switch to Lantern, then Overview → Copy source. Check **Keep review** and
   the captured Skyline identity. Try Light/Dark and narrow the preview
   (40 seconds).

Review/CI identities and branch cleanup are recorded on the delivery PR after
they finish. Native implementation requires owner approval of this flow.
