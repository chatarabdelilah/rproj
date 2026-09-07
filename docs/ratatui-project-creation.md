# Ratatui project creation: bounded implementation plan

Status: creation model and screens implemented on `codex/ratatui-project-creation`; acceptance verification and PR review in progress.
The diagnostic logger is merged in PR #11. Model import remains shelved and is
not a dependency. Direct-command prompts remain unchanged.

`commands::new::execute_confirmed` now separates the reviewed composition's
execution from direct-command prompts. It retains exclusive destination creation,
scaffolding, graph/setup persistence, Jest plugin refresh, and completion output.
The direct command uses this same path. Shared preparation/validation and the
hub state model/screens are implemented; callers validate and obtain explicit Create
confirmation before invoking this boundary, with the terminal restored.

Versioning: published 0.12.2 remains unchanged. This feature PR does not select a
release version. The logger and creation UI must be included in the next release's version/changelog preparation;
do not publish the unreleased source under the existing 0.12.2 version or move its tag.

## First slice

Move the workspace hub's **New Project** action into a consistent Ratatui flow.
Keep direct `rproj new <name>` and its flags working as today. Do not migrate
machine provisioning, Configure, Upgrade, or subprocess output into new screens.

The flow is:

1. Name and composition source: new guided/expert choices or an existing saved setup.
2. Dependency strategy: Wally, Git submodules, or none.
3. Packages: guided categories or the searchable expert multi-select.
4. Capabilities: compatible implementations, with testing remaining optional.
5. Review: graph choices and derived files, optional file removal, named setup save,
   revise an earlier decision, Create, or Cancel.

A saved setup goes directly to Review after validation. Keep its concrete
implementations, derived packages, and dropped artifacts. Invalidation follows
`ProjectGraph`/`Node`, not an independent wizard history. Incompatible runner
changes must be explained and resolved, never silently substituted.

## Execution boundary

- Extract reusable preparation/validation/execution seams from `commands::new`
  without changing direct-command behavior. Keep the graph and catalogs authoritative.
- The first hub-driven slice requires recorded machine setup. If absent, explain
  the prerequisite and offer the existing Setup action; do not install from a draft.
- Read and validate existing config/templates before presenting a usable draft.
  A read-only Rojo validation may run; no project directory is created while editing.
- Only explicit Create hands the approved graph to the existing scaffolder, after
  restoring the terminal. Preserve its exclusive destination creation and recheck
  safety if another process created the destination while Review was open.
- Show subprocess output in the normal terminal. Do not embed or replace tool runners.
- Emit diagnostic events for screen transitions, accepted choices, invalidations,
  validation failures, Create/Cancel, and execution outcome. No raw text transcript.

## Interaction contract

Reuse `TerminalSession`, shared inputs/pickers, responsive panes, and help styling.
Wide layouts show choices and explanations together; narrow layouts retain both
through focus switching. Too-small windows offer resize guidance and safe exit.
Filtering does not discard selected packages. Esc backs out or asks to discard
a changed draft; Ctrl+C restores the terminal and exits without creating a project.

Review saves only a new named setup, with atomic no-clobber persistence. Use a different
name when one already exists; the direct command's existing replacement semantics
remain unchanged. Project and setup names accept Unicode but reject reserved Windows
names and path separators. Paste is supported in the name inputs and search fields.

## Acceptance checks before merge

- Pure state-transition tests for guided/expert parity, filtering, saved setup
  replay, dependency revisions, optional testing, and dropped artifacts.
- Derived graph/plan parity against existing direct-command fixtures for none,
  Wally, submodules, TestEZ, and Jest where compatible.
- Render tests for wide/narrow/minimum size, help, validation errors, and Unicode.
- PTY tests for cancellation, terminal restoration, confirmation boundaries,
  no premature directory creation, and a concurrent destination writer.
- Explicit serial live tests for unchanged generated output, saved replay/refusal,
  and the existing Jest starter-spec success/failure scenario.
- Diagnostic events make the decision sequence inspectable without recording secrets.

## Delivery order

1. Review and merge the diagnostic logger and this plan.
2. Implement the hub-driven creation state model and screens with shared execution seams.
3. Run parity, PTY, and relevant live regressions; inspect CodeRabbit, merge, and verify main CI.
4. Select and prepare an alpha release once the actual shipped scope is known.
   The owner alone performs `cargo publish`; no version is promised by this plan.

There is no additional unrelated hardening milestone before step 2. New blockers
still take priority, and unverified Open Cloud/fresh-machine behavior remains
documented rather than being represented as passed.
