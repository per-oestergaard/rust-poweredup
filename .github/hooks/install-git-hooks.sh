#!/usr/bin/env bash
# Run once to install the git pre-commit hook into this clone.
# Usage: bash .github/hooks/install-git-hooks.sh
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
HOOK="$REPO_ROOT/.git/hooks/pre-commit"

cat > "$HOOK" <<'EOF'
#!/usr/bin/env bash
# Pre-commit: pin any unpinned GitHub Actions in staged workflow files.
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$REPO_ROOT/.github/skills/pin-actions-to-sha/scripts/pin-actions.py"

# Collect staged workflow files
staged=$(git diff --cached --name-only --diff-filter=ACM | grep -E '^\.github/workflows/.*\.ya?ml$' || true)
[ -z "$staged" ] && exit 0

echo "[pre-commit] Pinning GitHub Actions to SHA hashes..."
python3 "$SCRIPT"

# Re-stage any files the script modified
for f in $staged; do
  git add "$f"
done
EOF

chmod +x "$HOOK"
echo "Pre-commit hook installed at $HOOK"
