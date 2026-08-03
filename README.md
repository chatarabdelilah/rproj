# rproj

[![crates.io](https://img.shields.io/crates/v/rproj.svg)](https://crates.io/crates/rproj)
[![license](https://img.shields.io/crates/l/rproj.svg)](#license)

rproj is a guided CLI that provisions a Windows machine for Roblox/Luau development and scaffolds production-ready projects.

On a fresh PC it can install the development tools you need, configure them, and generate a project that is ready to open in Roblox Studio or VS Code. On machines that are already configured, it skips setup and goes straight to creating projects.

## Features

* Provision a Windows machine with the Roblox development toolchain
* Scaffold Roblox/Luau projects with sensible defaults
* Configure tools instead of only installing them
* Guide package selection with explanations
* Generate optional quality gates and CI
* Upgrade generated configuration as rproj evolves

## Requirements

* Windows
* winget
* Rust (for installation via Cargo)

## Installation

cargo install rproj

Quick Start

Create a new project:

rproj new my-first-game

The first run offers to install any missing applications, CLI tools, Studio plugins, and editor extensions before asking about your project.

Future runs skip machine setup unless you explicitly reconfigure it.

### Commands

Command	Description
rproj	Show the welcome screen.
rproj new <name>	Create a new project.
rproj setup	Configure the current machine.
rproj setup <tool>	Configure a single tool for the current project.
rproj configure [tool]	Modify a tool’s configuration interactively.
rproj upgrade	Update generated configuration in an existing project.
rproj watch	Install missing project tools and start the Rojo watcher.
rproj copy	Copy everything under src/ to the clipboard.
rproj info	Browse the catalog of supported tools and packages.
-v, --verbose	Show detailed output.
-V, --version	Print the version.

## What gets generated

A project typically looks like this:

my-first-game/
├── default.project.json
├── rokit.toml
├── wally.toml
├── selene.toml
├── stylua.toml
├── .luaurc
├── .gitattributes
├── rproj.toml
├── src/
│   ├── client/
│   ├── server/
│   └── shared/
├── tests/                  # optional
├── .lute/check.luau        # optional
├── .github/workflows/ci.yml# optional
└── .vscode/settings.json

The exact output depends on the features you choose during project creation.

## Supported tooling

rproj can work with:

* System applications (Git, VS Code, Roblox Studio, Blender, Figma)
* CLI tools (Rojo, Wally, Selene, StyLua, luau-lsp, Lute, Mantle, Tarmac, and others)
* Roblox Studio plugins
* VS Code extensions
* Wally packages

Run rproj info to browse the complete catalog.

## Environment variables

Variable	Description
RPROJ_NO_EMOJI=1	Replace emoji with plain text markers.
RPROJ_NO_UPDATE_CHECK=1	Disable update checks.

## Building from source

cargo build
cargo test
cargo clippy --all-targets -- -D warnings

## License

MIT © Chatar Abdelilah