# rproj 0.20.1 — Missing prerequisite recovery

Published October 6, 2026. Public alpha.

## Changes

- If Cargo is missing while installing Rokit, rproj now names the Rust
  prerequisite, links to rustup.rs, and asks you to open a new terminal,
  check `cargo --version`, and retry `rproj setup`.
- If WinGet is missing while installing applications, rproj now links to
  Microsoft's App Installer installation/repair instructions and asks you
  to verify PATH, open a new terminal, check `winget --version`, and retry.
- Both plain command execution and Machine Setup's reported execution retain
  the original OS error. Permission failures, Cargo build failures, installer
  output and WinGet hash-mismatch handling retain their existing diagnostics.

No project migration is required. This patch retains 0.20.0's persistent
Windows background Watch behavior and limits. It also includes a test-harness
repair: Projects resize assertions compare physical terminal rows rather
than soft-wrap serialization, while still rejecting changed row layouts.

## Verification and limits

The candidate passed 477 ordinary locked Windows tests, formatting, clippy
and both explicit installed-tool background Watch acceptances. Its 24 ignored
tests are not ordinary passes; only the two named acceptances were run explicitly
for this patch candidate. Packaging and review/CI evidence are recorded in
[the release audit](release-audit.md). Missing-tool regressions use disposable
nonexistent executables; they do not install machine applications or mutate
the user's PATH. A successful recovery after fresh Windows provisioning
remains unverified. The owner deferred Hyper-V/VM setup.

Authenticated Open Cloud execution, fresh Linux generated-project execution,
UI Labs Studio stories and Scribe Studio playtesting remain unverified.
These are acceptance gaps rather than passing checks. The patch does not
claim beta readiness. See [0.20.0's release notes](release-notes-0.20.0.md)
for retained background Watch and platform limits.

[Release PR #80](https://github.com/chatarabdelilah/rproj/pull/80) passed both
required local CodeRabbit reviews with zero findings, clean locked packaging,
and reviewed-head/main Windows stable, Rust 1.89 and package CI. Main's first
stable attempt failed three existing Template Explorer PTY pasted-text checks;
all 11 runnable editor PTY checks passed locally and unchanged main passed the
failed-job retry. Retry success does not fix the harness's input synchronization.

The owner-published archive's SHA-256 matches crates.io checksum
`17085cc851e7d73cbdf3de2671380e86bde3ff664a5ecaebd3aa594b329fd0bf`.
Its manifest and lockfile are 0.20.1, and its clean Git identity matches reviewed
release commit `07634cc90dc638abbea113574134d3ddc501df4b`.
