# Build the mistralrs-cli binary (`mistralrs`, EricLBuehler/mistral.rs) from
# source and stage it under src-tauri/binaries/ using the Tauri externalBin
# naming convention `mistralrs-server-<target-triple><ext>` (the on-disk name is
# kept for backward compat with the launcher; the file is the new `mistralrs`
# binary, driven via its `serve` subcommand - the old `mistralrs-server` binary
# is deprecated and mangles Gemma tool-call output).
#
# Usage:
#   pwsh -NoProfile -File scripts/build_mistralrs.ps1 -Target <triple> [-Cuda] [-OutDir <dir>]
#
# $env:MISTRALRS_TAG pins the upstream git tag (default: v0.8.3 - v0.8.2 brought
# the tool-calling/agentic fixes the DM agent depends on).
# Pass -Cuda to build with GPU acceleration (requires the CUDA toolkit on PATH);
# omit it for the portable CPU-only build that CI ships.
param(
  [Parameter(Mandatory = $true)][string]$Target,
  [switch]$Cuda,
  [string]$OutDir = "src-tauri/binaries"
)
$ErrorActionPreference = "Stop"

$tag = if ($env:MISTRALRS_TAG) { $env:MISTRALRS_TAG } else { "v0.8.3" }
$ext = if ($Target -like "*windows*") { ".exe" } else { "" }

# A CUDA build invokes nvcc, which needs the MSVC host compiler `cl.exe` on
# PATH. Outside a "Developer Command Prompt" cl.exe is absent and nvcc fails
# with "Cannot find compiler 'cl.exe' in PATH". Import the VC++ environment via
# vswhere + vcvars64.bat so a plain shell (or CI runner) can build unattended.
function Import-MsvcEnv {
  if (Get-Command cl.exe -ErrorAction SilentlyContinue) { return }
  $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
  if (-not (Test-Path $vswhere)) {
    Write-Warning "vswhere not found; cl.exe may be missing for the CUDA build."
    return
  }
  $vsPath = & $vswhere -latest -products * `
    -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
    -property installationPath
  if (-not $vsPath) { Write-Warning "No VC++ toolset found via vswhere."; return }
  $vcvars = Join-Path $vsPath 'VC\Auxiliary\Build\vcvars64.bat'
  if (-not (Test-Path $vcvars)) { Write-Warning "vcvars64.bat not found at $vcvars."; return }

  # CUDA's nvcc host_config.h rejects MSVC newer than VS2022 (toolset 14.4x).
  # VS2026 ships toolset 14.5x, which fails with "unsupported Microsoft Visual
  # Studio version". If a CUDA-compatible 14.2x-14.4x toolset is installed
  # side-by-side, pin it via -vcvars_ver so nvcc accepts the host compiler.
  $verArg = ''
  $msvcRoot = Join-Path $vsPath 'VC\Tools\MSVC'
  if (Test-Path $msvcRoot) {
    $compatible = Get-ChildItem $msvcRoot -Directory |
      Where-Object { $_.Name -match '^14\.([0-3]\d|4\d)\.' } |
      Sort-Object { [version]$_.Name } -Descending |
      Select-Object -First 1
    if ($compatible) {
      $verArg = "-vcvars_ver=$($compatible.Name)"
      Write-Host "Pinning CUDA-compatible MSVC toolset $($compatible.Name)"
    } else {
      Write-Warning "No VS2019-2022 (14.2x-14.4x) MSVC toolset found; nvcc may reject the host compiler."
    }
  }

  Write-Host "Importing MSVC environment from $vcvars $verArg"
  cmd /c "`"$vcvars`" $verArg >nul 2>&1 && set" | ForEach-Object {
    if ($_ -match '^([^=]+)=(.*)$') {
      [System.Environment]::SetEnvironmentVariable($matches[1], $matches[2])
    }
  }
}

$workdir = (New-Item -ItemType Directory -Path (Join-Path $env:TEMP ([System.Guid]::NewGuid()))).FullName
try {
  Write-Host "Cloning EricLBuehler/mistral.rs@$tag"
  # --quiet keeps git's progress off stderr; under ErrorActionPreference=Stop a
  # stderr write from a native command can otherwise abort the script.
  git clone --quiet --depth 1 --branch $tag `
    https://github.com/EricLBuehler/mistral.rs (Join-Path $workdir "mistral.rs")

  $features = @()
  if ($Cuda) {
    Write-Host "Building mistralrs-cli WITH CUDA"
    $features = @("--features", "cuda")
    Import-MsvcEnv
  } else {
    Write-Host "Building mistralrs-cli (CPU-only)"
  }

  Push-Location (Join-Path $workdir "mistral.rs")
  try {
    cargo build --release --package mistralrs-cli @features
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
  } finally {
    Pop-Location
  }

  New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
  # On-disk name stays `mistralrs-server-<triple>` for launcher/build.rs
  # compatibility; the file itself is the new `mistralrs` binary (driven via
  # its `serve` subcommand).
  $dest = Join-Path $OutDir "mistralrs-server-$Target$ext"
  Copy-Item (Join-Path $workdir "mistral.rs/target/release/mistralrs$ext") $dest -Force
  Write-Host "Staged $dest (mistralrs-cli binary)"
} finally {
  Remove-Item -Recurse -Force $workdir
}
