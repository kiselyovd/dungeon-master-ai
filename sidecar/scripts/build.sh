#!/usr/bin/env bash
# Build dmai-image-sidecar for the supplied Rust target triple.
# Defaults to x86_64-unknown-linux-gnu when no arg is given.

set -euo pipefail

TARGET="${1:-x86_64-unknown-linux-gnu}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

cd "$ROOT"
pip install pyinstaller==6.10
pyinstaller --noconfirm --clean build_spec.spec

case "$TARGET" in
  *windows*) EXT=".exe" ;;
  *) EXT="" ;;
esac
BIN_DIR="$ROOT/../src-tauri/binaries"
DEST="$BIN_DIR/dmai-image-sidecar-${TARGET}${EXT}"
mkdir -p "$BIN_DIR"
cp "dist/dmai-image-sidecar${EXT}" "$DEST"
chmod +x "$DEST" 2>/dev/null || true
echo "Staged $DEST"
