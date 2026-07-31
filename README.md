# rproj

[![crates.io](https://img.shields.io/crates/v/rproj.svg)](https://crates.io/crates/rproj)
[![license](https://img.shields.io/crates/l/rproj.svg)](#license)

A guided CLI that takes a fresh Windows PC to a working Roblox/Luau development setup, then scaffolds projects on it.

It installs the tools — Git, VS Code, Roblox Studio, Blender, and the Rokit/Rojo/Wally/Selene/StyLua toolchain — along with the Studio plugins and editor extensions, then writes a project that already has `default.project.json`, `wally.toml`, linter and formatter config, a `.gitattributes` that keeps the tree checking out cleanly on Windows, and a quality gate that CI runs unchanged.

Every choice comes with a one-line explanation and a maintenance badge, so you can pick deliberately instead of guessing — or skip past the guidance if you already know what you want.

## Requirements

Windows. Installs go through `winget`, and this is the only supported platform.

## Installation

```sh
cargo install rproj
```

## Usage

```sh
rproj new my-first-game
```

On a machine that has never been set up, this asks what to install first — system apps, CLI tools, Studio plugins, editor extensions — then asks about this project's packages. On every run after that it goes straight to the project questions.

Nothing already installed is reinstalled, and one item failing to install is a warning rather than an abort.

### Commands

| Command | |
| --- | --- |
| `rproj` | Welcome screen. Touches neither disk nor network. |
| `rproj new <name>` | Scaffold a project. `--like <setup>` reuses a saved selection, `--save-setup <name>` saves one, `--reconfigure` re-asks the machine questions. |
| `rproj setup` | Machine provisioning on its own. Optional — `rproj new` is self-sufficient. Safe to re-run. |
| `rproj setup <tool>` | Set one tool up in the current project: pin it, write its config, and explain the steps that need a human. |
| `rproj configure [tool]` | Walk through a tool's settings one at a time, explaining each, then merge them into its config file. Covers `stylua`, `selene`, `luau-lsp`. |
| `rproj upgrade` | Re-apply generated config to an existing project, so it picks up fixes shipped since. Shows the plan and asks first. |
| `rproj watch` | From inside a project: install anything missing, then run Rojo's sourcemap watcher. |
| `rproj copy` | Copy everything under `src/` to the clipboard, with path headers. |
| `rproj info` | Browse the catalog: pick a section, then type to filter. `rproj info <key>` prints one entry — what it's for, when to reach for it, and the commands to type. |
| `-v`, `--verbose` | Show every sub-process and its output. |
| `-V`, `--version` | Print the version. |

### What `rproj new` asks

Four questions, and each one narrows the next:

1. **Dependencies** — Wally (recommended), git submodules, or none.
2. **Packages** — guided, one prompt per category, or one expert list.
3. **What should this project do?** — capabilities, with the tool named:

   ```text
   [x] lint       Catch bugs and risky patterns before they ship   (Selene)
   [x] format     One consistent code style, applied automatically (StyLua)
   [x] typecheck  Strict Luau, so type errors are errors           (luau-lsp)
   [ ] test       Write and run tests against your game's own code (TestEZ)
   [x] gate       One command that runs every check above          (Lute)
   [ ] ci         Run that same gate on GitHub for every push      (GitHub Actions)
   ```

4. **A summary**, not a fourth picker. Every file says why it's there:

   ```text
   Creates
     src                    every Rojo project has this
     default.project.json   every Rojo project has this
     rokit.toml             pins 4 tool versions so teammates get the same ones
     wally.toml             this project uses Wally
     selene.toml            you chose lint
     .lute/check.luau       you chose gate
   ```

You never pick tools or files directly — those follow from what the project does. "Should I pin Selene?" and "generate `selene.toml`?" are the same decision as "do I want linting", asked two levels below where you think. Pick `customize` at the summary to drop anything, including down to just `src/` and `default.project.json`.

CI is off by default, since it changes what happens on a push. Testing is off by default too — the runner is your choice, and none is a valid answer.

### Choosing packages

`rproj new` offers a **guided** walkthrough — one prompt per category (state management, UI, data and profiles, utilities) — or an **expert** flat list across the whole catalog. Under git submodules, packages that can't be vendored are named up front rather than silently dropped.

Guided mode adds companion packages for you: pick `react` and you get `react-roblox`; pick `charm` alongside Vide and you get `vide-charm` rather than the React binding.

Dependencies come from Wally by default, or from git submodules if you prefer vendoring.

## What gets scaffolded

```text
my-first-game/
├── default.project.json     server/client/shared tree + place properties
├── rokit.toml               pins for the CLI tools this project selected
├── wally.toml               dependencies, split by realm
├── selene.toml              linter config
├── stylua.toml              formatter config
├── .luaurc                  strict mode
├── .gitattributes           pins the working tree to LF
├── rproj.toml               what you picked, and how
├── src/{shared,server,client}/
├── tests/                   if TestEZ was selected
├── .lute/check.luau         the quality gate
├── .github/workflows/ci.yml runs the same gate, unchanged
└── .vscode/settings.json    sourcemap, Studio bridge, formatter path
```

Every line above is a choice, not a guarantee — that list is what a project looks like when you say yes to everything.

The gate — `lute run check` — regenerates the sourcemap, type-checks with luau-lsp, lints with Selene, and verifies formatting with StyLua. CI runs the same script, so local and CI results cannot drift. Each step is only emitted if the project selected its tool.

## The catalog

Run `rproj info` for the full listing, or `rproj info <key>` for detail on any entry.

- **System apps** — Git, VS Code, Roblox Studio, Roblox, Blender, Figma
- **CLI tools** — rojo, wally, wally-package-types, selene, stylua, lute, luau-lsp, tarmac, mantle
- **Studio plugins** — Rojo, UI Labs, Hoarcekat, Resurface, luau-lsp companion, Roblox's Blender add-on
- **VS Code extensions** — luau-lsp, vscode-rojo, selene, StyLua, roblox-ui, TestEZ Companion, and themes including Catppuccin
- **Wally packages** — React, Vide, Fusion, Reflex, Charm, Lyra, ProfileStore, TestEZ, Janitor, Ripple, Remo, Promise, t, Sift, and their bindings

Maintenance badges are hand-curated rather than checked live.

## Environment

| | |
| --- | --- |
| `RPROJ_NO_EMOJI=1` | Use `+` / `-` / `!` markers instead of emoji — for log files, CI transcripts, or screen readers. |
| `RPROJ_NO_UPDATE_CHECK=1` | Don't check crates.io for a newer version. |

## Building from source

```sh
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
```

## License

MIT © Chatar Abdelilah
