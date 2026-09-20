#!/usr/bin/env bash
source "$(dirname "${BASH_SOURCE[0]}")/config.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
cd "$REPO_ROOT"

missing=(
  "In.Bruges|Test_Subs/FILM/In.Bruges-en.srt|Test_Subs/FILM/In.Bruges-it.srt|Test_Subs/FILM/In.Bruges.mkv"
  "Snatch|Test_Subs/FILM/Snatch-en.srt|Test_Subs/FILM/Snatch-it.srt|Test_Subs/FILM/Snatch.mp4"
  "Uncut Gems|Test_Subs/FILM/Uncut Gems-en.srt|Test_Subs/FILM/Uncut Gems-it.srt|Test_Subs/FILM/Uncut Gems.mkv"
  "Zootopia|Test_Subs/FILM/Zootopia-en.srt|Test_Subs/FILM/Zootopia-it.srt|Test_Subs/FILM/Zootopia.mkv"
)

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

count_media() {
  [ -d "$1" ] || { echo 0; return 0; }
  find "$1" -type f -name "*.$2" 2>/dev/null | wc -l | tr -d ' '
}

for media in "${missing[@]}"; do
  IFS='|' read -r name target native video <<< "$media"
  subcount=$(srt_count "$target")
  log "Running subs2srs on $name ($subcount subtitles)..."
  out="$WORK_DIR/subs2srs_${name}"; rm -rf "$out"; mkdir -p "$out"
  if ms=$(timed_median mono "$SUBS2SRS_EXE" --target "$target" ${native:+--native "$native"} \
            --video "$video" --output "$out" --deck "Bench"); then
    md="$out/Bench.media"
    a=$(count_media "$md" mp3); s=$(count_media "$md" jpg)
    v=$(( $(count_media "$md" avi) + $(count_media "$md" mp4) ))
    echo "$name,$subcount,subs2srs,subs2srs,tsv,$ms,$subcount,$a,$s,$v,1" >> "$RESULTS_CSV"
    ok "subs2srs for $name: ${ms} ms (audio=$a snap=$s video=$v)"
    log "Regenerating report..."
    ./benchmarking_against_subs2srs/4_generate_report.sh || true
  fi
done
