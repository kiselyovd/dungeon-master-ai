#!/usr/bin/env bash
#
# Single source of truth for the project's quality gates. The SAME script backs:
#   - the pre-commit hook   (--fast: seconds, no compile)
#   - the pre-push hook      (default/full: adds clippy + the test suites)
#   - the CI `lint` job      (--ci-lint: matches .github/workflows/lint.yml)
#   - `bun run gates`        (default/full, for a manual local run)
#
# Keeping one list here is what stops a check from existing in CI but not locally
# (the cargo-fmt gap that slipped through M11 before this script existed).
#
# Modes:
#   --fast      cargo fmt --check, biome ci, tsc, em-dash         (pre-commit)
#   --ci-lint   --fast + cargo clippy                              (CI lint job)
#   (default)   --ci-lint + cargo test + vitest                    (pre-push / manual)
#
# e2e (Playwright) is intentionally NOT here: it needs a browser download and is
# run as its own CI job. Add it manually with `bun run e2e` when relevant.
set -uo pipefail

MODE="${1:-full}"
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

FAILED=()

run() {
  local label="$1"
  shift
  echo ""
  echo ">>> $label"
  echo "    \$ $*"
  if "$@"; then
    echo "    OK: $label"
  else
    echo "    FAIL: $label"
    FAILED+=("$label")
  fi
}

# --- static checks (fast: no Rust compile) ---------------------------------
run "cargo fmt --check"   cargo fmt --all -- --check
run "biome ci"            bunx biome ci .
run "tsc (typecheck)"     bun run typecheck
run "em-dash"             bash scripts/check-no-em-dash.sh

# --- clippy (CI lint + pre-push) -------------------------------------------
if [ "$MODE" = "--ci-lint" ] || [ "$MODE" = "full" ]; then
  run "cargo clippy" cargo clippy --workspace --all-targets --all-features -- -D warnings
fi

# --- test suites (pre-push / manual only) ----------------------------------
if [ "$MODE" = "full" ]; then
  run "cargo test"  cargo test --workspace
  run "vitest"      bun run test
fi

echo ""
if [ "${#FAILED[@]}" -eq 0 ]; then
  echo "All gates passed ($MODE)."
  exit 0
fi
echo "GATES FAILED ($MODE): ${FAILED[*]}"
exit 1
