# Custom test arguments design

Approved November 18, 2026 against main `f942937`: a separate Soft proposal
for Checks → Run with arguments. Default Run tests stays unchanged. Exact
argument rows and the invocation preview form the review; Run needs no second
confirmation. One session-only draft captures its project, and unsubmitted
edits receive default-Keep protection. Native implementation follows approval
of this proposal, not this document alone.

Fixtures follow `src/commands/test.rs::invocation` and `uses_studio_cli`.
TestEZ invokes `lute test`; Jest invokes `jest-roblox-cli --passWithNoTests`.
The user arguments follow those prefixes unchanged. Rows are argument values,
not shell input. Documentation and examples do not guarantee acceptance by
every installed runner version. Editing changes no project configuration.
Running may synchronize tools/dependencies and refresh Jest configuration.

Watch follows the existing test guard: steady Watching is allowed, including
on the same project; active preparation/recovery/stopping blocks testing.
Tests never stop or retarget Watch. This differs deliberately from the
configuration-mutation guard.

Use local samples and deterministic simulated operations only. No Rust,
worker/API, configuration, release or earlier-wireframe changes. Native
previews remain paused. Published version remains 0.22.0.

## Interaction decisions

Each ordered row holds one exact value. A blank row needs explicit Pass empty
argument consent; NUL is refused. Preview uses JSON string notation to expose
argument boundaries and controls without changing submitted bytes. Add/remove
and labeled Up/Down actions are keyboard accessible. No shell expansion occurs.
Literal mode accepts a single line; an explicit JSON-string mode represents
CR/LF and other controls exactly. Multiline paste switches to that mode before
the text input can sanitize the line endings. Invalid JSON remains editable,
blocks Run and protects the unsubmitted text; it never replaces the last valid
argument. Switching presentation alone does not mark the draft dirty.

The captured draft survives browsing and project closure. Replacing, refreshing
or discarding unsubmitted edits defaults to Keep draft. Context changes block
Run until explicit refresh; arguments are retained for inspection against the
refreshed runner. Running freezes a captured invocation while later edits stay
unsubmitted. A successfully submitted invocation becomes the comparison baseline,
even if that run fails or is cancelled. Pre-start refusal does not submit edits.

Closing with unsubmitted edits offers Keep draft/Discard. Running close offers
Keep open/Cancel tests and close, waits for teardown and then protects remaining
drafts. Uncertain teardown blocks closing and further runs. Completion while a
dialog is open must not bypass its choice. Results retain captured ownership;
normal output and success do not reopen manually collapsed Activity.

See [acceptance](desktop-test-arguments-acceptance.md) for validation and limits.
