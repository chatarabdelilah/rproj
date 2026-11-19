# Native custom test arguments

Approved November 18, 2026 against clean main `c17eca8`. The owner accepted the
[Soft design](desktop-test-arguments-design.md) and selected cancellation-and-close
protection for both default and custom Test. Checks remains unchanged.

## Delivery sequence

1. Shared exact invocation/context logic and versioned private worker. Preserve
   CLI/Ratatui/default-worker behavior; read-only context, checked captured inputs,
   bounded identity-checked reports, accepted submission and owned cancellation.
2. Native Soft argument controls, captured session draft, literal/JSON-string
   editing, independent default Test, Activity and both Test close protections.

No configuration/preference schema changes, version bump, publication, installer
or native preview. Published version remains 0.22.0. Real-window, native clipboard,
memory stability and macOS/Linux desktop acceptance remain unverified/paused.

## Implementation and evidence

PR 1 branch: `codex/test-arguments-worker`. Establish invocation fixtures before
extracting the pure construction/backend logic. Context captures project identity
and relevant configuration, not test/source contents. Tool execution remains in
the CLI. A custom submission uses the reviewed project/runner/arguments; default
tests resolve current context independently. Preparation changes can require
explicit Refresh before another custom run without discarding its arguments.

PR 1 is delivered through [#160](https://github.com/chatarabdelilah/rproj/pull/160).
Shared invocation fixtures preserve TestEZ and
Jest prefixes, exact argument values and backend precedence. Configuration capture
is read-only and checks identities and digests, including absent inputs. The
private worker validates the captured context before acknowledging a submission,
retains runner exit codes and writes bounded atomic reports. The client waits for
owned descendants and stream draining; invalid reports yield uncertainty rather
than success. The legacy default-Test verb is unchanged.

Local evidence (November 18): focused shared/model/report tests 7 passed; checked
CLI invocation/context tests 14 passed; worker tests 10 passed, including exact
Unicode/empty/quote/CR/LF/tab arguments through Windows processes, stale-context
refusal, runner exit 7, corrupt reports, competing operations, cancellation and
descendant cleanup. Full CLI/core suites passed with 25 prerequisite-gated tests
ignored; headless desktop compatibility passed all 176 tests. CLI/core strict
Clippy passed. Native controls are still PR 2 work, not an acceptance claim.

An initial sandbox run of the seven existing invocation tests had four passes
and three Windows Watch-storage access failures. The tests now use isolated Watch
storage; the permitted rerun passed all seven. Build-cache hardlink warnings fell
back to copying and did not fail validation. Ignored prerequisites are not passes.

Strict desktop Clippy and the portable package build/smoke checks also passed.
Archive: `rproj-0.22.0-dev-windows-x86_64-20261118-093339-7ed951.zip` (development
working tree). The existing three legacy dependency notice gaps remain explicit.
No native window or real runner was launched. The review and delivery evidence
below supersedes these initial validation results.

### First CodeRabbit review (November 19 follow-up)

The first local review returned 14 findings/questions. Corrections bind parsed
record bytes to their captured digest, remove parser source values from diagnostics,
use an owned eight-second support probe with cancellation, derive Finished events
from the validated report, separate read-only refresh errors, return typed effects,
clarify the IPC-directory error and separate the fixture sleeper environment.
Checked custom runs also respect `--` when deciding Studio preflight; the old
terminal helper remains unchanged for compatibility. Its option parsing after
`--` is a recorded existing limitation for a separately scoped correction.

The post-spawn error concern is refuted by `execute_owned_os`: its entire
post-spawn body carries `OwnedTeardownUnconfirmed`; ordinary errors are pre-spawn.
Shared fixture state and real Watch-store mutation are refuted by a fresh TempDir
per fixture, executable copies beneath that root and the driver's explicit
`with_test_root(root/watch)`. Fixture sleepers exit after 60 seconds, bounding the
missing-event/pipe inheritance failure scenarios; elapsed assertions fail a delayed
return. They do not establish a general production teardown deadline.

Upstream [Windows release packaging](https://github.com/christopher-buss/jest-roblox-cli/blob/main/.github/workflows/release-binaries.yaml)
produces a native Node SEA `.exe`, not a batch shim. The
[current parser](https://github.com/christopher-buss/jest-roblox-cli/blob/main/src/cli-args.ts)
uses argument arrays and Node option parsing. This source inspection is not real
Jest/Roblox execution acceptance; actual installed versions/forks remain unverified.
No runner was installed or launched for that investigation.

The second review returned one API-consumer question and one timing advisory.
The API contract explicitly makes the returned completion/error authoritative;
events are progress, and a pre-spawn failure need not emit Finished. A regression
removes only the disposable worker after Started and verifies a failed, unaccepted
completion. Native consumers must clear their running state from that completion
(PR 2). Fixture timing ceilings now allow 30 seconds, still below the 60-second
sleepers, while retaining status and descendant-reaping assertions.

The post-correction full CLI/core run passed 878 tests with 25 prerequisite-gated
tests ignored (the subprocess driver's nested one-test log is not counted twice).
Strict CLI/core Clippy passed again. A second portable build/smoke pass produced
`rproj-0.22.0-dev-windows-x86_64-20261118-232951-f4183c.zip`, retaining the same three
notice gaps. The first offline metadata lookup included uncached non-Windows
dependencies and failed; the Windows-filtered lookup succeeded without downloads.

The third local CodeRabbit pass completed with zero findings. Final targeted worker
coverage passed all 14 tests; strict CLI/core Clippy and formatting passed again.
Final source/diff inspection found no unintended default-Test or project-format
changes. Reviewed head `fed2fbdb0937f87addf3f700ce48d4a664ee3ee9` passed all seven
executed jobs in [CI 44421839553](https://github.com/chatarabdelilah/rproj/actions/runs/44421839553),
attempt 1. Squash merge `0a67a590718a009e3b4c9cfadff16e81a5ce6546` passed the same
seven jobs in [main CI 44422941794](https://github.com/chatarabdelilah/rproj/actions/runs/44422941794),
attempt 1; badge freshness was skipped on both. Reviewed and merged trees match.
The merged local and remote worker branch were removed.

## Native controls (PR 2, November 19)

Branch: `codex/test-arguments-native`, based on the verified PR 1 merge. Checks
offers a separate Run with arguments flow with a captured project, ordered exact
values, literal/JSON modes, explicit empty consent and invocation/effect review.
Native paste interception preserves CR/LF/tab bytes before input normalization.
Malformed JSON stays editable and blocks execution while retaining the last valid
value. Session-only drafts survive browsing and closure; refreshing retains rows.

Run freezes its submission while later edits remain unsubmitted. Only an accepted
worker report advances the submission baseline; returned completions, not progress
events, clear Running. Results retain their owner and bounded Activity. Default
Test uses the existing worker verb independently of custom support or draft state.
Both Test paths now offer Keep open / Cancel tests and close, wait for owned
teardown and then protect other dirty drafts. Unconfirmed teardown blocks closing.
Checks retains its existing behavior. Steady Watch is never stopped or retargeted.

Initial headless validation exposed three test failures: the minimum-size layout
placed the first row below explanatory text, a second-row click omitted scrolling,
and a paste assertion ran before deferred input dispatch. The layout now prioritizes
the controls and removes the unrelated active-project path; tests scroll and await
dispatch explicitly. The targeted rerun passed all 11 tests. Additional integration
fixtures initially used an incorrect enum spelling; compilation caught it before
execution and it was corrected. Full headless desktop validation passed 190 tests;
strict desktop Clippy and formatting passed. Full CLI/core suites passed 879 tests
with 25 prerequisite-gated cases ignored. Strict CLI/core Clippy passed, with the
previously documented incremental-cache hardlink fallback warnings.

The first PR 2 portable build and version/inspection smoke checks passed:
`rproj-0.22.0-dev-windows-x86_64-20261119-054919-48d068.zip` (development tree).
590 package notices were collected; the same three legacy archive gaps remain.

Stronger initial-focus assertions then exposed that a primary label alone does
not focus the Keep control. The controller now enters Keep's tab stop through a
footer anchor after layout. Two intermediate test runs still failed because Kit's
headless `render_frame` does not deliver platform frame callbacks. The documented
`simulate_next_frame` drives that delivery explicitly; all 14 targeted tests then
passed, including actual Keep focus and Escape restoration. No assertions were
weakened, and no native window was opened.

### PR 2 CodeRabbit review

The first local pass returned six findings/questions. The support-probe failure
message is now visible in the retained argument panel. The five questions were
checked against execution paths and regression evidence:

- `start_default_test` uses `begin_test_operation`, which rejects a busy lane or
  unconfirmed teardown before changing ownership. An isolated worker test proves
  idle default Test runs despite an invalid custom draft/unsupported custom verb;
  a competing Check test proves both Test entry points preserve the active lane.
- `test_close` matches both Test variants regardless of panel visibility. Both
  cancellation paths wait for completion, protect another dirty draft and leave
  Watch unchanged. Typed unconfirmed teardown blocks closing and mutations.
- Stop-and-switch occupies the serialized lane until the existing worker returns
  after Stop acknowledgement and destination startup. Refresh cannot enter that
  lane or clear its pending destination. `pending_switch` is a pre-execution
  confirmation, not the worker's captured destination.
- Project inspection canonicalizes the root, and `enter_project` clears the old
  snapshot before a test entry is available. The context and captured operation
  therefore share canonical identity. The worker independently refuses reparse
  roots before reading context; foreign completion paths remain rejected.
- `Status::Uncertain` is produced only after owned execution and stream draining
  return. Unconfirmed teardown instead carries `OwnedTeardownUnconfirmed` through
  `Failure`, never an ordinary completion. Unknown outcome after confirmed exit
  requires Refresh but does not imply a still-running child.

Final inspection also found a support-probe lifecycle edge case: mapping a probe
error directly to text would lose `OwnedTeardownUnconfirmed`. Support events now
retain the typed failure. Even a superseded probe's unconfirmed teardown blocks
closing and mutations without replacing newer support availability. Its dedicated
regression passes; the full desktop rerun passed 191 tests and strict Clippy.
An ordinary unsupported-worker error still disables only custom Test.

A second portable build/smoke check passed after the focus correction:
`rproj-0.22.0-dev-windows-x86_64-20261119-055653-514905.zip`. A third build includes
the typed probe guard: `rproj-0.22.0-dev-windows-x86_64-20261119-055948-84e973.zip`;
version and inspection smoke checks passed with unchanged notice gaps.
The second local CodeRabbit pass returned zero findings; the final probe change
is undergoing another pass. Reviewed-head CI, merged-main CI and branch cleanup
remain pending.
