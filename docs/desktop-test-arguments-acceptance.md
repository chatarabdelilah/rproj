# Custom test arguments wireframe acceptance

The owner approved the [design scope](desktop-test-arguments-design.md) on
November 18, 2026 against main `f942937`. The separate
[Soft proposal](prototypes/test-arguments.html) uses sample data and simulated
execution only. Native implementation requires subsequent design approval.

## Automated evidence

The initial Node/DOM run passed all **25 checks**, with no failures or skips,
using the existing jsdom 26.1.0 installation:

```powershell
node docs/prototypes/test-arguments.test.cjs target/new-project-design/dom-runtime/node_modules/jsdom
```

Coverage includes exact TestEZ/Jest prefix and argument fixtures; literal
spaces/quotes/Unicode/empty arguments; NUL refusal; ordering and focus;
unchanged default invocation; context refresh; owner/draft retention; explicit
replacement; unavailable contexts; steady versus transitional Watch states;
serialized operations; immutable submissions; stale/duplicate events;
cancellation/completion races; bounded Activity, filtering and height retention;
unsubmitted/independent draft closing; and uncertainty blocking close.

Syntax, useful literal startup without host helpers, unique identifiers,
accessibility references and responsive CSS contracts are checked. Twelve
text/surface combinations pass 4.5:1 contrast in each theme (24 comparisons).
The DOM harness supplies dialog show/close shims; handler and focus assertions
do not establish real platform modality, keyboard activation or assistive
technology acceptance. CSS checks do not establish actual geometry or clipping.

The 25-check run passed again after captured-owner/context refinements. The
unchanged Project Tool Setup suite also passed all 25 checks. Whitespace checks
passed. No runtime or packaging files changed; local Rust/packaging suites were
not rerun for this design-only milestone.

No supported inline rendering-inspection tool was available. Rendering remains
unverified; no retired automation route or native preview was attempted. No
actual test runner, project write, installation or clipboard operation ran.
Native memory stability remains unresolved. Real-window, Studio/Open Cloud and
macOS/Linux desktop acceptance remain outside this milestone.

## Bounded owner review

1. Change the second argument to `inventory with spaces`. Check that the preview
   shows one quoted argument, then run and use **Finish preparation** and
   **Complete active run** under Sample scenarios (30 seconds).
2. Browse Lantern, then **Resume Skyline arguments**. Confirm the captured
   identity and result still name Skyline; default **Run tests** in Checks
   remains separate (30 seconds).
3. Edit an argument and close the sample. Escape should keep it. Run again,
   close and choose **Cancel tests and close**; the sample stays open until
   **Confirm cancellation / teardown** (45 seconds).

## Delivery ledger

Branch: `codex/test-arguments-design`. Local CodeRabbit review, final diff
inspection and exact reviewed-head/main CI verification are pending. No release
version change. Earlier working wireframes remain unchanged.

CodeRabbit 0.7.6 has a valid CodeRabbit Inc. Authenticode signature. The sandbox
reported signed out; the read-only check outside it found the existing account.
No reauthentication was required.

The first CodeRabbit review covered all nine files and found one issue:
single-line inputs silently remove CR/LF. Added explicit JSON-string editing,
multiline-paste preservation and editor-to-submission regressions. A separate
inspection also found that the unavailable-worker sample needed to block
default runs after project navigation; its regression passed. All **29 checks**
now pass, including exact CR/LF/tab submission and invalid-JSON draft protection.
The second review is pending.
