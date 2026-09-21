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
  scroll details without changing focus.
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
To select cloud execution for an existing project, use
`test = "jest-roblox-open-cloud"` and run `rproj upgrade`. The previous unreleased
`JEST_OPEN_CLOUD` repository-variable switch is superseded by this project choice.
Explicit CLI `--backend` arguments still override an individual test invocation.

Cloud projects need secret `ROBLOX_OPEN_CLOUD_API_KEY` and variables
`ROBLOX_UNIVERSE_ID` and `ROBLOX_PLACE_ID` in GitHub Actions. The key needs
`universe-places:write`, `universe.place.luau-execution-session:write`, and
`memory-store.sorted-map:read`/`memory-store.sorted-map:write`. Use a dedicated test
place because the runner uploads its test build there. Local cloud invocations
need the same credentials in the environment. Missing cloud credentials fail CI.

## Verification and limits

The implementation passed 388 ordinary tests, formatting, clippy, CodeRabbit
review, and Windows stable/Rust 1.89/package CI on both reviewed and merged heads.
The real pinned Jest Roblox 0.3.24/Studio regression passed eight package examples.
Release-candidate verification is recorded in `docs/release-audit.md` and its PR.

Nineteen prerequisite-dependent tests remain ignored in ordinary runs. Live Open
Cloud, fresh Linux generated-project execution, and fresh Windows provisioning
remain unverified. Settings replacement provides atomic visibility, not a crash
durability guarantee. This release adds no dependencies.
