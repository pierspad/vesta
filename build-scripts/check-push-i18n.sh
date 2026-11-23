#!/usr/bin/env bash
# Audit outgoing scorecards and localization, never let unstaged fixes mask a bad push.
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
SNAPSHOT="$(mktemp -d "${TMPDIR:-/tmp}/vesta-push.XXXXXX")"
trap 'rm -rf "$SNAPSHOT"' EXIT
check_tip() {
  local revision="$1"
  local destination="$SNAPSHOT/$revision"
  if [[ -d "$destination" ]]; then return; fi
  mkdir -p "$destination"
  git archive "$revision" | tar -x -C "$destination"
  python3 "$ROOT/build-scripts/generate_scorecard.py" --repo-root "$destination" --check
  python3 "$ROOT/build-scripts/scorecard_db.py" check --repo-root "$destination"
  ln -s "$ROOT/apps/vesta-gui/node_modules" "$destination/apps/vesta-gui/node_modules"
  bash "$ROOT/build-scripts/check-i18n.sh" "$destination"
}
seen=0
while read -r local_ref local_sha remote_ref remote_sha; do
  seen=1
  if [[ "$local_sha" =~ ^0+$ ]]; then continue; fi
  echo "Checking outgoing scorecard and localization: $local_ref"
  check_tip "$local_sha"
done
if [[ "$seen" == 0 ]]; then check_tip "$(git rev-parse HEAD)"; fi
