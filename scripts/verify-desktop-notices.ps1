#Requires -Version 7.0
param(
    [Parameter(Mandatory)][string]$Archive,
    [Parameter(Mandatory)][string]$Bundle
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Reviewed versions and their published fallback files are documented in third-party/README.md.
$exceptions = @('mac-0.1.1', 'windows-permissions-0.2.4')
$pending = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$packages = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$entries = [Collections.Generic.Dictionary[string, IO.Compression.ZipArchiveEntry]]::new([StringComparer]::Ordinal)
function Require-File([string]$Relative) {
    $entry = $null
    if (-not $entries.TryGetValue("$Bundle/$Relative", [ref]$entry)) { throw "Missing packaged file: $Relative" }
    return $entry
}
function Read-Text([string]$Relative) {
    $reader = [IO.StreamReader]::new((Require-File $Relative).Open())
    try { return $reader.ReadToEnd() } finally { $reader.Dispose() }
}
$zip = [IO.Compression.ZipFile]::OpenRead((Resolve-Path -LiteralPath $Archive).Path)
try {
    foreach ($entry in $zip.Entries) {
        if ($entry.Name -and -not $entries.TryAdd($entry.FullName, $entry)) { throw "Duplicate ZIP entry: $($entry.FullName)" }
    }
    $inventory = Read-Text 'dependencies.json' | ConvertFrom-Json -NoEnumerate
    if ($inventory -isnot [array] -or !$inventory.Count) { throw 'Expected a nonempty package inventory.' }
    $expectedLines = [Collections.Generic.List[string]]::new()
    foreach ($package in $inventory) {
        $key = "$($package.name)-$($package.version)"
        if ($key -match '[/\\:]' -or !$packages.Add($key)) { throw "Invalid or repeated package: $key" }
        $prefix = "licenses/$key/"
        if ($package.notices -isnot [array]) { throw "Invalid notice list: $key" }
        if (!$package.notices.Count) {
            if ($key -cnotin $exceptions) { throw "Unreviewed notice omission: $key" }
            if ([string]::IsNullOrWhiteSpace($package.caveat) -or [string]::IsNullOrWhiteSpace($package.license)) { throw "Missing omission explanation or license: $key" }
            foreach ($file in 'Cargo.toml', 'README.upstream') {
                if ((Require-File ($prefix + $file)).Length -eq 0) { throw "Empty fallback: $prefix$file" }
            }
            $null = $pending.Add($key)
            $expectedLines.Add("- $key ($($package.license))")
            continue
        }
        if ($package.caveat) { throw "Unexpected omission caveat for covered package: $key" }
        foreach ($notice in $package.notices) {
            if (!$notice.path.StartsWith($prefix, [StringComparison]::Ordinal) -or $notice.path -match '(^|/)(\.{1,2}|)(/|$)|[\\:]') { throw "Invalid notice path for ${key}: $($notice.path)" }
            $stream = (Require-File $notice.path).Open()
            try { $actual = (Get-FileHash -InputStream $stream -Algorithm SHA256).Hash }
            finally { $stream.Dispose() }
            if ($actual -ne $notice.sha256) { throw "Packaged notice checksum mismatch: $($notice.path)" }
        }
    }
    foreach ($key in $exceptions) {
        if (!$pending.Contains($key)) { throw "Retire reviewed exception: $key" }
    }
    $lines = (Read-Text 'THIRD-PARTY.md') -split '\r?\n'
    $bullets = @($lines -clike '- *')
    if (($pending.Count -gt 0) -ne ($lines -ccontains '## Pending notice completeness') -or
        $bullets.Count -ne $expectedLines.Count -or
        @(Compare-Object -CaseSensitive -ReferenceObject $expectedLines.ToArray() -DifferenceObject $bullets).Count) {
        throw 'THIRD-PARTY.md omissions disagree with dependencies.json.'
    }
    Write-Host "Verified notices for $($packages.Count) packaged dependencies with $($pending.Count) reviewed omissions."
} finally { $zip.Dispose() }
