$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$archive = Join-Path (Get-Location) 'rokit.zip'
$url = 'https://github.com/rojo-rbx/rokit/releases/download/v1.2.0/rokit-1.2.0-windows-x86_64.zip'
$expected = 'f9ba1704014ff67d51e8005f605955c7c26d2429a5312a9419dc477fc310e96d'
Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $archive
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $expected) {
    throw 'Rokit archive SHA-256 mismatch'
}
Expand-Archive -LiteralPath $archive -DestinationPath 'release'
& '.\release\rokit.exe' self-install
exit $LASTEXITCODE
