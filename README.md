# rproj

rproj is a guided Windows CLI that sets up your Roblox development tools and creates Luau projects with the dependencies, checks, and assets you choose.

```powershell
cargo install rproj
```

## Quickstart

Create a project and start watching it from PowerShell using the default projects folder:

```powershell
rproj new my-first-game
cd "$env:USERPROFILE\Documents\RobloxProjects\my-first-game"
rproj watch
```

New Project guides you through setup and shows the destination before creation. If you chose a different projects folder, use the path shown there. Watch restores dependencies and keeps the Rojo sourcemap current.

## Requirements

- Windows with WinGet.
- Rust and Cargo to install from crates.io or source. The portable Windows x64 CLI can bootstrap Rokit without Cargo.
- Network access to install tools and packages.

**Public alpha.** Commands, configuration, and generated files can change between releases. [Changelog](CHANGELOG.md).

## Other installation options

### Portable CLI for a fresh x64 Windows PC

Use the development archive when copying the CLI to a machine without the Visual C++ Redistributable. Its `rproj.exe` includes the MSVC runtime through static linking.

1. Sign in to GitHub and open a recent successful `main` run in [CI](https://github.com/chatarabdelilah/rproj/actions/workflows/ci.yml?query=branch%3Amain). Under **Artifacts**, download **rproj-desktop-windows-x86_64**.
2. Extract the downloaded ZIP, then extract the portable ZIP inside it. Copy **`rproj.exe`** from the inner archive to the target PC. The separate `rproj-desktop.exe` is the GUI.
3. Open PowerShell in the folder containing the copied CLI and run `.\rproj.exe --version`. To start Machine Setup, run `.\rproj.exe setup`.

### Install from source

The repository version can be installed with:

```powershell
git clone https://github.com/chatarabdelilah/rproj.git
cd rproj
cargo install --path .
```

## What rproj does

rproj connects two layers that are usually assembled by hand:

1. **Machine setup** installs and configures development applications, command-line tools, Roblox Studio plugins, and VS Code extensions.
2. **Project setup** turns your choices into a Roblox project with pinned tools, dependencies, editor settings, quality checks, and optional asset workflows.

It explains the available choices before applying them. It does not hide the underlying ecosystem: generated projects remain ordinary Rojo, Rokit, Wally, Luau, and Git projects that can be maintained without rproj.

Run `rproj app` to open the Windows desktop workbench. Bare `rproj` prints the command overview. Direct commands remain available from a terminal.

## Machine Setup

Open **Machine Setup** in the desktop app, or run `rproj setup` in an interactive terminal. Pick tools and applications by category, review the plan, then confirm before installation. Optional items start unchecked and saved choices carry forward. Unrecognized selections are retained without being installed.

Installations run sequentially. **Stop** in the desktop app or Ctrl+C in the CLI lets the active item finish before stopping. Cancelled or fatal attempts do not save machine selections. Completed attempts that encountered recoverable issues report warnings and save selections. Native installer dialogs and manual plugin or account steps may still appear.

Selecting catalog CLI tools runs Rokit first. Confirmed setup may replace a selected tool's global pin with its catalog pin. Project pins stay intact.

## Saved Setups

Open **Saved Setups** in the desktop app to inspect, edit, rename, duplicate, or reuse a named setup. **Use in New Project** loads the saved composition into a draft. Save a new named setup from **New Project → Review** during project creation, or use `rproj new <name> --save-setup <setup>`. The CLI also accepts `--like <setup>`.

Setups are stored in `<rproj config>/setups/<name>.toml`. Changed saves preserve unrelated fields and may reformat TOML. Rename and Duplicate preserve file bytes. Concurrent file changes require a reload before applying an edit.

## Ecosystems

| Layer | What rproj manages |
| --- | --- |
| Windows applications | Git, VS Code, Roblox Studio, Blender, Figma, and related prerequisites |
| CLI toolchain | Rokit plus pinned tools such as Rojo, Wally, Selene, StyLua, luau-lsp, Lute, jest-roblox, Asphalt, and Tungsten |
| Packages | Wally packages or no package manager, including UI, state, networking, data, testing, and ECS libraries |
| Roblox Studio | Development plugins selected during machine setup |
| VS Code | Recommended extensions and project-scoped settings |
| Generated project | Rojo mapping, source tree, dependency manifests, lint/format/type configuration, tests, CI, and asset configuration |

Use `rproj info` to browse the complete current catalog and `rproj info <key>` to inspect one entry.

## Commands

| Command | Purpose |
| --- | --- |
| `rproj` | Print the command overview |
| `rproj new <name>` | Set up the machine when needed and create a project |
| `rproj setup` | Review or change machine-level tools |
| `rproj setup <tool>` | Set up one supported tool for the current project |
| `rproj configure [key]` | Configure a project tool, or choose what to configure |
| `rproj configure project` | Edit or reset the global Rojo tree inherited by future projects |
| `rproj upgrade` | Review every maintained-file change, then apply the current `rproj.toml` decisions |
| `rproj watch` | Restore dependencies and start the Rojo development loop |
| `rproj watch start` | Start persistent background sourcemap Watch on Windows |
| `rproj watch status / logs / stop` | Inspect or stop the single background Watch from any directory |
| `rproj test [args]` | Restore dependencies and run the selected test runner; pass remaining arguments through |
| `rproj copy` | Copy source files with path headers |
| `rproj info [key]` | Open the TUI Catalog or print one entry when a key is supplied |
| `rproj --verbose ...` | Include commands and captured subprocess output |

### Project inspection, updates, and Watch

`rproj inspect [path] --json` reports schema-versioned declarations, lockfile
resolutions, package-file presence, tool pins, diagnostics, and configured actions.
It does not install tools or change project files. Human output is the default.
Watch now lists actual `wally.toml` aliases from every realm; `rproj.toml` retains
your saved composition choices. Watch restores dependencies but does not
automatically upgrade package requirements or tool pins.

Use `rproj updates [path]` to check direct Wally packages and project Rokit pins
on demand. The report shows local versions, the highest stable Wally release
within each requirement, and the highest stable release overall. Public GitHub
forks use their own releases. Custom registries and unavailable sources appear
with explanations. Add `--json` for the versioned report and Unix check time.
Partial reports exit successfully; failure to produce a report exits nonzero.

In the matching Windows desktop workbench, open **Dependencies** or **Tools → View
updates… → Check for updates**. Select one release and choose **Prepare review**
to see the declaration, lockfile or tool-pin changes. **Apply update…** asks for
confirmation. Local package edits and changes to another direct alias block
Apply. Stop Watch separately if it owns the selected project.

Assets 3D checks the installed Blender executable independently of old machine
setup selections. In an existing project, `rproj setup blender` creates a missing
`blender/scene.blend` and preserves an existing scene. Missing Blender is reported
before project creation; ordinary Upgrade does not generate binary asset files.

`rproj app [path]` locates a sibling or previously registered desktop
executable; installing the CLI with Cargo does not download the GUI. Desktop
distribution and acceptance evidence are tracked in the
[feasibility record](docs/archive/desktop-feasibility.md).

On Windows, `rproj watch start` runs that sourcemap Watch in the background after rproj closes. Only one project can run in the background per user. Use `rproj watch status`, `rproj watch logs`, and `rproj watch stop` from any directory. Stop finishes the active recovery command before skipping later work. During watching it terminates the owned process tree. Startup is manual; Watch does not restart after logout/reboot or failure. It does not run Rojo Serve.

### Projects

Projects lists recognized folders directly beneath the configured projects root, plus the launch directory when it contains `rproj.toml` or `default.project.json`. It does not recurse or follow linked child directories. Malformed projects stay visible with warnings; missing or unreadable roots are reported without creating folders.

Use the desktop project picker to open a project or resume a recent one. Project views provide configuration, upgrades, tests, source copying and background Watch controls. Stop Watch before changing the watched project's composition or configuration, and before Machine Setup. Use `rproj watch` for foreground Watch in a terminal.

Background states are Preparing, Watching, Stopping, Stopped, Failed, and Unresponsive. Start acknowledges Preparing; Watching requires successful recovery, initial sourcemap generation, and a spawned watcher. If the launcher prohibits process detachment, start from a normal Windows terminal. Unresponsive ownership blocks a replacement; rproj never kills a process using a cached PID. Watch logs retain two files of at most 1 MiB each under the user's local rproj data directory. Review tool output and local paths before sharing logs.

### Catalog

Use **Catalog** in the desktop app to browse packages, tools, and examples. Run `rproj info` for the plain catalog listing or `rproj info <key>` for one entry.

### New Project

Choose **New Project** in the desktop app. Guided and Expert modes let you choose dependencies, packages and capabilities before reviewing the generated files. You can also reuse a saved setup. Confirm **Create** to write the project and any optional named setup. If another process claims the destination before Create, rproj refuses to overwrite it. Direct `rproj new <name>` uses terminal prompts.

### Tool configuration

Run `rproj configure selene`, `stylua`, `luau-lsp`, or `stylua-vscode` in the project directory. Existing settings are the prompt defaults; missing settings use catalog defaults. An unlisted or unsupported existing value is kept unless you explicitly agree to replace it. Keeping all existing settings unchanged avoids rewriting the file. TOML layouts the writer cannot safely modify are refused without saving; edit those files manually.

### Upgrade review

Run `rproj upgrade` in an interactive terminal to see proposed file diffs and confirm with a default-No prompt. The desktop **Upgrade Project** view also supports review. If a source file changes during review, Upgrade refuses the stale plan before writing.

Use `rproj upgrade --yes` to apply without an interactive prompt, with the same preservation and conflict checks.

## Generated project

The exact tree follows your choices. A full project can include:

```text
my-first-game/
|-- default.project.json
|-- rokit.toml
|-- wally.toml
|-- selene.toml
|-- stylua.toml
|-- .luaurc
|-- .gitattributes
|-- .gitignore
|-- rproj.toml
|-- src/
|   |-- client/
|   |-- server/
|   `-- shared/
|-- tests/
|-- jest.project.json (Jest Roblox only)
|-- jest.config.json (Jest Roblox only)
|-- .lute/check.luau
|-- .github/workflows/ci.yml
|-- figma/
|-- asphalt.toml or tungsten.toml
`-- .vscode/settings.json
```

`rproj.toml` records the current project decisions used by `rproj upgrade`. During alpha, re-scaffolding is preferred when the schema changes substantially.

## Testing

Testing is optional. Wally projects can choose either **Jest Roblox** or **TestEZ**; dependency-free projects use TestEZ because Jest Roblox is distributed as Wally development packages. The choice is written explicitly to `rproj.toml`, and `rproj upgrade` never migrates one runner to the other.

Run the selected runner through:

```powershell
rproj test
rproj test -t inventory
```

TestEZ runs through `lute test`. Jest Roblox installs the official `Jest` and `JestGlobals` packages under `[dev-dependencies]`, mounts them as `ReplicatedStorage.devPackages` in both Rojo project files for type analysis (the disk folder remains `DevPackages`), keeps test mounts out of `default.project.json`, and builds tests through the generated `jest.project.json`. After choosing Jest, choose **Local Studio** or **Open Cloud**. Local Studio uses jest-roblox's hidden `studio-cli` backend and requires Roblox Studio to be installed, logged in, and accompanied by `JestRobloxRunner.rbxm`. rproj installs or refreshes that plugin when a local Jest project is created; rerun `rproj setup` if installation was interrupted. Explicit backend and workspace arguments are forwarded to jest-roblox unchanged.

Local Studio projects run tests before pushing and generate CI with quality checks only. Open Cloud projects use that backend for `rproj test` and include tests in generated CI, without a separate enable variable. For Open Cloud, add repository secret `ROBLOX_OPEN_CLOUD_API_KEY` and repository variables `ROBLOX_UNIVERSE_ID` and `ROBLOX_PLACE_ID`. The API key needs `universe-places:write`, `universe.place.luau-execution-session:write`, and `memory-store.sorted-map:read`/`memory-store.sorted-map:write`; the last pair carries streamed GitHub Actions results. Cloud CI fails with the missing names instead of silently skipping tests. Use a dedicated test place: the runner uploads its test build to the configured place. Queue scopes are unnecessary for rproj's generated single-session CI. Workspace or parallel sharding modes passed through `rproj test [args]` additionally require `memory-store.queue:add`, `memory-store.queue:dequeue`, and `memory-store.queue:discard`.

To migrate an existing project, run `rproj upgrade`, then change your own test imports from `ReplicatedStorage.DevPackages` to `ReplicatedStorage.devPackages`; upgrade does not rewrite user source. The Jest project omits Lighting and enables `ServerScriptService.LoadStringEnabled`; the default project retains its own settings.

The execution choice is saved in project records and saved setups: `[capabilities] test = "jest-roblox"` means Local Studio; `test = "jest-roblox-open-cloud"` means Open Cloud. Existing records keep local execution. To change an existing project, update that choice and run `rproj upgrade`; use a CLI `--backend` argument for a one-run override. Cloud runs also need the Roblox credentials available in the invoking environment.

`jest.config.json` is merge-managed: rproj owns `backend`, `rojoProject`, `jestPath`, and `test.projects`, while preserving other valid JSON fields for filters, coverage, snapshots, and runner options. `jest.project.json` is fully generated from the current production project and is refreshed by `rproj test`, `rproj watch`, and `rproj upgrade`.

## Project template

Open **Project Template** in the desktop workbench or launch it with `rproj configure project`. Edits apply to future projects and are stored at `<rproj config>/templates/default.project.json`.

Use the Explorer and Inspector for tree and property edits, or **Advanced JSON** for the whole document. **Review changes…** shows the proposed changes before Save. Invalid JSON stays available for repair, and external changes require **Reload from file**. Confirming **Restore built-in** removes the custom override.

Required source mounts and the DataModel root are protected. Rojo validates the reviewed template. Install the global fallback with `rokit add --global rojo` if needed.

## Behavior and scope

- rproj asks before optional machine-level installation and project generation.
- The project directory is created only after the final creation confirmation. Cancelling project choices leaves no project directory; if another process creates the destination while you answer, creation refuses to reuse it.
- External command output is summarized; `--verbose` exposes the underlying commands and captured output.
- Existing user-owned configuration is preserved where a command supports merging. Unreadable configuration is refused rather than replaced.
- The current implementation targets Windows. Cross-platform support is not claimed.
- rproj is a development orchestrator, not a package registry, build system, game framework, or Roblox Studio replacement.

## Development

### Diagnostic logs

rproj writes a local `.txt` log for each run except `--help` and `--version`. Direct commands print its path
on stderr when finished; Home shows the full path with failures and stays quiet
after ordinary navigation or cancellation. Attach that file when reporting a problem, after checking
it for private paths or names. Logs include choices, steps, and errors, but omit
credentials-related fields, raw keystrokes, source/clipboard contents, and raw
external-tool output. Nothing is uploaded automatically.

See [diagnostic logs](https://github.com/chatarabdelilah/rproj/blob/main/docs/archive/diagnostic-logs.md)
for location, limits, and opt-out.

### Checks

```powershell
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo package --locked -p rproj-core -p rproj
```

### Desktop development (Windows)

CLI/core retain Rust 1.89 support. The pinned GPUI dependencies use a separate
Rust 1.99.0 build; default workspace commands build CLI/core. With that toolchain
available, build the matching pair and open an existing project:

```powershell
cargo +1.99.0 build --locked -p rproj -p rproj-desktop
./target/debug/rproj.exe app "C:/path/to/project"
pwsh -File scripts/package-desktop.ps1
```

The portable script builds with a static MSVC runtime, bundles dependency
notices, and verifies the extracted executables. Its archive is a development
build until a coordinated release is prepared. Native acceptance and remaining
distribution checks are recorded in [the framework record](docs/archive/desktop-feasibility.md).

The codebase separates CLI dispatch, decision modeling, catalogs, command orchestration, and filesystem/process steps. See [Architecture](https://github.com/chatarabdelilah/rproj/blob/main/docs/architecture.md), [Development plan](https://github.com/chatarabdelilah/rproj/blob/main/docs/plan.md), [UX redesign](https://github.com/chatarabdelilah/rproj/blob/main/docs/archive/ux-redesign.md), and [Release process](https://github.com/chatarabdelilah/rproj/blob/main/docs/releasing.md).

## Support

[![crates.io](https://img.shields.io/crates/v/rproj.svg)](https://crates.io/crates/rproj)
[![license](https://img.shields.io/crates/l/rproj.svg)](#license)
[![Buy Me a Coffee](https://img.shields.io/badge/Support-Buy_Me_a_Coffee-FFDD00?logo=buy-me-a-coffee&logoColor=000)](https://buymeacoffee.com/mractr)

Support continued development through [Buy Me a Coffee](https://buymeacoffee.com/mractr).

## License

MIT © Chatar Abdelilah
