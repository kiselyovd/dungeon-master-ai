#!/usr/bin/env bash
# Downloads a prebuilt mistralrs-server binary from EricLBuehler/mistral.rs releases
# and renames it to the Tauri externalBin convention `mistralrs-server-{target-triple}`
# so `tauri.conf.json` resolves the right asset per platform.
#
# Usage:
#   bash scripts/download_mistralrs.sh <target-triple> [out-dir]
#
# `MISTRALRS_VERSION` env var pins the upstream tag (default: v0.8.0).

set -euo pipefail

VERSION="${MISTRALRS_VERSION:-v0.8.0}"
TARGET="${1:?missing target triple (e.g. x86_64-unknown-linux-gnu)}"
OUT_DIR="${2:-src-tauri/binaries}"

case "$TARGET" in
  x86_64-pc-windows-msvc)     ASSET="mistralrs-server-x86_64-pc-windows-msvc.exe"; SUFFIX=".exe" ;;
  aarch64-apple-darwin)       ASSET="mistralrs-server-aarch64-apple-darwin";       SUFFIX=""    ;;
  x86_64-apple-darwin)        ASSET="mistralrs-server-x86_64-apple-darwin";        SUFFIX=""    ;;
  x86_64-unknown-linux-gnu)   ASSET="mistralrs-server-x86_64-unknown-linux-gnu";   SUFFIX=""    ;;
  *) echo "Unknown target $TARGET" >&2; exit 1 ;;
esac

URL="https://github.com/EricLBuehler/mistral.rs/releases/download/$VERSION/$ASSET"
mkdir -p "$OUT_DIR"
DEST="$OUT_DIR/mistralrs-server-$TARGET$SUFFIX"
echo "Downloading $URL -> $DEST"
curl -fL -o "$DEST" "$URL"
chmod +x "$DEST"
