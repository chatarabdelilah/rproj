#Requires -Version 7.0
[CmdletBinding()]
param(
    [string]$Toolchain = '1.99.0',
    [ValidateSet('x86_64-pc-windows-msvc')]
    [string]$Target = 'x86_64-pc-windows-msvc'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
function Invoke-BundledCommand([string]$Executable, [string[]]$Arguments) {
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $Executable
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    foreach ($argument in $Arguments) { $info.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $info
    try {
        if (-not $process.Start()) { throw "Could not start $Executable" }
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit(30000)) {
            $process.Kill($true)
            throw "Packaged command exceeded its 30-second smoke-check deadline: $Executable"
        }
        [pscustomobject]@{ exit_code = $process.ExitCode; stdout = $stdout.GetAwaiter().GetResult(); stderr = $stderr.GetAwaiter().GetResult() }
    } finally { $process.Dispose() }
}

if (-not [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)) {
    throw 'The first portable desktop archive is built and verified on Windows.'
}
$repository = Split-Path -Parent $PSScriptRoot
$previousEncodedFlags = $env:CARGO_ENCODED_RUSTFLAGS
$buildFlags = if ($previousEncodedFlags) {
    $previousEncodedFlags
} elseif ($env:RUSTFLAGS) {
    ([regex]::Split($env:RUSTFLAGS.Trim(), '\s+')) -join [char]31
} else { '' }
$env:CARGO_ENCODED_RUSTFLAGS = ($buildFlags.TrimEnd([char]31) + [char]31 + '-C' + [char]31 + 'target-feature=+crt-static').TrimStart([char]31)
Push-Location -LiteralPath $repository
try {
    & cargo "+$Toolchain" build --locked --release --target $Target -p rproj -p rproj-desktop
    if ($LASTEXITCODE -ne 0) { throw 'Desktop/CLI release build failed.' }
    $metadataText = & cargo "+$Toolchain" metadata --locked --format-version 1 --filter-platform $Target
    if ($LASTEXITCODE -ne 0) { throw 'Cargo dependency inventory failed.' }
    $metadata = ($metadataText -join "`n") | ConvertFrom-Json
    $cli = @($metadata.packages | Where-Object { $_.name -eq 'rproj' -and $metadata.workspace_members -contains $_.id })
    $desktop = @($metadata.packages | Where-Object { $_.name -eq 'rproj-desktop' -and $metadata.workspace_members -contains $_.id })
    if ($cli.Count -ne 1 -or $desktop.Count -ne 1 -or $cli[0].version -ne $desktop[0].version) {
        throw 'The portable archive requires matching CLI and desktop workspace versions.'
    }
    $version = $cli[0].version
    $commit = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Cannot identify the source commit.' }
    $dirty = @(& git status --porcelain).Count -gt 0
    if ($LASTEXITCODE -ne 0) { throw 'Cannot identify working-tree changes.' }
    $stamp = [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss') + '-' + [Guid]::NewGuid().ToString('N').Substring(0, 6)
    $label = "rproj-$version-dev-windows-x86_64"
    $distribution = Join-Path $metadata.target_directory 'desktop-dist'
    $staging = Join-Path $distribution ("staging-" + $stamp)
    $bundle = Join-Path $staging $label
    New-Item -ItemType Directory -Path $bundle -Force | Out-Null
    $release = Join-Path $metadata.target_directory "$Target/release"
    foreach ($binary in @('rproj.exe', 'rproj-desktop.exe')) {
        Copy-Item -LiteralPath (Join-Path $release $binary) -Destination (Join-Path $bundle $binary)
    }
    $runtimeImports = [ordered]@{ verification = 'dumpbin unavailable'; binaries = @() }
    $dumpbin = Get-Command dumpbin.exe -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Source
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (-not $dumpbin -and (Test-Path -LiteralPath $vswhere)) {
        $visualStudio = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($visualStudio) {
            $dumpbin = Get-ChildItem -LiteralPath (Join-Path $visualStudio 'VC/Tools/MSVC') -Directory | Sort-Object Name -Descending | ForEach-Object {
                $candidate = Join-Path $_.FullName 'bin/Hostx64/x64/dumpbin.exe'
                if (Test-Path -LiteralPath $candidate) { $candidate }
            } | Select-Object -First 1
        }
    }
    if ($dumpbin) {
        $runtimeImports.verification = 'dumpbin /DEPENDENTS'
        foreach ($binary in @('rproj.exe', 'rproj-desktop.exe')) {
            $imports = & $dumpbin /DEPENDENTS (Join-Path $bundle $binary)
            if ($LASTEXITCODE -ne 0) { throw "Cannot inspect runtime imports for $binary" }
            $dlls = @($imports | ForEach-Object { if ($_ -match '^\s+([a-zA-Z0-9_.-]+\.dll)\s*$') { $Matches[1] } })
            if (@($dlls | Where-Object { $_ -match '^(VCRUNTIME|MSVCP|CONCRT)[0-9].*\.dll$' }).Count -gt 0) {
                throw "Portable $binary still requires an external Visual C++ runtime DLL."
            }
            $runtimeImports.binaries += [ordered]@{ name = $binary; imports = $dlls }
        }
    }
    Copy-Item -LiteralPath (Join-Path $repository 'LICENSE') -Destination (Join-Path $bundle 'LICENSE')
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'portable-desktop-README.txt') -Destination (Join-Path $bundle 'README.txt')
    & (Join-Path $PSScriptRoot 'write-desktop-notices.ps1') -Metadata $metadata -PackageIds @($cli[0].id, $desktop[0].id) -Destination $bundle

    $build = [ordered]@{
        schema_version = 1
        distribution_status = 'unreleased_development'
        version = $version
        target = $Target
        source_commit = $commit
        source_dirty = $dirty
        toolchain = $Toolchain
        static_crt = $true
        runtime_imports = $runtimeImports
        built_at_utc = [DateTime]::UtcNow.ToString('o')
        binaries = @('rproj.exe', 'rproj-desktop.exe') | ForEach-Object {
            [ordered]@{ name = $_; sha256 = (Get-FileHash -LiteralPath (Join-Path $bundle $_) -Algorithm SHA256).Hash.ToLowerInvariant() }
        }
    }
    [IO.File]::WriteAllText((Join-Path $bundle 'build.json'), ($build | ConvertTo-Json -Depth 5) + "`n", [Text.UTF8Encoding]::new($false))
    $archive = Join-Path $distribution ($label + '-' + $stamp + '.zip')
    # Crate archives often preserve 1970 timestamps; ZIP cannot represent them.
    $earliestZipTime = [DateTime]::new(1980, 1, 1, 0, 0, 0, [DateTimeKind]::Utc)
    Get-ChildItem -LiteralPath $bundle -File -Recurse | Where-Object { $_.LastWriteTimeUtc -lt $earliestZipTime } | ForEach-Object { $_.LastWriteTimeUtc = $earliestZipTime }
    Compress-Archive -LiteralPath $bundle -DestinationPath $archive -CompressionLevel Optimal

    # Verify the distributed copy, including filenames containing spaces, rather
    # than only the Cargo output. This does not claim real-window acceptance.
    $unpacked = Join-Path $staging 'unpacked smoke'
    Expand-Archive -LiteralPath $archive -DestinationPath $unpacked
    $extracted = Join-Path $unpacked $label
    foreach ($binary in @('rproj.exe', 'rproj-desktop.exe')) {
        $reported = Invoke-BundledCommand (Join-Path $extracted $binary) @('--version')
        $expected = [IO.Path]::GetFileNameWithoutExtension($binary) + ' ' + $version
        if ($reported.exit_code -ne 0 -or $reported.stdout.Trim() -cne $expected) {
            throw "Packaged $binary version smoke check failed: $($reported.stderr)"
        }
    }
    $selfCheck = Invoke-BundledCommand (Join-Path $extracted 'rproj-desktop.exe') @('--self-check')
    if ($selfCheck.exit_code -ne 0) { throw "Packaged desktop could not validate its matching CLI: $($selfCheck.stderr)" }
    $selfCheck = $selfCheck.stdout | ConvertFrom-Json
    if ($selfCheck.desktop_version -cne $version -or $selfCheck.matching_cli -ne $true) { throw 'Desktop self-check did not confirm the matching bundled CLI.' }
    $fixture = Join-Path $staging 'plain Rojo fixture'
    New-Item -ItemType Directory -Path $fixture | Out-Null
    [IO.File]::WriteAllText((Join-Path $fixture 'default.project.json'), '{"name":"Portable smoke","tree":{"$className":"DataModel"}}')
    $snapshotResult = Invoke-BundledCommand (Join-Path $extracted 'rproj.exe') @('inspect', $fixture, '--json')
    if ($snapshotResult.exit_code -ne 0) { throw "Packaged CLI inspection failed: $($snapshotResult.stderr)" }
    $snapshot = $snapshotResult.stdout | ConvertFrom-Json
    if ($snapshot.schema_version -ne 1 -or $snapshot.kind -ne 'rojo' -or -not $snapshot.actions.watch.available) {
        throw 'Packaged CLI inspection returned an unexpected snapshot.'
    }
    if (Test-Path -LiteralPath (Join-Path $fixture 'rproj.toml')) { throw 'Inspection unexpectedly adopted the smoke fixture.' }
    $archiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText(($archive + '.sha256'), "$archiveHash  $([IO.Path]::GetFileName($archive))`n")
    if ($env:GITHUB_OUTPUT) {
        [IO.File]::AppendAllText($env:GITHUB_OUTPUT, "archive=$archive`nchecksum=$archive.sha256`n", [Text.UTF8Encoding]::new($false))
    }
    Write-Host "Portable archive: $archive"
    Write-Host 'Packaged version and inspection smoke checks passed; real-window acceptance is recorded separately.'
} finally {
    Pop-Location
    $env:CARGO_ENCODED_RUSTFLAGS = $previousEncodedFlags
}
