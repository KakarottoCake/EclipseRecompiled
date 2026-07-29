[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$IsoPath
)

$ErrorActionPreference = "Stop"
$repo = Split-Path $PSScriptRoot -Parent
. (Join-Path $PSScriptRoot "build-env.ps1")

if (-not (Test-Path -LiteralPath $IsoPath -PathType Leaf)) {
    throw "Eclipse ISO not found: $IsoPath"
}
Push-Location $repo
try {
    & $EclipseRustup run $EclipseRustToolchain cargo run `
        --target $EclipseRustTarget -p gcrecomp-cli -- prepare `
        --disc-image $IsoPath `
        --output-dir eclipse
    if ($LASTEXITCODE -ne 0) {
        throw "Disc preparation failed with exit code $LASTEXITCODE."
    }
} finally {
    Pop-Location
}
