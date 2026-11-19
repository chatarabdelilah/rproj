# Project update discovery

The [interactive Soft wireframe](prototypes/update-discovery.html) opens on a saved
Skyline check. All project data, versions, timestamps, and release responses are
illustrative. The design adds a single project Updates view reached from Dependencies
or Tools. Opening either entry selects its group. Only **Check for updates** starts
a check, and it covers both groups. An open project can be checked during Watch.

## Reading the comparison

- **Packages** use direct declarations from all three Wally dependency sections.
  The detail shows the alias, canonical identity, configured registry, requirement,
  lockfile resolution, and file presence. The sample manual alias Signals is
  locked to 1.2.0. Its declaration allows 1.3.0. Choosing 2.0.0 would require a
  declaration change. A missing or ambiguous lockfile match leaves the comparison
  unknown even when the registry supplies release information.
- **Tools** use the project alias, configured pin, and pinned repository. Rojo's
  sample release comes from the repository in its project pin. PATH detection is
  a separate observation. A custom registry and a fork illustrate why release
  data must stay with its source. The fork's non-version pin stays unknown.
- **Freshness** belongs to each project. A failed check displays the error alongside
  its previous results and timestamp. A partial check marks unavailable entries
  unknown and identifies any last observed release with its own timestamp. Lantern
  starts without a cache. The sample cache lasts for the session.
- **Navigation** retains each project's entries and selection during a check.
  Closing a project clears the workspace. Completion updates the captured project
  and its Activity output. The existing Watch owner stays visible. Resizing changes
  only the pane arrangement at 600 pixels of main-content width.

The sample delivers discovery only. Native networking, persistent cache policy,
applying updates, and rproj self-updates require separate implementation decisions.
Current Watch and inspection behavior is documented in the [README](../README.md).

## Verification

Run the DOM checks with an existing jsdom installation. Omit the final argument
if Node resolves jsdom normally.

```powershell
node docs/prototypes/update-discovery.test.cjs target/new-project-design/dom-runtime/node_modules/jsdom
```

The checks exercise both entry points, on-demand refresh, source identity,
failure recovery, stale results, focus, Activity, theme contrast, and the
responsive CSS rule. DOM assertions do not measure rendered pane geometry or
confirm assistive-technology output. Inline rendering awaits owner observation.
Native previews remain paused.
