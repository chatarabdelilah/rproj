# Native guided New Project acceptance

October 16–17, 2026. Approved Soft flow: Project → Dependencies → Capabilities →
Files → Review. This is the third delivery stage after shared composition
([PR #113](https://github.com/chatarabdelilah/rproj/pull/113)) and the owned
creation worker ([PR #114](https://github.com/chatarabdelilah/rproj/pull/114)).
No version bump, publication, installer or new preference schema is included.

## Implemented boundary

The compact project menu and empty view open one session-only draft. Browsing
existing projects retains its step, choices and result. Guided/Expert/saved
choices use core derivation; saved data with unsupported selections remains an
actionable error. Workflow changes explain invalidation and retain incompatible
testing until the user chooses a compatible implementation or None explicitly.
Required files cannot be excluded. Saving uses a new setup name only.

Context and preparation run off the UI thread. Preparation creates no destination.
Create requires a prepared review and explicit confirmation; the CLI revalidates
sources, compatibility and destination before the existing exclusive executor.
Operation IDs and captured destinations keep output/results independent of
browsing. Success offers Open project explicitly and never changes Watch.
Activity retains bounded output, filter behavior, manual collapse and panel height.

Normal window close during creation offers Keep open by default or Cancel creation
and close, naming the destination and warning about partial files. Escape keeps
the window open. Close waits for the owned worker outcome after descendant teardown
and stream draining; uncertain teardown keeps the window open. Dirty unsubmitted
drafts have a separate default-Keep discard confirmation. Detached Watch stays
running. Force termination and OS shutdown are outside this normal-close contract.
An actual teardown error deliberately has no second-close bypass: without an
owned-process completion signal, this milestone cannot safely declare cleanup
finished. The blocked state is explicit in Activity and disables new creation;
read-only project browsing remains available. CodeRabbit suggested a second-close
bypass, which conflicts with the owner's approved close requirement and was not
adopted. This conservative error path has deterministic test coverage, not an
injected real OS teardown-failure observation.

## Automated evidence

Local desktop coverage: 32 passing tests, including native GPUI interaction tests
for five-step navigation, Unicode name entry, companions, required-file protection,
minimum layout, draft retention, default-Keep/Escape, cancellation pending until
outcome, teardown failure, stale preparation/context/results, actionable template
errors, explicit result opening and unchanged Watch ownership. Tests use the pinned
Kit's reduced-motion mode for deterministic dialog hit testing; they do not verify
animated rendering or Windows pixels. Existing Soft/inspection/Activity tests remain.
Guided conversion also rejects multiple primary choices before changing the draft,
reuses shared companion derivation and preserves unfamiliar data on rejection.

CLI/core regression suites pass; 24 prerequisite-dependent cases remain ignored.
Worker fixtures verify real scaffold helpers through tool stubs,
source/destination/setup races, Unicode/spaces, structured reports and descendant
cleanup. Those cases do not establish real Git/Wally/Blender/Studio acceptance.
Strict desktop/CLI/core Clippy, formatting, both fresh-cache crate tarballs and
the portable Windows build/smoke checks pass. Final local CodeRabbit reviewed
all 18 changed files at corrected runtime head `cea8e63`, with one minor
presentation suggestion and no critical/warning findings. The suggestion to clear
the previous saved-source identity after a rejected reload was evaluated: that
identity describes the retained, last-applied graph. It stays captured while the
visible error blocks Review until an explicit valid source choice; a regression
checks both retention and recovery. The earlier review refresh quota cleared.
All seven [runtime-head CI jobs](https://github.com/chatarabdelilah/rproj/actions/runs/38901332447)
passed; badge freshness was skipped. [PR #115](https://github.com/chatarabdelilah/rproj/pull/115)
merged at `8efb026`. Its [final head](https://github.com/chatarabdelilah/rproj/actions/runs/38902515284)
and [merged main](https://github.com/chatarabdelilah/rproj/actions/runs/38903823480)
also passed all seven executed jobs; badge freshness was skipped. Guarded
local/remote branch cleanup is complete.

## Disposable Windows preview

From the repository, Windows PowerShell 5.1 works:

```powershell
powershell.exe -NoProfile -File scripts/open-creation-preview.ps1
```

The launcher builds the native view and a fixture-only CLI bridge beside it,
with explicit configuration/setup/Watch storage beneath a unique
`target/creation-preview/` directory. It does not redirect or replace owner
configuration. Desktop preferences use the existing override path. Git is a stub;
other external tools are deliberately absent. Choose **Minimal fixture** in Saved
setup to exercise dependency-free creation without provisioning. Unsupported
fixture demonstrates visible rejection. Actual Watch controls are disabled in
this fixture; its state is not evidence about an actual running supervisor.

`-Scenario Failure` makes fixture Git fail after destination claim.
`-Scenario Cancellation` makes it wait with a descendant so Activity Cancel or
normal window close can exercise owned teardown. Close each preview before
opening another scenario. `-PrepareOnly` prepares storage without opening a window.
The launcher records source commit/dirty state and fixture paths in
`target/latest-creation-preview.json`; it leaves partial files for inspection.

## One bounded Windows checklist

1. **Success, 2 minutes:** New project, enter a name, Next, select Minimal fixture,
   advance through Capabilities and Files, wait for Review preparation, confirm
   Create, then choose Open project. Confirm responsive navigation and expected files.
2. **Recovery, 2 minutes:** start another draft; select Unsupported fixture and
   observe its explanation. Select Minimal fixture explicitly to recover. In a
   Failure preview, confirm failure/claim/path and preserved files; no success claim.
3. **Close/cancel, 2 minutes:** in a Cancellation preview create Minimal fixture,
   close normally, choose Keep open once, then Cancel creation and close. Expect
   closure only after teardown and preserved partial files. Separately cancel via
   Activity and confirm the window remains usable.
4. **Layout/input, 2 minutes:** Light/Dark/System, Tab/Shift+Tab and Enter/Space,
   menu/dialog Escape, minimum/default sizes and expanded Activity; inspect long
   paths and package details. Retain Activity height after collapse/reopen.
5. **Real ownership, 1 minute when available:** record real Watch ownership through
   the ordinary worker before and after the isolated review. The fixture cannot
   switch it. Real tool/Roblox/credential prerequisites remain separate acceptance.

## Manual observations

On October 16, the owner answered “yes all successful” to the isolated New Project
check: enter “Native example”, select Minimal fixture, advance to Review, confirm
Create project, then choose Open project. This confirms successful creation,
responsiveness and explicit opening of the result in the observed preview.
The exact preview build used for this reply was not reported. The check used
disposable storage and a Git stub; it does not establish real Git/Wally/Blender/
Studio behavior or manual inspection of generated file contents.

In the reopened preview built from clean main `dfe336d`, the owner confirmed
both saved-setup checks: Unsupported fixture shows an explanation and blocks
Review; selecting Minimal fixture explicitly clears the error and successfully
prepares Review without creating a destination. This confirms rejection and
recovery, not execution-failure handling.

In that same preview, with the fixture Git stub changed to pause after claiming
the destination, the owner confirmed the normal-close check. The dialog named
the destination and warned about partial files; Keep open retained a usable
window, and Cancel creation and close closed it after cancellation completed.
A subsequent filesystem/process observation found the fixture claim marker,
the preserved `Recovery example/.git` directory and no remaining fixture
desktop, bridge or Git processes. That later observation does not independently
establish teardown ordering; the worker tests cover the owned-process contract.

With the same build reopened and the fixture Git stub set to fail, the owner
confirmed that creating `Failure example` reports failure, identifies the
destination/possible partial files, offers no success/Open project action and
leaves navigation responsive. This is a tool-stub execution-failure observation,
not a real Git failure or a malformed/missing-report test. A subsequent inspection
found the empty `Failure example` destination preserved and only the preview
desktop process still active, with no fixture bridge or Git process.

On October 17, the owner confirmed all four checks in the reopened cancellation
preview of the same build: the Minimal fixture flow stayed readable in Light/Dark
after shrinking and restoring the window; Activity Cancel reached Cancelled while
leaving the window usable and output labeled for `Activity example`; a manually
taller Activity retained its height after collapse/reopen; and closing a dirty
`Draft example` showed Keep draft as the primary choice, with Escape preserving
the entered name. These are owner observations, not instrumented pixel, timing
or focus measurements.

Actual dirty-draft discard, full keyboard traversal, new-flow System appearance
changes/scaling, detailed package/long-path inspection and real Watch ownership
remain **unverified**. The fixture observations do not establish real external-tool
behavior. Earlier Soft acceptance does not establish these new controls. Browser
prototype rendering also remains unverified; no retired native automation or
failed browser route was repeated.
