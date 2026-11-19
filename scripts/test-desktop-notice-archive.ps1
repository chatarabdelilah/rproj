#Requires -Version 7.0
param([Parameter(Mandatory)][string]$FixtureRoot)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Join-Path $FixtureRoot 'archive fixtures'
$bundle = Join-Path $root 'bundle'
$report = Join-Path $bundle 'THIRD-PARTY.md'
$cover = Join-Path $bundle 'licenses/sample-1/LICENSE'
$rows = @(foreach ($pair in @('mac 0.1.1', 'windows-permissions 0.2.4', 'sample 1')) {
    $name, $version = $pair.Split(' ')
    $folder = Join-Path $bundle "licenses/$name-$version"
    New-Item -ItemType Directory -Path $folder -Force | Out-Null
    foreach ($file in 'Cargo.toml','README.upstream') { [IO.File]::WriteAllText((Join-Path $folder $file), 'fixture fallback') }
    @{ name=$name; version=$version; license='MIT'; notices=@(); caveat='Reviewed omission' }
})
[IO.File]::WriteAllText($cover, 'fixture notice')
$rows[2].caveat = $null
$rows[2].notices = @(@{path='licenses/sample-1/LICENSE'; sha256=(Get-FileHash $cover).Hash})
$reportText = "## Pending notice completeness`n- mac-0.1.1 (MIT)`n- windows-permissions-0.2.4 (MIT)`n"
function Check-Archive([string]$Case, [string]$Expected) {
    [IO.File]::WriteAllText((Join-Path $bundle 'dependencies.json'), (ConvertTo-Json -InputObject $rows -Depth 6))
    $archive = Join-Path $root "$Case.zip"
    [IO.Compression.ZipFile]::CreateFromDirectory($bundle, $archive, [IO.Compression.CompressionLevel]::Fastest, $true)
    try { & "$PSScriptRoot/verify-desktop-notices.ps1" -Archive $archive -Bundle 'bundle' }
    catch {
        if (!$Expected -or !$_.Exception.Message.StartsWith($Expected)) { throw }
        return
    }
    if ($Expected) { throw "Fixture '$Case' unexpectedly passed." }
}
[IO.File]::WriteAllText($report, $reportText)
Check-Archive valid ''
$rows[0].version = '0.1.2'
Check-Archive changed-version 'Unreviewed notice omission'
$rows[0].version = '0.1.1'
$rows[0].notices = @(@{path='licenses/mac-0.1.1/Cargo.toml'; sha256=(Get-FileHash (Join-Path $bundle 'licenses/mac-0.1.1/Cargo.toml')).Hash})
$rows[0].caveat = $null
Check-Archive resolved 'Retire reviewed exception'
$rows = @($rows[1],$rows[2])
Check-Archive removed 'Retire reviewed exception'
$rows = @(@{name='mac';version='0.1.1';license='MIT';notices=@();caveat='Reviewed omission'}) + $rows
[IO.File]::WriteAllText($report, $reportText.Replace('0.2.4','0.2.5'))
Check-Archive disagreement 'THIRD-PARTY.md omissions disagree'
[IO.File]::WriteAllText($report, $reportText)
[IO.File]::WriteAllText($cover, 'changed bytes')
Check-Archive corrupt 'Packaged notice checksum mismatch'
[IO.File]::Delete($cover)
Check-Archive missing-notice 'Missing packaged file: licenses/sample-1/LICENSE'
[IO.File]::Delete((Join-Path $bundle 'licenses/mac-0.1.1/Cargo.toml'))
Check-Archive missing-manifest 'Missing packaged file: licenses/mac-0.1.1/Cargo.toml'
Write-Host 'Portable notice archive fixtures passed.'
