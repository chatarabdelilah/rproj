# Native Saved Setups implementation

Approved October 27, 2026 against clean main `58d08e0`. The owner approved the
Soft review-first design and explicit New Project handoff, then approved native
implementation through three sequential reviewed PRs.

## Settled boundaries

- Share document planning, composition validation and checked management before
  adding a private worker and native controls. Preserve terminal navigation.
- Preserve unknown metadata in saved documents. New Project may reuse the known,
  validated composition despite extra top-level metadata; unknown composition
  choices and excluded-file keys continue to block reuse. Editing and reuse have
  separate eligibility. No silent normalization.
- Retain captured session-only drafts and default-Keep confirmations. Mutations
  have no cancellation action. Close waits for confirmed completion and then
  protects other dirty drafts. Watch is unaffected by saved-setup management.
- Native previews, Windows troubleshooting, VM setup and installation remain
  paused. No publication, version bump or persisted-format change. Version 0.22.0.

## Delivery sequence

1. Shared document/draft/review/outcome types, authoritative checked file operations,
   terminal adapters and New Project metadata compatibility.
2. Versioned private support/load/prepare/execute/reuse operations, bounded atomic
   reports and owned noncancellable execution through the matching CLI.
3. Native Soft browser, review-first editor, management dialogs, creation handoff,
   Activity integration and close/focus protections.

Each PR requires applicable tests, formatting, Clippy, package checks, local
CodeRabbit review, direct diff inspection, reviewed-head CI, merged-main CI and
merged branch removal. Failed attempts and reruns are recorded below.

## Evidence

### PR 1: shared planning and checked management

Established original-byte/TOML-metadata fixtures before extraction. Core now owns
decoding, separate reuse validation, preserved-composition drafts, semantic/diff
reviews and typed mutation outcomes. The CLI keeps filesystem execution: captured
storage/source identities, authoritative review verification, permission-preserving
staging, no-clobber duplication, checked deletion and explicit partial Rename.
Terminal actions use this executor. New Project accepts extra metadata while
retaining strict known-choice/exclusion validation and captured-source checks.

The first baseline test run inside the filesystem sandbox passed five and failed
seven tests because temporary-file replacement was denied (Windows error 5).
The exact rerun outside the sandbox passed all twelve; existing tests failed under
the same restriction. No runtime fix was made for sandbox permissions. Early
formatting ran before the new test module existed and was rerun after adding it.
The first creation regression run passed 11/12 and caught a parser diagnostic
that echoed malformed source text. Location-only errors and a dedicated core test
restore the privacy boundary. Strict Clippy initially found two extraction-only
unused members/imports; test-only fields/imports now carry the appropriate cfg.
The first full CLI run passed 519 tests, ignored ten prerequisite-dependent cases,
and found one terminal message regression: stale-save recovery must still mention
refresh. The shared message now retains that guidance alongside Reload from file.

Final local validation passed 738 CLI/core tests (24 prerequisite-dependent tests
remain ignored), 100 headless desktop tests, formatting and strict Clippy for both
toolchains. Fresh core/CLI package verification and the Windows portable version
and inspection smoke passed. Portable notice completeness remains pending for
three legacy dependency archives, as recorded in THIRD-PARTY.md. Local CodeRabbit
review completed with no findings across all eleven changed files; the final diff
was inspected directly. Reviewed-head/main CI evidence follows after delivery.
No native acceptance is claimed. Native memory stability remains unresolved.

PR #136 merged at `5c3c72e`. Reviewed head `592ffa8` (CI `40472652485`) and
merged main (CI `40473410829`) passed all seven executed jobs on first attempts.
Reviewed and merged trees match; both merged branches were removed.

### PR 2: private worker

The worker derives storage from the CLI's own configuration location, bounds
document reads and IPC at 2 MiB, and shares the checked executor. Browsing missing
storage creates nothing; missing machine context remains a visible warning.
Separate support, browsing, loading, preparation, mutation and reuse operations
retain captured document ownership. Mutations have no cancellation API.

The initial compile check caught an unnecessary serialization requirement on
shared diff rows. Reviews now transport the existing document model; diff rows
are computed off the UI thread, matching the other editors. Ten focused core
tests and four direct worker tests pass; owned-process validation is in progress.
An approval-service usage limit prevented one Git alignment command from running.
After the owner asked to continue, the approved retry succeeded without bypassing
the review boundary or changing repository data to work around it.

