[CmdletBinding()]
param(
    [string]$SymbolMap
)

$ErrorActionPreference = "Stop"
$repo = Split-Path $PSScriptRoot -Parent
. (Join-Path $PSScriptRoot "build-env.ps1")
$dol = Join-Path $repo "eclipse\main.dol"

if (-not $SymbolMap) {
    $SymbolMap = Join-Path (Split-Path $repo -Parent) `
        "third_party\BetterSunshineEngine\maps\us.map"
}
if (-not (Test-Path -LiteralPath $dol -PathType Leaf)) {
    throw "Run scripts\prepare-eclipse.ps1 first."
}
if (-not (Test-Path -LiteralPath $SymbolMap -PathType Leaf)) {
    throw "Sunshine symbol map not found: $SymbolMap"
}

Push-Location $repo
try {
    & $EclipseRustup run $EclipseRustToolchain cargo run --release `
        --target $EclipseRustTarget -p gcrecomp-cli -- recompile `
        --dol-file $dol `
        --symbol-map $SymbolMap
    if ($LASTEXITCODE -ne 0) {
        throw "Static recompilation failed with exit code $LASTEXITCODE."
    }

    & $EclipseRustup run $EclipseRustToolchain cargo build --release `
        --target $EclipseRustTarget -p game
    if ($LASTEXITCODE -ne 0) {
        throw "Native game build failed with exit code $LASTEXITCODE."
    }
} finally {
    Pop-Location
}
