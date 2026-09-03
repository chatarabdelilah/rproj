# rproj

[![crates.io](https://img.shields.io/crates/v/rproj.svg)](https://crates.io/crates/rproj)
[![license](https://img.shields.io/crates/l/rproj.svg)](#license)

`rproj` is a guided Windows CLI for setting up Roblox/Luau development and creating consistent projects.

> **Status: pre-release alpha.** The main workflows work, but the command surface, generated files, and `rproj.toml` schema may change without migration support. Use it for real projects only when you are willing to review generated changes and update with the tool.

## What rproj does

rproj connects two layers that are usually assembled by hand:

1. **Machine setup** installs and configures development applications, command-line tools, Roblox Studio plugins, and VS Code extensions.
2. **Project setup** turns your choices into a Roblox project with pinned tools, dependencies, editor settings, quality checks, and optional asset workflows.

It explains the available choices before applying them. It does not hide the underlying ecosystem: generated projects remain ordinary Rojo, Rokit, Wally, Luau, and Git projects that can be maintained without rproj.

## Ecosystems

| Layer | What rproj manages |
| --- | --- |
| Windows applications | Git, VS Code, Roblox Studio, Blender, Figma, and related prerequisites |
| CLI toolchain | Rokit plus pinned tools such as Rojo, Wally, Selene, StyLua, luau-lsp, Lute, Asphalt, and Tungsten |
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
| `rproj` | Show the product overview and command list |
| `rproj new <name>` | Set up the machine when needed and create a project |
| `rproj setup` | Review or change machine-level tools |
| `rproj setup <tool>` | Set up one supported tool for the current project |
| `rproj configure [tool]` | Edit supported generated configuration interactively |
| `rproj upgrade` | Re-render maintained files from the current `rproj.toml` decisions |
| `rproj watch` | Restore dependencies and start the Rojo development loop |
| `rproj copy` | Copy source files with path headers |
| `rproj info [key]` | Browse tools, packages, capabilities, and usage notes |
| `rproj --verbose ...` | Include commands and captured subprocess output |

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
|-- .lute/check.luau
|-- .github/workflows/ci.yml
|-- figma/
|-- asphalt.toml or tungsten.toml
`-- .vscode/settings.json
```

`rproj.toml` records the current project decisions used by `rproj upgrade`. During alpha, re-scaffolding is preferred when the schema changes substantially.

## Behavior and scope

- rproj asks before optional machine-level installation and project generation.
- External command output is summarized; `--verbose` exposes the underlying commands and captured output.
- Existing user-owned configuration is preserved where a command supports merging. Unreadable configuration is refused rather than replaced.
- The current implementation targets Windows. Cross-platform support is not claimed.
- rproj is a development orchestrator, not a package registry, build system, game framework, or Roblox Studio replacement.

## Development

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo package
```

The codebase separates CLI dispatch, decision modeling, catalogs, command orchestration, and filesystem/process steps. See [Architecture](https://github.com/chatarabdelilah/rproj/blob/main/docs/architecture.md), [Development plan](https://github.com/chatarabdelilah/rproj/blob/main/docs/plan.md), [UX redesign](https://github.com/chatarabdelilah/rproj/blob/main/docs/ux-redesign.md), and [Release process](https://github.com/chatarabdelilah/rproj/blob/main/docs/releasing.md).

## License

MIT © Chatar Abdelilah
