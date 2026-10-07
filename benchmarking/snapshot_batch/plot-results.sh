#!/usr/bin/env bash
set -euo pipefail
benchmark_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$benchmark_dir/../.." && pwd)"
plot_python=python3
if [[ -x "$repo_dir/.venv/bin/python3" ]]; then
  plot_python="$repo_dir/.venv/bin/python3"
fi
exec "$plot_python" "$benchmark_dir/plot_results.py" "$@"
