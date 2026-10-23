#!/usr/bin/env bash
# Audit outgoing tips, never let an unstaged translation fix mask a bad push.
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
SNAPSHOT="$(mktemp -d "${TMPDIR:-/tmp}/vesta-push.XXXXXX")"
trap 'rm -rf "$SNAPSHOT"' EXIT
check_tip() {
  local revision="$1"
  local destination="$SNAPSHOT/$revision"
  mkdir -p "$destination"
  git archive "$revision" apps/srt-gui/src apps/srt-gui/scripts | tar -x -C "$destination"
  ln -s "$ROOT/apps/srt-gui/node_modules" "$destination/apps/srt-gui/node_modules"
  bash "$ROOT/build-scripts/check-i18n.sh" "$destination"
}
seen=0
while read -r local_ref local_sha remote_ref remote_sha; do
  seen=1
  if [[ "$local_sha" =~ ^0+$ ]]; then continue; fi
  echo "Checking outgoing localization: $local_ref"
  check_tip "$local_sha"
done
if [[ "$seen" == 0 ]]; then check_tip "$(git rev-parse HEAD)"; fi
