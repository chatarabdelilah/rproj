# Catalog browser wireframe acceptance

The owner approved the [proposal](desktop-catalog-browser-design.md) on November 7.
The [source](prototypes/catalog-browser.html) is a standalone conversation
fragment. Native implementation and rendering acceptance are not implied.

## Automated evidence

All 16 [Node tests](prototypes/catalog-browser.test.cjs) pass with jsdom 26.1.0.
Run from the repository root:

```powershell
node docs/prototypes/catalog-browser.test.cjs target/new-project-design/dom-runtime/node_modules/jsdom
```

The final argument may point to another installed jsdom 26.1.0 module directory.
Omit it if Node can already resolve `jsdom`. No dependency was added to rproj.

- Literal startup markup includes eight sample entries, six sections, the
  Scribe guide and its exact imported example before JavaScript runs. Syntax
  and interactions pass without host helpers or icons. Host appearance updates
  are optional and never save on initial load or restoration.
- Tests exercise all-library and scoped search, same-name entries, no matches,
  missing guides, recognized-dependency entry, manual-dependency inspection,
  related-entry Back, focus restoration and captured scroll values.
- Both package examples and documentation URLs match local catalog fixtures.
  Clipboard tests verify exact text, unavailable-clipboard selection fallback
  and rejection of obsolete completion feedback. No example is executed.
- Project navigation/closure, no-project browsing and long Unicode paths keep
  Watch, independent draft labels and retained Activity ownership. Filtering,
  manual collapse and the stored panel-height style survive navigation and
  simulated Stop acknowledgement.
- IDs, accessible references and native control markup pass source/DOM checks.
  The 600-pixel main-pane and 550-pixel shell breakpoints, wrapping and coarse
  pointer targets are present. Eighteen Light/Dark text comparisons exceed
  4.5:1. Native focus styling is retained.

The first run had 15 passes and one failing clipboard assertion: the test
awaited only one microtask while the stub returned a promise across jsdom's
realm. Waiting one full microtask turn corrected the harness; the final run
passes. Direct review also added retained page-position restoration and a
hidden-origin focus fallback.

The existing Projects prototype suite also passed all 14 tests. All eight earlier
conversation fragments retained their SHA-256 hashes; the new inline copy and
tracked source are byte-identical.

Local CodeRabbit 0.7.6 reviewed all seven changed files after the owner
reauthenticated. Its first attempt failed authentication despite local status
reporting signed in; that attempt returned no findings. The successful review
reported one minor parity-date ambiguity, corrected by explicitly dating the
Catalog row. The small documentation correction was inspected directly rather
than spending another remote review. No prototype findings were reported.

No supported inline rendering-inspection route was available. Actual browser
geometry, clipping, keyboard activation, screen-reader behavior, OS clipboard
integration and external documentation navigation remain unverified. jsdom
focus/scroll properties and CSS rules are not real rendering evidence. Native
previews and memory-stability acceptance remain paused.

## Bounded design review

1. Open **Scribe Studio** from Scribe's related entries, then **Back**. Compare
   the library and plugin explanations (20 seconds).
2. Search for **StyLua** and compare its Tool and Configurable settings entries.
   Then open Dependencies → View Scribe in Catalog (30 seconds).
3. Try **Copy example**, switch Light/Dark and narrow the preview. Confirm the
   example and guide remain readable. **Try without a project** should retain
   the selected guide and Skyline Watch (40 seconds).

The [native implementation record](desktop-catalog-native.md) tracks delivery. The earlier
conversation wireframes are unchanged. Reviewed-head and merged-main CI
identities are recorded on the delivery PR after those runs finish.
