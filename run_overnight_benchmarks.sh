#!/usr/bin/env bash
#
# run_overnight_benchmarks.sh — Complete Overnight Benchmark Suite: Vesta vs subs2srs
#
# Tests 8 films × 9 test configurations:
#   1. subs2srs                     (1 worker, TSV, Direct / No Transcoding)
#   2. Vesta 1 core Direct (TSV)    (1 worker, TSV, Direct / No Transcoding)
#   3. Vesta 1 core Direct (APKG)   (1 worker, APKG, Direct / No Transcoding)
#   4. Vesta 1 core GPU (TSV)       (1 worker, TSV, GPU Pre-Transcoding)
#   5. Vesta 1 core GPU (APKG)      (1 worker, APKG, GPU Pre-Transcoding)
#   6. Vesta Multi Direct (TSV)     (16 workers, TSV, Direct / No Transcoding)
#   7. Vesta Multi Direct (APKG)    (16 workers, APKG, Direct / No Transcoding)
#   8. Vesta Multi GPU (TSV)        (16 workers, TSV, GPU Pre-Transcoding)
#   9. Vesta Multi GPU (APKG)       (16 workers, APKG, GPU Pre-Transcoding)
#
# Usage:
#   ./run_overnight_benchmarks.sh           # Defaults to 'all' after 5s countdown
#   ./run_overnight_benchmarks.sh all       # Full clean run from scratch (~5.5h)
#   ./run_overnight_benchmarks.sh missing   # Run only missing variants (~1.2h)
#

# --- 0. Prevent system sleep/suspend during overnight benchmarks ---
if [ -z "${IN_SYSTEMD_INHIBIT:-}" ] && command -v systemd-inhibit >/dev/null 2>&1; then
  export IN_SYSTEMD_INHIBIT=1
  echo "🛡️  Acquiring systemd sleep inhibitor lock (preventing idle suspend)..."
  exec systemd-inhibit --what=idle:sleep:shutdown --who="VestaBenchmark" --why="Running overnight benchmarks" "$0" "$@"
fi

set -euo pipefail

C_BOLD="\033[1m"; C_GREEN="\033[32m"; C_YELLOW="\033[33m"; C_BLUE="\033[34m"; C_CYAN="\033[36m"; C_RED="\033[31m"; C_NC="\033[0m"
log()   { echo -e "  ${C_BLUE}▶${C_NC} $*"; }
ok()    { echo -e "  ${C_GREEN}✔${C_NC} $*"; }
warn()  { echo -e "  ${C_YELLOW}⚠${C_NC} $*" >&2; }
err()   { echo -e "  ${C_RED}✖${C_NC} $*" >&2; }
title() { echo -e "\n${C_BOLD}${C_CYAN}═══════════════════════════════════════════════════════════════${C_NC}"; echo -e "${C_BOLD}  $*${C_NC}"; echo -e "${C_BOLD}${C_CYAN}═══════════════════════════════════════════════════════════════${C_NC}\n"; }

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

source benchmarking_against_subs2srs/config.sh

# Python for reports
PY="python3"
[ -x ".venv/bin/python3" ] && PY=".venv/bin/python3"

RESULTS_FULL_CSV="benchmarking_against_subs2srs/results/results_full.csv"
WORK_DIR="benchmarking_against_subs2srs/.work"
mkdir -p "$(dirname "$RESULTS_FULL_CSV")" "$WORK_DIR"

title "VESTA vs SUBS2SRS — OVERNIGHT BENCHMARK SUITE"

echo -e "${C_BOLD}Hardware Configuration:${C_NC}"
echo "  • Cores / Threads: $(nproc) (Vesta Multi-Core will use: ${vesta_JOBS} workers)"
echo "  • GPU Acceleration: VA-API (/dev/dri/renderD128)"
echo "  • Media Discovered: ${#TEST_MEDIA[@]} films in $BENCH_MEDIA_DIR"
echo

