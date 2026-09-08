# rproj Working Agreement

Read this file at the start of every task. These are persistent maintainer rules,
not just context from an earlier chat.

## Ownership And Releases

- The repository owner alone runs `cargo publish --locked`. Never run Cargo
  publication for them. Treat "publish" as a request to prepare and give the
  command, unless the owner explicitly changes this ownership rule.
- The agent owns version preparation, Cargo.toml/Cargo.lock alignment, current
  documentation, branches, commits, pushes, PRs, CodeRabbit review, merging,
  post-merge CI verification, and deletion of fully merged local/remote branches.
- Use `codex/` branches and reviewed PRs; do not push runtime changes directly
  to main. Verify checks on the actual reviewed head and then on merged main.
- For meaningful runtime, CI, or release changes, run CodeRabbit locally before
  opening the PR: use `cr review --agent --base main` for the final branch and
  `cr review --agent --uncommitted` while iterating. Do not spend a review on a
  documentation-only correction; inspect that small diff directly instead.
- Do not create or push a release tag before crates.io confirms publication.
  Verify the published archive's Git identity, create an annotated tag at that
  commit, and create a matching GitHub alpha prerelease. Never move a shipped tag.
- Read `docs/releasing.md` before release work. No version bump or publication
  is required for each test/docs/refactor PR. Unreleased runtime changes need a
  new version when their release candidate is prepared.

## Scope And Continuity

- Read `docs/plan.md` and `docs/release-audit.md` for current priorities and
  evidence. Consult architecture/UX sections relevant to the change, not every
  document in full. Verify Git/PR state rather than trusting an old chat summary.
- Ratatui is the UI direction. Reuse existing project graph, catalogs, and
  scaffolding. Keep direct commands and external Rojo, Selene, StyLua, luau-lsp,
  Rokit, Wally, Git, Lute, and test-runner boundaries.
- Tauri and embedded lint/format/parser migrations are dropped. Preserve
  `codex/m7-model-import` as a shelved experiment; do not merge or delete it.
- Keep comments for non-obvious invariants. Put broader rationale in docs;
  do not add comments just to satisfy a docstring-percentage advisory.
- Missing prerequisites and ignored tests are not passes. Do not provision
  machine-wide applications or replace user data merely to complete an audit.
- When `.codegraph/` exists, use CodeGraph before searching unfamiliar code.
  Fall back to focused source reads if its results are missing or stale.

## Communication

- Lead with the next action or current state. Keep explanations concise,
  practical, and accessible; use at most five numbered steps when needed.
- "What's next?" means rproj development, not instructions to develop a Roblox
  game. Own authorized work end to end instead of asking the owner to repeat it.
- Recommend model/reasoning/mode changes only when the next task warrants them.
  Do not enable goals or create separate tasks without an explicit request.
- Before recommending a new chat, update the durable state and provide a short
  handoff naming these rules, current branch/release, and one next task. A new
  chat is not a usage reset; keep tool output and repeated reads bounded.
