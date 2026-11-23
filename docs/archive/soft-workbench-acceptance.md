# Soft workbench acceptance

Approved and merged October 15, 2026 in [PR #107](https://github.com/chatarabdelilah/rproj/pull/107)
at `24595f6`. The fully merged `codex/soft-workbench` branch was removed.
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

- 21 desktop tests pass: switcher/sidebar keyboard behavior; folder-picker
  cancellation and Close project; dialog Escape and
  focus restoration; stale confirmation rejection; dependency layout geometry;
  Soft mode switching and measurable text/focus contrast; operation origin, filtering, pending cancellation and stale
  events; bounded faithful output; inspection recovery; unavailable actions;
  preference preservation; bundled icons; catalog failure visibility.
  A regression case covers a long failure and Watch path at the minimum Activity
  height, including collapse/reopen and retained panel height.
  A separate real-drag regression verifies arbitrary resized height survives
  collapse/reopen; the earlier minimum-height case did not detect that reset.
- 27 shared-core tests pass. Desktop/core Clippy and repository formatting pass.
- CLI compatibility: 537 CLI tests and 27 shared-core tests pass; 24 live or
  prerequisite-dependent tests remain ignored. The separate desktop suite has no
  ignored tests. The original Soft delivery's final local CodeRabbit review inspected all 14 changed files at
  `b824aba` and returned zero findings. Earlier review findings were corrected:
  workbench notices no longer inherit a running operation's project label;
  bundle-test coverage includes both Activity arrow icons; failure messages use
  a plain exit code or "no exit code". The opt-in GitHub review threads are resolved.
- The latest clean `bfb52d5` portable archive passed extracted CLI/desktop version checks,
  matching-worker self-check, read-only inspection, executable hashes and static
  CRT dependency checks. License notices were preserved for 589 dependencies;
  completeness for three legacy archives remains documented in `THIRD-PARTY.md`.
  Windows PowerShell 5.1 ran the preview fixture preparation successfully.
- All seven executed CI jobs passed on the final reviewed head `b824aba` in
  [38790704521](https://github.com/chatarabdelilah/rproj/actions/runs/38790704521)
  and merged main `24595f6` in
  [38792261666](https://github.com/chatarabdelilah/rproj/actions/runs/38792261666).
  The scheduled/manual badge job was intentionally skipped. Nonblocking cache
  annotations reported missing packaging scratch paths; package and portable
  checks passed. No version, tag, installer, or publication was created.

The corrected-head stable Windows run exposed an existing test-fixture ordering
race: its fake supervisor could release ownership after 200 ms while the persisted
snapshot still said Stopping. CI failed 2.2 seconds into the unit batch, before the
fixture's five-second lifetime limit. The fixture now requires its terminal state
to have been published before releasing ownership. Success, failed-Stop and timeout
cases each passed ten local repetitions; production Watch code is unchanged.
The local full suite also reproduced a Hub layout test reading the owner's live
Watch summary. Its test helper now supplies a fixed Stopped summary; the previously
failing layout case and complete 563-test CLI/core suite pass without changing the
actual Watch session. Ratatui production rendering is unchanged.

The October 16 documentation-only main run
[38816125972](https://github.com/chatarabdelilah/rproj/actions/runs/38816125972)
later exposed a different Stop-fixture timeout. Its status handler flushed state
on every probe; a delayed write could exceed the 300 ms status timeout. A new
600 ms delayed-persistence regression reproduces the failure. The fixture now
serves status independently of terminal persistence, publishes the saved terminal
snapshot before releasing ownership, and joins its workers after action failure.
All four focused Stop cases passed 160 case executions across 40 runs with four
concurrent processes. The full CLI/core suite passes 564 tests, with 24 prerequisite
tests still ignored; strict Clippy and formatting pass. Production Watch and its
timeouts are unchanged. Corrected-head review/CI are recorded with follow-up delivery.

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
The example's `example/unfamiliar` dependency is deliberately fictional to exercise
unknown-package inspection. Starting real Watch there fails during Wally resolution;
the October 16 read-only status/log check confirmed this cause for the older preview's
Failed state. The separate `Another project` fixture has no Wally dependencies.
Any real Watch acceptance still needs installed prerequisites and a disposable session.

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
On October 16, the owner checked the verified clean `bfb52d5` Soft preview and
confirmed that System appearance follows Windows default app mode between Light
and Dark automatically. Dependencies and expanded Activity fit and respond after
changing Windows Scale from 100% to 125% and restoring the original settings.
These observations apply to Soft separately from the earlier prototype's results.
The owner then reported that a taller Activity panel resets after collapse/reopen,
while its content stays inside. A native drag regression reproduced 271.5 px
becoming 233.5 px. The workbench now owns the resizer state across collapsed
renders; all 21 desktop tests and strict desktop Clippy pass. In the clean
`297f53f` replacement preview, the owner confirmed that the chosen height stays
unchanged through two collapse/reopen cycles and all content fits. This completes
the reported Activity height fix's native check. Real Watch
switching needs a separate disposable-session check; it must not stop an unrelated
user Watch as part of this checklist. macOS/Linux desktop, fresh-machine runtime,
real Roblox Test runners, and authenticated Open Cloud acceptance remain outside
this milestone. Installers and publication are not included.

## Real Watch acceptance preparation (October 16)

After Git connectivity recovered, local main was synchronized to `7d78871` and
the fully merged acceptance branches were deleted with exact-head guards.
The verified `297f53f` portable preview is also the source for the real Watch check.
Unique source/destination projects under `target/watch-acceptance` omit both
Wally and Rokit manifests. The existing Watch was Failed on an older disposable
preview, so no active unrelated Watch was stopped. Already-installed Rojo 7.7.1
is supplied through a process-local PATH; no tools or packages are installed.

The real source session reached Watching. Two newly added modules appeared in
its generated sourcemap, and Watch remained running while the earlier preview
window was replaced with isolated preferences listing both test projects.
Two read-only probes during a two-minute monitor briefly reported Unresponsive
and recovered to Watching; the cause remains unverified. The production status
handler does not persist state for reads. After the owner reported difficulty
switching or stopping, a fresh owner-context status query confirmed Watching on
the destination. This confirms destination startup, but does not establish which
GUI action initiated it. The owner could not
confirm which instructions they followed and reported disabled Start/Switch
controls. Those controls are intentionally disabled when the selected project
already owns Watch; this report alone does not establish a defect. A targeted
Stop observation then confirmed a defect: Stop is disabled, Activity remains at
`Watch destination · Starting Watch · Running`, and section navigation still
responds. The production lifecycle fixture reproduced this: the launcher exits,
but its captured output pipes remain open for the detached Watch's lifetime.
Native Close project, browsing, and confirmation Escape remain unverified
separately from the confirmed backend transition. No native capture/activation
automation was retried.

The fix starts the supervisor without inheriting handles, using winsafe's pinned
kernel-only safe wrapper. This preserves the application's unsafe-code ban,
detachment/breakaway behavior, environment/tool PATH, and existing control
acknowledgements. The startup nonce moves from a supervisor environment override
to its private invocation argument; control authentication tokens remain private.
The public CLI, workbench worker verbs, session data and configuration are unchanged.
The lifecycle fixture now requires EOF on both captured streams before stopping
Watch and uses paths containing spaces and Unicode. It failed before the fix;
replacement-preview acceptance remains separate from automated validation.

Cleaning up the owned session also exposed a false Stop failure: a transient
status timeout after Stop was accepted made the acknowledgement loop fail
immediately, although the same session then reached Stopped. A delayed-status
regression reproduces that exact error. Stop now continues observing the same
session within its existing deadline; it still requires persisted Stopped and
released ownership, rejects changed owners and Failed outcomes, and never starts
a replacement after an unacknowledged Stop. A permanently slow/still-owned fixture
must time out rather than claim success.

Final local validation passes 566 CLI/core tests (24 prerequisite-dependent tests
remain ignored), all 21 desktop tests, formatting, and strict CLI/core/desktop
Clippy. The initial full run while the disposable real Watch was active failed a
setup test's mutation guard; after acknowledged cleanup, the full suite passed.
The lifecycle regression initially also exposed a PowerShell 5.1 fixture encoding
issue; generated scripts now include a UTF-8 BOM for Unicode fixture paths.
Final CodeRabbit review of `b99ec49` reports zero findings across all seven
changed files. Its clean portable archive passes version, worker/inspection and
static-CRT checks, with 590 dependency notices including winsafe's license; the
existing three legacy notice-completeness warnings remain. In the extracted
copy, real Rojo source Start reaches Watching, both captured output streams close
after launcher exit, and acknowledged Stop reaches Stopped with both streams
closed. One replacement window uses isolated preferences and the same disposable
projects. In that `b99ec49` replacement window, the owner confirmed all three
native steps: source Start reaches Watching and Activity Succeeded with Stop
enabled; confirmed Switch reaches Watching on the destination and Succeeded;
Stop reaches Stopped. This confirms the reported stuck operation/disabled Stop
fix. Native Close project and confirmation Escape are not included in this
three-step observation. Reviewed head `67f572a` passed all seven executed jobs in
[CI](https://github.com/chatarabdelilah/rproj/actions/runs/38841289738); merged main
`9910df1` passed all seven in [CI](https://github.com/chatarabdelilah/rproj/actions/runs/38841859391).
The badge job was skipped intentionally. PR #111 and merged-branch cleanup are
complete. No version bump or publication was performed.
