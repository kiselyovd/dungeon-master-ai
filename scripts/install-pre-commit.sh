#!/usr/bin/env bash
# Install the project git hooks. Both delegate to scripts/gates.sh so local
# checks can never silently drift from CI.
#   pre-commit -> gates.sh --fast  (cargo fmt --check, biome ci, tsc, em-dash)
#   pre-push   -> gates.sh         (adds clippy + cargo test + vitest)
# Idempotent: overwrites any existing hooks. Bypass once with --no-verify.
set -e

ROOT="$(git rev-parse --show-toplevel)"

cat > "$ROOT/.git/hooks/pre-commit" <<'HOOK_BODY'
#!/usr/bin/env bash
set -e
ROOT="$(git rev-parse --show-toplevel)"
exec bash "$ROOT/scripts/gates.sh" --fast
HOOK_BODY
chmod +x "$ROOT/.git/hooks/pre-commit"

cat > "$ROOT/.git/hooks/pre-push" <<'HOOK_BODY'
#!/usr/bin/env bash
set -e
ROOT="$(git rev-parse --show-toplevel)"
exec bash "$ROOT/scripts/gates.sh"
HOOK_BODY
chmod +x "$ROOT/.git/hooks/pre-push"

echo "Installed git hooks:"
echo "  pre-commit -> scripts/gates.sh --fast"
echo "  pre-push   -> scripts/gates.sh (full)"
