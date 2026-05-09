# Downloads a prebuilt mistralrs-server binary from EricLBuehler/mistral.rs releases
# and renames it to the Tauri externalBin convention so `tauri.conf.json` resolves
# the right asset per platform.
#
# Usage:
#   pwsh -NoProfile -File scripts/download_mistralrs.ps1 [-Target <triple>] [-OutDir <dir>]
#
# $env:MISTRALRS_VERSION pins the upstream tag (default: v0.8.0).

param(
  [string]$Version = $env:MISTRALRS_VERSION,
  [string]$Target = "x86_64-pc-windows-msvc",
  [string]$OutDir = "src-tauri/binaries"
)

if (-not $Version) { $Version = "v0.8.0" }

$asset = "mistralrs-server-$Target.exe"
$url   = "https://github.com/EricLBuehler/mistral.rs/releases/download/$Version/$asset"

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$dest = Join-Path $OutDir "mistralrs-server-$Target.exe"
Write-Host "Downloading $url -> $dest"

# Upstream EricLBuehler/mistral.rs ships source-only releases as of v0.8.0 -
# no prebuilt binaries on GitHub. A 404 here is non-fatal: build.rs's
# ensure_mistralrs_placeholder lays down an empty file so tauri build
# resolves the externalBin entry. Local Mode will be a no-op until a real
# binary lands here, which is fine for cloud-only first-GA releases.
try {
    Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing -ErrorAction Stop
} catch {
    Write-Warning "$url not available ($($_.Exception.Message)); leaving placeholder for build.rs to create."
}
