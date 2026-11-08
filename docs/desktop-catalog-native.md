# Native Catalog browser

Approved November 7, 2026, against clean main `b7d997b`. The owner approved
the [Soft Catalog proposal](desktop-catalog-browser-design.md) and two sequential
reviewed PRs: shared catalog guidance, then native controls.

Catalog is bundled static guidance, available without a project, setup or a
working CLI connection. Entry identity includes kind and key. Terminal output
and the existing package-worker response remain compatible. Native browsing
preserves project, Watch, Activity and independent draft ownership.

## Delivery evidence

### Shared guidance

Before extraction, the unchanged CLI captured the redirected catalog listing,
96 detail pages, unknown-key output and package-worker JSON in
`tests/fixtures/catalog-baseline.json`. The integration test compares those
bytes, normalizing CRLF and allowing only the Jest correction below. Package examples and tool/topic guidance now live
in core with CLI re-exports. The existing worker projects the same package data.

The shared browser provides all six inventories, typed identities, structured
guides and metadata-derived related links. Tests cover inventory completeness,
overlapping keys, exact examples/imports, settings destinations/values,
capability requirements, file reasons and Scribe/ProfileStore/Studio links.
Related guidance does not imply unconditional installation requirements.

Initial CLI/core validation passed 798 tests with 24 ignored; ignored/live tests
are not acceptance. Clippy passed with warnings denied. All 16 existing Catalog
prototype checks passed after updating the fixture's authoritative source path;
the prototype HTML is unchanged. An initial build failed because Realm has no
Debug implementation; explicit realm labels corrected that display error.

All 143 headless desktop compatibility tests passed. Fresh joint core/CLI
archives verified offline with no version change. CodeRabbit's first local pass
reported zero findings but omitted untracked additions; the complete staged
change received a second review. It found one pre-existing invalid Jest sample:
the outer describe call lacked its closing parenthesis. That sample is corrected;
the original fixture stays intact and the compatibility test explicitly allows
only that exact text correction in terminal pages and worker JSON. Follow-up
CodeRabbit review covered all 17 files and returned zero findings. Core, CLI
and desktop Clippy passed with warnings denied; formatting checks passed.

After the sample correction, the CLI unit rerun had 541 passes and one timeout
in the unchanged terminal JSON-paste fixture. Its exact isolated rerun passed
without code changes. The Catalog compatibility test and all-feature core
suite then passed. The original whole-suite pass and this retry remain separate
evidence. Shared delivery CI is recorded below; the following section covers
the second PR's native controls.

Shared guidance merged in [PR #148](https://github.com/chatarabdelilah/rproj/pull/148)
at `54a65f3`. Reviewed head `f42f600` (CI `42416878677`) and merged main (CI
`42447597395`) passed all seven executed jobs on their first attempts. Their
trees match; the merged local/remote branch was removed. Final corrected core/CLI
archives also verified successfully.

### Native controls (November 8)

The app-level Catalog reads all six inventories directly from core. Startup
connection no longer requests the worker's package catalog; existing creation,
composition, saved-setup and dependency details use the same bundled projection.
Recognized dependency links match canonical Wally identities, never aliases.

The initial guide is Scribe. Search defaults to all sections; kind plus key
distinguishes overlapping names. Filters keep the selected guide, explaining
when it is outside the results. A virtualized list and complete selectable
guide use adjacent panes at 600 logical pixels of main-content width and stack
below. Examples wrap and Copy example submits their exact text, reporting only
Copy requested. Documentation uses the platform URL boundary.

Session state retains query, scope, selection, pane/page offsets and at most 30
Back bookmarks. Related-entry Back restores its originating control's focus.
Sidebar navigation retains the Catalog. Recent-project selection and closure
keep it visible; explicit Projects Open returns to Overview. Existing drafts,
operation ownership, Watch, output filtering and Activity height remain separate.
Catalog adds no operation, preference or dirty-close state.

The full headless desktop suite passed 151 tests, including eight Catalog tests
and extended Projects-open coverage. Test-platform clipboard and browser checks
verify exact requests; they do not establish real OS integration. Initial test
builds exposed a wildcard test-macro collision and fixture API mistakes. The
first interaction run had two failures: its Signal sample was not actually in
the catalog, and deferred Back restoration was not driven by the headless draw
route. The canonical fixture now uses Scribe; restoration is synchronous with
retained session handles, eliminating a delayed focus/scroll overwrite. Eight
targeted tests then passed. Two redundant test conversions found by Clippy were
removed. The final eight-test Catalog rerun includes Light/Dark headless pane
geometry at 800×580 and 1120×780. Desktop Clippy and formatting passed. Shared
Catalog tests and the exact CLI/worker compatibility regression passed again.

The portable archive build, static-runtime checks, matching-CLI self-check,
version and read-only inspection smoke checks passed. It preserved notices for
590 dependency packages; the existing completeness limitation for three legacy
archives remains recorded in THIRD-PARTY.md. No packaging command opened a GUI.
Local CodeRabbit 0.7.6 reviewed all 18 changed files and found no code issues.
Its one minor finding identified stale delivery wording above; that documentation
correction was inspected directly. Actual reviewed-head/merged-main CI identities
are recorded on the delivery PR once complete.

Native previews, real-window acceptance and native memory stability remain
paused/unverified. No version bump or publication; published version is 0.22.0.
