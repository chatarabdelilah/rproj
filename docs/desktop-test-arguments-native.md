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

PR 1 implementation is in review. Shared invocation fixtures preserve TestEZ and
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
No native window or real runner was launched. CodeRabbit, final diff/CI verification
and branch delivery are pending.

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
changes. Reviewed-head CI, merged-main CI and branch cleanup remain delivery gates.
