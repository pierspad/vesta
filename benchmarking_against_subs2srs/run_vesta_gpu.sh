#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

source benchmarking_against_subs2srs/config.sh

C_BOLD="\033[1m"; C_GREEN="\033[32m"; C_YELLOW="\033[33m"; C_NC="\033[0m"
log()  { echo -e "  ${C_YELLOW}▶${C_NC} $*"; }
ok()   { echo -e "  ${C_GREEN}✔${C_NC} $*"; }
warn() { echo -e "  ${C_YELLOW}⚠${C_NC} $*" >&2; }

vbin="target/release/srt-flashcards"
if [ ! -x "$vbin" ]; then
    echo "Building release binary..."
    cargo build --release -p srt-flashcards-cli
fi

RESULTS_GPU_CSV="benchmarking_against_subs2srs/results/results_gpu.csv"
mkdir -p "$(dirname "$RESULTS_GPU_CSV")"

# Initialize results_gpu.csv with baseline subs2srs and vesta max rows from results.csv
echo "title,subtitle_count,tool,variant,format,elapsed_ms,lines,audio,snapshots,video,jobs" > "$RESULTS_GPU_CSV"
if [ -f "$RESULTS_CSV" ]; then
    # Keep baseline subs2srs
    grep ",subs2srs,subs2srs," "$RESULTS_CSV" >> "$RESULTS_GPU_CSV" || true
    # Keep baseline vesta CPU (max)
    grep ",vesta,max,tsv," "$RESULTS_CSV" >> "$RESULTS_GPU_CSV" || true
fi

jobs="${vesta_JOBS:-16}"
echo -e "${C_BOLD}Benchmarking Vesta with GPU / Auto Pre-Transcoding (${jobs} workers)${C_NC}"
echo

count_media() {
  local dir="$1" ext="$2"
  [ -d "$dir" ] || { echo 0; return; }
  find "$dir" -maxdepth 1 -type f -name "*.$ext" 2>/dev/null | wc -l
}

srt_count() {
  local f="$1"
  grep -c '^[0-9]\+$' "$f" 2>/dev/null || wc -l < "$f"
}

for media in "${TEST_MEDIA[@]}"; do
    IFS='|' read -r name target native video <<< "$media"
    [ -f "$target" ] || { warn "skip $name: target subs not found ($target)"; continue; }
    [ -f "$video" ]  || { warn "skip $name: video not found ($video)"; continue; }
    subcount=$(srt_count "$target")
    echo -e "${C_BOLD}▌ $name${C_NC}  (${subcount} subtitles)"

    out="$WORK_DIR/vesta_gpu_tsv_${name}"
    rm -rf "$out"
    mkdir -p "$out"

    log "vesta:gpu (tsv, ${jobs} worker(s))…"
    t_start=$(date +%s%3N)
    "$vbin" generate --target "$target" ${native:+--native "$native"} \
        --video "$video" --output "$out" --format "tsv" --deck "Bench" \
        -j "$jobs" --quiet
    t_end=$(date +%s%3N)
    ms=$(( t_end - t_start ))

    md="$out/Bench.media"
    a=$(count_media "$md" mp3)
    s=$(( $(count_media "$md" jpg) + $(count_media "$md" webp) ))
    v=$(( $(count_media "$md" mp4) + $(count_media "$md" avi) ))

    echo "$name,$subcount,vesta,gpu,tsv,$ms,$subcount,$a,$s,$v,$jobs" >> "$RESULTS_GPU_CSV"
    ok "vesta:gpu/tsv: ${ms} ms (audio=$a snap=$s video=$v)"
    echo
done

ok "Wrote $RESULTS_GPU_CSV"
