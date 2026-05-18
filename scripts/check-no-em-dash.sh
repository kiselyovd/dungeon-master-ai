#!/usr/bin/env bash
# Reject em-dash (U+2014) and en-dash (U+2013) in committed source.
# Project policy: plain hyphen "-" only.
# Coverage: tracked files under src/, src-tauri/, crates/, docs/
# (uses git ls-files so .gitignore is respected automatically).
set -e

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

TRACKED=$(git ls-files src/ src-tauri/ crates/ docs/ 2>/dev/null \
  | grep -E '\.(ts|tsx|js|json|rs|md)$' || true)

if [[ -z "$TRACKED" ]]; then
  exit 0
fi

FOUND=$(echo "$TRACKED" | xargs -d '\n' grep -n $'[\xe2\x80\x94\xe2\x80\x93]' 2>/dev/null || true)

if [[ -n "$FOUND" ]]; then
  echo "ERROR: em-dash (U+2014) or en-dash (U+2013) found. Project policy: plain hyphen only." >&2
  echo "$FOUND" >&2
  exit 1
fi
exit 0
