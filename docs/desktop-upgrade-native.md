# Native Upgrade review

The approved Soft workbench offers **Review configuration changes…** on Overview,
identified as `rproj upgrade`. It uses the matching private worker and shared
checked executor. Opening/reviewing makes no project changes; package/tool
updating is separate. Missing records, unsafe merges, relevant incompatible tool
pins and unreadable/linked files remain actionable preparation failures.

## Captured review and presentation

One session-only review keeps its captured display name, canonical path, proposed
files, reasons and safe skips across browsing and project closure. Resume appears
on Overview and in the compact project menu. Replacing or explicitly refreshing
defaults to **Keep review**. Creation/composition drafts are independent. An idle
read-only review adds no dirty-draft close prompt; their existing protections stay.

The captured path remains above the scrolling review. File list/detail panes are
adjacent at 600 logical pixels of main-content width and stack below that point.
Selection survives resizing. Unified diffs show old/new line numbers, explicit
markers, escaped controls, CRLF/missing-final-newline annotations and three-line
context. Long lines wrap by default. Tab reaches the diff, and keyboard
arrows/Page Up/Down/Home/End scroll it with a theme focus border. Resizing or
toggling wrapping retains the logical scroll position. Diff construction runs off the UI
thread; the pinned GPUI list renders only its viewport, including variable-height
wrapped rows. Invalid/incomplete review data blocks Apply rather than showing an
empty applicable diff. Safely skipped files show explanations, never invented diffs.

## Applying and closing

Only **Apply all proposed changes…** is offered. Its confirmation names the
captured project, counts proposed/skipped files and defaults to **Keep reviewing**.
Revision/project/Watch checks invalidate stale confirmations; the worker checks
actual source identities before writes. Source changes require explicit Refresh
review. Success retains its captured result and asks for a fresh review before
claiming anything about current or remaining changes. Skipped-only results are
distinct from up-to-date projects.

Same-project Watch blocks Apply; its global Stop control remains separate and
must acknowledge completion. Another owner's Watch remains unchanged. Browsing,
reviewing and applying never start, switch or stop Watch. Upgrade shares the
serialized operation guard, captured Activity ownership, bounded searchable
output, retained panel height and manual-collapse rules.

Cancel stays pending until the owned worker/descendants exit and streams drain.
Confirmed completion wins a late cancellation request. During Apply, close offers
**Keep open** or **Cancel changes and close**, naming the project and warning about
partial changes. It waits for teardown, then honors other dirty drafts. Unconfirmed
teardown blocks closing and new operations. An uncertain file report after
confirmed teardown remains distinct and does not falsely imply living workers.

## Evidence and boundaries

The desktop suite has 64 passing tests: 50 existing tests, four new pure Upgrade
checks and ten new headless GPUI interactions. Coverage includes pane layouts,
selection, default-Keep/focus, captured ownership, stale confirmations/events,
outcomes, cancellation/close coordination, minimum-size expanded Activity and
both themes. A 2,203-row wrapped diff exposes its final row through actual keyboard
End without rendering every row. Disabled actions are tested by activation;
absent Kit accessibility disabled flags are not interpreted as evidence.

A separate real bundled-CLI integration test verifies support/context/owned
preparation dispatch and unchanged fixture files. It executes no desktop window
or external tool. The existing worker fixtures cover mutation/teardown separately.
The FileText action icon is included in the existing asset bundle and tested;
existing licenses/notices remain in place. Toolchain, initial/minimum window
sizes, appearance persistence and preference/project/inspection schemas are unchanged.

Native previews remain paused because memory stability is unresolved. Headless
layout and keyboard observations do not establish real-window, GPU, scaling,
System-appearance or assistive-technology acceptance. No installer, OS/VM work,
version bump or publication belongs to this milestone. Exact reviewed-head/main
CI evidence and branch cleanup are recorded on each implementation PR.
