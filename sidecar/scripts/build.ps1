# Build dmai-image-sidecar for the current Windows host. Output lands in
# src-tauri/binaries/python_sidecar_x86_64-pc-windows-msvc/ so Tauri's
# externalBin picks it up automatically.

param(
    [string]$Target = 'x86_64-pc-windows-msvc'
)

$ErrorActionPreference = 'Stop'
$Root = Resolve-Path "$PSScriptRoot/.."

Push-Location $Root
try {
    pip install pyinstaller==6.10
    pyinstaller --noconfirm --clean build_spec.spec
    $Dest = Join-Path $Root "../src-tauri/binaries/python_sidecar_$Target"
    New-Item -ItemType Directory -Force -Path $Dest | Out-Null
    Copy-Item -Recurse -Force "dist/dmai-image-sidecar/*" $Dest
} finally {
    Pop-Location
}
