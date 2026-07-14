#!/usr/bin/env bash
#
# Benchmark configuration — Vesta (whisper-common) vs crate `whisper-subs`.
# Sourced by every numbered script. All paths are relative to the repo root.

# ── Test media ────────────────────────────────────────────────────────────────
# Audio source and reference subtitles (Detour, 1945 — public domain).
# The reference is a human-made OpenSubtitles track: not a verbatim
# transcription, so absolute WER is a proxy — but it is the SAME reference
# for every variant, so the comparison between tools is fair.
BENCH_INPUT="Test_Subs/FILM/Detour(1945).mp4"
BENCH_REFERENCE="Test_Subs/FILM/Detour-en.srt"
BENCH_LANGUAGE="en"

# ── Model ─────────────────────────────────────────────────────────────────────
# Same ggml weights for both tools (~/.cache/whisper/ggml-<id>.bin — the dir
# whisper-common already uses). Override: WHISPER_BENCH_MODEL=medium ./2_run…
WHISPER_BENCH_MODEL="${WHISPER_BENCH_MODEL:-small}"
MODELS_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/whisper"
MODEL_FILE="$MODELS_DIR/ggml-${WHISPER_BENCH_MODEL}.bin"

# Silero VAD (auto-detected by whisper-subs when placed next to the model).
VAD_FILE="$MODELS_DIR/ggml-silero-v5.1.2.bin"
VAD_URL="https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v5.1.2.bin"

# ── Contenders ────────────────────────────────────────────────────────────────
# whisper-subs is installed pinned into .bin/ by 1_setup.sh.
WHISPER_SUBS_VERSION="0.1.1"
WHISPER_SUBS_BIN="benchmarking_against_whisper_subs/.bin/bin/whisper-subs"
VESTA_TRANSCRIBE_BIN="target/release/srt-transcribe"

# Threads: 0 = auto for whisper-subs; whisper-common picks cores-1 by itself.
WHISPER_SUBS_THREADS="${WHISPER_SUBS_THREADS:-0}"

# ── Layout ────────────────────────────────────────────────────────────────────
RESULTS_DIR="benchmarking_against_whisper_subs/results"
RESULTS_CSV="$RESULTS_DIR/results.csv"
SUMMARY_MD="$RESULTS_DIR/summary.md"

# ── Repeats ───────────────────────────────────────────────────────────────────
# Transcribing a 69-minute film on CPU is slow; default to a single pass.
REPEATS="${REPEATS:-1}"
