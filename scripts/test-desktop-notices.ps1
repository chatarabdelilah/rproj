#Requires -Version 7.0
param([Parameter(Mandatory)][string]$Writer, [Parameter(Mandatory)][string]$FixtureRoot)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

foreach ($relative in @('scripts', 'crate', 'third-party/licenses/first', 'third-party/licenses/second')) {
    New-Item -ItemType Directory -Path (Join-Path $FixtureRoot $relative) -Force | Out-Null
}
$fixtureWriter = Join-Path $FixtureRoot 'scripts/write-desktop-notices.ps1'
Copy-Item -LiteralPath $Writer -Destination $fixtureWriter
foreach ($entry in @{
    'crate/Cargo.toml' = '[package]'
    'crate/LICENSE' = 'published notice'
    'third-party/licenses/first/LICENSE' = 'first supplement'
    'third-party/licenses/second/LICENSE' = 'second supplement'
    'third-party/licenses/second/NOTICE' = 'separate notice'
}.GetEnumerator()) {
    [IO.File]::WriteAllText((Join-Path $FixtureRoot $entry.Key), $entry.Value)
}
$metadata = [pscustomobject]@{
    workspace_members = @('fixture')
    resolve = [pscustomobject]@{ nodes = @([pscustomobject]@{ id = 'fixture'; deps = @() }) }
    packages = @([pscustomobject]@{
        id = 'fixture'; name = 'fixture'; version = '1.0.0'; license = 'MIT'; license_file = $null
        authors = @('Fixture'); repository = $null; source = $null; readme = $null
        manifest_path = Join-Path $FixtureRoot 'crate/Cargo.toml'
    })
}
function Set-Supplements([string[]]$Paths) {
    $files = @($Paths | ForEach-Object {
        [ordered]@{
            path = $_; url = "https://fixture.invalid/$_"
            sha256 = (Get-FileHash -LiteralPath (Join-Path $FixtureRoot $_) -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    })
    $supplement = [ordered]@{ name = 'fixture'; version = '1.0.0'; commit = 'fixture'; files = $files }
    [IO.File]::WriteAllText((Join-Path $FixtureRoot 'third-party/supplements.json'), (ConvertTo-Json -InputObject @($supplement) -Depth 5))
}
function Assert-Collision([string]$Name, [string]$Expected) {
    $destination = Join-Path $FixtureRoot $Name
    $refused = $false
    try { & $fixtureWriter -Metadata $metadata -PackageIds @('fixture') -Destination $destination }
    catch {
        if ($_.Exception.Message -notlike 'Duplicate license notice destination:*') { throw }
        $refused = $true
    }
    if (-not $refused) { throw "${Name}: duplicate destination was accepted" }
    $target = Join-Path $destination 'licenses/fixture-1.0.0/upstream-LICENSE'
    if ([IO.File]::ReadAllText($target) -cne $Expected) { throw "${Name}: prior notice was overwritten" }
    if (Test-Path -LiteralPath (Join-Path $destination 'dependencies.json')) { throw "${Name}: inventory was written despite collision" }
}

Set-Supplements @('third-party/licenses/first/LICENSE', 'third-party/licenses/second/NOTICE')
$valid = Join-Path $FixtureRoot 'valid output'
& $fixtureWriter -Metadata $metadata -PackageIds @('fixture') -Destination $valid
$inventory = @(Get-Content -LiteralPath (Join-Path $valid 'dependencies.json') -Raw | ConvertFrom-Json)
if ($inventory.Count -ne 1 -or $inventory[0].notices.Count -ne 3) { throw 'Valid inventory lost a notice.' }
foreach ($record in $inventory[0].notices) {
    $copied = Join-Path $valid $record.path
    if ((Get-FileHash -LiteralPath $copied -Algorithm SHA256).Hash.ToLowerInvariant() -cne $record.sha256) { throw 'Valid inventory checksum does not match copied notice.' }
}
if (-not (Test-Path -LiteralPath (Join-Path $valid 'licenses/fixture-1.0.0/upstream-LICENSE'))) { throw 'Valid supplement path changed.' }

Set-Supplements @('third-party/licenses/first/LICENSE', 'third-party/licenses/second/LICENSE')
Assert-Collision 'same basename' 'first supplement'
Set-Supplements @('third-party/licenses/first/LICENSE', 'third-party/licenses/second/license')
Assert-Collision 'case-only basename' 'first supplement'

$metadata.packages[0].license_file = 'upstream-LICENSE'
[IO.File]::WriteAllText((Join-Path $FixtureRoot 'crate/upstream-LICENSE'), 'declared published notice')
Set-Supplements @('third-party/licenses/first/LICENSE')
Assert-Collision 'published destination' 'declared published notice'
Write-Host 'Notice fixture passed: valid inventory and three collision refusals.'
