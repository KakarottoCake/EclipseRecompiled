$ErrorActionPreference = "Stop"

$script:EclipseRepo = Split-Path $PSScriptRoot -Parent
$workspace = Split-Path $script:EclipseRepo -Parent
$llvmRoot = Join-Path $workspace "toolchains\llvm-mingw-20260616-ucrt-x86_64"
$llvmBin = Join-Path $llvmRoot "bin"
$cmake = Join-Path $workspace "toolchains\python-cmake-3.31\cmake\data\bin\cmake.exe"
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"

$script:EclipseRustup = Join-Path $cargoBin "rustup.exe"
$script:EclipseRustToolchain = "stable-x86_64-pc-windows-gnu"
$script:EclipseRustTarget = "x86_64-pc-windows-gnu"

$required = @(
    $script:EclipseRustup,
    (Join-Path $llvmBin "x86_64-w64-mingw32-clang.exe"),
    $cmake
)
foreach ($path in $required) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Portable build tool missing: $path. Run scripts\bootstrap-windows-toolchain.ps1."
    }
}

$env:PATH = "$llvmBin;$cargoBin;$env:PATH"
$env:CC = Join-Path $llvmBin "x86_64-w64-mingw32-clang.exe"
$env:CXX = Join-Path $llvmBin "x86_64-w64-mingw32-clang++.exe"
$env:CC_x86_64_pc_windows_gnu = $env:CC
$env:CXX_x86_64_pc_windows_gnu = $env:CXX
$env:AR_x86_64_pc_windows_gnu = Join-Path $llvmBin "llvm-ar.exe"
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = $env:CC
$env:RUSTFLAGS = "-C link-self-contained=no"
$env:CMAKE = $cmake
$env:CMAKE_GENERATOR = "Ninja"
$env:CARGO_TARGET_DIR = Join-Path $script:EclipseRepo "target-llvm"
