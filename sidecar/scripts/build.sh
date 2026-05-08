#!/usr/bin/env bash
# Build dmai-image-sidecar for the supplied Rust target triple.
# Defaults to x86_64-unknown-linux-gnu when no arg is given.

set -euo pipefail

TARGET="${1:-x86_64-unknown-linux-gnu}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

cd "$ROOT"
pip install pyinstaller==6.10
pyinstaller --noconfirm --clean build_spec.spec

DEST="$ROOT/../src-tauri/binaries/python_sidecar_${TARGET}"
mkdir -p "$DEST"
cp -R dist/dmai-image-sidecar/* "$DEST/"
