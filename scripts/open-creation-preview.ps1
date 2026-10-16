#Requires -Version 5.1
[CmdletBinding()]
param(
    [ValidateSet('Success', 'Failure', 'Cancellation')][string]$Scenario = 'Success',
    [switch]$PrepareOnly
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repository = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $repository
try {
    # The test executable injects explicit storage. Environment profile overrides
    # do not reliably redirect directories::ProjectDirs on Windows.
    $messages = & cargo test --locked --bin rproj --no-run --message-format=json
    if ($LASTEXITCODE -ne 0) { throw 'Could not build the isolated creation worker.' }
    $artifacts = @($messages | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object {
        $_.reason -eq 'compiler-artifact' -and $_.target.name -eq 'rproj' -and $_.profile.test -and $_.executable
    })
    if ($artifacts.Count -ne 1) { throw 'Expected one CLI test executable.' }
    & cargo '+1.99.0' build --locked -p rproj-desktop
    if ($LASTEXITCODE -ne 0) { throw 'Could not build the native preview.' }
    $preview = Join-Path $repository ('target/creation-preview/' + [Guid]::NewGuid().ToString('N'))
    foreach ($directory in @($preview, (Join-Path $preview 'tools'), (Join-Path $preview 'watch'), (Join-Path $preview 'setups'), (Join-Path $preview 'Browse example'))) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
    }
    $utf8 = New-Object Text.UTF8Encoding($false)
    $projectRoot = Join-Path $preview 'Disposable projects with spaces'
    $tomlRoot = $projectRoot.Replace('\', '/')
    [IO.File]::WriteAllText((Join-Path $preview 'machine.toml'), "roblox_projects_root = '$tomlRoot'`nlast_checked = 'isolated native fixture'`n", $utf8)
    [IO.File]::WriteAllText((Join-Path $preview 'setups/Minimal fixture.toml'), "mode = 'expert'`npackage_workflow = 'none'`n", $utf8)
    [IO.File]::WriteAllText((Join-Path $preview 'setups/Unsupported fixture.toml'), "mode = 'expert'`npackages = ['future-package']`n", $utf8)
    [IO.File]::WriteAllText((Join-Path $preview 'Browse example/default.project.json'), '{"name":"Browse example","tree":{"$className":"DataModel"}}', $utf8)
    $mode = switch ($Scenario) { 'Failure' { 'fail' } 'Cancellation' { 'cancel-after' } default { 'success' } }
    [IO.File]::WriteAllText((Join-Path $preview 'mode'), $mode, $utf8)
    foreach ($name in @('start.lock', 'owner.lock', 'state.lock', 'state-writer.lock')) {
        [IO.File]::WriteAllText((Join-Path $preview ('watch/' + $name)), '', $utf8)
    }
    $previousBinary = [Environment]::GetEnvironmentVariable('RPROJ_TEST_WORKER_BINARY', 'Process')
    $previousVersion = [Environment]::GetEnvironmentVariable('RPROJ_TEST_WORKER_VERSION', 'Process')
    try {
        $env:RPROJ_TEST_WORKER_BINARY = $artifacts[0].executable
        $version = (& (Join-Path $repository 'target/debug/rproj-desktop.exe') --version) -replace '^rproj-desktop ', ''
        if ($LASTEXITCODE -ne 0) { throw 'Could not identify desktop version.' }
        $env:RPROJ_TEST_WORKER_VERSION = $version.Trim()
        & rustc --edition=2024 (Join-Path $repository 'tests/fixtures/creation_bridge.rs') -o (Join-Path $preview 'rproj.exe')
        if ($LASTEXITCODE -ne 0) { throw 'Could not build the fixture bridge.' }
    } finally {
        [Environment]::SetEnvironmentVariable('RPROJ_TEST_WORKER_BINARY', $previousBinary, 'Process')
        [Environment]::SetEnvironmentVariable('RPROJ_TEST_WORKER_VERSION', $previousVersion, 'Process')
    }
    Copy-Item -LiteralPath (Join-Path $preview 'rproj.exe') -Destination (Join-Path $preview 'tools/git.exe')
    Copy-Item -LiteralPath (Join-Path $repository 'target/debug/rproj-desktop.exe') -Destination (Join-Path $preview 'rproj-desktop.exe')
    $preferences = Join-Path $preview 'desktop-state.json'
    [IO.File]::WriteAllText($preferences, (@{ schema_version = 1; appearance = 'system'; desktop_executable = $null; recent_projects = @((Join-Path $preview 'Browse example')) } | ConvertTo-Json), $utf8)
    $result = [ordered]@{ fixture = $true; scenario = $Scenario; storage = $preview; projects_root = $projectRoot; preferences = $preferences; source_commit = (& git rev-parse HEAD); source_dirty = [bool](& git status --porcelain) }
    if (-not $PrepareOnly) {
        $start = New-Object Diagnostics.ProcessStartInfo
        $start.FileName = Join-Path $preview 'rproj-desktop.exe'
        $start.UseShellExecute = $true
        $start.WindowStyle = [Diagnostics.ProcessWindowStyle]::Normal
        $start.Arguments = '--preferences-path "{0}"' -f $preferences
        $process = [Diagnostics.Process]::Start($start)
        try {
            if ($process.WaitForExit(3000)) { throw "Native preview exited during startup: $($process.ExitCode)" }
            $result.process_id = $process.Id
        } finally { $process.Dispose() }
    }
    $json = $result | ConvertTo-Json
    [IO.File]::WriteAllText((Join-Path $preview 'preview.json'), $json, $utf8)
    [IO.File]::WriteAllText((Join-Path $repository 'target/latest-creation-preview.json'), $json, $utf8)
    $json | Write-Output
} finally { Pop-Location }
