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

### PR 2: private worker (in progress)

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
