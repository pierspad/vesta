#!/usr/bin/env bash
#
# 2_compile_vesta.sh — build the headless Vesta flashcard CLI (release).
#
# Produces target/release/srt-flashcards, the "main" variant the benchmark
# compares against subs2srs. To benchmark another Vesta build, compile it
# separately and add it to vesta_VARIANTS in config.sh.

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
cd "$REPO_ROOT"

need cargo
need ffmpeg

log "Building srt-flashcards-cli in release mode (LTO, opt-level 3)…"
cargo build --release -p srt-flashcards-cli

BIN="target/release/srt-flashcards"
[ -x "$BIN" ] || die "Expected binary not found at $BIN"
ok "Built $BIN"
"$BIN" --version 2>/dev/null || true

echo
ok "Vesta CLI ready. Smoke test:"
echo "   $BIN info Test_Subs/FILM/Detour-en.srt"