# Select Mode
MODE="${1:-}"
if [ -z "$MODE" ]; then
  echo -e "${C_BOLD}Select benchmark execution mode:${C_NC}"
  echo -e "  ${C_GREEN}[1] all${C_NC}     - Full suite from scratch (all 8 films × 9 variants, ~5.5 hours)"
  echo -e "  ${C_YELLOW}[2] missing${C_NC} - Run ONLY missing tests, keeping existing data (~1.2 hours)"
  echo
  echo -ne "${C_BOLD}Starting in mode 'all' in 5 seconds (press '2' for missing, or Enter for all): ${C_NC}"
  if read -t 5 -r user_choice; then
    case "$user_choice" in
      2|missing|m) MODE="missing" ;;
      *)           MODE="all" ;;
    esac
  else
    echo
    MODE="all"
  fi
fi

case "$MODE" in
  missing|m)
    MODE="missing"
    echo -e "\n${C_BOLD}Mode: ${C_YELLOW}RUN ONLY MISSING TESTS${C_NC}\n"
    ;;
  *)
    MODE="all"
    echo -e "\n${C_BOLD}Mode: ${C_GREEN}FULL OVERNIGHT RUN (FROM SCRATCH)${C_NC}\n"
    ;;
esac

# Compilation Check
VBETA="target/release/srt-flashcards"
echo -e "${C_BOLD}Checking binaries...${C_NC}"
cargo build --release -p srt-flashcards-cli
[ -x "$VBETA" ] || { err "Failed to build $VBETA"; exit 1; }
ok "Vesta binary: $VBETA ready"

if [ ! -f "$SUBS2SRS_EXE" ]; then
  log "Building subs2srs-headless harness..."
  ./benchmarking_against_subs2srs/1_compile_subs2srs.sh
fi
[ -f "$SUBS2SRS_EXE" ] || { err "subs2srs harness missing ($SUBS2SRS_EXE)"; exit 1; }
ok "subs2srs harness: $SUBS2SRS_EXE ready"

# Initialize CSV according to mode
CSV_HEADER="title,subtitle_count,tool,variant,format,pipeline,elapsed_ms,lines,audio,snapshots,video,jobs"

declare -A COMPLETED_CELLS=()

if [ "$MODE" = "all" ]; then
  if [ -f "$RESULTS_FULL_CSV" ]; then
    bak="$RESULTS_FULL_CSV.bak.$(date +%Y%m%d_%H%M%S)"
    cp "$RESULTS_FULL_CSV" "$bak"
    log "Backed up existing results to $bak"
  fi
  echo "$CSV_HEADER" > "$RESULTS_FULL_CSV"
  ok "Initialized clean $RESULTS_FULL_CSV"
else
  # Mode == missing
  if [ ! -f "$RESULTS_FULL_CSV" ]; then
    # Populate from results.csv and results_gpu.csv if present
    log "Populating $RESULTS_FULL_CSV from existing benchmarks..."
    echo "$CSV_HEADER" > "$RESULTS_FULL_CSV"
    if [ -f "benchmarking_against_subs2srs/results/results.csv" ]; then
      awk -F, 'NR>1 {
        title=$1; subc=$2; tool=$3; var=$4; fmt=$5; ms=$6; lines=$7; a=$8; s=$9; v=$10; jobs=$11;
        if (tool=="subs2srs") { var="subs2srs"; pipe="direct" }
        else if (var=="single") { var="single_direct"; pipe="direct" }
        else if (var=="max") { var="multi_direct"; pipe="direct" }
        else { pipe="direct" }
        print title","subc","tool","var","fmt","pipe","ms","lines","a","s","v","jobs"
      }' "benchmarking_against_subs2srs/results/results.csv" >> "$RESULTS_FULL_CSV"
    fi
    if [ -f "benchmarking_against_subs2srs/results/results_gpu.csv" ]; then
      awk -F, 'NR>1 {
        title=$1; subc=$2; tool=$3; var=$4; fmt=$5; ms=$6; lines=$7; a=$8; s=$9; v=$10; jobs=$11;
        if (var=="gpu") {
          print title","subc","tool","multi_gpu","fmt","gpu","ms","lines","a","s","v","jobs"
        }
      }' "benchmarking_against_subs2srs/results/results_gpu.csv" >> "$RESULTS_FULL_CSV"
    fi
  fi

  # Read already completed cells
  while IFS=',' read -r r_title r_subc r_tool r_var r_fmt r_pipe r_ms rest; do
    [ "$r_title" = "title" ] && continue
    COMPLETED_CELLS["$r_title|$r_var|$r_fmt"]="$r_ms"
  done < "$RESULTS_FULL_CSV"
  ok "Loaded ${#COMPLETED_CELLS[@]} existing test cells from $RESULTS_FULL_CSV"
