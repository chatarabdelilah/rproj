# rproj

[![crates.io](https://img.shields.io/crates/v/rproj.svg)](https://crates.io/crates/rproj)
[![license](https://img.shields.io/crates/l/rproj.svg)](#license)
[![Buy Me a Coffee](https://img.shields.io/badge/Support-Buy_Me_a_Coffee-FFDD00?logo=buy-me-a-coffee&logoColor=000)](https://buymeacoffee.com/mractr)

`rproj` is a guided Windows CLI for setting up Roblox/Luau development and creating consistent projects.

> **Status: pre-release alpha.** The main workflows work, but the command surface, generated files, and `rproj.toml` schema may change without migration support. Use it for real projects only when you are willing to review generated changes and update with the tool.

## What rproj does

rproj connects two layers that are usually assembled by hand:

1. **Machine setup** installs and configures development applications, command-line tools, Roblox Studio plugins, and VS Code extensions.
2. **Project setup** turns your choices into a Roblox project with pinned tools, dependencies, editor settings, quality checks, and optional asset workflows.

It explains the available choices before applying them. It does not hide the underlying ecosystem: generated projects remain ordinary Rojo, Rokit, Wally, Luau, and Git projects that can be maintained without rproj.

Run `rproj` with no arguments in a terminal to open the workspace hub. It summarizes the current directory, machine setup, saved setups, project template, and version state, then launches the selected existing command after restoring the terminal. Direct commands remain the stable interface for scripts and repeatable workflows.

## Ecosystems

| Layer | What rproj manages |
| --- | --- |
| Windows applications | Git, VS Code, Roblox Studio, Blender, Figma, and related prerequisites |
| CLI toolchain | Rokit plus pinned tools such as Rojo, Wally, Selene, StyLua, luau-lsp, Lute, jest-roblox, Asphalt, and Tungsten |
| Packages | Wally packages or Git submodules, including UI, state, networking, data, testing, and ECS libraries |
| Roblox Studio | Development plugins selected during machine setup |
| VS Code | Recommended extensions and project-scoped settings |
| Generated project | Rojo mapping, source tree, dependency manifests, lint/format/type configuration, tests, CI, and asset configuration |

Use `rproj info` to browse the complete current catalog and `rproj info <key>` to inspect one entry.

## Requirements

- Windows with `winget`
- Rust and Cargo for installation from crates.io or source
- Network access while installing tools and packages

## Install

```powershell
cargo install rproj
```

The repository version can be installed with:

```powershell
git clone https://github.com/chatarabdelilah/rproj.git
cd rproj
cargo install --path .
```

## First project

```powershell
rproj new my-first-game
```

On a new machine, rproj first offers the missing machine-level tools. It then asks how the project should manage packages, which packages and capabilities it needs, and which generated files to keep. The final summary is reviewable before anything is created and lists the relevant `rproj configure` commands for the chosen project.

After creation:

```powershell
cd my-first-game
rproj watch
```

`rproj watch` restores missing project dependencies, regenerates the Rojo sourcemap, and starts the development watcher.

## Commands

| Command | Purpose |
| --- | --- |
| `rproj` | Open the interactive workspace hub; print a plain overview when redirected |
| `rproj new <name>` | Set up the machine when needed and create a project |
| `rproj setup` | Review or change machine-level tools |
| `rproj setup <tool>` | Set up one supported tool for the current project |
| `rproj configure [key]` | Configure a project tool, or choose what to configure |
| `rproj configure project` | Edit or reset the global Rojo tree inherited by future projects |
| `rproj upgrade` | Re-render maintained files from the current `rproj.toml` decisions |
| `rproj watch` | Restore dependencies and start the Rojo development loop |
| `rproj test [args]` | Restore dependencies and run the selected test runner; pass remaining arguments through |
| `rproj copy` | Copy source files with path headers |
| `rproj info [key]` | Open the TUI Catalog or print one entry when a key is supplied |
| `rproj --verbose ...` | Include commands and captured subprocess output |

The hub keeps project actions visible when they cannot run, along with the exact missing file or directory. It examines only the current directory: it does not search for or manage a projects-root library. Select New Project to enter a name, Catalog to browse the ecosystem, or another task to leave the hub and run the corresponding command normally. Use arrows and Enter to navigate, Esc to leave, and `?` for help.

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

Testing is optional. Wally projects can choose either **Jest Roblox** or **TestEZ**; git-submodule and dependency-free projects use TestEZ because Jest Roblox is distributed as Wally development packages. The choice is written explicitly to `rproj.toml`, and `rproj upgrade` never migrates one runner to the other.

Run the selected runner through:

```powershell
rproj test
rproj test -t inventory
```

TestEZ runs through `lute test`. Jest Roblox installs the official `Jest` and `JestGlobals` packages under `[dev-dependencies]`, keeps them and the test mounts out of production `default.project.json`, and builds tests through the generated `jest.project.json`. The default local run uses jest-roblox's hidden `studio-cli` backend and requires Roblox Studio to be installed, logged in, and accompanied by `JestRobloxRunner.rbxm`. rproj installs or refreshes that plugin when a Jest project is created; rerun `rproj setup` if installation was interrupted. Explicit backend and workspace arguments are forwarded to jest-roblox unchanged.

Jest CI uses Open Cloud. Add repository secret `ROBLOX_OPEN_CLOUD_API_KEY` and repository variables `ROBLOX_UNIVERSE_ID` and `ROBLOX_PLACE_ID`. The API key needs `universe-places:write`, `universe.place.luau-execution-session:write`, and `memory-store.sorted-map:read`/`memory-store.sorted-map:write`; the last pair carries streamed GitHub Actions results. CI fails with the missing names instead of silently skipping tests. Queue scopes are unnecessary for rproj's generated single-session CI. Workspace or parallel sharding modes passed through `rproj test [args]` additionally require `memory-store.queue:add`, `memory-store.queue:dequeue`, and `memory-store.queue:discard`.

`jest.config.json` is merge-managed: rproj owns `backend`, `rojoProject`, `jestPath`, and `test.projects`, while preserving other valid JSON fields for filters, coverage, snapshots, and runner options. `jest.project.json` is fully generated from the current production project and is refreshed by `rproj test`, `rproj watch`, and `rproj upgrade`.

## Project template

Run `rproj configure project` to customize the `default.project.json` used by projects created afterward. It opens a built-in, keyboard-driven Explorer; it never launches another editor. Existing projects are never changed.

Use the arrow keys to navigate, Enter to edit, Tab to switch panes, and type in searchable class and property pickers. `A` adds an instance, `F2` renames, `D` duplicates, `M` reparents, Delete removes, and Ctrl+Z/Ctrl+Y undo and redo. The Inspector provides typed controls for common Roblox properties and attributes plus common project settings. Ctrl+E opens the complete JSON in rproj's internal text editor for uncommon Rojo values and advanced settings. Press `?` in the editor for the full key list.

Ctrl+S checks the JSON, rproj-owned paths, and every reachable plain, Wally, and git-submodule mount combination with Rojo before atomically saving it under the rproj configuration directory. Rojo must resolve from the current project or Rokit's global manifest; `rokit add --global rojo` installs the global fallback. Validation failures leave the draft open and the last valid saved template untouched. A malformed saved file opens directly in JSON repair mode. Ctrl+R restores the built-in template after confirmation.

Compound values use comma-separated components in the Inspector; rproj writes their required Rojo representation. UDim offsets must be signed 32-bit whole numbers, and numeric inputs must be finite. If an older draft contains a malformed UDim, UDim2, Rect, or CFrame attribute, re-enter that value in the Inspector before saving; existing templates are not silently rewritten.

The project name, DataModel root, `src/shared`, `src/server`, and `src/client` mounts are visible but locked because rproj owns them. Package, module, server-package, development-package, and test mount names and paths are reserved because those directories depend on each new project's choices. Custom `$path` entries may target only the always-created source directories; use static instances and properties for other additions. The saved template is `<rproj config>/templates/default.project.json`.

## Behavior and scope

- rproj asks before optional machine-level installation and project generation.
- The project directory is created only after the final creation confirmation. Cancelling project choices leaves no project directory; if another process creates the destination while you answer, creation refuses to reuse it.
- External command output is summarized; `--verbose` exposes the underlying commands and captured output.
- Existing user-owned configuration is preserved where a command supports merging. Unreadable configuration is refused rather than replaced.
- The current implementation targets Windows. Cross-platform support is not claimed.
- rproj is a development orchestrator, not a package registry, build system, game framework, or Roblox Studio replacement.

## Development

### Diagnostic logs (source development)

The source build writes a local `.txt` log for each command and prints its path
on stderr when finished. Attach that file when reporting a problem, after checking
it for private paths or names. Logs include choices, steps, and errors, but omit
credentials-related fields, raw keystrokes, source/clipboard contents, and raw
external-tool output. Nothing is uploaded automatically.

See [diagnostic logs](https://github.com/chatarabdelilah/rproj/blob/main/docs/diagnostic-logs.md)
for location, limits, and opt-out. This is not yet included in published 0.12.2.

### Checks

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo package
```

The codebase separates CLI dispatch, decision modeling, catalogs, command orchestration, and filesystem/process steps. See [Architecture](https://github.com/chatarabdelilah/rproj/blob/main/docs/architecture.md), [Development plan](https://github.com/chatarabdelilah/rproj/blob/main/docs/plan.md), [UX redesign](https://github.com/chatarabdelilah/rproj/blob/main/docs/ux-redesign.md), and [Release process](https://github.com/chatarabdelilah/rproj/blob/main/docs/releasing.md).

## Support

Support continued development through [Buy Me a Coffee](https://buymeacoffee.com/mractr).

## License

MIT © Chatar Abdelilah
