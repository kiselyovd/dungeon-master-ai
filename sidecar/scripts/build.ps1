# Build dmai-image-sidecar for the current Windows host. Output lands in
# src-tauri/binaries/dmai-image-sidecar-<target>.exe using Tauri's externalBin
# naming convention.

param(
    [string]$Target = 'x86_64-pc-windows-msvc'
)

$ErrorActionPreference = 'Stop'
$Root = Resolve-Path "$PSScriptRoot/.."

Push-Location $Root
try {
    pip install pyinstaller==6.10
    pyinstaller --noconfirm --clean build_spec.spec
    $BinDir = Join-Path $Root "../src-tauri/binaries"
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    $Extension = if ($Target -like '*windows*') { '.exe' } else { '' }
    $Source = Join-Path $Root "dist/dmai-image-sidecar$Extension"
    $Dest = Join-Path $BinDir "dmai-image-sidecar-$Target$Extension"
    Copy-Item -LiteralPath $Source -Destination $Dest -Force
    Write-Host "Staged $Dest"
} finally {
    Pop-Location
}
