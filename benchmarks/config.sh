#!/usr/bin/env bash
#
# Benchmark configuration — edit this file to change what gets measured.
# Sourced by every numbered script. All paths are relative to the repo root.

# ── Test media ────────────────────────────────────────────────────────────────
# One entry per movie/episode. Format:
#   "name|target_subs.srt|native_subs.srt|video_file"
# The target subs are the language being learned; native subs are matched to it.
# Every film below has a full en/it subtitle pair + a video file. ("8 e mezzo"
# is intentionally left out — it ships without subtitles, so nothing to time.)
TEST_MEDIA=(
  "Detour|Test_Subs/FILM/Detour-en.srt|Test_Subs/FILM/Detour-it.srt|Test_Subs/FILM/Detour(1945).mp4"
  "Good Will Hunting|Test_Subs/FILM/Good.Will.Hunting-en.srt|Test_Subs/FILM/Good.Will.Hunting-it.srt|Test_Subs/FILM/Good.Will.Hunting.mp4"
  "In Bruges|Test_Subs/FILM/In.Bruges-en.srt|Test_Subs/FILM/In.Bruges-it.srt|Test_Subs/FILM/In.Bruges.mkv"
  "Interstellar|Test_Subs/FILM/Interstellar-en.srt|Test_Subs/FILM/Interstellar-it.srt|Test_Subs/FILM/Interstellar.mp4"
  "Snatch|Test_Subs/FILM/Snatch-en.srt|Test_Subs/FILM/Snatch-it.srt|Test_Subs/FILM/Snatch.mp4"
  "Trainspotting|Test_Subs/FILM/Trainspotting-en.srt|Test_Subs/FILM/Trainspotting-it.srt|Test_Subs/FILM/Trainspotting.mp4"
  "Uncut Gems|Test_Subs/FILM/Uncut Gems-en.srt|Test_Subs/FILM/Uncut Gems-it.srt|Test_Subs/FILM/Uncut Gems.mkv"
  "Zootopia|Test_Subs/FILM/Zootopia-en.srt|Test_Subs/FILM/Zootopia-it.srt|Test_Subs/FILM/Zootopia.mkv"
)

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
