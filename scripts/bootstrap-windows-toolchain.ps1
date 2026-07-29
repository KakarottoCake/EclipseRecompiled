[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$repo = Split-Path $PSScriptRoot -Parent
$workspace = Split-Path $repo -Parent
$toolchains = Join-Path $workspace "toolchains"
$llvmName = "llvm-mingw-20260616-ucrt-x86_64"
$llvmRoot = Join-Path $toolchains $llvmName
$archive = Join-Path $workspace "$llvmName.zip"
$download = "https://github.com/mstorsjo/llvm-mingw/releases/download/20260616/$llvmName.zip"
$python = (Get-Command python.exe -ErrorAction SilentlyContinue).Source
$localPython = Join-Path $env:LOCALAPPDATA "Programs\Python\Python312\python.exe"
$cmakeRoot = Join-Path $toolchains "python-cmake-3.31"
$rustup = Join-Path $env:USERPROFILE ".cargo\bin\rustup.exe"

New-Item -ItemType Directory -Path $toolchains -Force | Out-Null

if (-not (Test-Path -LiteralPath $llvmRoot -PathType Container)) {
    if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) {
        Invoke-WebRequest -Uri $download -OutFile $archive
    }
    Expand-Archive -LiteralPath $archive -DestinationPath $toolchains
}

if (-not $python -and (Test-Path -LiteralPath $localPython -PathType Leaf)) {
    $python = $localPython
}
if (-not $python -or -not (Test-Path -LiteralPath $python -PathType Leaf)) {
    throw "Python with pip is required to install the portable CMake distribution."
}
if (-not (Test-Path -LiteralPath $cmakeRoot -PathType Container)) {
    & $python -m pip install --target $cmakeRoot "cmake==3.31.6"
    if ($LASTEXITCODE -ne 0) {
        throw "Portable CMake installation failed."
    }
}
if (-not (Test-Path -LiteralPath $rustup -PathType Leaf)) {
    throw "Install rustup from https://rustup.rs, then rerun this script."
}

& $rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal `
    --component rustfmt --component clippy
if ($LASTEXITCODE -ne 0) {
    throw "Rust GNU toolchain installation failed."
}

$rustRoot = & $rustup run stable-x86_64-pc-windows-gnu rustc --print sysroot
$rustLib = Join-Path $rustRoot "lib\rustlib\x86_64-pc-windows-gnu\lib\self-contained"
$llvmLib = Join-Path $llvmRoot "x86_64-w64-mingw32\lib"
foreach ($library in @("libgcc.a", "libgcc_eh.a", "libgcc_s.a")) {
    Copy-Item -LiteralPath (Join-Path $rustLib $library) -Destination $llvmLib -Force
}

Write-Host "Portable Eclipse recomp toolchain is ready."
