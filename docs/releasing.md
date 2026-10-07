# Releasing rproj

Every public version must use the same version number in `Cargo.toml`, `Cargo.lock`, the Git tag, the crates.io package, and the GitHub release.

The published baseline is **v0.20.4**, at `1a046683275faa0e9c9214b030bc183aadf3722c`.
Owner publication, archive checksum, annotated tag and
[GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.20.4)
are verified. See [release notes](release-notes-0.20.4.md),
[audit evidence](release-audit.md), and [PR #91](https://github.com/chatarabdelilah/rproj/pull/91).
This release packages the official Wally type-tool
release and safe upgrade preflight from [PR #90](https://github.com/chatarabdelilah/rproj/pull/90).
The active unreleased candidate is **0.21.0 alpha**, containing the intentional
Git-submodule retirement from [PR #93](https://github.com/chatarabdelilah/rproj/pull/93).
Cargo manifest and root lockfile are aligned at 0.21.0. See
[candidate notes](release-notes-0.21.0.md) and [audit evidence](release-audit.md).
Release preparation and final head/main CI are tracked in
[PR #94](https://github.com/chatarabdelilah/rproj/pull/94).
Owner publication, the candidate tag and matching GitHub release are pending.
Previous shipped tags remain immutable.

The agent owns preparation, CI and CodeRabbit follow-through, merging, post-merge verification, merged-branch cleanup, and tag/GitHub release alignment after publication. The repository owner alone runs `cargo publish --locked`.

## Maintainer preparation

The owner published **0.20.3** on October 7, 2026, after [PR #88](https://github.com/chatarabdelilah/rproj/pull/88)'s first-attempt final-head and merged-main CI passed. Registry checksum `f2403a0d54155e7ec3c37dee9a9f99d9adfee66f0a242cdc2d98f4bfb3ea013c`, archive Git identity, annotated tag and GitHub alpha prerelease match clean release commit `2932862af2cfc7a7670e54da06e133c799a53592`. The merged candidate branch was verified and removed. Fresh-Windows installation/retry and deferred acceptance gaps remain unverified; VM provisioning stays deferred.

The owner published **0.20.2** on October 7, 2026, after [PR #85](https://github.com/chatarabdelilah/rproj/pull/85)'s first-attempt final-head and merged-main CI passed. Registry checksum `260f61d5baa9b5e1ea422b4f85a4356d790bb2fd36c24fbc91a874f09c526745`, archive Git identity, annotated tag and GitHub alpha prerelease match clean release commit `eaa0447d73751f147c2138b106f1ceb263617e10`. The merged candidate branch was verified and removed. Fresh-machine/Open Cloud/platform Studio acceptance remains unverified; VM provisioning remains deferred.

The owner published **0.20.1** on October 6, 2026, after [PR #80](https://github.com/chatarabdelilah/rproj/pull/80)'s reviewed-head and merged-main CI passed. Registry checksum `17085cc851e7d73cbdf3de2671380e86bde3ff664a5ecaebd3aa594b329fd0bf`, archive Git identity, annotated tag and GitHub alpha prerelease match clean release commit `07634cc90dc638abbea113574134d3ddc501df4b`. The merged candidate branch was verified and removed. Fresh-machine installation acceptance remains unverified; VM provisioning remains deferred.

The owner published **0.20.0** on October 6, 2026, after [PR #75](https://github.com/chatarabdelilah/rproj/pull/75)'s reviewed-head and merged-main CI passed. Registry checksum `175f1bb4b3b746960c623bfdaf29600c3069707bd3e70c253ff8deb421b2f190`, archive Git identity, annotated tag and GitHub alpha prerelease match the clean release commit `f3cdea8d1c88a8acb5831c00a3d7f44c2d6654b0`. The merged release branch was verified and removed. Fresh-machine installation acceptance remains unverified.

The owner published **0.19.1** on October 4, 2026, after [PR #53](https://github.com/chatarabdelilah/rproj/pull/53)'s reviewed-head and merged-main CI passed. Registry checksum `be193ef19a8277f78d788bb9494834d91ba3146d7d7a6daf1d32e7f854cdb210`, archive Git identity, annotated tag and GitHub prerelease match the clean release commit `957a0aa67fa679dca4194e6fd3c739594c409dc6`. Fresh-machine installation acceptance remains unverified.

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

## CodeRabbit review

Run `cr review --agent --uncommitted` while changing a meaningful runtime, CI, or
release candidate. Before opening its PR, run `cr review --agent --base main` to
review the whole branch. Address valid critical or major findings; record why an
inapplicable finding was not changed. Use `--light` only for a narrow, repeated
follow-up, not for release or compatibility work.

The repository configuration keeps GitHub PR reviews opt-in: add
`coderabbit:review` to a ready PR description when a remote review is wanted.
It disables incremental reviews, automatic request-change workflow, generated AI
prompts, web search, and chat replies. These limits keep the review signal focused
and do not replace required local checks or CI. Do not invoke CodeRabbit for a
documentation-only correction unless the wording changes a technical guarantee.

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
a shipped tag. Use the selected candidate's dedicated release-notes file as the GitHub release body.

```powershell
$version = '<published version>'
$publishedCommit = '<verified Git SHA from the published archive>'
$notes = '<dedicated release-notes path>'
git tag -a "v$version" $publishedCommit -m "rproj v$version"
git push origin "v$version"
gh release create "v$version" --verify-tag --notes-file $notes --prerelease --title "rproj v$version"
```

Verify that the GitHub release points to the same commit as the tag, remains marked as a prerelease throughout the alpha phase, and that crates.io reports the selected version as current.
