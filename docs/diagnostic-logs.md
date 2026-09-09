# Diagnostic logs

rproj automatically writes one local `.txt` diagnostic log per command run. At
completion direct commands print `Diagnostic log: <full path>` on stderr. Home
shows the full path with failures, but prints no closing notice for ordinary
navigation or cancellation. The file lives outside the project, so failed creation and machine
setup can be diagnosed too. Nothing is uploaded automatically.

On Windows the default directory is `%LOCALAPPDATA%\rproj\data\logs` (resolved by
the platform directory API). Use the exact path printed by your run. Log names
include a Unix timestamp, process ID, and random suffix, so simultaneous runs do
not overwrite each other. `--help` and `--version` do not create logs.

## What to send

1. Reproduce the issue and copy the path printed at the end.
2. Open that `.txt` file and review it for private names or paths.
3. Attach it with a short description of what you expected to happen.

For a force-closed process, find the newest file in the log directory. Completed
events are written immediately, but an abrupt exit can lack a final exit code.
Handled panics record their source location, not their potentially private payload.

## Coverage and privacy

The log records rproj version, platform, current directory, timestamp/elapsed
time, command intent, accepted catalog/prompt choices, project-graph reviews,
settings answers, steps, warnings, operation-level errors, subprocess invocation
metadata/exit statuses, and semantic TUI actions/screen transitions. Paste events record
length rather than arbitrary values; individual navigation keys are not logged. Resize, save, reset, and
cancellation events help reconstruct an interactive session.

Hub creation also records committed composition/strategy/package/capability choices,
screen transitions, review confirmation, and cancellation. Search text and uncommitted
name input remain omitted. The confirmed executor retains normal step and subprocess
events after the full-screen terminal has been restored.

This is **not a recording of everything on the screen or a keylogger**:

- Environment values, source files, clipboard contents, raw typed characters,
  template JSON/text values, and raw child-process output are not recorded.
- Test-runner/Lute argument values are omitted because they can contain credentials.
- Parser error source excerpts are omitted. Each error cause keeps its first line.
- Recognizable credential-related fields/token formats are redacted before writing.
  Redaction is conservative, not a guarantee of anonymity: project/setup names,
  paths, and ordinary selected values can remain. Review before sharing.
- An external tool's full failure output may still be needed separately. For Jest,
  a runner report such as `--outputFile results.json` complements the rproj log.

Logs stop accepting events at 2 MiB and show a truncation marker. Individual
events are limited to 4,096 characters. Logging failures warn but do not change
the command's result. Logs are not automatically deleted; remove old logs from
this directory when no longer needed.

## Controls

In PowerShell, change the log directory for this shell and its child processes:

```powershell
$env:RPROJ_LOG_DIR = 'C:\MyDiagnostics\rproj'
```

Disable logging for this shell:

```powershell
$env:RPROJ_NO_LOG = '1'
```

Remove the override to enable the default again:

```powershell
Remove-Item Env:RPROJ_NO_LOG
```

The logger adds no project files and does not change normal stdout or tool exit
codes. These controls also let automated tests use isolated temporary directories.
