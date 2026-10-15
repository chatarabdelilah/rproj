#Requires -Version 5.1
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Bundle,
    [Parameter(Mandatory = $true)][ValidatePattern('^[0-9a-f]{40}$')][string]$ExpectedCommit,
    [switch]$CheckOnly,
    [switch]$PrepareOnly
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$Bundle = (Resolve-Path -LiteralPath $Bundle).Path
$build = Get-Content -LiteralPath (Join-Path $Bundle 'build.json') -Raw | ConvertFrom-Json
if ($build.source_commit -cne $ExpectedCommit -or $build.source_dirty) {
    throw 'The preview requires the specified clean source commit.'
}
foreach ($name in @('rproj.exe', 'rproj-desktop.exe')) {
    $entry = @($build.binaries | Where-Object { $_.name -ceq $name })
    if ($entry.Count -ne 1) { throw "Missing or duplicate build hash for $name" }
    $actual = (Get-FileHash -LiteralPath (Join-Path $Bundle $name) -Algorithm SHA256).Hash
    if ($actual -ine $entry[0].sha256) { throw "Build hash mismatch for $name" }
}
if ($CheckOnly) {
    Write-Output 'Source identity and both executable hashes verified. Window not launched.'
    return
}

$repository = Split-Path -Parent $PSScriptRoot
$preview = Join-Path $repository ('target/soft-preview/' + [Guid]::NewGuid().ToString('N'))
$project = Join-Path $preview 'Soft example with a longer project path'
$other = Join-Path $preview 'Another project'
$invalid = Join-Path $preview 'Configuration error'
foreach ($directory in @($project, $other, $invalid, (Join-Path $project '.lute'))) {
    New-Item -ItemType Directory -Path $directory -Force | Out-Null
}
$utf8 = New-Object Text.UTF8Encoding($false)
[IO.File]::WriteAllText((Join-Path $project 'default.project.json'), '{"name":"Soft example","tree":{"$className":"DataModel"}}', $utf8)
[IO.File]::WriteAllText((Join-Path $other 'default.project.json'), '{"name":"Another project","tree":{"$className":"DataModel"}}', $utf8)
[IO.File]::WriteAllText((Join-Path $invalid 'default.project.json'), '{ broken configuration', $utf8)
[IO.File]::WriteAllText((Join-Path $project 'wally.toml'), @'
[dependencies]
Signal = "sleitnick/signal@2.0.1"
Unfamiliar = "example/unfamiliar@1.0.0"
'@, $utf8)
[IO.File]::WriteAllText((Join-Path $project '.lute/check.luau'), @'
--!strict
local task = require("@lute/task")
task.spawn(function()
    print("PROBE start")
    for tick = 1, 5 do
        task.wait(1)
        print(string.format("PROBE tick %d/5", tick))
    end
    print("PROBE complete")
end)
'@, $utf8)
$preferences = Join-Path $preview 'desktop-state.json'
# The app owns preference creation and its schema. Only the override path is supplied.
$result = [ordered]@{ project = $project; other_project = $other; invalid_project = $invalid; preferences = $preferences; source_commit = $ExpectedCommit }
if (-not $PrepareOnly) {
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = Join-Path $Bundle 'rproj-desktop.exe'
    $start.UseShellExecute = $true
    $start.WindowStyle = [Diagnostics.ProcessWindowStyle]::Normal
    $start.Arguments = '--preferences-path "{0}" "{1}"' -f $preferences, $project
    $process = [Diagnostics.Process]::Start($start)
    try {
        if ($process.WaitForExit(3000)) {
            throw "The preview exited during startup (exit code $($process.ExitCode))."
        }
        $result.process_id = $process.Id
    } finally { $process.Dispose() }
}
$result | ConvertTo-Json | Write-Output
