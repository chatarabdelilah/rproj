# Project Tool Setup wireframe acceptance

The owner approved the [design scope](desktop-project-tool-setup-design.md)
against `0d3f6a6` on November 16, 2026. This separate
[Soft proposal](prototypes/project-tool-setup.html) uses labeled samples and
simulated effects only. Native implementation requires subsequent design approval.

## Automated evidence

All 25 Node/DOM checks pass for Tool → Review → Results with jsdom 26.1.0.
Reproduce using an already installed module directory:

```powershell
node docs/prototypes/project-tool-setup.test.cjs target/new-project-design/dom-runtime/node_modules/jsdom
```

The optional final argument can name another jsdom installation. No package
dependency was added. Coverage includes:

- Useful literal startup without host helpers/icons, JavaScript syntax,
  fixture inventory, all ten project Rokit tools, distinct Blender and
  machine-wide guidance samples, search and CLI/extension identities.
- Missing/existing/custom pins, preserved configuration, both asset layouts,
  missing/existing Blender scenes, plain Rojo projects, missing prerequisites,
  malformed manifests and unsafe/unreadable paths. New text previews label
  omitted generated comments; binary scenes have descriptions, not fake diffs.
- Named default-Keep confirmations, captured owner through navigation and
  project closure, explicit replacement/refresh, stale confirmations, separate
  Watch Stop acknowledgement and competing-operation guards.
- Per-item progress, installation failure followed by created files, manual
  next steps, obsolete/duplicate completions, uncertainty and confirmed teardown,
  wait-for-completion closing, independent dirty-draft protection and focus races.
- Bounded searchable Activity, manual collapse/height retention, unique IDs and
  accessibility references, wrapped content, 600-pixel content/550-pixel shell
  rules and 24 Light/Dark text-contrast comparisons above 4.5:1.

The first run passed 21/22 checks. The failed search assertion assumed that
“Luau” matched only the CLI/extension names, although descriptions also match.
Using the precise “Luau LSP” query corrected the harness expectation; the rerun
passed 22/22. A further reviewed-tool replacement regression was then added;
that run passed 23/23. The unchanged Copy Source suite also passes 23/23.
CodeRabbit found that reopening the close dialog and choosing Keep open left
an earlier close-after-setup request armed. Dismissal now clears that request,
and completion waits while the replacement close dialog is open. The regression
covers both Keep and Escape, including completion races. App-level navigation
also retains the captured review/drafts; the final suite passes 25/25.
Whitespace checks pass. Earlier wireframes are untouched. No Rust or packaging
code changed, so local Rust suites were not rerun for this design-only pass.

The sample does not install, write project files, collect credentials or upload
assets. Source/path refusals represent the proposed native contract, not new
guarantees in today's CLI. The existing helper's warning-only failed-add behavior
remains a native execution concern; this pass changes no runtime code.

No supported inline rendering-inspection route was available. Actual geometry,
clipping, keyboard activation, platform dialogs and assistive technology remain
unverified. The test harness supplies dialog open/close shims; DOM focus assertions
do not establish native modality or real-window acceptance. Native previews and
memory-stability acceptance remain paused.

## Bounded owner review

1. Select **Review setup**, then **Set up tool…**. Check that Skyline and the
   ordered pin/file effects are named; Escape should keep reviewing (30 seconds).
2. Confirm setup, browse Lantern, and use **Resume Skyline setup**. Advance the
   simulated setup until complete; ownership and Watch should stay unchanged
   (30 seconds).
3. In Sample scenarios, load **Missing manifest and configuration**, choose
   **Tool failed, configuration created**, then review/apply/complete. Expect
   **Failed** with separate file results, never overall success (60 seconds).

## Delivery ledger

Branch: `codex/project-tool-setup-design`. Signed CodeRabbit 0.7.6 is available.
The sandbox reported signed out; checking outside it found the existing account,
so reauthentication was unnecessary. Its first review covered all eight files
and returned the one close-choice finding documented above. The second review
covered all eight files with zero findings. The subsequent evidence-only update
was inspected directly. The tracked fragment and inline copy have identical
SHA-256 hashes. [PR #154](https://github.com/chatarabdelilah/rproj/pull/154)
is the delivery ledger for the exact final reviewed-head and merged-main SHAs,
CI runs/attempts and branch cleanup. Initial commit `23998e2` preceded this
evidence-link correction; its CI does not substitute for final-head verification.
Head `e733ef9` then failed the unchanged Watch fixture's executable-path assertion
on both attempts of CI run `43776424821`. Separately approved
[PR #155](https://github.com/chatarabdelilah/rproj/pull/155) corrected the locator
and a subsequently exposed equivalent Windows Clippy expression. It merged at
`0a0dcf8` with reviewed-head and main CI verified; its ledger retains the unrelated
terminal paste timeout and passing unchanged-commit rerun. This design branch
then incorporated verified main, retaining both release-audit entries. Prototype,
fixture and JavaScript test contents are unchanged from their reviewed versions.
Published version remains 0.22.0. No native window was launched.
