#!/usr/bin/env bash
# One localization gate for hooks, CI and manual validation.
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
TARGET="${1:-$ROOT}"
REPORT="$(mktemp "${TMPDIR:-/tmp}/vesta-i18n.XXXXXX")"
trap 'rm -f "$REPORT"' EXIT
python3 "$ROOT/apps/srt-gui/scripts/check_missing_translations.py" --locales-dir "$TARGET/apps/srt-gui/src/lib/i18n/locales" --fail-on-issues --output "$REPORT"
node "$TARGET/apps/srt-gui/scripts/check-i18n.mjs"
