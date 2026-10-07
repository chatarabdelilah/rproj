# rproj 0.20.2 — Watch acknowledgment reliability

Unpublished candidate prepared October 7, 2026. Public alpha.

## Changes

- Windows background Watch now allows two seconds to receive Stop and
  engine-ready acknowledgments, which follow durable session-state writes.
  Status probes, connections and sends retain their 300 ms deadlines.
- A peer that cannot receive its acknowledgment no longer terminates the
  supervisor and its owned engine. Reply-delivery failures are logged; state
  persistence, authentication, owner identity and process cleanup retain their
  existing checks.
- Control frames are serialized before sending, and connection, send and
  acknowledgment failures now identify the failing phase.

No project migration is required. This patch also includes terminal test-harness
repairs: Template Explorer waits for complete JSON text/cursor acknowledgments,
and unit-test UI children run serially to keep libtest's slow-test warning out of
their active screens. These changes retain file-protection assertions.

## Verification and limits

Release-candidate checks, packaging, CodeRabbit review and exact-head/main CI
evidence are recorded in [the release audit](release-audit.md).

Controlled regressions reproduce a delayed mutation acknowledgment exceeding
the old deadline and an aborted reply escaping the supervisor. They do not
measure the historic CI runner's exact disk or scheduler timing.

Fresh-Windows installation, authenticated Open Cloud execution, fresh Linux
generated-project execution, UI Labs Studio stories and Scribe Studio
playtesting remain unverified. Ignored tests are separate acceptance checks,
not ordinary passes. The owner deferred Hyper-V/VM provisioning. This patch
retains [0.20.0's Watch and platform limits](release-notes-0.20.0.md) and does not
claim beta readiness.

The owner alone publishes with `cargo publish --locked`. Registry/archive
identity verification, the annotated tag and GitHub alpha prerelease follow
successful publication.
