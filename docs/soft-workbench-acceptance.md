# Soft workbench acceptance

Approved October 15, 2026. Implementation branch: `codex/soft-workbench`.
Published baseline: 0.22.0, unchanged. This is an unreleased Windows preview.

## Approved design and implemented behavior

The working Soft comparison is the visual baseline: warm light/dark surfaces,
roomier spacing, rounded controls, platform font, labeled Lucide navigation icons,
and restrained blue accents. The native title bar and rproj wordmark remain.
The layout has a compact project menu, four sidebar sections, appearance controls
at the bottom, project identity above content, and global Watch controls.

Dependencies split at 600 logical pixels of main-content width and stack below
that threshold. Default/minimum window sizes remain 1120×780 and 800×580.
The project path scrolls horizontally when necessary. Known and unfamiliar
packages retain distinct declaration, resolution, and presence information.
Tool pins remain separate from executable detection. Opening remains read-only.

Activity starts collapsed, opens for operations, explicit Watch output, or new
failures, and respects manual collapse on ordinary output/success. Panel height,
bounded output, and run outcomes remain session-only. Every retained operation
line carries its originating project, including after history eviction. Desktop
run IDs reject late progress/completions, and cancellation waits for worker outcome.
No-operation, waiting, empty output, and no-match states have separate text.

Project navigation and Close project preserve the actual Watch. Switching Watch
requires a dialog naming source and destination, revalidation of selection and
observed Watch generation, and the existing successful Stop-acknowledgement path.
The native menu/dialog own dismissal and focus restoration. No core public API,
worker protocol, project configuration, or preference schema changed.

## Automated evidence

- 20 desktop tests pass: switcher/sidebar keyboard behavior; folder-picker
  cancellation and Close project; dialog Escape and
  focus restoration; stale confirmation rejection; dependency layout geometry;
  Soft mode switching and measurable text/focus contrast; operation origin, filtering, pending cancellation and stale
  events; bounded faithful output; inspection recovery; unavailable actions;
  preference preservation; bundled icons; catalog failure visibility.
  A regression case covers a long failure and Watch path at the minimum Activity
  height, including collapse/reopen and retained panel height.
- 27 shared-core tests pass. Desktop/core Clippy and repository formatting pass.
- CLI compatibility: 536 CLI tests and 27 shared-core tests pass; 24 live or
  prerequisite-dependent tests remain ignored. The separate desktop suite has no
  ignored tests. Full changed-file CodeRabbit review returned zero findings on
  `15f79c7`, including all new source files. The follow-up fixture correction is
  reviewed separately; it adds the required Wally package metadata. The clipping
  follow-up received a full review with one minor attribution finding: workbench
  notices inherited a running operation's project label. An untagged append path
  and regression assertions correct it. The final CodeRabbit recheck is currently
  rate-limited; it has not been reported as a clean final-head review.
- A clean `15f79c7` portable archive passed extracted CLI/desktop version checks,
  matching-worker self-check, read-only inspection, executable hashes and static
  CRT dependency checks. License notices were preserved for 589 dependencies;
  completeness for three legacy archives remains documented in `THIRD-PARTY.md`.
  Windows PowerShell 5.1 ran the preview fixture preparation successfully.
- Delivery checks and merge status: [PR #107](https://github.com/chatarabdelilah/rproj/pull/107).
  Final run links are recorded with that PR's delivery evidence.

Headless tests verify state, controls and geometry. They do not verify Windows
pixels, GPU rendering, OS appearance notifications, or monitor scaling. The previous
prototype's manual acceptance remains historical evidence only. The stalled native
automation route has not been retried.

## Preview launcher

Run `scripts/open-soft-preview.ps1` with Windows PowerShell 5.1 or newer, supplying
an extracted portable bundle and its exact clean source commit:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/open-soft-preview.ps1 -Bundle "<extracted bundle>" -ExpectedCommit "<40-character commit>"
```

The launcher checks source identity and both binary hashes, creates unique
disposable projects under `target/soft-preview`, and uses isolated desktop
preferences. `-CheckOnly` verifies the bundle without launching; `-PrepareOnly`
also creates fixtures and reports their paths. A sample Check prints five timer
ticks using an already-installed Lute; no tools or packages are installed.
Watch remains the existing machine/user session, not an isolated fake Watch.

## Bounded owner review (two minutes)

1. Open the preview; inspect sidebar icons and Light/Dark readability. Choose
   System and confirm it follows an OS appearance change when convenient.
2. Open the project switcher with Enter, dismiss with Escape, then cancel
   Open project’s folder picker. Confirm selection and keyboard focus return.
3. Resize between default and minimum size. Inspect Dependencies, its selected
   detail, and the long project path. Check 100%/125% scaling when convenient.
4. Run Checks on the disposable example. Switch sections/themes while it runs,
   collapse Activity, then reopen, resize, and filter `PROBE tick 3/5`.
5. Use Close project and confirm Activity and global Watch identity remain.

The owner confirmed the first Soft preview's sidebar icons, Light/Dark readability,
project-menu/folder-picker focus restoration, dependency layout at minimum size,
and responsive background Check. They reported that shrinking Activity let a long
failure summary overlap the Watch bar, with a screenshot confirming it. The fix
uses a single-line project/status summary with full accessible/tooltip details,
fixed-height status/toolbar rows, a 144-logical-pixel Activity minimum, and a
single-line Watch identity. Output retains horizontal scrolling. Automated geometry
checks pass. In the clean `008db96` portable preview, the owner confirmed the
summary, Search/Clear controls and output remain inside the minimum Activity panel
above Watch, including after collapse/reopen. Its extracted-copy packaging checks
also passed. This confirms the reported clipping fix.
System OS-change behavior, 100%/125% scaling on Soft, and arbitrary panel-resize persistence
remain unverified separately from the prior prototype's results. Real Watch
switching needs a separate disposable-session check; it must not stop an unrelated
user Watch as part of this checklist. macOS/Linux desktop, fresh-machine runtime,
real Roblox Test runners, and authenticated Open Cloud acceptance remain outside
this milestone. Installers and publication are not included.