Owned-worker tests first passed 10/11: one corruption fixture reused a source
already changed by an earlier case, so its final case correctly returned a stale
source failure rather than a contradictory-exit result. Each corruption case now
captures a fresh review. A new Watch comparison initially attempted equality on
a type without PartialEq; it now compares serialized state and is Windows-gated.
All thirteen focused worker tests pass. Strict Clippy requested boxing the large
Execute variant; that correction passes Clippy without changing the wire format.

Local CodeRabbit reviewed all fourteen files and raised one minor verification
question about errors after spawning. Direct inspection confirmed the existing
`execute_owned_os` wraps its entire post-spawn body with `OwnedTeardownUnconfirmed`;
the worker preserves that typed error. Existing regression
`owned_errors_distinguish_pre_spawn_from_post_spawn_reader_failure` exercises the
boundary. No code correction was required for that finding.

Final validation passed 754 CLI/core tests (24 prerequisite-dependent tests
ignored), 100 headless desktop tests, formatting, strict CLI/core and desktop
Clippy, fresh core/CLI package verification and portable version/inspection smoke.
The three previously recorded legacy dependency notice gaps remain. Final diff
inspection found no unresolved defect. Reviewed-head/main CI follows delivery;
no native or real-window acceptance is claimed.

PR #137 merged at `b683ffb`. Reviewed head `d79b742` (CI `40548779626`)
and merged main (CI `40550000483`) passed all seven executed jobs on first
attempts. Reviewed and merged trees match; both merged branches were removed.

### PR 3: native Soft controls

The app-level Saved Setups browser now keeps its selection separate from one
captured editing draft. The review-first editor shares package, capability,
companion and required-file rules, preserves provenance/metadata/exclusions, and
renders the complete wrapped diff through the existing viewport renderer.
Rename, Duplicate and Delete use the checked worker; default-Keep dialogs protect
dirty edits. Explicit reuse protects an occupied New Project draft and starts at
Project with the captured saved bytes. Browsing and mutations leave Watch alone.

Activity retains setup ownership across navigation. Mutations have no Cancel
control. Close after operation waits for confirmed completion and then protects
other drafts; uncertain teardown blocks closing and further mutations. Reload
requirements are tracked per document and clear only for an explicit reload or
confirmed result for that document, never because another setup completes.

Initial test compilation exposed a glob-import collision with GPUI's test macro,
incorrect theme imports and test-helper visibility; explicit imports and scoped
visibility corrected them. The first behavioral run passed 12/15: three failures
were missing observation identifiers. After exposing those, 14/15 passed; the
remaining assertion needed to scroll the narrow details pane to its action.
All fifteen focused checks then passed. Additional ownership/recovery coverage
brought the full suite to 118 passing tests. Strict Clippy found three collapsible
conditions, a large task variant and ambiguous assignment spacing; these were
corrected without lint suppression. Final validation and review follow below.

An approval-service usage limit prevented an earlier patch/test call from running.
After the owner's Continue, the normal approved retry ran. No approval bypass or
native preview was used. Native memory stability and real-window acceptance remain
unresolved/unverified; headless evidence does not replace those gates.

The first CodeRabbit review covered all seventeen changed files and found one
minor recovery defect: failed Edit initialization could prepare the prior clean
draft and erase the error. The controller now prepares only after successful
initialization, with a regression test. Direct review also corrected reopening a
hidden editor from a close prompt and resetting diff rows when preparation
finishes while browsing. Dedicated controls tests cover both. One new test first
omitted the shared diff helper's filename argument; that compilation-only error
was corrected before the final suite. The final CodeRabbit rerun is recorded
separately, rather than presenting the first review as clean.

Final local validation passed 122 headless desktop tests, 754 CLI/core tests
(24 prerequisite-dependent cases remain ignored), formatting, strict Clippy for
both toolchains, fresh core/CLI package verification and portable version and
inspection smoke. Portable smoke was repeated after the final runtime fixes.
The final CodeRabbit rerun reviewed all seventeen files with zero findings;
the final diff was also inspected directly. Three pre-existing legacy dependency
notice gaps remain documented. Actual reviewed-head and merged-main CI and branch
cleanup are recorded in the delivery PR after they finish. No real-window,
memory-stability or macOS/Linux desktop acceptance is claimed.

PR #138 merged at `f70da78`. Reviewed head `647efe23438981c2c2a1c0bad04593a26323306b`
(CI `40646615845`) and merged main (CI `40647112205`) passed all seven executed
jobs on their first attempts; the badge job was intentionally skipped. Reviewed
and merged trees match. Both merged branches were removed. This completes the
three-PR implementation delivery, without changing the paused acceptance gates.