fi

# Helpers
srt_count() {
  grep -cE '[0-9]{2}:[0-9]{2}:[0-9]{2}[,.][0-9]{3}[[:space:]]*-->' "$1" 2>/dev/null || echo 0
}

count_media_files() {
  local dir="$1"
  [ -d "$dir" ] || { echo "0 0 0"; return; }
  local a s v
  a=$(find "$dir" -maxdepth 1 -type f \( -name "*.mp3" -o -name "*.opus" \) 2>/dev/null | wc -l)
  s=$(find "$dir" -maxdepth 1 -type f \( -name "*.jpg" -o -name "*.webp" -o -name "*.jpeg" \) 2>/dev/null | wc -l)
  v=$(find "$dir" -maxdepth 1 -type f \( -name "*.mp4" -o -name "*.avi" \) 2>/dev/null | wc -l)
  echo "$a $s $v"
}

# The 9 Test Configurations (tool | variant | format | pipeline | jobs | label)
VARIANTS_MATRIX=(
  "subs2srs|subs2srs|tsv|direct|1|subs2srs (TSV)"
  "vesta|single_direct|tsv|direct|1|Vesta 1c Direct (TSV)"
  "vesta|single_direct|apkg|direct|1|Vesta 1c Direct (APKG)"
  "vesta|single_gpu|tsv|gpu|1|Vesta 1c GPU (TSV)"
  "vesta|single_gpu|apkg|gpu|1|Vesta 1c GPU (APKG)"
  "vesta|multi_direct|tsv|direct|${vesta_JOBS}|Vesta Multi Direct (TSV)"
  "vesta|multi_direct|apkg|direct|${vesta_JOBS}|Vesta Multi Direct (APKG)"
  "vesta|multi_gpu|tsv|gpu|${vesta_JOBS}|Vesta Multi GPU (TSV)"
  "vesta|multi_gpu|apkg|gpu|${vesta_JOBS}|Vesta Multi GPU (APKG)"
)

total_films="${#TEST_MEDIA[@]}"
film_idx=0
suite_start=$(date +%s)

