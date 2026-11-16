# Project Tool Setup design

Approved November 16, 2026 against clean main `0d3f6a6`: a separate Soft
Tool → Review → Results conversation proposal. One tool per review; close waits
for confirmed completion. Native implementation requires subsequent design approval.

Tools offers Set up project tool alongside Configure tool settings. Captured
project identity survives navigation, closure and unrelated drafts. Project
Rokit tools, Blender scene creation and machine-wide guidance are distinct.
Review names pin/file/cache/trust effects and manual steps, preserves existing
files/pins, and never invents a resolved version. Source changes require Refresh.
Same-project Watch blocks Apply until separate Stop acknowledges completion.
Another project's Watch remains unchanged. Operations are serialized.

Use local fixtures only. No installation, real project access, Rust, worker,
API, configuration-format or release changes. Native previews remain paused.
Existing wireframes remain unchanged. Published version remains 0.22.0.

## Authority and implementation concern

The fixture authority is `src/commands/setup.rs`, `src/steps/toolchain.rs`,
`src/steps/asphalt.rs`, `src/steps/tungsten.rs`, `src/steps/blender.rs` and the
shared tool catalog. Asphalt/Tungsten create configuration only when absent,
prefer existing figma/exports, otherwise create assets/. Blender preserves an
existing scene and creates a missing one through the installed executable.
The project record is not rewritten; setup is not capability selection.

The current toolchain helper can return Ok after warning about a failed Rokit
add. The future native executor must verify each result explicitly. This
prototype represents a failed installation and created configuration separately;
it never reports overall success from log wording or configuration creation.

## Validation

See [acceptance](desktop-project-tool-setup-acceptance.md) for checks and limits.
