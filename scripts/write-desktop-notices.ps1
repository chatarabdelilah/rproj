#Requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)]$Metadata,
    [Parameter(Mandatory)][string[]]$PackageIds,
    [Parameter(Mandatory)][string]$Destination
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repository = Split-Path -Parent $PSScriptRoot
$supplementPath = Join-Path $repository 'third-party/supplements.json'
$supplements = if (Test-Path -LiteralPath $supplementPath) { @(Get-Content -LiteralPath $supplementPath -Raw | ConvertFrom-Json) } else { @() }
$nodes = @{}
foreach ($node in $Metadata.resolve.nodes) { $nodes[$node.id] = $node }
$selected = [Collections.Generic.HashSet[string]]::new()
$pending = [Collections.Generic.Queue[string]]::new()
foreach ($id in $PackageIds) { $pending.Enqueue($id) }
while ($pending.Count -gt 0) {
    $id = $pending.Dequeue()
    if (-not $selected.Add($id)) { continue }
    if (-not $nodes.ContainsKey($id)) { throw "Dependency graph is missing $id" }
    foreach ($dependency in $nodes[$id].deps) {
        if (@($dependency.dep_kinds | Where-Object { $_.kind -ne 'dev' }).Count -gt 0) { $pending.Enqueue($dependency.pkg) }
    }
}
$licenses = Join-Path $Destination 'licenses'
New-Item -ItemType Directory -Path $licenses -Force | Out-Null
$inventory = [Collections.Generic.List[object]]::new()
$omissions = [Collections.Generic.List[string]]::new()
foreach ($package in $Metadata.packages | Where-Object { $selected.Contains($_.id) } | Sort-Object name, version) {
    $crateRoot = Split-Path -Parent $package.manifest_path
    $key = $package.name + '-' + $package.version
    if ($key -match '[/\\:]') { throw "Unsafe dependency identity: $key" }
    $folder = Join-Path $licenses $key
    New-Item -ItemType Directory -Path $folder -Force | Out-Null
    $records = [Collections.Generic.List[object]]::new()
    $provenance = [Collections.Generic.List[object]]::new()
    $noticeSearch = @{ LiteralPath = $crateRoot; File = $true }
    if ($Metadata.workspace_members -notcontains $package.id) { $noticeSearch.Recurse = $true }
    $texts = @(Get-ChildItem @noticeSearch | Where-Object {
        $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT)([._-]|$)' -and $_.Extension -notin @('.svg', '.png', '.jpg', '.gif', '.rs', '.toml')
    })
    if ($package.license_file) {
        $declared = [IO.Path]::GetFullPath((Join-Path $crateRoot $package.license_file))
        if (-not $declared.StartsWith([IO.Path]::GetFullPath($crateRoot) + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Declared license escapes the published crate: $key"
        }
        if (-not (Test-Path -LiteralPath $declared -PathType Leaf)) { throw "Missing declared license file: $key" }
        $texts += Get-Item -LiteralPath $declared
    }
    foreach ($text in $texts | Sort-Object FullName -Unique) {
        $relative = [IO.Path]::GetRelativePath($crateRoot, $text.FullName)
        $target = Join-Path $folder $relative
        New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force | Out-Null
        Copy-Item -LiteralPath $text.FullName -Destination $target
        $records.Add([ordered]@{ path = "licenses/$key/" + $relative.Replace('\', '/'); origin = 'published crate'; sha256 = (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash.ToLowerInvariant() })
    }
    foreach ($supplement in $supplements | Where-Object { $_.name -eq $package.name -and $_.version -eq $package.version }) {
        $noticeCommit = if ($supplement.PSObject.Properties['notice_commit']) { $supplement.notice_commit } else { $supplement.commit }
        $provenanceNote = if ($supplement.PSObject.Properties['provenance_note']) { $supplement.provenance_note } else { $null }
        $provenance.Add([ordered]@{ published_commit = $supplement.commit; notice_commit = $noticeCommit; note = $provenanceNote })
        foreach ($file in $supplement.files) {
            $source = [IO.Path]::GetFullPath((Join-Path $repository $file.path))
            $supplementRoot = [IO.Path]::GetFullPath((Join-Path $repository 'third-party/licenses')) + [IO.Path]::DirectorySeparatorChar
            if (-not $source.StartsWith($supplementRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'License supplement escapes third-party/licenses.' }
            if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant() -ne $file.sha256) { throw "License supplement checksum changed: $($file.path)" }
            $target = Join-Path $folder ('upstream-' + [IO.Path]::GetFileName($source))
            Copy-Item -LiteralPath $source -Destination $target
            $records.Add([ordered]@{ path = "licenses/$key/" + [IO.Path]::GetFileName($target); origin = $file.url; sha256 = $file.sha256 })
        }
    }
    $notice = $null
    if ($records.Count -eq 0) {
        $notice = 'Published crate contains a license declaration but no standalone notice text. Available manifest/README attribution is preserved; notice completeness remains unverified.'
        $omissions.Add("$key ($($package.license))")
        Copy-Item -LiteralPath $package.manifest_path -Destination (Join-Path $folder 'Cargo.toml')
        if ($package.readme) {
            $readme = [IO.Path]::GetFullPath((Join-Path $crateRoot $package.readme))
            if ($readme.StartsWith([IO.Path]::GetFullPath($crateRoot) + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -and (Test-Path -LiteralPath $readme -PathType Leaf)) {
                Copy-Item -LiteralPath $readme -Destination (Join-Path $folder 'README.upstream')
            }
        }
    }
    if (-not $package.license -and -not $package.license_file) { throw "Dependency has no declared license: $key" }
    $inventory.Add([ordered]@{ name = $package.name; version = $package.version; license = $package.license; authors = @($package.authors); repository = $package.repository; source = $package.source; notices = $records.ToArray(); supplement_provenance = $provenance.ToArray(); caveat = $notice })
}
$report = @(
    '# Third-party dependencies', '',
    'This conservative inventory follows the locked Windows dependency graph reachable from the bundled CLI and desktop. It includes normal and build dependencies, and excludes dev-only edges; Cargo metadata may also include packages enabled by the workspace feature union. All original license/notice files found in those published crates are preserved. Version-specific supplements identify their exact upstream URL and checksum.', '',
    'dependencies.json records package versions, declared license expressions, authors, source repositories, copied files, and provenance. Its supplement_provenance distinguishes published commits from any later upstream notice-only correction. This is an inventory of available evidence, not a claim that an automated license audit establishes every distribution obligation.', ''
)
if ($omissions.Count -gt 0) {
    $report += @('## Pending notice completeness', '', 'These legacy distributions contain no standalone license text. Their available declarations and attribution are included. Legal-notice completeness remains unverified for public distribution:', '')
    $report += $omissions | ForEach-Object { '- ' + $_ }
    Write-Warning ("Notice completeness remains pending for " + $omissions.Count + ' legacy dependency archives; see THIRD-PARTY.md.')
} else {
    $report += 'Every selected package has at least one preserved or supplemented notice. Review any additional third-party obligations before publication.'
}
[IO.File]::WriteAllText((Join-Path $Destination 'THIRD-PARTY.md'), ($report -join "`n") + "`n", [Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText((Join-Path $Destination 'dependencies.json'), (ConvertTo-Json -InputObject $inventory.ToArray() -Depth 8) + "`n", [Text.UTF8Encoding]::new($false))
Write-Host ("Preserved dependency notices for " + $inventory.Count + ' packages.')