for media in "${TEST_MEDIA[@]}"; do
  film_idx=$(( film_idx + 1 ))
  IFS='|' read -r name target native video <<< "$media"
  [ -f "$target" ] || { warn "skip $name: target subs not found ($target)"; continue; }
  [ -f "$video" ]  || { warn "skip $name: video not found ($video)"; continue; }
  subcount=$(srt_count "$target")

  title "[$film_idx/$total_films] Film: $name ($subcount subtitles)"

  for spec in "${VARIANTS_MATRIX[@]}"; do
    IFS='|' read -r tool variant fmt pipeline jobs label <<< "$spec"

    # In missing mode, skip if already exists
    if [ "$MODE" = "missing" ] && [ -n "${COMPLETED_CELLS["$name|$variant|$fmt"]:-}" ]; then
      prev_ms="${COMPLETED_CELLS["$name|$variant|$fmt"]}"
      echo -e "  ⏩  ${C_CYAN}${label}${C_NC}: already present (${prev_ms} ms) — skipping"
      continue
    fi

    out="$WORK_DIR/bench_${variant}_${fmt}_${name}"
    rm -rf "$out"
    mkdir -p "$out"

    echo -ne "  ⏳  ${C_BOLD}${label}${C_NC} (workers=${jobs}) running... "

    t_start=$(date +%s%3N)
    cmd_status=0

    if [ "$tool" = "subs2srs" ]; then
      mono "$SUBS2SRS_EXE" --target "$target" ${native:+--native "$native"} \
           --video "$video" --output "$out" --deck "Bench" > "$WORK_DIR/last_run.log" 2>&1 || cmd_status=$?
    else
      # Vesta execution: check if direct (--no-optimize) or gpu (optimized)
      opt_flag=""
      [ "$pipeline" = "direct" ] && opt_flag="--no-optimize"

      "$VBETA" generate --target "$target" ${native:+--native "$native"} \
              --video "$video" --output "$out" --format "$fmt" --deck "Bench" \
              -j "$jobs" $opt_flag --quiet > "$WORK_DIR/last_run.log" 2>&1 || cmd_status=$?
    fi

    t_end=$(date +%s%3N)
    ms=$(( t_end - t_start ))
    sec=$(awk "BEGIN { printf \"%.1f\", $ms / 1000.0 }")

    if [ "$cmd_status" -ne 0 ]; then
      echo -e "\r  ${C_RED}✖  ${label}: FAILED${C_NC} (exit code $cmd_status, see $WORK_DIR/last_run.log)"
      continue
    fi

    md="$out/Bench.media"
    read -r a s v <<< "$(count_media_files "$md")"
    if [ "$fmt" = "apkg" ]; then
      # In APKG format media is packed into the .apkg file
      a="$subcount"; s="$subcount"; v="$subcount"
    fi

    # Append to CSV immediately
    echo "$name,$subcount,$tool,$variant,$fmt,$pipeline,$ms,$subcount,$a,$s,$v,$jobs" >> "$RESULTS_FULL_CSV"
    COMPLETED_CELLS["$name|$variant|$fmt"]="$ms"

    echo -e "\r  ${C_GREEN}✔  ${label}${C_NC}: ${C_BOLD}${sec}s${C_NC} (${ms} ms) [audio=$a snap=$s video=$v]"

    # Clean up output scratch folder to prevent disk filling
    rm -rf "$out"
  done

  # Progressive report update after each film
  echo
  log "Updating charts and report for completed films..."
  "$PY" benchmarking_against_subs2srs/report/generate_full_report.py "$RESULTS_FULL_CSV" benchmarking_against_subs2srs/results >/dev/null 2>&1 || true
  ok "Charts updated."
done

suite_end=$(date +%s)
suite_total_min=$(awk "BEGIN { printf \"%.1f\", ($suite_end - $suite_start) / 60.0 }")

title "BENCHMARK SUITE COMPLETE in ${suite_total_min} minutes!"

echo -e "${C_BOLD}Generating final charts and markdown report...${C_NC}"
"$PY" benchmarking_against_subs2srs/report/generate_full_report.py "$RESULTS_FULL_CSV" benchmarking_against_subs2srs/results

echo
ok "Results saved to:      $RESULTS_FULL_CSV"
ok "Overview Chart:       benchmarking_against_subs2srs/results/benchmark_overview.svg"
ok "Speedup Chart:        benchmarking_against_subs2srs/results/benchmark_speedup_summary.svg"
ok "Min/Avg/Max Chart:    benchmarking_against_subs2srs/results/benchmark_speedup_range.svg"
ok "Summary Report:       benchmarking_against_subs2srs/results/summary.md"
ok "Docs Synced:          docs/BENCHMARK_REPORT.md & docs/*.svg"
echo

# Terminal bell
echo -e "\a"
if command -v notify-send >/dev/null 2>&1; then
  notify-send "Vesta Benchmark Suite" "Benchmark finished successfully in ${suite_total_min} min!" || true
fi
