# Release-Hardening Audit

Updated September 8, 2026. Published baseline: **0.13.0**, alpha, with diagnostic logging and hub-driven Ratatui creation. Its crates.io archive records `50f2e3419d9fb252d71810536944e1a5fe445a79`, matching the annotated tag and [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.13.0). SHA256: `3ea4689594edb17f5de95fa55f5382b9a42ac36e9d2be202657c8ce542c53617`. Model import, embedded tools, and frontend migration remain out of scope. See [release notes](release-notes-0.13.0.md).

The published package and annotated `v0.12.2` tag correspond to commit `f71bf4e`; the [GitHub release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.12.2) is a prerelease. The automated Jest regression was merged afterward in [PR #7](https://github.com/chatarabdelilah/rproj/pull/7), at `a855a2f`. It is present on main, not in the published 0.12.2 archive; no runtime code or version changed in that PR.

## Confirmed Defects And Fixes

1. **Project creation touched the destination before confirmation.** Escape could leave an empty directory, and explicit Cancel recursively removed it even if another process had added files. Creation now happens after confirmation, uses an exclusive directory creation, and never deletes the destination on cancellation. Unit tests cover an existing directory and missing parents; a live regression covers another writer creating a sentinel while the prompt is open, for both Create and Cancel.
2. **Some Inspector values did not match Rojo's serialization.** UDim/UDim2/Rect now use explicit nested representations with integer offsets; CFrame attributes use position/orientation fields. Reopening values flattens components for the existing input controls. Unit round trips and a real-Rojo validation test cover the fix. NaN/infinity and fractional/out-of-range offsets are rejected before changing the draft.
3. **Live tests were stale and unsafe to rerun against an occupied project root.** They selected TestEZ in the old package picker, assumed VS Code was selected, manually parsed the wrong global-config path, and deleted fixed-name directories. The harness now follows current capability/runner prompts, reads actual TOML via the configured directory API, checks provisioning up front, and uses unique scratch names without removing pre-existing directories.
4. **Jest Roblox projects could not run their selected test runner.** Rokit 1.2 rejected untrusted project-local sources, the generated runner invoked `jest-roblox` while Rokit installs `jest-roblox-cli`, and `test.projects` used DataModel-path strings that 0.3.24 interprets as configuration-file paths. rproj now trusts selected sources before every add, invokes the installed executable, and emits inline project entries with filesystem include globs.

## Evidence

### Unreleased Configuration Preservation

Three new regressions first reproduced destructive behavior in 0.13.0: an unlisted choice was reset to its catalog default, a structured JSON value was replaced with a boolean, and accepting handwritten TOML produced duplicate keys/tables. The fix omits unchanged answers, keeps unsupported values by default, and validates a proposed TOML merge against the expected parsed document before writing. Explicit replacement remains available. This does not introduce file locking or crash-atomic writes, and the legacy TOML writer may refuse unusual valid layouts rather than rewrite them unsafely. Shared upgrade/scaffolding writers are unchanged.

September 8: all 309 ordinary tests passed (19 explicitly ignored, 328 discovered). Eight regressions were added: two unit tests and six PTY tests. An initial new replacement test used the heading-wait helper for an inline confirmation; correcting that test synchronization produced the passing full run. Formatting, clippy, reviewed-head CI, and main CI evidence are recorded on the fix PR. Live Studio/Rojo/provisioning checks are not rerun because their execution paths are unchanged; the dated 0.13.0 evidence below remains historical, not a new pass.

### Released Evidence

| Check | Result |
| --- | --- |
| 0.13.0 candidate local gates | September 8: 301 ordinary tests passed (19 explicitly ignored); formatting and clippy passed. All 14 serial live regressions passed in 183.89 seconds. The three real-Rojo checks passed in 5.22 seconds. Verified installed Rokit 1.2.0, Rojo 7.7.0, and Wally 0.3.2; the Jest runner was exercised through its project-local pin, not a global shim in this repository. Packaged verification and exact reviewed-head/main CI are recorded on the release PR before publication. |
| Creation merge/main CI | PR #14 merged at `a90037f`, tree-identical to reviewed `ef1d5df`. [Main CI](https://github.com/chatarabdelilah/rproj/actions/runs/34162680901) passed Windows stable, Rust 1.89, and package builds. Both CodeRabbit findings were resolved; the merged feature branch was deleted. |
| Creation source verification | 301 ordinary tests passed; 19 explicitly ignored; 320 discovered (273 unit + 47 integration). Formatting, clippy, and `cargo package --locked` passed (80 files). The three real-Rojo template/Inspector checks passed. Windows stable, Rust 1.89, and package CI passed on `ea07979`; [PR #14](https://github.com/chatarabdelilah/rproj/pull/14) records subsequent reviewed-head and merge/main verification. |
| Creation CodeRabbit review | Both actionable findings were verified and fixed: pasting into name/setup modals now preserves the Review selection (regression assertions added), and architecture live-test counts/breakdowns now match 14 live tests. Unfinished capability choices also have explicit back-navigation regressions. |
| Creation CI correction | The first Windows stable/1.89 run exposed an existing hub-name test that depended on this machine's saved setup. The fixture now explicitly marks setup ready; the separate disabled-action test covers the unconfigured state. No machine provisioning was added to CI. |
| Creation live regression | September 7: all 14 serial live tests passed in 125.57 seconds, including hub cancellation, real confirmation, concurrent destination refusal, named setup save/replay, direct saved-setup replay/refusal, Wally/submodules, generated quality gates, and Jest starter success/deliberate failure. The initial hub test used the wrong prompt label; corrected to the existing `Project folder name` before the passing run. Only unique temporary fixtures were removed. |
| Ordinary suite with saved-setup refusal regression | 273 passed; 16 deliberately ignored; 289 discovered (253 unit + 36 integration) |
| Diagnostic-logger source suite | 286 passed; 16 deliberately ignored; 302 discovered (258 unit + 44 integration). Five logger unit tests and eight integration tests cover unique bounded logs, redaction/control escaping, command outcomes, accepted settings, TUI navigation without text capture, opaque runner arguments, opt-out, and non-fatal logging failures. |
| Live regression rerun with diagnostic logging | Passed September 7: saved-setup refusal/replay together in 28.53 seconds; Jest starter specs passed, then the deliberate failure returned 1 with two passed / one failed in 28.93 seconds. Existing unique temporary fixtures were removed by the harness. |
| Formatting and clippy | Passed locally |
| Existing live project suite | Seven passed in 38.39 seconds on September 5 after harness corrections |
| Concurrent-destination live regression | Passed for both cancellation and confirmation |
| Existing real-Rojo template checks | Passed with Rojo 7.7.0 |
| Compound Inspector output | Passed real-Rojo validation across the template matrix |
| Live Jest Roblox execution | `jest-roblox-cli` 0.3.24, Studio CLI backend: three generated starter specs passed in 11.58 seconds on September 6 |
| Automated live Jest regression, PR #7 | Reviewed test passed in 26.84 seconds on September 6: scaffolded Wally + Jest in a unique temporary directory, verified project-local pins and three passing starter specs, then broke the shared spec and verified exit code 1, the assertion failure, and two passing / one failing test. Temporary project removed. |
| Automated saved-setup replay | Passed in 20.07 seconds on September 6: real `new --save-setup` then `new --like` for both Wally and Git submodules, with charm/promise, lint/format, and a dropped `.gitignore`. Verified exact graphs, no repeated choices, installed package links, selected tool pins, generated/omitted files, unchanged saved setup, and cleanup. |
| Automated saved-setup refusal | Passed in 7.78 seconds on September 6: missing setup, malformed TOML, and Jest with no manager or Git submodules each exited 1 with a case-specific error before explicit `--reconfigure`. No input was sent; no destination/parent was created; machine config, source fixtures, and an existing output-setup sentinel remained byte-identical. Temporary directories removed. |
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
cargo test --locked --test live invalid_saved_setups_refuse_before_reconfiguration_or_creation -- --ignored --test-threads=1 --nocapture
cargo test --locked --test live -- --ignored --test-threads=1
cargo test --locked steps::rojo::tests:: -- --ignored --test-threads=1
cargo test --locked guided_compound_values_pass_real_rojo_validation -- --ignored
cargo test --locked badges_do_not_contradict_upstream -- --ignored --nocapture
cargo package --locked
```

The live project suite uses unique directories under the configured projects root and shares real Rokit/Wally caches. The Jest regression requires Studio and JestRobloxRunner to be installed; normal scaffolding refreshes that plugin. Only its temporary project is isolated and removed. Run serially on an explicitly provisioned machine, not as an unattended installer on a fresh host. The regression remains ignored by ordinary CI and must be invoked explicitly.

## Remaining Limits

The refusal regression is also ignored because it reads real machine configuration and uses the configured projects/setup roots. It owns unique fixture directories and never answers provisioning prompts; a newly created empty setup directory may remain. A saved global template can invoke Rojo validation before setup refusal. This is not fresh-machine provisioning evidence.

The saved-setup regression uses the real configured projects root and setup directory, reserving a unique setup filename before the CLI writes it. Both projects and the reserved setup are removed, including partial runs; a newly created empty setup directory may remain. Existing setups and machine configuration are not changed. Rokit/Wally caches and trust state remain shared; this test requires provisioned tools and network access, runs serially, and does not install Studio plugins. Like the Jest regression, it is ignored by ordinary CI and does not change the published 0.12.2 package.

- **Jest evidence covers the local Studio CLI backend.** The automated starter-spec pass/fail gap is closed by PR #7. This does not verify Open Cloud, an attached Studio session, or a fresh Windows installation. The older ignored `real_jest_stack_installs_validates_retypes_and_executes` test still exercises no-test success; the new live regression verifies actual execution through `rproj test`.
- **Open Cloud execution was not performed.** No audit universe/place and credentials were supplied. Generated preflight and workflow text are tested, but that is not equivalent to a current hosted run.
- **Fresh Windows provisioning was not performed.** winget/editor/Studio/plugin installation changes the machine. Existing unit coverage and inspection are not a substitute for a clean-machine acceptance test.
- **Saved-setup evidence is bounded.** Replay covers valid Wally and Git-submodule compositions. Refusal covers missing/malformed records and Jest without Wally, before explicit reconfiguration or creation. It does not cover every hand-edited/legacy setup or unknown-package fallback. Clipboard contents, machine provisioning/recovery, and every template keyboard path also remain gaps. No claim of beta/1.0 readiness is made.
- **Template validity remains Rojo-authoritative.** This patch repairs specific existing compound controls, not every possible explicit/future property representation. Unsupported advanced values and old malformed drafts are not automatically normalized.

## Handoff

Keep Ratatui and all external tool boundaries. The model-import experiment stays on its separate local branch and is not a dependency of this release.

Release 0.13.0 is aligned and complete. The agent handles review findings, merge, post-merge CI, merged-branch cleanup, and release alignment; the owner alone runs `cargo publish --locked` when a new package is ready. Test-only and documentation-only follow-ups do not require publication or moving an existing release tag.

Saved-setup replay/refusal, diagnostic logging (PR #11), shared confirmed execution (PR #12), and Ratatui creation (PR #14) shipped in 0.13.0 through release PR #15. [Merged-main CI](https://github.com/chatarabdelilah/rproj/actions/runs/34181012236) passed. The release branch is deleted; the model-import experiment remains preserved. The configuration-preservation implementation is complete and locally verified but remains unreleased; [PR #16](https://github.com/chatarabdelilah/rproj/pull/16) records review and merge checks. Prepare a separate patch candidate after that PR is merged and main CI passes. Do not repeat the implementation, republish, or move v0.13.0. Remaining alpha audit gaps stay tracked.

## Post-Handoff Review: September 7, 2026

Reviewed the task history and changes from v0.12.1 through main at `fe8f4f4`.

- **Release identity:** crates.io 0.12.2 is not yanked; its archive records
  `f71bf4e103735a532b2e8de53cbbd560f5c2a3d3`, matching the annotated v0.12.2
  tag and GitHub alpha release. Do not undo or retag that publication.
- **Process correction:** 0.12.2 was pushed directly to main and Cargo-published
  by the agent after a short "publish 0.12.2" request. That bypassed the intended
  PR workflow and owner-operated publication handoff. Root `AGENTS.md` now makes
  the ownership rule and new-task startup requirements explicit.
- **Change assessment:** the Jest executable/configuration/trust fixes address
  observed failures. PRs #7-#10 add regression coverage and documentation;
  PR #11 adds the requested logger; PR #12 extracts existing confirmed execution.
  No runtime blocker was found in this review. No rollback is indicated.
- **Independent verification:** 286 ordinary tests passed, 16 deliberately
  ignored; formatting and clippy passed locally. GitHub records successful
  main CI for every post-0.12.1 merge, including
  [fe8f4f4](https://github.com/chatarabdelilah/rproj/actions/runs/34101827132).
  CodeRabbit's documentation findings in PRs #8/#10 are addressed; logger and
  execution-boundary reviews report no actionable findings. The existing live
  test evidence is historical, not a new live run during this review.
- **Remaining caution:** logs contain local paths and selected names, have no
  automatic retention cleanup, and are not a complete screen transcript. Review
  them before sharing. Open Cloud and fresh-machine acceptance remain unverified.

Update September 8: **0.13.0 is published and aligned** after
[release PR #15](https://github.com/chatarabdelilah/rproj/pull/15) and successful
merged-main CI. The owner performed publication; the agent verified the archive
identity before creating the annotated tag and GitHub alpha prerelease. The next
configuration-preservation fix is separate and unreleased.
