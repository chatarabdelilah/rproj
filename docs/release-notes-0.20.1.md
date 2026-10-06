# rproj 0.20.1 — Missing prerequisite recovery

Release candidate. Public alpha; owner publication is pending.

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

The candidate passes 477 ordinary locked Windows tests, formatting, clippy
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
