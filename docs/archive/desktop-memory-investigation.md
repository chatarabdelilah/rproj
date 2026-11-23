# Native preview memory-exhaustion investigation

**November 23, 2026 — closed by owner decision.** Proceed with Windows desktop
development and ordinary real-window acceptance. Reopen this investigation only
if the memory or display symptoms recur. Their cause remains undetermined.
The [Windows guest observation](release-audit.md#november-21--windows-guest-preview-closed-normally)
records the November 21 result. The earlier decisions below are historical.

## October 17 incident report

The owner reports that opening native rproj previews, leaving them open and
closing them is followed by out-of-memory warnings in other applications or a
display that fails to wake. The owner confirmed this concerns native desktop
windows, not the conversation wireframes.

## Scope and evidence boundary

The report prompted a pause in native acceptance at the time. Earlier
functional checks do not establish memory stability. The allocating component,
the role of other software and the display symptom's cause are unresolved;
no rproj memory-leak fix or successful reproduction is claimed.

Detailed host diagnostics are retained locally and excluded from this record.
Do not add process telemetry, installed-software details or raw machine logs
to the repository. Static inspection found that the desktop's last-window-closed
handler calls `quit`; this is not a memory-lifetime test or proof against leaks
while a window is open or resources retained outside the desktop process.

## November 20 evidence review

October 17 dates the [incident report](https://github.com/chatarabdelilah/rproj/pull/118).
The available record gives the following sequence from the owner's recollection.
Event timestamps, durations and synchronized memory readings are unknown.

| Event | What the record establishes |
| --- | --- |
| Open | The owner opened a native desktop preview. |
| Idle | A preview was left open. Memory use during this interval is unknown. |
| Close | The owner recalled closing previews. Process exit and memory recovery were unobserved in the available record. |
| Memory warning | Other applications reported memory exhaustion after preview use. The interval from close is unknown. |
| Display wake failure | The owner also reported a display failing to wake after preview use. Whether both symptoms occurred in the same session or shared a cause is unknown. |

At inspected revision `37352a9`, the workbench close handler can keep the
window open while an owned operation finishes or teardown is uncertain. Detached
Watch can continue independently. Once the last window closes, the application
requests GPUI shutdown. The incident record does not identify an active operation,
a surviving desktop process or a Watch session. [Close handling](../../crates/rproj-desktop/src/workbench/creation/controller.rs)
and [application shutdown](../../crates/rproj-desktop/src/main.rs)

**November 20 decision (superseded November 23).** The record contains no measurement
that attributes growth to rproj or to a capture/overlay process. The most useful
comparison is whether system committed memory stays elevated after confirmed
desktop process exit, and which process's private bytes grew over that same
interval. Growth that persists in a capture process would justify investigating
it. Growth confined to the desktop while open would direct attention there.
System growth without a corresponding process trend leaves attribution open.
The display symptom needs its own observation.

## Historical November 20 observation plan

A single observation is justified to obtain that comparison if the owner
explicitly approves a suitable isolated Windows environment with the relevant
graphics and capture configuration. Until then, leave native previews paused.

1. With the preview closed, observe a one-minute baseline. Confirm stable
   memory with adequate commit headroom and a responsive display. Defer the run
   if either condition is uncertain.
2. On one clock, sample system committed bytes and commit limit, desktop process
   existence and private bytes, and private bytes for an already-running capture
   or overlay process if present. Record whether detached Watch is already active.
   Keep process identities, measurements and raw logs local.
3. Open one idle preview for at most one minute, then request normal close once.
   Mark window closure and process exit separately. Observe the same counters for
   one further minute. End immediately on a memory warning, rapidly shrinking
   commit headroom or display instability. Keep capture settings fixed throughout.
4. Record only the qualitative trend and whether exit was confirmed. If the
   symptom is absent or the measurements are incomplete, record the result as
   inconclusive and keep the gate open. A second launch requires a new decision.
