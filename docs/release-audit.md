# Release-Hardening Audit

Updated September 6, 2026. Published baseline: **0.12.2**, alpha. No new release candidate is selected. Scope: confirmed defects in existing workflows; no new features, model importer, embedded tools, or frontend migration.

The published package and annotated `v0.12.2` tag correspond to commit `f71bf4e`; the [GitHub release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.12.2) is a prerelease. The automated Jest regression was merged afterward in [PR #7](https://github.com/chatarabdelilah/rproj/pull/7), at `a855a2f`. It is present on main, not in the published 0.12.2 archive; no runtime code or version changed in that PR.

## Confirmed Defects And Fixes

1. **Project creation touched the destination before confirmation.** Escape could leave an empty directory, and explicit Cancel recursively removed it even if another process had added files. Creation now happens after confirmation, uses an exclusive directory creation, and never deletes the destination on cancellation. Unit tests cover an existing directory and missing parents; a live regression covers another writer creating a sentinel while the prompt is open, for both Create and Cancel.
2. **Some Inspector values did not match Rojo's serialization.** UDim/UDim2/Rect now use explicit nested representations with integer offsets; CFrame attributes use position/orientation fields. Reopening values flattens components for the existing input controls. Unit round trips and a real-Rojo validation test cover the fix. NaN/infinity and fractional/out-of-range offsets are rejected before changing the draft.
3. **Live tests were stale and unsafe to rerun against an occupied project root.** They selected TestEZ in the old package picker, assumed VS Code was selected, manually parsed the wrong global-config path, and deleted fixed-name directories. The harness now follows current capability/runner prompts, reads actual TOML via the configured directory API, checks provisioning up front, and uses unique scratch names without removing pre-existing directories.
4. **Jest Roblox projects could not run their selected test runner.** Rokit 1.2 rejected untrusted project-local sources, the generated runner invoked `jest-roblox` while Rokit installs `jest-roblox-cli`, and `test.projects` used DataModel-path strings that 0.3.24 interprets as configuration-file paths. rproj now trusts selected sources before every add, invokes the installed executable, and emits inline project entries with filesystem include globs.

## Evidence

| Check | Result |
| --- | --- |
| Ordinary suite with saved-setup regression | 273 passed; 15 deliberately ignored; 288 discovered (253 unit + 35 integration) |
| Formatting and clippy | Passed locally |
| Existing live project suite | Seven passed in 38.39 seconds on September 5 after harness corrections |
| Concurrent-destination live regression | Passed for both cancellation and confirmation |
| Existing real-Rojo template checks | Passed with Rojo 7.7.0 |
| Compound Inspector output | Passed real-Rojo validation across the template matrix |
| Live Jest Roblox execution | `jest-roblox-cli` 0.3.24, Studio CLI backend: three generated starter specs passed in 11.58 seconds on September 6 |
| Automated live Jest regression, PR #7 | Reviewed test passed in 26.84 seconds on September 6: scaffolded Wally + Jest in a unique temporary directory, verified project-local pins and three passing starter specs, then broke the shared spec and verified exit code 1, the assertion failure, and two passing / one failing test. Temporary project removed. |
| Automated saved-setup replay | Passed in 20.07 seconds on September 6: real `new --save-setup` then `new --like` for both Wally and Git submodules, with charm/promise, lint/format, and a dropped `.gitignore`. Verified exact graphs, no repeated choices, installed package links, selected tool pins, generated/omitted files, unchanged saved setup, and cleanup. |
| Upstream badge check | Passed September 6; advisory to review Matter's Active badge (last reported push December 31, 2024); not proof that the project is abandoned |
| Previous CodeRabbit findings, PR #4 | Both addressed in baseline commit 4737c84; plugin identity and ServerPackages exclusion regression present |
| Published 0.12.2 package | cargo package --locked passed at `f71bf4e`; 73 files, 852.5 KiB uncompressed; crates.io reports 0.12.2 |
| PR CI and CodeRabbit | Historical 0.12.1 review: [PR #6](https://github.com/chatarabdelilah/rproj/pull/6). Jest regression: [PR #7](https://github.com/chatarabdelilah/rproj/pull/7), CodeRabbit reported no actionable findings at `2496a42`; Windows stable, Rust 1.89, and package checks passed. [Post-merge main CI](https://github.com/chatarabdelilah/rproj/actions/runs/34038222655) passed at `a855a2f`. |

Reproduction commands:

```powershell
cargo fmt --all --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
$env:RPROJ_TEST_TIMEOUT = '180'
cargo test --locked --test live jest_starter_specs_pass_and_report_failure -- --ignored --test-threads=1 --nocapture
cargo test --locked --test live saved_setup_replays_workflow_packages_capabilities_and_dropped_files -- --ignored --test-threads=1 --nocapture
cargo test --locked --test live -- --ignored --test-threads=1
cargo test --locked steps::rojo::tests:: -- --ignored --test-threads=1
cargo test --locked guided_compound_values_pass_real_rojo_validation -- --ignored
cargo test --locked badges_do_not_contradict_upstream -- --ignored --nocapture
cargo package --locked
```

The live project suite uses unique directories under the configured projects root and shares real Rokit/Wally caches. The Jest regression requires Studio and JestRobloxRunner to be installed; normal scaffolding refreshes that plugin. Only its temporary project is isolated and removed. Run serially on an explicitly provisioned machine, not as an unattended installer on a fresh host. The regression remains ignored by ordinary CI and must be invoked explicitly.

## Remaining Limits

The saved-setup regression uses the real configured projects root and setup directory, reserving a unique setup filename before the CLI writes it. Both projects and the reserved setup are removed, including partial runs; a newly created empty setup directory may remain. Existing setups and machine configuration are not changed. Rokit/Wally caches and trust state remain shared; this test needs provisioned tools and network, runs serially, and does not install Studio plugins. Like the Jest regression, it is ignored by ordinary CI and does not change the published 0.12.2 package.

- **Jest evidence covers the local Studio CLI backend.** The automated starter-spec pass/fail gap is closed by PR #7. This does not verify Open Cloud, an attached Studio session, or a fresh Windows installation. The older ignored `real_jest_stack_installs_validates_retypes_and_executes` test still exercises no-test success; the new live regression verifies actual execution through `rproj test`.
- **Open Cloud execution was not performed.** No audit universe/place and credentials were supplied. Generated preflight and workflow text are tested, but that is not equivalent to a current hosted run.
- **Fresh Windows provisioning was not performed.** winget/editor/Studio/plugin installation changes the machine. Existing unit coverage and inspection are not a substitute for a clean-machine acceptance test.
- **Saved-setup replay evidence covers valid Wally and Git-submodule compositions.** Missing, malformed, and incompatible saved setups still need targeted refusal-path evidence. Clipboard contents, machine provisioning/recovery, and every template keyboard path also remain gaps. No claim of beta/1.0 readiness is made.
- **Template validity remains Rojo-authoritative.** This patch repairs specific existing compound controls, not every possible explicit/future property representation. Unsupported advanced values and old malformed drafts are not automatically normalized.

## Handoff

Keep Ratatui and all external tool boundaries. The model-import experiment stays on its separate local branch and is not a dependency of this release.

Release 0.12.2 is aligned and complete. The agent handles review findings, merge, post-merge CI, merged-branch cleanup, and release alignment; the owner alone runs `cargo publish --locked` when a new package is ready. Test-only and documentation-only follow-ups do not require publication or moving an existing release tag.

Next focused hardening check: saved-setup refusal safety. Verify that missing, malformed, or incompatible setups fail before project creation or provisioning. This is an existing-workflow check, not a new feature milestone.
