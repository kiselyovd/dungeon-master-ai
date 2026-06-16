#!/usr/bin/env bash
# Build the mistralrs-cli binary (`mistralrs`, EricLBuehler/mistral.rs) from
# source and stage it under src-tauri/binaries/ using the Tauri externalBin
# naming convention `mistralrs-server-<target-triple><ext>` (the on-disk name is
# kept for launcher/build.rs compat; the file is the new `mistralrs` binary,
# driven via its `serve` subcommand - the old `mistralrs-server` is deprecated
# and mangles Gemma tool-call output).
#
# Usage:
#   bash scripts/build_mistralrs.sh <target-triple> [--cuda] [out-dir]
#
# MISTRALRS_TAG pins the upstream git tag (default: v0.8.3 - v0.8.2 brought the
# tool-calling/agentic fixes the DM agent depends on).
#
# Pass --cuda to build with GPU acceleration. This requires the CUDA toolkit
# (nvcc) on PATH and is the documented manual step for GPU users (RTX 3080).
# Omit it for the portable CPU-only build that CI ships.
set -euo pipefail

TAG="${MISTRALRS_TAG:-v0.8.3}"
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
  echo "Building mistralrs-cli WITH CUDA"
  FEATURES=(--features cuda)
else
  echo "Building mistralrs-cli (CPU-only)"
fi

( cd "$WORKDIR/mistral.rs" \
  && cargo build --release --package mistralrs-cli "${FEATURES[@]}" )

mkdir -p "$OUT_DIR"
# On-disk name stays `mistralrs-server-<triple>` for launcher/build.rs compat;
# the file itself is the new `mistralrs` binary (driven via `serve`).
DEST="$OUT_DIR/mistralrs-server-$TARGET$EXT"
cp "$WORKDIR/mistral.rs/target/release/mistralrs$EXT" "$DEST"
chmod +x "$DEST" 2>/dev/null || true
echo "Staged $DEST (mistralrs-cli binary)"
