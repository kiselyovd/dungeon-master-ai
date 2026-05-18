#!/usr/bin/env bash
# Install the em-dash check as a local pre-commit hook.
# Idempotent: overwrites any existing .git/hooks/pre-commit.
set -e

ROOT="$(git rev-parse --show-toplevel)"
HOOK="$ROOT/.git/hooks/pre-commit"

cat > "$HOOK" <<'HOOK_BODY'
#!/usr/bin/env bash
set -e
ROOT="$(git rev-parse --show-toplevel)"
exec bash "$ROOT/scripts/check-no-em-dash.sh"
HOOK_BODY

chmod +x "$HOOK"
echo "Installed pre-commit hook at $HOOK"
