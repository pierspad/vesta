#!/usr/bin/env bash
#
# Shared helpers for the benchmark scripts. Source after config.sh.

set -euo pipefail

# Colors
C_RED='\033[0;31m'; C_GREEN='\033[0;32m'; C_YELLOW='\033[1;33m'
C_BLUE='\033[0;34m'; C_BOLD='\033[1m'; C_NC='\033[0m'

# Resolve repo root from this file's location (benchmarking_against_subs2srs/lib/common.sh -> repo root).
BENCH_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$BENCH_LIB_DIR/../.." && pwd)"

log()   { echo -e "${C_BLUE}▶${C_NC} $*"; }
ok()    { echo -e "${C_GREEN}✔${C_NC} $*"; }
warn()  { echo -e "${C_YELLOW}⚠${C_NC} $*"; }
err()   { echo -e "${C_RED}x${C_NC} $*" >&2; }
die()   { err "$*"; exit 1; }

# Require a command to exist on PATH.
need() { command -v "$1" >/dev/null 2>&1 || die "Required tool not found: $1"; }

# Count subtitle cues in an .srt file (number of "-->" timing lines).
srt_count() { grep -cE '[0-9]{2}:[0-9]{2}:[0-9]{2}[,.][0-9]{3}[[:space:]]*-->' "$1" 2>/dev/null || echo 0; }

# Wall-clock milliseconds since epoch.
now_ms() { date +%s%3N; }

# Median of integers passed as arguments.
median() {
  local sorted n
  sorted=$(printf '%s\n' "$@" | sort -n)
  n=$#
  if (( n % 2 == 1 )); then
    echo "$sorted" | sed -n "$(( n / 2 + 1 ))p"
  else
    local a b
    a=$(echo "$sorted" | sed -n "$(( n / 2 ))p")
    b=$(echo "$sorted" | sed -n "$(( n / 2 + 1 ))p")
    echo $(( (a + b) / 2 ))
  fi
}
