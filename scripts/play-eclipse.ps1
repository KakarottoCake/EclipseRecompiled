[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$repo = Split-Path $PSScriptRoot -Parent
$release = Join-Path $repo "target-llvm\x86_64-pc-windows-gnu\release\game.exe"
$debug = Join-Path $repo "target-llvm\x86_64-pc-windows-gnu\debug\game.exe"
$game = if (Test-Path -LiteralPath $release -PathType Leaf) {
    $release
} else {
    $debug
}

if (-not (Test-Path -LiteralPath $game -PathType Leaf)) {
    throw "Run scripts\build-eclipse.ps1 first."
}

$env:GCRECOMP_ASSETS = Join-Path $repo "game\assets.bin"
Push-Location $repo
try {
    & $game
} finally {
    Pop-Location
}
