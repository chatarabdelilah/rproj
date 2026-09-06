# Releasing rproj

Every public version must use the same version number in `Cargo.toml`, `Cargo.lock`, the Git tag, the crates.io package, and the GitHub release.

The published baseline is `v0.12.1`; the current alpha candidate is `v0.12.2`, release hardening of the Jest Roblox toolchain and generated configuration. See [the audit](release-audit.md) for verification and remaining prerequisites. The shelved model-import branch's `0.13.0` version is not a publication target.

## Maintainer preparation

1. Establish scope from verified defects and approved changes. Roadmap-only edits do not require Cargo publication.
2. Update the version and current-state documentation on a release branch when preparing an actual package release.
3. Run `cargo fmt --all --check`, `cargo test --locked`, and `cargo clippy --locked --all-targets -- -D warnings`.
4. Run applicable ignored/live tests serially on a deliberately provisioned environment. Record commands, tool versions, outcomes, and missing prerequisites. Skipped tests are not passes; follow [the roadmap's workflow coverage](plan.md).
5. Run `cargo package --locked` and inspect the packaged file list.
6. Push a pull request, inspect CodeRabbit findings, and verify Windows stable, Rust 1.89, and package CI. Address findings or document why they do not apply.
7. Merge into `main`, push, and verify main CI. Remove merged local and remote branches only after checking their work is preserved. Do not delete unmerged experiments as if they were merged.
8. Request publication only with a ready candidate, clean checkout, and recorded residual risks. The repository owner performs Cargo publication, not the agent.

## Cargo publication

The repository owner publishes from a clean `main` checkout:

```powershell
cargo publish --locked
```

`--locked` requires Cargo to use the exact dependency versions in `Cargo.lock`; publication fails instead of silently resolving a newer dependency graph than the one tested.

Do not create the version tag before crates.io accepts the package. A failed publication must leave no public tag claiming that the version shipped.

## Tag and GitHub release

After the matching version is visible on crates.io:

```powershell
git tag -a vX.Y.Z -m "rproj vX.Y.Z"
git push origin vX.Y.Z
gh release create vX.Y.Z --verify-tag --generate-notes --prerelease --title "rproj vX.Y.Z"
```

Verify that the GitHub release points to the same commit as the tag, remains marked as a prerelease throughout the alpha phase, and that crates.io reports `X.Y.Z` as the current version.
