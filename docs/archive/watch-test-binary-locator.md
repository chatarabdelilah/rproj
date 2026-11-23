# Cargo and Windows lint compatibility

The owner approved the separate Watch test correction on November 16, 2026,
after Project Tool Setup design PR #154 was blocked. Production behavior,
desktop previews, dependencies and version 0.22.0 remain unchanged.

## Reproduction and correction

PR #154's unchanged head `e733ef9` failed CI run `43776424821` on attempts 1
and 2. The stable Windows job used Rust 1.100.0 and placed its unit-test harness
at `target/debug/build/rproj/020df31b818906bf/out/rproj-020df31b818906bf.exe`.
The Watch fixture's `exe()` assumed the older `target/debug/deps` layout and
walked up two directories, so it looked for the CLI in the wrong directory.
Both failures occurred before lifecycle execution, at the fixture-binary assertion.

The test helper now recognizes both observed layouts and derives the matching
profile's `rproj.exe`. It preserves custom target roots, target triples and
profiles; it does not search PATH or arbitrary ancestors for another binary.
Unknown layouts fail with the harness path in the diagnostic. Existing assertions
still require building the CLI and exercising the real detached lifecycle.

The first fix-head CI run (`43819861520`, head `4306c68`) passed the formerly
failing lifecycle and all tests. It then exposed Rust 1.100's Windows-only
`needless_bool` lint in `inspection::executable_file`, before the Format step.
The compatibility PR also simplifies that expression without changing behavior:
non-Unix still requires a regular file; Unix still requires a regular file with
an execute bit. No lint suppression or test assertion was weakened.

## Evidence

- Two new deterministic path regressions first failed against the old locator,
  then passed after correction. They cover both layouts, spaces/Unicode,
  cross-target/custom profile paths and rejection of unrelated layouts.
- Local Rust 1.94.0: `cargo build --locked --bin rproj`, followed by
  `cargo test --locked --bin rproj background_watch::windows::tests -- --nocapture`
  passed 22 tests; two existing prerequisite cases stayed ignored, not passed.
  Normal, stop-recovery, failure, crash, flood, blocked and simultaneous lifecycle
  scenarios ran against isolated CLI fixtures. No native window was launched.
- Formatting and strict all-target Clippy passed. Incremental-cache hard-link
  warnings fell back to copying. CodeRabbit 0.7.6 reviewed all three files with
  zero findings; the final diff and this evidence-only update were inspected
  directly. After the lint cleanup, all 21 inspection tests, formatting and
  strict all-target Clippy pass locally. CodeRabbit's second review covered all
  four files with zero findings. Actual reviewed-head/main CI is tracked in the
  delivery PR.

The delivery PR records exact SHAs, CI attempts and cleanup. After this fix is
merged and main CI is verified, resume PR #154 on the corrected baseline and
verify that design's own final-head and merged-main CI separately.
