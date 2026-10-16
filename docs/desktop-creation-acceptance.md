# Native guided New Project acceptance

October 16, 2026. Approved Soft flow: Project → Dependencies → Capabilities →
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
all 18 changed files with zero findings at runtime head `71542ef`. The subsequent
Guided-conversion correction awaits the required review refresh; the first retry
hit the service quota. Exact-head/main CI remain separate delivery checks recorded
in the release audit after completion.

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

The isolated native window launched and stayed alive past startup on October 16.
Owner observations of the new flow, rendering, cancellation/close and real Watch
ownership are **pending/unverified**. Earlier Soft acceptance does not establish
these new controls. Browser prototype rendering also remains unverified; no retired
native automation or failed browser route was repeated.
