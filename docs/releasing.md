# Releasing rproj

Every public version must use the same version number in `Cargo.toml`, `Cargo.lock`, the Git tag, the crates.io package, and the GitHub release.

The current release candidate is `v0.12.0`, M4: Jest Roblox as a TestEZ peer. It remains an alpha prerelease.

## Maintainer preparation

1. Update the version and current-state documentation on a release branch.
2. Run `cargo test`.
3. Run `cargo clippy --all-targets -- -D warnings`.
4. Run `cargo package` and inspect the packaged file list.
5. Merge the release branch into `main` and push it.

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
