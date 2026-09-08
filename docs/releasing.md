# Releasing rproj

Every public version must use the same version number in `Cargo.toml`, `Cargo.lock`, the Git tag, the crates.io package, and the GitHub release.

The published baseline is **v0.13.0** at `50f2e3419d9fb252d71810536944e1a5fe445a79`, an alpha release containing diagnostic logging and hub-driven Ratatui project creation. The crates.io archive Git identity, annotated tag, and [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.13.0) are aligned. [Release notes](release-notes-0.13.0.md) define its scope; [the audit](release-audit.md) records verification and remaining limits. Configuration-preservation fixes after this commit are unreleased; Cargo manifests stay at 0.13.0 until the next candidate is prepared.

The agent owns preparation, CI and CodeRabbit follow-through, merging, post-merge verification, merged-branch cleanup, and tag/GitHub release alignment after publication. The repository owner alone runs `cargo publish --locked`.

## Maintainer preparation

1. Establish scope from verified defects and approved changes. Test-only and documentation-only changes do not require a version bump or Cargo publication.
2. Update the version and current-state documentation on a release branch when preparing an actual package release.
3. Run `cargo fmt --all --check`, `cargo test --locked`, and `cargo clippy --locked --all-targets -- -D warnings`.
4. Run applicable ignored/live tests serially on a deliberately provisioned environment. Record commands, tool versions, outcomes, and missing prerequisites. Skipped tests are not passes; follow [the roadmap's workflow coverage](plan.md).
5. Run `cargo package --locked` and inspect the packaged file list.
6. Push a pull request, inspect CodeRabbit findings, and verify Windows stable, Rust 1.89, and package CI. Address findings or document why they do not apply.
7. Merge into `main`, push, and verify main CI. Remove merged local and remote branches only after checking their work is preserved. Do not delete unmerged experiments as if they were merged.
8. Request publication only with a ready candidate, clean checkout, and recorded residual risks. The repository owner performs Cargo publication, not the agent.

## Proportionate verification

During implementation, run focused tests for the changed behavior and its callers. Run the ordinary suite, formatting, and clippy before the runtime PR is ready. Capture summaries and failures rather than repeatedly loading successful test output. Wait for CI state changes instead of frequent unchanged polling.

Run external/live checks when the changed paths require them, not for unrelated configuration or documentation edits. Do not rerun the same successful local gate after a documentation-only correction; record the unchanged runtime/dependency identity instead. Reviewed-head and merged-main CI remain mandatory. A release candidate still requires the preparation checklist above, including locked packaging and applicable live evidence. Never count a skipped prerequisite as a pass.

## Cargo publication

The repository owner publishes from a clean `main` checkout:

```powershell
cargo publish --locked
```

`--locked` requires Cargo to use the exact dependency versions in `Cargo.lock`; publication fails instead of silently resolving a newer dependency graph than the one tested.

Do not create the version tag before crates.io accepts the package. A failed publication must leave no public tag claiming that the version shipped.

## Tag and GitHub release

After the matching version is visible on crates.io:

Download the published crate archive and inspect `.cargo_vcs_info.json`. Verify its
Git SHA against the clean, reviewed release commit and manifest version. Create
the annotated tag at that exact commit, not whichever commit happens to be HEAD.
If publication and repository identity disagree, stop and investigate; never move
a shipped tag. For 0.13.0, use `docs/release-notes-0.13.0.md` as the GitHub release body.

```powershell
$publishedCommit = '<verified Git SHA from the published archive>'
git tag -a v0.13.0 $publishedCommit -m "rproj v0.13.0"
git push origin v0.13.0
gh release create v0.13.0 --verify-tag --notes-file docs/release-notes-0.13.0.md --prerelease --title "rproj v0.13.0"
```

Verify that the GitHub release points to the same commit as the tag, remains marked as a prerelease throughout the alpha phase, and that crates.io reports `0.13.0` as the current version.
