#!/usr/bin/env bash
set -euo pipefail
benchmark_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$benchmark_dir/../.." && pwd)"
for argument in "$@"; do
  if [[ "$argument" == "--dry-run" || "$argument" == "--help" || "$argument" == "-h" ]]; then
    exec python3 "$benchmark_dir/verify_native.py" "$@"
  fi
done
command -v python3 >/dev/null
command -v ffmpeg >/dev/null
command -v ffprobe >/dev/null
cd -- "$repo_dir"
cargo build --release -p srt-flashcards-cli
exec python3 "$benchmark_dir/verify_native.py" "$@"
