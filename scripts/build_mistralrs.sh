#!/usr/bin/env bash
# Build mistralrs-server from source (EricLBuehler/mistral.rs) and stage the
# binary under src-tauri/binaries/ using the Tauri externalBin naming
# convention `mistralrs-server-<target-triple><ext>`.
#
# Usage:
#   bash scripts/build_mistralrs.sh <target-triple> [--cuda] [out-dir]
#
# MISTRALRS_TAG pins the upstream git tag (default: v0.8.0).
#
# Pass --cuda to build with GPU acceleration. This requires the CUDA toolkit
# (nvcc) on PATH and is the documented manual step for GPU users (RTX 3080).
# Omit it for the portable CPU-only build that CI ships.
set -euo pipefail

TAG="${MISTRALRS_TAG:-v0.8.0}"
TARGET="${1:?missing target triple (e.g. x86_64-unknown-linux-gnu)}"
shift || true

CUDA=0
OUT_DIR="src-tauri/binaries"
for arg in "$@"; do
  case "$arg" in
    --cuda) CUDA=1 ;;
    *) OUT_DIR="$arg" ;;
  esac
done

case "$TARGET" in
  *windows*) EXT=".exe" ;;
  *)         EXT=""     ;;
esac

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

echo "Cloning EricLBuehler/mistral.rs@$TAG"
git clone --depth 1 --branch "$TAG" \
  https://github.com/EricLBuehler/mistral.rs "$WORKDIR/mistral.rs"

FEATURES=()
if [[ "$CUDA" == "1" ]]; then
  echo "Building mistralrs-server WITH CUDA"
  FEATURES=(--features cuda)
else
  echo "Building mistralrs-server (CPU-only)"
fi

( cd "$WORKDIR/mistral.rs" \
  && cargo build --release --package mistralrs-server "${FEATURES[@]}" )

mkdir -p "$OUT_DIR"
DEST="$OUT_DIR/mistralrs-server-$TARGET$EXT"
cp "$WORKDIR/mistral.rs/target/release/mistralrs-server$EXT" "$DEST"
chmod +x "$DEST" 2>/dev/null || true
echo "Staged $DEST"
