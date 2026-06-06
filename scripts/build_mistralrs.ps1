# Build mistralrs-server from source (EricLBuehler/mistral.rs) and stage the
# binary under src-tauri/binaries/ using the Tauri externalBin naming
# convention `mistralrs-server-<target-triple><ext>`.
#
# Usage:
#   pwsh -NoProfile -File scripts/build_mistralrs.ps1 -Target <triple> [-Cuda] [-OutDir <dir>]
#
# $env:MISTRALRS_TAG pins the upstream git tag (default: v0.8.0).
# Pass -Cuda to build with GPU acceleration (requires the CUDA toolkit on PATH);
# omit it for the portable CPU-only build that CI ships.
param(
  [Parameter(Mandatory = $true)][string]$Target,
  [switch]$Cuda,
  [string]$OutDir = "src-tauri/binaries"
)
$ErrorActionPreference = "Stop"

$tag = if ($env:MISTRALRS_TAG) { $env:MISTRALRS_TAG } else { "v0.8.0" }
$ext = if ($Target -like "*windows*") { ".exe" } else { "" }

$workdir = (New-Item -ItemType Directory -Path (Join-Path $env:TEMP ([System.Guid]::NewGuid()))).FullName
try {
  Write-Host "Cloning EricLBuehler/mistral.rs@$tag"
  git clone --depth 1 --branch $tag `
    https://github.com/EricLBuehler/mistral.rs (Join-Path $workdir "mistral.rs")

  $features = @()
  if ($Cuda) {
    Write-Host "Building mistralrs-server WITH CUDA"
    $features = @("--features", "cuda")
  } else {
    Write-Host "Building mistralrs-server (CPU-only)"
  }

  Push-Location (Join-Path $workdir "mistral.rs")
  try {
    cargo build --release --package mistralrs-server @features
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
  } finally {
    Pop-Location
  }

  New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
  $dest = Join-Path $OutDir "mistralrs-server-$Target$ext"
  Copy-Item (Join-Path $workdir "mistral.rs/target/release/mistralrs-server$ext") $dest -Force
  Write-Host "Staged $dest"
} finally {
  Remove-Item -Recurse -Force $workdir
}
