# Generated project review — September 20, 2026

## Applied fixes

The normal quality gate regenerates the sourcemap from `default.project.json`.
Jest projects therefore mount the disk directory `DevPackages` as
`ReplicatedStorage.devPackages` in both Rojo documents. Only `jest.project.json`
contains test trees, omits Lighting, and enables `ServerScriptService.LoadStringEnabled`.
Development libraries consequently also appear in builds of the default project.
Existing user tests must change uppercase `DevPackages` imports after upgrade;
rproj does not rewrite user source.

Generated CI performs quality checks without cloud credentials. To run cloud
tests, explicitly set repository variable `JEST_OPEN_CLOUD=true`, then configure
the key, universe, and dedicated test place described in the README. Both the
credential check and test invocation use the same condition. A missing credential
then fails the job. Exiting successfully from a separate credential step cannot
skip a later test step. Local `rproj test` uses Studio; an explicit
`--backend open-cloud` argument selects cloud execution.

The ignore audit adds generated place builds, Studio place locks, coverage, and
local environment files, with an exception for `.env.example`. It retains the
existing package-directory, sourcemap, editor-state, temporary type-definition,
asset-tool debug, and Blender-backup exclusions. `wally.lock`, tool manifests,
Rojo projects, `.lute/check.luau`, Jest configuration, submodule pointers/link
files, and model assets remain trackable. Existing local `.vscode/` and
`rproj.toml` exclusions are unchanged. Custom build/report output names may need
project-specific rules; there is no safe wildcard for every possible output.

## Lessons from the supplied sources

| Source | Recommendation |
| --- | --- |
| [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and [SemVer](https://semver.org/spec/v2.0.0.html) | Use an Unreleased section with Added/Changed/Fixed headings for human-readable changes. Choose versions when preparing a release; do not infer them from commit count. rproj already has version alignment and per-release notes. A generated game changelog could be an optional artifact later. |
| [KnitTemplate](https://github.com/littensy/KnitTemplate) and the supplied CI | Reuse the separation of source, assets, dependencies, quality checks, and Rojo build validation. rproj already restores submodules and fetches Roblox definitions. Do not copy old Aftman/action versions or impose Knit on projects using another architecture. |
| The supplied Slither package manifest | TypeScript compilation, pnpm lockfiles, and Mantle environment deployment belong to an explicit roblox-ts workflow. They are not drop-in dependencies for the current Luau scaffold. Keep environment-specific deployment optional. |
| [RbxNet](https://github.com/roblox-aurora/rbx-net) / [Vorlias](https://github.com/Vorlias) | RbxNet supports both Luau and TypeScript and is a plausible optional networking catalog entry. Before adding it, verify its current Wally dependency graph, exported API, and a runnable client/server example. A profile URL alone is not an integration specification. |
| [Lune](https://github.com/lune-org/lune) and [upload-release-assets](https://github.com/AButler/upload-release-assets) | Lune is useful for standalone Luau automation, but does not replace Studio tests and is distinct from the current Lute gate. Release-asset upload can be an optional release workflow after build targets are defined; publishing a GitHub release is separate from deploying a Roblox place. |

## Vide pixel helper

The supplied helper is a useful candidate for a Catalog example, not a required
dependency or automatic scaffold file. Before adopting it:

1. Guard a nil camera and zero viewport dimensions before division or logarithms.
2. Reconnect when `workspace.CurrentCamera` changes and disconnect both subscriptions on cleanup.
3. Give scale state an explicit owner; multiple `usePx` calls currently write the same module-global source.
4. Define rounding deliberately: the supplied loop selects the next higher scale point even on exact matches, rather than the nearest point.
5. Validate finite positive manual scale values and decide whether manual scale overrides viewport updates.

No networking package, alternative runtime, release uploader, or unverified UI
helper was added to every generated project as part of this fix.
