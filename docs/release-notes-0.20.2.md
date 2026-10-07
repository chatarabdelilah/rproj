# rproj 0.20.2 — Watch acknowledgment reliability

Published October 7, 2026. Public alpha.

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

The candidate passed 481 ordinary locked Windows tests, strict all-target
Clippy, formatting and clean locked packaging. Both installed-tool background
Watch acceptances passed serially using Rojo 7.7.0 and Wally 0.3.2: real
detachment/update/stop and Wally/Jest recovery. Its 24 ignored tests remain a
separate inventory; only those two named checks ran explicitly for this candidate.
Package inspection, CodeRabbit review and exact-head/main CI evidence are
recorded in [the release audit](release-audit.md) and the candidate PR.

Controlled regressions reproduce a delayed mutation acknowledgment exceeding
the old deadline and an aborted reply escaping the supervisor. They do not
measure the historic CI runner's exact disk or scheduler timing.

Fresh-Windows installation, authenticated Open Cloud execution, fresh Linux
generated-project execution, UI Labs Studio stories and Scribe Studio
playtesting remain unverified. Ignored tests are separate acceptance checks,
not ordinary passes. The owner deferred Hyper-V/VM provisioning. This patch
retains [0.20.0's Watch and platform limits](release-notes-0.20.0.md) and does not
claim beta readiness.

The owner-published archive's SHA-256 matches crates.io checksum
`260f61d5baa9b5e1ea422b4f85a4356d790bb2fd36c24fbc91a874f09c526745`.
Its manifest and lockfile are 0.20.2, and its clean Git identity matches reviewed
release commit `eaa0447d73751f147c2138b106f1ceb263617e10`. The annotated `v0.20.2`
tag and [GitHub alpha prerelease](https://github.com/chatarabdelilah/rproj/releases/tag/v0.20.2)
point to that same immutable commit. [Release PR #85](https://github.com/chatarabdelilah/rproj/pull/85)
passed both local CodeRabbit reviews with zero findings and first-attempt final-head
and merged-main Windows stable, Rust 1.89 and package CI.
