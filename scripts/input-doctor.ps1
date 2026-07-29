[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$repo = Split-Path $PSScriptRoot -Parent
. (Join-Path $PSScriptRoot "build-env.ps1")
$runtimeDirectory = Join-Path $env:CARGO_TARGET_DIR "$EclipseRustTarget\debug"
$env:PATH = "$runtimeDirectory;$env:PATH"

Push-Location $repo
try {
    & $EclipseRustup run $EclipseRustToolchain cargo run `
        --target $EclipseRustTarget -p gcrecomp-runtime --example input_doctor
    if ($LASTEXITCODE -ne 0) {
        throw "Input diagnostic failed with exit code $LASTEXITCODE."
    }
} finally {
    Pop-Location
}
