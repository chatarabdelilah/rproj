# Projects browser wireframe acceptance

The [design](desktop-projects-browser-design.md) is ready for owner review.
The conversation fragment and [reviewable source](prototypes/projects-browser.html)
are byte-identical. The seven earlier conversation fragments retained their
SHA-256 hashes.

## Automated evidence

All 14 [Node tests](prototypes/projects-browser.test.cjs) passed with jsdom
26.1.0. To reproduce from the repository root, run:

```powershell
node docs/prototypes/projects-browser.test.cjs target/new-project-design/dom-runtime/node_modules/jsdom
```

The last argument can point to another local installation of jsdom 26.1.0.
Omit it if Node can already resolve `jsdom`. No runtime dependency was added.

- Literal markup displays the initial project and warning before JavaScript
  initializes. The script parses and the interactions work without icons or
  host state. Only appearance is eligible for host restoration.
- Deterministic fixtures exercise recognition, shallow discovery, sorting,
  duplicate names, root/launch deduplication, nested and linked entries,
  Unicode, malformed files, empty and unavailable roots and missing folders.
- DOM events verify filtering, keyboard preview navigation, focus restoration,
  retaining list position, explicit Open, recent-only folders, and Back to an
  empty workspace. Refresh coalescing, stale scan/preview completion, and
  disappearance before Open are covered. A failed recent selection cannot
  discard another folder's preview.
- Source checks cover the 600-pixel main-pane breakpoint, narrow shell layout,
  wrapping paths, accessible IDs and visible focus rules. Eighteen Light/Dark
  text and focus color comparisons meet 4.5:1. The DOM scroll test verifies
  saving/restoring the scroll property, not measured browser geometry.

No supported inline rendering-inspection route was available. Actual layout,
clipping, touch targets and assistive-technology behavior remain unverified.
Native previews and memory-stability acceptance remain paused.

## Bounded design review

1. With Lantern previewed, check that Skyline appears in the switcher and
   Watch bar. Choose **Open project**. Overview should change to Lantern
   while Watch still shows Skyline (20 seconds).
2. Use the switcher to browse again and search for “Relay.” Compare the two
   paths, then choose Back and reopen the browser. The query and selected path
   should remain (30 seconds).
3. Expand **Design sample**, remove the selected folder, then try Open and
   Refresh. The active workspace should survive and the missing preview should
   clear (30 seconds).


## Native automated verification

The owner subsequently approved the wireframe as the baseline for native
implementation. The [native record](desktop-projects-browser-native.md)
separates automated evidence from paused real-window acceptance. Windows
headless tests exercise the switcher and empty entry, keyboard Back, filtering,
retained selection and scroll, both pane layouts, and clicking Open at minimum
size. Disposable filesystem tests cover malformed records, redirects, and
disappearance after preview. The successful handoff and failed Open fixtures
check workspace ownership separately from global Watch and running operations.
