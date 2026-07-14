#!/usr/bin/env bash
#
# 2_run_benchmark.sh — time both tools on the same film, then score the
# transcripts against the human reference and write results/summary.md.
#
# Variants (stesso modello ggml per tutti):
#   vesta              — srt-transcribe (whisper-common): greedy, threads auto
#   whisper-subs-fast  — decoder equivalente a Vesta (beam 1, no re-decode)
#   whisper-subs-best  — default qualità del crate (Silero VAD + beam 5)
#
# Per ogni variante misura: wall time, CPU user+sys, %CPU, picco RSS (via
# /usr/bin/time -v) e dimensione del binario; poi report/score.py calcola
# WER e metriche di timing rispetto a Detour-en.srt.

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"
source "$(dirname "${BASH_SOURCE[0]}")/../benchmarking_against_subs2srs/lib/common.sh"
cd "$REPO_ROOT"

need ffmpeg
[ -x "$VESTA_TRANSCRIBE_BIN" ] || die "Manca $VESTA_TRANSCRIBE_BIN — esegui 1_setup.sh"
[ -x "$WHISPER_SUBS_BIN" ]     || die "Manca $WHISPER_SUBS_BIN — esegui 1_setup.sh"
[ -f "$MODEL_FILE" ]           || die "Manca $MODEL_FILE — esegui 1_setup.sh"
[ -f "$BENCH_INPUT" ]          || die "Manca $BENCH_INPUT — esegui Test_Subs/FILM/download_detour.sh"
TIME_BIN="$(command -v /usr/bin/time || true)"
[ -n "$TIME_BIN" ] || die "/usr/bin/time non trovato (pacchetto 'time')"

mkdir -p "$RESULTS_DIR/raw"
TIMING_CSV="$RESULTS_DIR/timings.csv"
echo "variant,repeat,wall_s,user_s,sys_s,cpu_pct,max_rss_kb,binary_bytes,model_bytes" > "$TIMING_CSV"

# run_variant <name> <output_srt> <binary> <cmd...>
run_variant() {
  local name="$1" out_srt="$2" binary="$3"; shift 3
  local bin_size model_size
  bin_size=$(stat -c%s "$binary")
  model_size=$(stat -c%s "$MODEL_FILE")

  for rep in $(seq 1 "$REPEATS"); do
    log "[$name] run $rep/$REPEATS…"
    rm -f "$out_srt"
    local tlog="$RESULTS_DIR/raw/${name}.time.$rep"
    "$TIME_BIN" -v -o "$tlog" "$@" || die "[$name] esecuzione fallita (vedi $tlog)"
    # whisper-subs marca gli output con loop rilevato come "<stem>-truncated.srt":
    # per il benchmark lo accettiamo (lo score rifletterà il testo perso).
    if [ ! -f "$out_srt" ] && [ -f "${out_srt%.srt}-truncated.srt" ]; then
      warn "[$name] output marcato -truncated dal loop detector"
      mv "${out_srt%.srt}-truncated.srt" "$out_srt"
    fi
    [ -f "$out_srt" ] || die "[$name] nessun SRT prodotto in $out_srt"

    local wall user sys cpu rss
    wall=$(awk -F': ' '/Elapsed \(wall clock\)/{print $2}' "$tlog" \
           | awk -F: '{ if (NF==3) print $1*3600+$2*60+$3; else print $1*60+$2 }')
    user=$(awk -F': ' '/User time/{print $2}' "$tlog")
    sys=$(awk -F': '  '/System time/{print $2}' "$tlog")
    cpu=$(awk -F': '  '/Percent of CPU/{gsub(/%/,"",$2); print $2}' "$tlog")
    rss=$(awk -F': '  '/Maximum resident set size/{print $2}' "$tlog")
    echo "$name,$rep,$wall,$user,$sys,$cpu,$rss,$bin_size,$model_size" >> "$TIMING_CSV"
    ok "[$name] wall=${wall}s cpu=${cpu}% rss=$((rss/1024))MiB"
  done
}

# ── vesta (whisper-common, la stessa pipeline della GUI) ─────────────────────
run_variant "vesta" "$RESULTS_DIR/raw/vesta.srt" "$VESTA_TRANSCRIBE_BIN" \
  "$VESTA_TRANSCRIBE_BIN" run "$BENCH_INPUT" \
    --output "$RESULTS_DIR/raw/vesta.srt" \
    --model "$WHISPER_BENCH_MODEL" --language "$BENCH_LANGUAGE" --quiet

# ── whisper-subs, decoder pari a Vesta (greedy, niente VAD) ──────────────────
run_variant "whisper-subs-fast" "$RESULTS_DIR/raw/whisper-subs-fast.srt" "$WHISPER_SUBS_BIN" \
  "$WHISPER_SUBS_BIN" "$BENCH_INPUT" \
    --model "$MODEL_FILE" --language "$BENCH_LANGUAGE" --format srt \
    --output "$RESULTS_DIR/raw/whisper-subs-fast.srt" \
    --threads "$WHISPER_SUBS_THREADS" \
    --no-vad --beam-size 1 --logprob-threshold -1.0

# ── whisper-subs, default qualità (Silero VAD + beam 5) ──────────────────────
run_variant "whisper-subs-best" "$RESULTS_DIR/raw/whisper-subs-best.srt" "$WHISPER_SUBS_BIN" \
  "$WHISPER_SUBS_BIN" "$BENCH_INPUT" \
    --model "$MODEL_FILE" --language "$BENCH_LANGUAGE" --format srt \
    --output "$RESULTS_DIR/raw/whisper-subs-best.srt" \
    --threads "$WHISPER_SUBS_THREADS"

# ── Scoring ───────────────────────────────────────────────────────────────────
log "Scoring vs $BENCH_REFERENCE…"
PY="$(command -v python3 || command -v python)"
"$PY" benchmarking_against_whisper_subs/report/score.py \
  --reference "$BENCH_REFERENCE" \
  --timings "$TIMING_CSV" \
  --out-csv "$RESULTS_CSV" \
  --out-md "$SUMMARY_MD" \
  "vesta=$RESULTS_DIR/raw/vesta.srt" \
  "whisper-subs-fast=$RESULTS_DIR/raw/whisper-subs-fast.srt" \
  "whisper-subs-best=$RESULTS_DIR/raw/whisper-subs-best.srt"

echo
ok "Risultati: $RESULTS_CSV"
ok "Report:    $SUMMARY_MD"
