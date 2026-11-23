# rproj 0.23.0 — Project update discovery and Windows setup fixes

Public alpha for the Cargo CLI and its shared core library.

## CLI changes

- Run `rproj updates` to check an existing project's Wally packages and
  Rokit tool pins for newer public releases. The command reports findings
  without applying changes.
- Machine Setup bootstraps Rokit when a recognized CLI tool is selected.
  Git-only setup runs Git followed by the projects folder. A Rojo selection
  retains Rokit before Rojo.
- Git installation uses WinGet's community source. Machine Setup reports
  already installed when WinGet finds the installed package and returns
  its no-applicable-update code.
- Background Watch recovers from transient Stop status and releases its
  startup pipes. Cargo test harness discovery also handles package-specific
  target selection.

## Verification and desktop distribution

On the Windows acceptance guest, Git-only Apply completed in two seconds
with Git already installed. After Git was removed, the same build installed
it in 31 seconds through WinGet. A fresh PowerShell reported
`git version 2.56.1.windows.1`. Both runs omitted Rokit. See the
[release audit](../archive/release-audit.md) for the launcher PATH and results.

Released Wally and Rokit tools were exercised in disposable projects for
the reviewed-update work. Live Git-only verification used the Windows
acceptance guest. Studio and authenticated Open Cloud workflows were
outside this candidate's live checks.

The Windows desktop workbench is developed and built separately. Its
portable ZIP is held from this release while native host acceptance is
paused. See the [desktop investigation](../archive/desktop-memory-investigation.md).
