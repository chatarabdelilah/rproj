# Release-Hardening Audit

Updated September 6, 2026. Candidate: **0.12.1**, alpha. Published baseline: 0.12.0. Scope: confirmed defects in existing workflows; no new features, model importer, embedded tools, or frontend migration.

## Confirmed Defects And Fixes

1. **Project creation touched the destination before confirmation.** Escape could leave an empty directory, and explicit Cancel recursively removed it even if another process had added files. Creation now happens after confirmation, uses an exclusive directory creation, and never deletes the destination on cancellation. Unit tests cover an existing directory and missing parents; a live regression covers another writer creating a sentinel while the prompt is open, for both Create and Cancel.
2. **Some Inspector values did not match Rojo's serialization.** UDim/UDim2/Rect now use explicit nested representations with integer offsets; CFrame attributes use position/orientation fields. Reopening values flattens components for the existing input controls. Unit round trips and a real-Rojo validation test cover the fix. NaN/infinity and fractional/out-of-range offsets are rejected before changing the draft.
3. **Live tests were stale and unsafe to rerun against an occupied project root.** They selected TestEZ in the old package picker, assumed VS Code was selected, manually parsed the wrong global-config path, and deleted fixed-name directories. The harness now follows current capability/runner prompts, reads actual TOML via the configured directory API, checks provisioning up front, and uses unique scratch names without removing pre-existing directories.

## Evidence

| Check | Result |
| --- | --- |
| Ordinary suite | 272 passed; 13 deliberately ignored; 285 discovered (252 unit + 33 integration) |
| Formatting and clippy | Passed locally |
| Existing live project suite | Seven passed in 38.39 seconds on September 5 after harness corrections |
| Concurrent-destination live regression | Passed for both cancellation and confirmation |
| Existing real-Rojo template checks | Passed with Rojo 7.7.0 |
| Compound Inspector output | Passed real-Rojo validation across the template matrix |
| Upstream badge check | Passed September 6; advisory to review Matter's Active badge (last reported push December 31, 2024); not proof that the project is abandoned |
| Previous CodeRabbit findings, PR #4 | Both addressed in baseline commit 4737c84; plugin identity and ServerPackages exclusion regression present |
| Candidate package | cargo package --locked passed; 73 files, 851.8 KiB uncompressed |
| PR CI and CodeRabbit | Results are attached to [PR #6](https://github.com/chatarabdelilah/rproj/pull/6); successful review and Windows stable/1.89/package checks are merge gates |

Reproduction commands:

```powershell
cargo fmt --all --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --test live -- --ignored --test-threads=1
cargo test --locked steps::rojo::tests:: -- --ignored --test-threads=1
cargo test --locked guided_compound_values_pass_real_rojo_validation -- --ignored
cargo test --locked badges_do_not_contradict_upstream -- --ignored --nocapture
cargo package --locked
```

The live project suite uses unique directories under the configured projects root and shares real Rokit/Wally caches. Run serially on an explicitly provisioned machine, not as an unattended installer on a fresh host.

## Remaining Limits

- **Live Jest execution is unverified in this audit.** Neither the jest-roblox executable nor JestRobloxRunner.rbxm is installed on this machine. They were not installed as an audit side effect. The existing ignored test currently exercises no-test success, not a failing/passing starter-spec pair; this remains a coverage gap, not evidence of test execution.
- **Open Cloud execution was not performed.** No audit universe/place and credentials were supplied. Generated preflight and workflow text are tested, but that is not equivalent to a current hosted run.
- **Fresh Windows provisioning was not performed.** winget/editor/Studio/plugin installation changes the machine. Existing unit coverage and inspection are not a substitute for a clean-machine acceptance test.
- **Not every workflow has new end-to-end coverage.** Saved-setup replay, clipboard contents, machine provisioning/recovery, and every template keyboard path still need targeted manual or isolated integration evidence. No claim of beta/1.0 readiness is made.
- **Template validity remains Rojo-authoritative.** This patch repairs specific existing compound controls, not every possible explicit/future property representation. Unsupported advanced values and old malformed drafts are not automatically normalized.

## Handoff

Keep Ratatui and all external tool boundaries. The model-import experiment stays on its separate local branch and is not a dependency of this release.

The owner runs cargo publish --locked only after PR #6 is reviewed, merged, main CI passes, and the ready-to-publish instruction is given. Verify crates.io before tagging v0.12.1 and creating the GitHub alpha prerelease. Do not move the existing v0.12.0 tag to a later commit.
