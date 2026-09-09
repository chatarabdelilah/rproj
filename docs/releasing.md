# Releasing rproj

Every public version must use the same version number in `Cargo.toml`, `Cargo.lock`, the Git tag, the crates.io package, and the GitHub release.

The published baseline is **v0.13.1** at `0fad0db8213dda1bfa9c3b0f97e24c808894fc0f`; its archive, tag, and GitHub alpha release are aligned. The next candidate is **v0.14.0, T2: Home navigation and Catalog clarity**, prepared on `codex/t2-home-catalog`. [Release notes](release-notes-0.14.0.md) define its scope; [the audit](release-audit.md) records verification and remaining limits. It is not published until the owner runs Cargo publication after the reviewed merge.

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
