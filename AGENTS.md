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
- Start independent tasks from a freshly fetched `origin/main`. In a clean
  checkout, use `git switch --no-track -c codex/<task> origin/main` so the
  task branch does not track main. If another task is active or the checkout
  has unrelated edits, use a separate worktree from the same base.
- For meaningful runtime, CI, or release changes, run CodeRabbit locally before
  opening the PR: use `cr review --agent --base main` for the final branch and
  `cr review --agent --uncommitted` while iterating. Do not spend a review on a
  documentation-only correction; inspect that small diff directly instead.
- Before merging, read the review summary and open threads. Answer questions
  in-thread, fix valid findings or explain disagreements, and resolve addressed
  threads. Answer late feedback on the merged PR and use a follow-up PR for fixes.
- Capture the final reviewed PR head SHA whose checks passed. If the head moves,
  review the changes and verify checks again. Pass that SHA to
  `gh pr merge --match-head-commit <sha>`. After a successful guarded merge,
  the merged PR's `headRefOid` is the durable cleanup reference. Retrieve it
  with `gh pr view <pr> --json state,headRefOid`.
- Before deleting task refs, confirm the PR is `MERGED`, the task checkout is
  clean, and each remaining local or remote task branch still matches that SHA.
  Investigate any mismatch. Remove only the task's worktree and branches.
- Do not create or push a release tag before crates.io confirms publication.
  Verify the published archive's Git identity, create an annotated tag at that
  commit, and create a matching GitHub alpha prerelease. Never move a shipped tag.
- Read `docs/releasing.md` before release work. No version bump or publication
  is required for each test/docs/refactor PR. Unreleased runtime changes need a
  new version when their release candidate is prepared.

## Scope And Continuity

- Read `docs/plan.md` and `docs/archive/release-audit.md` for current priorities and
  evidence. Consult architecture/UX sections relevant to the change, not every
  document in full. Verify Git/PR state rather than trusting an old chat summary.
- Follow the approved desktop transition in `docs/archive/desktop-transition.md`.
  Desktop becomes the primary human interface after verified workflow parity;
  preserve Ratatui during migration and direct commands permanently. Extract
  reusable behavior incrementally into rproj-core in this repository. Keep
  external Rojo, Selene, StyLua, luau-lsp, Rokit, Wally, Git, Lute, and test-runner
  boundaries. GPUI must pass its feasibility gate before broad GUI work.
- Tauri, embedded lint/format/parser migrations, and model-to-template import remain
  dropped. The owner permanently retired the model-import experiment on September
  9, 2026; its branch was deleted. Do not restore or plan it as future work.
- Keep comments for non-obvious invariants. Put broader rationale in docs;
  do not add comments just to satisfy a docstring-percentage advisory.
- Missing prerequisites and ignored tests are not passes. Do not provision
  machine-wide applications or replace user data merely to complete an audit.
- When `.codegraph/` exists, use CodeGraph before searching unfamiliar code.
  Fall back to focused source reads if its results are missing or stale.
- For routine CodeGraph lookups, name the crate/file and specific symbols and
  start with `maxFiles: 2` to `4` (`--max-files` in the CLI). Avoid broad names
  such as `run` alone; expand only when the returned call path requires it.

## Communication

- Lead with the next action or current state. Keep explanations concise,
  practical, and accessible; use at most five numbered steps when needed.
- "What's next?" means rproj development, not instructions to develop a Roblox
  game. Own authorized work end to end instead of asking the owner to repeat it.
- Recommend model/reasoning/mode changes only when the next task warrants them.
  Do not enable goals or create separate tasks without an explicit request.
- End every task with a concise handoff labeled **Done**, **Broken**, and
  **Next**, even when continuing in the same chat. Done names completed changes
  and verification, plus the current branch/release when relevant. Broken names
  unresolved defects or blockers; say "No known broken items" when none are
  confirmed and distinguish unverified checks from defects. Next names one
  concrete development task and its first action doable in under two minutes.
  Include the recommended model, reasoning effort and mode for that next task.
- Before recommending a new chat, update the durable state and provide a short
  handoff naming these rules, current branch/release, and one next task. A new
  chat is not a usage reset; keep tool output and repeated reads bounded.
