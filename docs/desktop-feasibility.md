# Desktop feasibility and first window

The first desktop window serves an existing project: inspect its actual packages
and tools, control Watch, and run configured checks. It does not create or edit
project configuration merely by opening a folder.

## Wireframe

```text
+----------------------+----------------------------------------------------+
| rproj                | Project name                              Refresh  |
| Open project...      | Full project path                                  |
|                      +----------------------------------------------------+
| Recent projects      | Overview | Dependencies | Tools | Checks            |
| > Current project    |                                                    |
|   Another project    | Current configuration and actionable diagnostics   |
|                      |                                                    |
|                      | Selected package details / documentation link      |
|                      +----------------------------------------------------+
| System / Light / Dark| Activity          Search output...           Clear |
|                      | Operation / time / outcome                         |
+----------------------+----------------------------------------------------+
| Watch: stopped / active project / starting / stopping / unavailable        |
+--------------------------------------------------------------------------+
```

Navigation stays subordinate to the project content. The activity region owns
its scrolling and can be resized. Package identity is the alias plus dependency
realm, not its current row number. Declared requirements, resolved versions, and
package-directory presence have separate labels.

The keyboard path begins at Open project, continues through recent-project and
section buttons, then through section actions and the activity filter. Native
folder selection owns its own focus and dismissal. Empty, loading, inspection
failure, malformed configuration, unavailable prerequisites, and operation
cancellation have visible text. Opening another project does not silently move
the active Watch.

The first styling uses the system font, GPUI semantic theme tokens, normal
control density, and labelled Lucide actions. Charmed Icons is recorded as the
candidate for file/folder identity, separate from action icons; no icon bundle
is necessary to establish the framework gate.

## Framework gate

Candidate: released `gpui-kit = "=0.7.1"`, default `component` and `assets`
features. Its released manifest pins the GPUI snapshot family to `0.3.8`.
The package was retrieved from crates.io on October 8, 2026 with rustc/Cargo
1.94.0. An isolated consumer under ignored `target/gpui-feasibility/` is used
before introducing desktop dependencies into the shared workspace lockfile.

Dependency resolution succeeded. The first compile failed because the upstream
GPUI snapshot uses `std::hint::cold_path`, stabilized in Rust 1.95. A separate
Rust 1.99.0 toolchain (`b940084d7`, released October 1) was downloaded through
rustup's official distribution into ignored `target/desktop-rustup/`; commands
set `RUSTUP_HOME` only within that shell process. The user's existing default
toolchain and CLI/core MSRV were not changed.

The probe compiled on Rust 1.99.0 and opened a real native Windows window.
Computer Use verified rendered content and the semantic button in accessibility.
Adding an explicit initial application `FocusHandle` made Tab enter the button
and draw the visible focus ring. The native maximize command was accepted;
the final stable resized screenshot was interrupted by a session reset.

The actual `rproj-desktop` member compiles with the core inspection and operation
APIs. It uses background inspection/control workers, a bounded event queue,
bounded virtualized output, a virtualized dependency list, an asynchronous
serialized preferences writer, and semantic theme controls. The sidebar keeps
recent projects scrollable independently of the appearance controls. Package
presence labels describe the alias module and package index checked by core.

On October 11, 2026, all eight desktop tests passed without ignored tests. They
cover bounded output retention, stdout/stderr chunk joining, complete retention
of 40,000-character streamed lines, and a headless GPUI interaction test. The
interaction test verifies recovery after an unavailable project becomes readable
with the same snapshot, initial Tab focus, dependency selection, unavailable
check/test actions, and preservation of a preference error. Registration tests
verify that an already matching executable leaves preference bytes and unknown
fields intact, while a changed executable updates the saved registration.
Watch-output coverage blocks concurrent requests, discards stale success/error
results after context changes, and verifies that current output still appears.
Workspace formatting
and desktop Clippy with `--all-targets -- -D warnings` passed using Rust 1.99.0.

Portable Windows packaging passed matching CLI/desktop version checks, the
desktop `--self-check`, and inspection from the unpacked ZIP. Static CRT linking
and import inspection found only Windows system DLL dependencies. These checks
do not establish visual or interactive acceptance.

The actual portable workbench launched against an ignored fixture using an
explicit `--preferences-path` inside `target/desktop-fixture/`. Computer Use
discovered its native window, title `rproj`. Its `get_window_state` capture then
failed to return despite a 30-second tool timeout; the attempt was interrupted after 191
seconds. An earlier actual-workbench capture had also stalled. No screenshot of
the actual workbench was obtained, and this capture route was stopped. The
successful visual/focus evidence above applies only to the small framework probe.

On October 11, a third attempt followed the updated Computer Use plugin
`26.1007.21434` documentation. The extracted `e39a665` portable build was started
with `Start-Process -WindowStyle Normal`, using the fixture folder and an explicit
`target/desktop-fixture/preferences-20261011.json` preferences path. Both app
enumeration and window enumeration returned promptly; the latter identified a
native `rproj` window. The next tool call, containing `get_window` followed by
`activate_window`, did not return despite a 20-second timeout and was interrupted
after 477 seconds. It is unknown which of those two operations stalled. No
screenshot request was reached. The JavaScript kernel was not reset, so reuse of
an earlier imported helper binding cannot be excluded. Normal launch was
explicit, but actual visible rendering was not verified.

Native automation was stopped after this third failed attempt. The doubtful
assumption is that Computer Use can reliably activate and capture this GPUI
surface in the current environment. This is a tooling/verification limit; the
failure does not identify a framework or application defect. Further native
acceptance remains manual rather than another unbounded capture retry.

The full native gate remains pending: stable resize and scaling, light/dark
appearance, background-work responsiveness, the complete keyboard flow, and
visual/interactive acceptance of the final portable build. This is an unresolved
verification limit, not evidence that the framework failed. Headless tests and
successful native process launch must not be reported as a full gate pass.

The gate must establish native Windows compilation and window launch, basic
keyboard interaction, resizing and scaling, background work responsiveness,
light/dark appearance, and launch from the eventual portable distribution.
No machine-wide prerequisites are provisioned to make the gate pass.

## Upstream implementation guidance

Implementation uses the upstream [GPUI Kit coding skill](https://github.com/longbridge/gpui-kit/blob/main/skills/gpui-kit/SKILL.md)
and [design skill](https://github.com/longbridge/gpui-kit/blob/main/skills/gpui-kit-design-guides/SKILL.md),
including their coding/design references and tested application recipe.
These establish retained state ownership, stable identity, semantic components,
theme tokens, explicit background work, and real-window verification.
