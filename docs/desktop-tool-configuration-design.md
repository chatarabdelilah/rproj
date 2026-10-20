# Native Configure Tools proposal

## Native implementation approval — October 20, 2026

The owner approved Settings → Review against main `3ccade4`, delivered through
three sequential reviewed PRs: shared planning/checked saving, private worker,
then native Soft controls. Ineffective legacy-setting edits must be refused,
not silently discarded; catalog modernization is deferred. Save has no Cancel
action, and Close after save waits for confirmed completion before protecting
other dirty drafts. Native previews remain paused. Automated evidence and
real-window acceptance remain separate; no version bump or publication is planned.

Approved scope, October 19, 2026, main `820dae7`: a separate Soft conversation
wireframe for **Settings → Review**, before native implementation. This approves
prototype scope and behavior; the interface still needs owner design review.

## Entry and captured editing

Tools offers **Configure tool settings…**. The existing compact project switcher,
four sections, appearance controls, Activity and global Watch remain. The initial
Skyline/Wally sample opens useful StyLua settings, including its current column
width and team-owned values, even without optional host helpers or icon loading.

The four targets come directly from `src/catalog/tool_settings.rs`: StyLua (nine
fields, `stylua.toml`), Selene (nine, `selene.toml`), Luau Language Server (thirteen)
and StyLua VS Code behavior (two). Both VS Code targets share
`.vscode/settings.json`. All 33 fields retain catalog descriptions, accepted
choices, defaults and documentation links; there are no invented tool settings.

The project name/path, target and file stay visible. The target selector sits
beside settings at 600 pixels of main-content width and stacks below. The form
has a bounded scrolling region; Review shows the full wrapped diff. Narrow
previews reflow the sidebar, controls and paths without changing the draft.

Current file values, suggested defaults and pending choices are distinct. Missing
values stay unset until explicitly chosen; **Use suggested default** is an action,
not an automatic write. **Revert change** restores the captured value or absence.
Boolean/choice controls use catalog options; integer input represents signed
64-bit values without JavaScript number rounding. Invalid input blocks Review.
Unfamiliar values, including explicit JSON null, remain visible and locked until
default-**Keep existing** replacement consent. Unlocking alone stages nothing.

One session-only project/target draft survives browsing, project closure and
section changes. Resume names its captured owner; browsing never retargets it.
Dirty target/project replacement and explicit Reload default to **Keep draft**.
Independent creation/composition drafts and the Upgrade review remain intact.
Changing a setting invalidates Review. Source changes, including creation of a
previously missing target, require explicit reload rather than automatic merging.

## Review and Save

Review shows changed values and the complete proposed single-file unified diff:
old/new positions, three context lines, explicit markers, escaped controls,
CRLF and missing-final-newline annotations. Wrapping is on. Missing and empty
files have distinct identities; new files contain only explicitly selected values.

Source-derived samples preserve TOML comments, unrelated keys and table ownership.
JSON samples show the current writer's sorted pretty-printing and superseded-key
cleanup when replacement keys exist, including effects beyond the selected field.
Neither shared VS Code target may imply ownership of the whole file. Unrelated
editor values remain. The diff is an exact before/after **sample**, not a real
project execution plan or evidence of a general-purpose prototype parser.

**Save settings…** confirms the captured project, full path, target, destination
and file creation/replacement; **Keep reviewing** is focused and primary. Source,
review and Watch changes expire confirmation. Same-project Watch blocks Save
until the separate Stop control acknowledges Stopped. Another project's Watch
is unchanged. Configuration editing needs no installed executable, completed
global setup, dependency workflow or rproj record. It installs/enables no tool.

Simulated Save is serialized and retains captured operation identity. Success
advances that draft's baseline and clears submitted edits while retaining the
reviewed result. Known failure keeps edits; uncertainty requires inspecting and
explicitly reloading the file before retry. A source race before replacement
fails without overwriting the sample's external edit. Late results cannot replace
a newer/completed outcome. Output remains bounded, searchable and project-labeled
across navigation; successful output respects manual Activity collapse and height.

During Save, Close defaults to **Keep open**, with **Close after save** as the
explicit alternative. There is no cancellation action. Closure waits for confirmed
completion, then protects remaining dirty drafts. Unconfirmed completion keeps
the sample open. A separate, clearly labeled simulation control can acknowledge
that the sample Save ended; this is not a proposed native user control or a
substitute for worker teardown evidence. Reload still requires explicit consent.
Escape/Keep dismissals return focus to the originating control.

## Fixtures, scope and next boundary

Fixtures cover Wally/None, an unrecorded folder, long Unicode paths, absent/empty
files, unfamiliar values, formatting/endings, loading, malformed/unsafe TOML,
commented/trailing-comma JSON, unreadable/linked paths, Watch conflicts, stale
sources, failure/uncertainty, empty output and other dirty drafts. Refusal cases
are labeled simulations. In particular, linked-path refusal is a proposed native
boundary, not a claim that this pass changed terminal configuration behavior.

No raw editor, key removal, reset-all, installation, activation, version updating,
credentials or template editing belongs to this milestone. The prototype accesses
no real projects or tools. Earlier wireframes are unchanged. No Rust, public API,
worker protocol, persisted format, preference schema, version or publication changes.

[Acceptance](desktop-tool-configuration-acceptance.md) separates source/DOM checks
from rendering and native acceptance. Native previews remain paused because memory
stability is unresolved. Native implementation planning follows owner design approval.
