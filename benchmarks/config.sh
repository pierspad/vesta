#!/usr/bin/env bash
#
# Benchmark configuration — edit this file to change what gets measured.
# Sourced by every numbered script. All paths are relative to the repo root.

# ── Test media (auto-discovered) ──────────────────────────────────────────────
# Films are discovered automatically from $BENCH_MEDIA_DIR. Expected layout:
#   <Title>-<target_lang>.srt      e.g. Detour-en.srt   (language being learned)
#   <Title>-<native_lang>.srt      e.g. Detour-it.srt   (matched by time overlap)
#   <Title>*.mp4|mkv|avi|m4v|webm  e.g. Detour(1945).mp4
# A film enters the benchmark only when BOTH subtitle tracks AND a video are
# present; anything incomplete (e.g. "8 e mezzo": video without subtitles) is
# skipped with a warning. Drop new films into the folder and re-run — no config
# edits needed. Each discovered entry uses the canonical format:
#   "name|target_subs.srt|native_subs.srt|video_file"
BENCH_MEDIA_DIR="${BENCH_MEDIA_DIR:-Test_Subs/FILM}"
BENCH_TARGET_LANG="${BENCH_TARGET_LANG:-en}"
BENCH_NATIVE_LANG="${BENCH_NATIVE_LANG:-it}"

# Populates TEST_MEDIA scanning $1/$BENCH_MEDIA_DIR ($1 = repo root).
discover_test_media() {
  local root="$1" dir="$1/$BENCH_MEDIA_DIR"
  local srt title nat_srt video ext v entry claimed
  TEST_MEDIA=()
  [ -d "$dir" ] || { echo "⚠ media dir not found: $dir" >&2; return 0; }

  local _nullglob_was_off=0
  shopt -q nullglob || _nullglob_was_off=1
  shopt -s nullglob

  for srt in "$dir"/*-"$BENCH_TARGET_LANG".srt; do
    title="$(basename "$srt")"
    title="${title%-"$BENCH_TARGET_LANG".srt}"

    nat_srt="$dir/$title-$BENCH_NATIVE_LANG.srt"
    if [ ! -f "$nat_srt" ]; then
      echo "⚠ skip '$title': missing ${BENCH_NATIVE_LANG} subtitles" >&2
      continue
    fi

    video=""
    for ext in mp4 mkv avi m4v webm; do
      for v in "$dir/$title"*."$ext"; do
        video="$v"
        break 2
      done
    done
    if [ -z "$video" ]; then
      echo "⚠ skip '$title': subtitles present but no video found" >&2
      continue
    fi

    # Store paths relative to the repo root (scripts cd there before running).
    TEST_MEDIA+=("$title|${srt#"$root/"}|${nat_srt#"$root/"}|${video#"$root/"}")
  done

  # Point out videos that have no usable subtitle pair (informational only).
  for v in "$dir"/*.mp4 "$dir"/*.mkv "$dir"/*.avi "$dir"/*.m4v "$dir"/*.webm; do
    claimed=0
    for entry in "${TEST_MEDIA[@]}"; do
      [ "${entry##*|}" = "${v#"$root/"}" ] && { claimed=1; break; }
    done
    [ "$claimed" = 1 ] || echo "ℹ ignored video (no ${BENCH_TARGET_LANG}/${BENCH_NATIVE_LANG} subtitle pair): $(basename "$v")" >&2
  done

  [ "$_nullglob_was_off" = 1 ] && shopt -u nullglob
  return 0
}

_cfg_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
discover_test_media "$_cfg_repo_root"

# ── Vesta variants ────────────────────────────────────────────────────────────
# Compare one or more Vesta builds against each other and against subs2srs.
# Format: "label|path_to_srt-flashcards_binary|jobs"
#   jobs = MAX  -> Vesta's "max" plan (cores-1, the GUI's top setting)
#   jobs = 1    -> single ffmpeg worker, an apples-to-apples match for subs2srs
#   jobs = N    -> exactly N parallel workers
# `2_compile_vesta.sh` builds the binary into target/release. To compare a
# branch/fork, build it elsewhere and add a line pointing at it, e.g.:
#   "experimental|/path/to/other/target/release/srt-flashcards|MAX"
#
# With the two variants below, each film is timed as 5 series:
#   subs2srs (TSV) · Vesta export TSV/APKG (max) · Vesta export TSV/APKG (1 core)
VESTA_VARIANTS=(
  "max|target/release/srt-flashcards|MAX"
  "single|target/release/srt-flashcards|1"
)

# Export formats to benchmark for Vesta (subs2srs only produces TSV).
VESTA_FORMATS=(tsv apkg)

# ── Parallelism ───────────────────────────────────────────────────────────────
# Vesta runs on its "max" plan: all logical cores minus one.
# subs2srs is left exactly as written (single-threaded, sequential ffmpeg) — we
# never add parallelism it does not have. Honest comparison.
CORES="$(nproc 2>/dev/null || echo 4)"
VESTA_JOBS="$(( CORES > 1 ? CORES - 1 : 1 ))"

# ── Repeats ───────────────────────────────────────────────────────────────────
# Number of timed repetitions per (tool, media, format). The median is reported.
REPEATS="${REPEATS:-1}"

# ── Layout ────────────────────────────────────────────────────────────────────
SUBS2SRS_EXE="benchmarks/subs2srs-headless/subs2srs-headless.exe"
RESULTS_DIR="benchmarks/results"
RESULTS_CSV="${RESULTS_DIR}/results.csv"
WORK_DIR="benchmarks/.work"   # scratch output for generated decks (gitignored)

# Optional local override (gitignored): create benchmarks/config.local.sh to
# point TEST_MEDIA / VESTA_VARIANTS at your own files without editing this file.
_cfg_local="$(dirname "${BASH_SOURCE[0]}")/config.local.sh"
[ -f "$_cfg_local" ] && source "$_cfg_local"
