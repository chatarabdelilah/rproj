# Native Saved Setups design

Approved design scope October 27, 2026, against clean main `d035610`.
Deliver a separate Soft conversation prototype, not native controls.
Review-first editing, Use in New Project, resolving same-setup edits before reuse,
and default-Keep wait-for-completion closing are settled decisions.

The prototype uses frozen sample documents and simulated operations only.
Native previews and real installation acceptance remain paused. No Rust, public
API, worker protocol, persisted format, version or publication changes belong to
this pass. Existing working wireframes stay unchanged.

## Placement and browser

Add app-level **Saved Setups…** beside Machine Setup below Overview,
Dependencies, Tools and Checks. Keep the compact project switcher, appearance,
Activity and global Watch. The initial sample is Skyline with its list entry,
full storage path and composition visible before JavaScript or optional icons.
Search filters names; adjacent browser/detail panes stack below 600 logical
pixels of content width. Long paths wrap and remain selectable.

The fixtures include Wally, None, explicit TestEZ/local Studio/Open Cloud choices,
unknown metadata and exclusions, malformed and unsupported documents, unreadable
and linked files, linked storage, empty storage and missing machine context.
Browsing creates nothing. Empty storage points to New Project's named-setup save.
Recorded composition is not installation or package-resolution evidence.

Readable safe documents retain Rename, Duplicate and Delete even when malformed
or unsupported. Such documents cannot enter guided editing or New Project reuse.
Unreadable files and linked/reparse files or storage refuse operations visibly.

## Review-first composition editing

Edit captures one document and opens **Review**, with targeted Dependencies,
Capabilities and Files links. It can change workflow, add/remove packages, choose
implementations/backends and change eligible optional exclusions. The frozen
catalog contains 28 packages, nine capabilities and 24 artifact descriptions;
catalog keys/text and implementation references were checked against the current
core sources and their hashes recorded locally.

Companion packages follow the existing React/Reflex/Charm/Ripple rules. CI requires
Gate. TestEZ supports Wally and None; Jest requires Wally and an explicit backend.
Changing to None with selected packages or Jest requires an explicit decision:
keep the workflow, remove packages and use TestEZ, or remove packages/Jest testing.
Required source/test/Jest files cannot be excluded. Unknown and unrelated
exclusions remain retained. Without machine context, app-dependent optional file
choices are unavailable; other composition choices remain editable.

Review lists semantic changes and offers an expandable complete wrapped sample
unified diff with old/new line numbers, three-line context, explicit +/− markers,
escaped control characters and CRLF/missing-final-newline annotations. Unmarked
line endings are LF. The sample writer deliberately demonstrates comment loss,
line-ending changes and TOML formatting effects; it is a frozen design fixture,
not a newly implemented serializer or a byte-for-byte promise for every real
document. Provenance and unfamiliar metadata survive changed saves. No-op and
reverted edits preserve the captured original bytes and perform no operation.

**Save setup…** confirms the captured name/path and semantic change count,
defaulting to **Keep reviewing**. Saving affects future reuse only: no existing
project changes, tool installation or Watch mutation.

## Ownership and management

One session-only editor draft remains attached to its captured setup across setup
selection, project browsing and project closure. Resume names that setup. Replacing,
discarding or explicitly reloading a dirty draft defaults to **Keep draft**.
Creation, composition, settings, machine and Upgrade state remain independent;
the prototype exercises creation-draft conflicts directly and represents the other
flows as retained state, without reproducing their editors.

Rename and Duplicate preserve original document bytes and confirm source and
destination. Names follow existing safe-filename/collision rules; case-insensitive
collisions cannot overwrite a setup. A case-only rename is refused with guidance
to use a distinct temporary name. Delete names the exact path, defaults to
**Keep setup**, and explains permanent removal without affecting existing projects.

Use in New Project, Rename, Duplicate and Delete first resolve unsaved edits to
that same setup. **Keep draft**, **Review and save first**, or explicit discard
are distinct choices. Saving still requires its own reviewed confirmation.
Reuse loads the saved composition at New Project's **Project** step, protecting a
nonempty creation draft with default **Keep draft**. No project is created or
automatically saved. The handoff does not change Watch.

## Execution and recovery simulation

Source changes invalidate review and pending actions. Explicit **Reload from file**
captures the current sample document; no automatic merge occurs. Simulated
mutations serialize and capture operation ID, setup owner and source revision.
Watch may continue on any project throughout setup management.

Simulation controls explicitly deliver success, known failure, partial Rename,
or uncertain completion. Success advances the saved baseline. Failure retains
edits. Partial Rename leaves both original and copied names and requires reload.
Uncertainty blocks another mutation and closing until termination is explicitly
confirmed; source reload is then required before retry. Output is never proof of
success. These controls simulate events, not filesystem or process guarantees.

Activity retains captured setup ownership across navigation, at most 100 sample
output records, search and distinct empty/no-match states. It expands on start
or new failure; normal completion respects manual collapse. Height is retained
through collapse/reopen. Watch output remains accessible separately.

Close during Save/Rename/Duplicate/Delete defaults to **Keep open**, with
**Close after operation** waiting for confirmed completion and then protecting
remaining dirty drafts. There is no cancellation action. Completion under an open
dialog removes the stale close choice without implicitly closing. Dismissal returns
focus to its trigger; if a stale-source refusal disables the trigger, focus moves
to Reload. Unconfirmed completion keeps the sample window open.

## Evidence and next boundary

[Acceptance](desktop-saved-setups-acceptance.md) records local model/DOM checks,
numeric contrast comparisons and explicit rendering limits. The conversation
prototype and validation scripts are ignored, thread-local artifacts; this PR
contains documentation only. Earlier wireframes remain unchanged.

Owner design review precedes any native implementation plan. Native memory
stability remains unresolved; real-window acceptance, installers and macOS/Linux
desktop acceptance remain outside this milestone. Published version remains 0.22.0.
