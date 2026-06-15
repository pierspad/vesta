#!/usr/bin/env bash
#
# 4_generate_report.sh — turn results.csv into a chart + a markdown summary.
#
# Produces:
#   benchmarks/results/benchmark.png      grouped bar chart (log scale)
#   benchmarks/results/summary.md         table with per-medium speed-ups

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
cd "$REPO_ROOT"

[ -f "$RESULTS_CSV" ] || die "No results — run ./benchmarks/3_run_benchmarks.sh first"

PNG="$RESULTS_DIR/benchmark.png"
MD="$RESULTS_DIR/summary.md"

# ── Chart (matplotlib) ───────────────────────────────────────────────────────
# Prefer the repo's virtualenv (it ships matplotlib); fall back to system python3.
PY="python3"
[ -x ".venv/bin/python3" ] && PY=".venv/bin/python3"
if command -v "$PY" >/dev/null 2>&1 || [ -x "$PY" ]; then
  log "Rendering chart with $PY…"
  if chart=$("$PY" benchmarks/report/plot.py "$RESULTS_CSV" "$PNG"); then
    ok "Chart: $chart"
  else
    warn "Chart failed — is matplotlib installed?  ($PY -m pip install matplotlib)"
  fi
else
  warn "python3 not found — skipping chart."
fi

# ── Markdown summary with speed-ups (awk pivot) ──────────────────────────────
log "Writing markdown summary…"
{
  echo "# Vesta vs subs2srs — benchmark results"
  echo
  echo "_Generated $(date '+%Y-%m-%d %H:%M')_ · CPU cores: ${CORES} · Vesta workers: ${VESTA_JOBS} · repeats: ${REPEATS}"
  echo
  echo "Wall-clock time per run (lower is better). Speed-up = subs2srs ÷ Vesta."
  echo
  echo "| Medium | Subtitles | subs2srs (TSV) | Vesta | Speed-up |"
  echo "|---|--:|--:|--:|--:|"
} > "$MD"

awk -F, 'NR>1 {
    key=$1; subc[$1]=$2
    if ($3=="subs2srs") label="subs2srs"
    else { core=($11=="1")?" (1 core)":""; label="Vesta export " toupper($5) core }
    ms[$1 SUBSEP label] = $6
    if ($3=="subs2srs") base[$1]=$6
    seen[$1]=1
    if (!(label in labelset) && $3!="subs2srs") { vlabels[++vn]=label; labelset[label]=1 }
    order[$1]=order[$1]?order[$1]:(++ocount)
}
END {
    for (t in seen) titles[order[t]]=t
    for (i=1;i<=ocount;i++) {
        t=titles[i]; if (t=="") continue
        for (j=1;j<=vn;j++) {
            lbl=vlabels[j]; v=ms[t SUBSEP lbl]
            if (v=="") continue
            b=base[t]
            sp=(b!="" && v>0) ? sprintf("%.2f×", b/v) : "—"
            bms=(b!="") ? sprintf("%.0f ms", b) : "—"
            printf("| %s | %s | %s | %s: %.0f ms | %s |\n", t, subc[t], bms, lbl, v, sp)
        }
    }
}' "$RESULTS_CSV" >> "$MD"

ok "Summary: $MD"
echo
cat "$MD"
