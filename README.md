# rproj

[![crates.io](https://img.shields.io/crates/v/rproj.svg)](https://crates.io/crates/rproj)
[![license](https://img.shields.io/crates/l/rproj.svg)](LICENSE)

**Guided bootstrap-to-game-dev CLI for Roblox.**

Takes a fresh Windows PC all the way to a working Roblox/Luau development setup, then scaffolds projects on top of it — and explains every tool and package as it goes, so you end up knowing why your setup looks the way it does.

```text
🎮  rproj 0.1.0 - guided bootstrap-to-game-dev CLI for Roblox

    Takes a fresh PC all the way to a working Roblox dev setup, then
    scaffolds projects on top of it. Explains every tool and package
    as it goes, so you end up knowing why your setup looks like it does.

📚  Commands

  🚀  rproj new <name>        Scaffold a project. Sets your machine up first if it
                              hasn't been, then asks only about this project's packages.
                              --like <setup> reuses a saved selection, --save-setup saves one

  🔧  rproj setup             Install or change the machine-wide tools: system apps, CLI
                              tools, Studio plugins, editor extensions

  🔩  rproj configure [tool]  Walk through a tool's settings one at a time - StyLua,
                              Selene, luau-lsp - explaining each, then write them out

  🔄  rproj upgrade           Re-apply rproj's generated config to a project you already made,
                              so it picks up fixes shipped since it was scaffolded

  👀  rproj watch             Resume the dev loop: install what's missing, watch the sourcemap

  📋  rproj copy              Copy every file under src/ to the clipboard, with path headers

  📖  rproj info [key]        Look up what a tool or package does, and how to use it

✨  New here?  rproj new my-first-game  sets your machine up and
    scaffolds your first project in one go.
```

---

## Why

Setting up a Roblox project properly means installing Git, VS Code, Roblox Studio, and Blender; installing Rokit and then Rojo, Wally, Selene, StyLua, Lute and luau-lsp through it; installing the Rojo and luau-lsp Studio plugins; installing four or five VS Code extensions; and then writing `default.project.json`, `wally.toml`, `selene.toml`, `stylua.toml`, `.luaurc`, `.gitattributes`, a sourcemap watcher, and a CI workflow that agrees with all of them.

Each of those has a gotcha that costs an evening the first time you meet it. `rproj` knows them and gets them right by construction:

- Wally deletes `Packages/` when a project has no dependencies, and strips `export type` lines out of link files on every install.
- ProfileStore is a **server-realm** package — listed under `[dependencies]` it doesn't just land in the wrong folder, `wally install` refuses to resolve it at all.
- Without a `.gitattributes` pinning LF, a fresh Windows clone fails `stylua --check` on every file it has.
- luau-lsp's Studio-plugin bridge is **off** by default, which is why parts you create in Studio never autocomplete in your editor.
- Vide's API is a mixed table by construction, so selene's `mixed_table` lint fails every UI file a Vide project will ever contain — and selene exits 1 on warnings, not just errors.

Every one of those is a real bug that was reproduced and fixed here, not a hypothetical. See [docs/architecture.md](docs/architecture.md) §7 for the full list.

## Requirements

- **Windows.** Installs go through `winget`. This is the only supported platform.
- **Rust 1.89+** to build. Edition 2024 sets a floor of 1.85, but `notify-rust` declares 1.89 and is the highest in the dependency tree.

## Install

```powershell
cargo install rproj
```

Or from source, to get unreleased changes:

```powershell
cargo install --git https://github.com/chatarabdelilah/rproj --locked
```

## Quick start

```powershell
rproj new my-first-game
```

That's the whole thing. On a machine that has never been set up it asks what to install first (system apps → CLI tools → Studio plugins → VS Code extensions), then asks about *this project's* packages, then scaffolds. On every run after that it skips straight to the project questions — the machine answers almost never change, and re-asking four multi-selects per project was friction with no payoff.

Nothing already installed is ever reinstalled, and a single item failing to install is a warning, not an abort.

## Commands

| Command | What it does |
| --- | --- |
| `rproj` | Welcome screen. Touches neither disk nor network. |
| `rproj new <name>` | Scaffolds `<RobloxProjects>/<name>`. Provisions the machine inline only on a machine that's never been set up, or with `--reconfigure`. |
| `rproj new <name> --like <setup>` | Reuses a package selection you saved earlier instead of asking. |
| `rproj new <name> --save-setup <name>` | Saves this project's selection and workflow for later `--like` reuse. |
| `rproj setup` | Machine provisioning on its own, without creating a project. Optional — `rproj new` is self-sufficient. Safe to re-run. |
| `rproj configure [tool]` | Walks through one tool's settings, explaining each before prompting, then merges them into its config file. Covers `stylua`, `selene`, `luau-lsp`, `stylua-vscode`. |
| `rproj upgrade [--yes]` | Re-applies rproj's generated config to a project you already made, so it picks up fixes shipped since. Shows the plan and asks first. |
| `rproj watch` | Run from inside a project: installs anything missing (including fetching submodules a plain `git clone` left empty), then blocks on Rojo's sourcemap watcher. |
| `rproj copy` | Concatenates everything under `src/` with `// --- path ---` headers to the clipboard. |
| `rproj info [key]` | With no key, a compact listing of the whole catalog. With one, full detail: what it's for, when to reach for it, the commands to type, and the gotchas. |
| `-v` / `--verbose` | Global. Shows every sub-process and its full output. Without it, sub-process output appears only when a step fails. |

### Choosing packages

`rproj new` offers two modes:

- **Guided** — one prompt per category, in order: State management → UI → Data & profiles → Testing → Utilities. State/UI/Data are single-pick (you don't run two UI frameworks); Testing and Utilities are multi-pick. Guided mode adds companion packages automatically — pick `react` and you get `react-roblox`, pick `charm` alongside Vide and you get `vide-charm` rather than the React binding.
- **Expert** — one flat multi-select across the whole catalog, no categories, no automatic companions.

Then: **Wally** (default) or **git submodules**. Submodules clone each package's repo into `modules/submodules/` and generate link files, with dependencies resolved transitively — submodules have no dependency resolution of their own, so a selection that isn't closed over its requirements mounts packages next to siblings that aren't there.

## What gets scaffolded

```text
my-first-game/
├── default.project.json     server/client/shared tree + place properties
├── rokit.toml               project-local pins for every selected CLI tool
├── wally.toml               [dependencies] / [server-dependencies], split by realm
├── selene.toml              std = "roblox" (or "roblox+testez")
├── stylua.toml              Luau syntax
├── .luaurc                  languageMode = "strict"
├── .gitattributes           pins the working tree to LF
├── .gitignore
├── rproj.toml               what you picked, and how
├── sourcemap.json
├── src/
│   ├── shared/hello.luau
│   ├── server/hello.server.luau
│   └── client/hello.client.luau
├── tests/                   TestEZ only
│   ├── .luaurc              TestEZ's globals, for luau-lsp
│   └── {shared,server,client}/hello.spec.luau
├── .lute/check.luau         the quality gate
├── .github/workflows/ci.yml runs the same gate, unchanged
└── .vscode/settings.json    sourcemap, Studio bridge, StyLua path, LF
```

Plus `testez.yml` and `testez-companion.toml` if TestEZ was selected, `Packages/` (Wally) or `modules/` (submodules), and a Blender starter scene if Blender was enabled at setup.

### The quality gate

One script, `.lute/check.luau`, that you run locally with `lute run check` and CI runs unchanged — so local and CI results can't drift.

| Step | Tool | What it does |
| --- | --- | --- |
| sourcemap | rojo | Regenerates `sourcemap.json` so requires resolve against the current tree |
| analyze | luau-lsp | Fetches Roblox global types, type-checks with `LuauSolverV2`, cleans up after itself |
| lint | selene | Lints at the severity `selene.toml` sets |
| format | stylua | `--check` only — CI must not rewrite the tree it was asked to check |

A step is emitted only if the project selected its tool, so a project without StyLua gets a script that never calls it rather than a guaranteed CI failure. All four run before the script exits, so one run reports every problem instead of stopping at the first. Targets are `src`, plus `tests` when TestEZ is in play.

## The catalog

Every entry — app, CLI tool, plugin, extension, package — carries a one-line description and a maintenance badge (**Active** / **Community-stable** / **Legacy**), shown inline in every picker. Badges are hand-curated, not checked live.

**System apps** (winget): Git · VS Code · Roblox Studio · Roblox · Blender

**CLI tools** (rokit): rojo · wally · wally-package-types · selene · stylua · lute · luau-lsp · tarmac · mantle

**Studio plugins:** Rojo · Hoarcekat · luau-lsp companion · Roblox's Blender add-on

**VS Code extensions:** luau-lsp · vscode-rojo · selene · StyLua · roblox-ui · TestEZ Companion · GitHub Actions · three themes

**Wally packages**, 22 of them:

| Category | Packages |
| --- | --- |
| UI | react (+ react-roblox) · vide · fusion |
| State management | reflex (+ react-reflex) · charm (+ charm-sync, react-charm, vide-charm) |
| Data & profiles | lyra · profilestore |
| Testing | testez |
| Utilities | janitor · ripple (+ react-ripple, vide-ripple) · remo · promise · greentea · t · sift |

`rproj info <key>` prints the long version for any of them.

## Development

```powershell
cargo build
cargo test                                  # 94 tests, ~3s
cargo clippy --all-targets -- -D warnings
```

There's also a live suite that scaffolds genuine projects — real network, real `wally install`, real `rojo build`, real `lute run check` — and removes them on `Drop` however the test ends:

```powershell
cargo test --test live -- --ignored --test-threads=1     # ~69s
```

`--test-threads=1` is not optional: the tests share rokit's global manifest and wally's package cache, and two scaffolds racing on those is an irreproducible flake.

The interactive commands are tested by driving the real binary through a **pseudo-terminal** ([tests/common/mod.rs](tests/common/mod.rs)). Two things forced that: `inquire` reads the console input handle rather than stdin, so a piped `rproj configure stylua` renders its first prompt and then hangs forever; and the pty byte stream is not what the program printed — ConPTY may express a line break as `\n` or as a cursor move depending on machine load, so assertions run against a rendered screen instead of the raw bytes. Synchronisation is by expecting output, never by sleeping.

Set `RPROJ_NO_EMOJI=1` for `+` / `-` / `!` markers instead of `✅` / `➖` / `⚠️` — for a log file, a CI transcript, or a screen reader.

[docs/architecture.md](docs/architecture.md) is the real reference: data model, subsystem map, flow diagrams, the data-driven matrices behind the catalog and the gate, and §7's list of landmines with the reproduction behind each one.

## Status

v0.1.0, the first published release. Windows-only and Luau-only.

Deliberately not planned: macOS support and roblox-ts. On the roadmap: a GUI over the same core logic, and live maintenance-status checking once the catalog is large enough that hand-curation becomes a burden.

## License

[MIT](LICENSE) © Chatar Abdelilah
