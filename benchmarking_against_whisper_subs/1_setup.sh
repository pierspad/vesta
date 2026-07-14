#!/usr/bin/env bash
#
# 1_setup.sh — prepare both contenders and the shared model.
#
#   1. builds Vesta's `srt-transcribe` CLI (release) — the exact same
#      whisper-common pipeline the GUI uses;
#   2. installs the `whisper-subs` crate (pinned) into .bin/;
#   3. downloads the shared ggml model + the Silero VAD model whisper-subs
#      auto-detects next to it.

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"
source "$(dirname "${BASH_SOURCE[0]}")/../benchmarking_against_subs2srs/lib/common.sh"
cd "$REPO_ROOT"

need cargo
need ffmpeg
need curl

[ -f "$BENCH_INPUT" ] || die "Film non trovato: $BENCH_INPUT — esegui Test_Subs/FILM/download_detour.sh"

log "1/4 Building Vesta srt-transcribe (release)…"
cargo build --release -p srt-transcribe-cli
[ -x "$VESTA_TRANSCRIBE_BIN" ] || die "Binario mancante: $VESTA_TRANSCRIBE_BIN"
ok "$VESTA_TRANSCRIBE_BIN"

log "2/4 Installing whisper-subs v${WHISPER_SUBS_VERSION} (crates.io, pinned)…"
# Plain CPU build: whisper-common è anch'esso plain CPU (whisper-rs senza
# feature GPU/BLAS), quindi il confronto è ad armi pari.
if [ ! -x "$WHISPER_SUBS_BIN" ]; then
  cargo install --locked whisper-subs --version "$WHISPER_SUBS_VERSION" \
    --root "benchmarking_against_whisper_subs/.bin"
fi
[ -x "$WHISPER_SUBS_BIN" ] || die "Binario mancante: $WHISPER_SUBS_BIN"
ok "$WHISPER_SUBS_BIN"

log "3/4 Whisper model '${WHISPER_BENCH_MODEL}' (condiviso da entrambi)…"
mkdir -p "$MODELS_DIR"
if [ ! -f "$MODEL_FILE" ]; then
  "$VESTA_TRANSCRIBE_BIN" download "$WHISPER_BENCH_MODEL"
fi
[ -f "$MODEL_FILE" ] || die "Modello non trovato: $MODEL_FILE"
ok "$MODEL_FILE ($(du -h "$MODEL_FILE" | cut -f1))"

log "4/4 Silero VAD model (usato solo da whisper-subs)…"
if [ ! -f "$VAD_FILE" ]; then
  curl -L --fail --retry 3 -o "$VAD_FILE" "$VAD_URL"
fi
ok "$VAD_FILE ($(du -h "$VAD_FILE" | cut -f1))"

echo
ok "Setup completo. Ora: ./benchmarking_against_whisper_subs/2_run_benchmark.sh"
