#!/usr/bin/env bash
# Reject em-dash (U+2014) and en-dash (U+2013) in committed source.
# Project policy: plain hyphen "-" only.
# Coverage: tracked files under src/, src-tauri/, crates/, docs/
# (uses git ls-files so .gitignore is respected automatically).
set -e

# Without a UTF-8 locale, grep treats input as raw bytes and the bracket
# expression [\xe2\x80\x94\xe2\x80\x93] matches any byte equal to e2/80/94/93,
# producing false positives on any Cyrillic/Greek/CJK character that happens
# to share a UTF-8 lead/continuation byte. GitHub Actions Ubuntu runners
# default to C.UTF-8 only sometimes, so pin it explicitly.
export LC_ALL=C.UTF-8

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
