# Native Upgrade review design

Approved scope October 18, 2026, against clean main `490e2ee`: build a separate
interactive Soft proposal before planning native implementation. The owner chose
unified diffs and retained the terminal's Apply-all policy. Native preview launches
remain paused; this milestone changes no Rust, public API, protocol, project
format, preference schema or release.

## Design contract

Overview offers **Review configuration changes…**, identified as `rproj upgrade`.
The captured project and full path stay visible. Review lists proposed files,
reasons and safely skipped files, with a read-only unified diff. Apply confirms
the captured project and all proposed changes, defaulting to Keep reviewing.
Package/tool version updates remain separate from generated-configuration refresh.

One captured review survives browsing and project closure; replacement defaults
to Keep review. Source changes require explicit Refresh review. Same-project Watch
blocks Apply; acknowledged Stop remains separate. Another project's Watch and
the independent New Project/composition drafts are preserved.

Activity retains operation ownership, bounded searchable output and its chosen
height. Cancellation remains pending until simulated teardown acknowledges it.
Close defaults to Keep open; cancelling and closing warns about partial changes
and waits for teardown. Unconfirmed teardown keeps the sample window open.

## Review presentation

The initial screen shows Skyline's captured review, four proposed files and the
first unified diff. Added/removed markers, old/new line numbers, three context
lines, escaped controls, CRLF and missing final newlines remain readable without
depending on color. Long lines wrap by default. List/detail panes are adjacent
with at least 600 logical pixels of main-content width and otherwise stack.

File rows select details; they are not per-file Apply checkboxes. Skipped files
show an explanation without a fabricated diff. A safe plan with skipped files
can still apply its proposed changes; skipped-only results do not claim the whole
project is up to date. Unsafe merges and preparation errors block the plan.

Fixtures cover Wally/None, excluded CI, preserved custom settings and Jest's
compatible `default.project.json` development-package mount. Housekeeping follows
the existing Upgrade eligibility rules, not the composition editor's stricter
exclusion policy. The Wally CI pin requirement applies only when relevant; it
does not install or update a tool. Missing/malformed records, unreadable/linked
files and unsafe merges remain actionable errors.

## Proposal and evidence

The separate thread-local proposal is `rproj-upgrade-review-wireframe.html`.
Its SHA-256 is `e1f05504b62b4fb692d993f847424a7f5045ba2db7581b13b74e9d5e4cdb7db1`.
It uses representative sample before/after contents derived from current source
inspection, not a freshly executed Rust plan or real project files. Optional
host state/icons cannot prevent initial review content from appearing.

[Upgrade design acceptance](desktop-upgrade-acceptance.md) records 30 passing
simulation/DOM checks, 28 numeric contrast comparisons and unchanged hashes of
both earlier working wireframes. Rendering remains unverified: no supported
inline inspection route is exposed. Browser/native automation was not attempted.

This proposal awaits owner design approval. Review the selected file diff and
the default-Keep Apply-all confirmation; native implementation planning follows
that review. No Rust extraction, worker interface or native acceptance is approved
by completing this documentation/prototype milestone.

Documentation delivery and exact head/main CI evidence are tracked in
[PR #123](https://github.com/chatarabdelilah/rproj/pull/123).
