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
evidence. Actual reviewed-head and merged-main CI identities are recorded on
the delivery PR after those runs complete. Native controls remain the second PR.

Native previews, real-window acceptance and native memory stability remain
paused/unverified. No version bump or publication; published version is 0.22.0.
