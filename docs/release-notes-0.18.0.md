# rproj 0.18.0 — Jest execution choice and project UI refinements

Alpha release candidate. Not published yet.

## Changes

- Choosing Jest now opens a Local Studio/Open Cloud picker. The choice survives
  project records and saved setups and selects the default backend for `rproj test`.
  Local projects generate CI without test steps; cloud projects generate credential
  validation and cloud tests. Only local creation provisions the Studio runner plugin.
- Jest imports resolve through the normal quality-gate sourcemap:
  `DevPackages` on disk mounts as `ReplicatedStorage.devPackages` in both Rojo
  project files. The Jest project omits Lighting and enables `LoadStringEnabled`.
- Catalog rows show names only; descriptions remain in Overview/Details. Enter
  opens groups without focusing read-only entries. Page Up/Down and Ctrl+Home/End
  scroll details without changing focus; the mouse wheel now scrolls details too.
- New Project opens on Review, where packages, capabilities, files, naming, and
  the final Create action are visible immediately. Start point still opens Guided,
  Expert, and saved-setup choices.
- Projects now offers Edit Packages & Capabilities for recorded projects. It adds
  choices without removing existing ones, preserves unrelated Wally dependencies
  and comments, regenerates managed files and test wiring, and refuses an external
  `rproj.toml` edit made while the editor was open.
- `rproj test` reuses an already complete Wally package tree instead of running a
  destructive `wally install` while Watch may be observing it. Missing selected
  aliases still trigger normal dependency recovery.
- TestEZ always includes `testez-companion.toml`, so installing the extension later
  needs no repair step. Generated Rojo trees explicitly class StarterPlayer and
  StarterPlayerScripts.
- Projects > Configure Tools stays in Ratatui. Explicit changes are reviewed before
  saving; unedited settings and unrelated values are preserved. Cancellation and
  external edits are protected. Direct CLI configure retains its prompt interface.
- Generated ignore rules now include place builds, Studio place locks, coverage,
  and local environment files, while preserving `.env.example`, model assets,
  dependency lockfiles, and tool/project configuration.

## Existing projects

Run `rproj upgrade` after installing this version. Change your own Jest imports
from `ReplicatedStorage.DevPackages` to `ReplicatedStorage.devPackages`; upgrade
does not rewrite user source. Development libraries now also appear in builds
of the default project.

Existing `[capabilities] test = "jest-roblox"` records retain Local Studio.
Existing projects can add Testing from Projects > Edit Packages & Capabilities.
Stop an active Watch before saving package additions. To select cloud execution
manually, use `test = "jest-roblox-open-cloud"` and run `rproj upgrade`. The previous unreleased
`JEST_OPEN_CLOUD` repository-variable switch is superseded by this project choice.
Explicit CLI `--backend` arguments still override an individual test invocation.

Cloud projects need secret `ROBLOX_OPEN_CLOUD_API_KEY` and variables
`ROBLOX_UNIVERSE_ID` and `ROBLOX_PLACE_ID` in GitHub Actions. The key needs
`universe-places:write`, `universe.place.luau-execution-session:write`, and
`memory-store.sorted-map:read`/`memory-store.sorted-map:write`. Use a dedicated test
place because the runner uploads its test build there. Local cloud invocations
need the same credentials in the environment. Missing cloud credentials fail CI.

## Verification and limits

The current implementation passed 404 ordinary tests, formatting, and clippy.
The installed real Rojo validated every built-in template variant. Earlier release
candidate work passed CodeRabbit review and Windows stable/Rust 1.89/package CI on
both reviewed and merged heads, and the real pinned Jest Roblox 0.3.24/Studio
regression passed eight package examples. Current review and CI evidence is
recorded in `docs/release-audit.md` and the implementing PR.

Nineteen prerequisite-dependent tests remain ignored in ordinary runs. Live Open
Cloud, fresh Linux generated-project execution, and fresh Windows provisioning
remain unverified. Settings replacement provides atomic visibility, not a crash
durability guarantee. This release adds no dependencies.
