# Native preview memory-exhaustion investigation

October 17, 2026. **Open blocker for further native preview acceptance.**
The owner reports that opening native rproj previews, leaving them open and
closing them is followed by out-of-memory warnings in other applications or a
display that fails to wake. The owner confirmed this concerns native desktop
windows, not the conversation wireframes.

## Scope and evidence boundary

The report needs investigation before further native acceptance. Earlier
functional checks do not establish memory stability. The allocating component,
the role of other software and the display symptom's cause are unresolved;
no rproj memory-leak fix or successful reproduction is claimed.

Detailed host diagnostics are retained locally and excluded from this record.
Do not add process telemetry, installed-software details or raw machine logs
to the repository. Static inspection found that the desktop's last-window-closed
handler calls `quit`; this is not a memory-lifetime test or proof against leaks
while a window is open or resources retained outside the desktop process.

## Immediate boundary and next action

Further native launches and repeated host reproduction cycles are paused. No
preview was opened to reproduce this report, and the investigation made no
process, capture, audio or Windows settings changes. No speculative renderer or
screen-capture exclusion was added to rproj.

The next step is to agree with the owner on isolating potential capture/overlay
interactions without opening another native window. A later reproduction needs
explicit owner agreement on a bounded run or isolated environment, a healthy
baseline and synchronized measurements. A settings change alone is not a
verified fix. Resume native acceptance only after this blocker is resolved.
