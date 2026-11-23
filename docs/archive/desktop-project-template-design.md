# Project Template proposal

Approved design scope, October 29, 2026, against main `f70da78`.
This milestone is a separate conversation prototype. Native implementation follows
owner review. Explorer plus Advanced JSON, mandatory Review before Save, and
wait-for-completion closing are settled decisions.

The Soft shell adds app-level Project Template beside Machine Setup and Saved
Setups. It edits a captured machine-wide override for future projects; browsing
projects never retargets the draft or changes Watch. Existing projects remain
untouched. One session-only template draft stays independent of other drafts.

Explorer provides tree operations, typed properties/attributes, project settings,
search and undo/redo. The sample metadata is representative, not a complete Roblox
catalog. Advanced JSON applies to the same draft only after structural validation;
invalid text remains available for repair and cannot replace the last valid tree.
Required nodes, source mounts, reserved names and path restrictions follow the
terminal model. Unknown fields survive edits; incompatible properties require an
explicit repair. No model import or arbitrary-file opening.

Review presents semantic changes, complete wrapped unified diff, formatting effects
and simulated structural/Rojo validation. Save confirms the captured path and
defaults to Keep reviewing. No-op/reverted edits preserve bytes and create nothing.
Reset separately reviews removal of the override and the resulting built-in tree,
defaults to Keep custom template and discards pending edits only after success.

Changed sources require explicit Reload from file. Dirty reload/discard defaults
to Keep draft. Operations are serialized and own searchable bounded Activity;
ordinary output respects manual collapse. Failure preserves edits; uncertainty
requires reload. Closing during validation, Save or Reset offers Keep open or
Close after operation, waits for confirmed completion and protects other drafts.
There is no cancellation control; unconfirmed termination keeps the sample open.

All operations use local sample data only. No Rust/API/worker/format/release changes,
native previews, real project writes, tools, installation or Windows troubleshooting.
Version 0.22.0 remains unchanged. Earlier wireframes are left unchanged. Validation
and rendering limitations are recorded in the [acceptance record](desktop-project-template-acceptance.md).
