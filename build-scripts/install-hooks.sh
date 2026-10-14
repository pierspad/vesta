#!/usr/bin/env bash
# Install small dispatchers: hook policy always comes from the current checkout.
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
HOOKS="$(git rev-parse --git-path hooks)"
mkdir -p "$HOOKS"
for hook in pre-commit pre-push; do
  target="$HOOKS/$hook"
  if [[ -f "$target" ]] && ! cmp -s "$target" "$ROOT/build-scripts/$hook" && ! grep -Eq '^# Git pre-push hook for Vesta$|^# Vesta managed hook dispatcher$|^exec bash .*build-scripts/(pre-commit|pre-push)' "$target"; then
    echo "Refusing to overwrite an unrelated hook: $target" >&2
    exit 1
  fi
  cat > "$target" <<EOF
#!/usr/bin/env bash
# Vesta managed hook dispatcher
set -euo pipefail
exec bash "\$(git rev-parse --show-toplevel)/build-scripts/$hook" "\$@"
EOF
  chmod +x "$target"
done
echo "Installed Vesta pre-commit and pre-push dispatchers."
