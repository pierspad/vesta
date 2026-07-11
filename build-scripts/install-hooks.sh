#!/bin/bash
# ===========================================================================
# Installer script for Vesta git hooks
# ===========================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
GIT_HOOKS_DIR="$PROJECT_ROOT/.git/hooks"

if [ ! -d "$GIT_HOOKS_DIR" ]; then
  echo "❌ Error: .git/hooks directory not found. Are you at the root of the Git repository?" >&2
  exit 1
fi

echo "Installing pre-push git hook..."
cp "$SCRIPT_DIR/pre-push" "$GIT_HOOKS_DIR/pre-push"
chmod +x "$GIT_HOOKS_DIR/pre-push"
chmod +x "$SCRIPT_DIR/pre-push"

echo "✅ Git hooks successfully installed!"
exit 0
