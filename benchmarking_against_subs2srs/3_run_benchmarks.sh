#!/usr/bin/env bash
#
# 3_run_benchmarks.sh — measure subs2srs vs Vesta (and Vesta variants).
#
# For every test medium it runs:
#   * subs2srs-headless          (TSV — subs2srs has no .apkg export)
#   * each Vesta variant × each vesta_FORMATS (tsv, apkg)
# timing the full wall-clock of each invocation. Results are appended to a
# long-format CSV that 4_generate_report.sh turns into a chart.
#
#   Vesta runs on its max plan (cores-1 parallel ffmpeg workers).
#   subs2srs runs exactly as written (single-threaded, sequential).
#
# Env:  REPEATS=3 ./3_run_benchmarks.sh   # median of 3 runs per cell

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
cd "$REPO_ROOT"

need mono; need ffmpeg
[ -f "$SUBS2SRS_EXE" ] || die "subs2srs harness missing — run ./benchmarking_against_subs2srs/1_compile_subs2srs.sh"

mkdir -p "$RESULTS_DIR" "$WORK_DIR"
echo "title,subtitle_count,tool,variant,format,elapsed_ms,lines,audio,snapshots,video,jobs" > "$RESULTS_CSV"

# Run a command N times, echo the median elapsed wall-clock in ms (stderr shows each).
timed_median() {
  local -a samples=()
  local i start end
  for (( i = 1; i <= REPEATS; i++ )); do
    start=$(now_ms)
    if ! "$@" >/dev/null 2>"$WORK_DIR/last_stderr.log"; then
      err "  run failed (see $WORK_DIR/last_stderr.log)"; tail -3 "$WORK_DIR/last_stderr.log" >&2; return 1
    fi
    end=$(now_ms)
    samples+=( $(( end - start )) )
    echo "    repeat $i: $(( end - start )) ms" >&2
  done
  median "${samples[@]}"
}

count_media() {  # <media_dir> <ext>  (apkg embeds media, so the dir may not exist)
  [ -d "$1" ] || { echo 0; return 0; }
  find "$1" -type f -name "*.$2" 2>/dev/null | wc -l | tr -d ' '
}

echo -e "${C_BOLD}Benchmark plan:${C_NC} ${#TEST_MEDIA[@]} medium/media, Vesta jobs=${vesta_JOBS} (of ${CORES} cores), repeats=${REPEATS}"
echo

for media in "${TEST_MEDIA[@]}"; do
  IFS='|' read -r name target native video <<< "$media"
  [ -f "$target" ] || { warn "skip $name: target subs not found ($target)"; continue; }
  [ -f "$video" ]  || { warn "skip $name: video not found ($video)"; continue; }
  subcount=$(srt_count "$target")
  echo -e "${C_BOLD}▌ $name${C_NC}  (${subcount} subtitles)"

  # ── subs2srs (TSV) ──────────────────────────────────────────────────────────
  out="$WORK_DIR/subs2srs_${name}"; rm -rf "$out"; mkdir -p "$out"
  log "subs2srs (single-threaded)…"
  if ms=$(timed_median mono "$SUBS2SRS_EXE" --target "$target" ${native:+--native "$native"} \
            --video "$video" --output "$out" --deck "Bench"); then
    md="$out/Bench.media"
    a=$(count_media "$md" mp3); s=$(count_media "$md" jpg)
    v=$(( $(count_media "$md" avi) + $(count_media "$md" mp4) ))
    echo "$name,$subcount,subs2srs,subs2srs,tsv,$ms,$subcount,$a,$s,$v,1" >> "$RESULTS_CSV"
    ok "subs2srs: ${ms} ms (audio=$a snap=$s video=$v)"
  fi

  # ── Vesta variants × formats ────────────────────────────────────────────────
  for variant in "${vesta_VARIANTS[@]}"; do
    IFS='|' read -r vlabel vbin vjobs <<< "$variant"
    if [ ! -x "$vbin" ]; then warn "skip vesta:$vlabel — binary not found ($vbin)"; continue; fi
    # Resolve the worker count: MAX (or empty) -> the "max" plan (cores-1).
    if [ -z "$vjobs" ] || [ "$vjobs" = "MAX" ]; then jobs="$vesta_JOBS"; else jobs="$vjobs"; fi
    for fmt in "${vesta_FORMATS[@]}"; do
      out="$WORK_DIR/vesta_${vlabel}_${fmt}_${name}"; rm -rf "$out"; mkdir -p "$out"
      log "vesta:$vlabel ($fmt, ${jobs} worker(s))…"
      if ms=$(timed_median "$vbin" generate --target "$target" ${native:+--native "$native"} \
                --video "$video" --output "$out" --format "$fmt" --deck "Bench" \
                -j "$jobs" --quiet); then
        md="$out/Bench.media"
        a=$(count_media "$md" mp3); s=$(count_media "$md" jpg)
        v=$(( $(count_media "$md" mp4) + $(count_media "$md" avi) ))
        echo "$name,$subcount,vesta,$vlabel,$fmt,$ms,$subcount,$a,$s,$v,$jobs" >> "$RESULTS_CSV"
        ok "vesta:$vlabel/$fmt: ${ms} ms (audio=$a snap=$s video=$v)"
      fi
    done
  done
  echo
done

ok "Wrote $RESULTS_CSV"
echo "Next: ./benchmarking_against_subs2srs/4_generate_report.sh"
