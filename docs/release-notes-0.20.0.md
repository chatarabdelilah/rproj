# rproj 0.20.0 — Persistent background Watch on Windows

Release candidate. Public alpha; owner publication is pending.

## Changes

- `rproj watch start` starts one background sourcemap Watch per Windows user.
  It continues after rproj closes. Use `rproj watch status`, `rproj watch logs`
  and `rproj watch stop` from any directory.
- Home shows the watched project and state. Project actions offer background
  start, status, logs, stop and a separate foreground Watch action.
- Startup restores missing dependencies and generates the initial sourcemap
  before reporting Watching. Stop during Preparing finishes the active recovery
  command and skips later steps; during Watching it terminates the owned tree.
- Logs retain two files of at most 1 MiB each. Disk draining yields to control
  requests, and snapshot readers coordinate with atomic state replacement.
- Watch acceptance covers competing starts, recovery cancellation, failures,
  supervisor crashes, process-tree cleanup and heavy tool output. PTY resize
  checks cover Projects, Catalog, Saved Setups and New Project revisions.

## Existing projects

No project migration is required. Bare `rproj watch` remains foreground Watch.
Tests can run alongside Watching; Preparing and Stopping block them. Stop Watch
before changing the watched project's composition, tools or configuration, and
before Machine Setup. Existing external tools and project pins remain responsible
for development work. UI Labs remains available at 2.4.2.

## Limits and recovery

Background Watch is Windows-only and starts manually. It does not provide Rojo
Serve, a Windows service, sign-in startup or automatic restart after failure,
logout or reboot. A different project cannot replace an owned session. Lost
control refuses conflicting work; rproj never kills a process using a cached PID.
Lock contention reports that state is busy and asks you to retry.

Windows launchers that prohibit process detachment may refuse background start;
use a normal Windows terminal. Watch diagnostics include tool output and local
paths, so review them before sharing.

## Verification

The candidate passed 472 ordinary locked Windows tests, formatting and clippy.
Its 24 ignored tests are not ordinary passes; explicit serial runs passed all
14 live workflows, four real-Rojo template checks, both installed background
Watch acceptances and the installed Jest missing-credential refusal check.
The [release audit](release-audit.md) records commands, tool versions and limits.
The release PR records clean locked packaging, CodeRabbit review and
exact-head/main Windows stable, Rust 1.89 and locked-package CI.

Authenticated Open Cloud execution, fresh Windows provisioning, fresh Linux
generated-project execution, UI Labs Studio stories and Scribe Studio playtesting
remain unverified. Windows symlink privileges are unavailable on this audit
machine; Unix permission-mode checks require Unix. This candidate retains the
0.19.1 Template Explorer protections and their optimistic conflict-check limit.
