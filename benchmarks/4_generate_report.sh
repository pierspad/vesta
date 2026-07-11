#!/usr/bin/env bash
#
# 4_generate_report.sh — turn results.csv into charts + a markdown summary.
#
# Produces (all inside benchmarks/results/):
#   benchmark.svg        combined grouped-bar chart, log scale (SVG renders on GitHub)
#   films/<slug>.svg     one linear-scale chart per film
#   summary.md           detailed analysis: per-film speed-ups, throughput, totals

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
cd "$REPO_ROOT"

[ -f "$RESULTS_CSV" ] || die "No results — run ./benchmarks/3_run_benchmarks.sh first"

MD="$RESULTS_DIR/summary.md"

# ── Charts (matplotlib → SVG) ────────────────────────────────────────────────
# Prefer the repo's virtualenv (it ships matplotlib); fall back to system python3.
PY="python3"
[ -x ".venv/bin/python3" ] && PY=".venv/bin/python3"
if command -v "$PY" >/dev/null 2>&1 || [ -x "$PY" ]; then
  log "Rendering charts with $PY…"
  if "$PY" benchmarks/report/plot.py "$RESULTS_CSV" "$RESULTS_DIR"; then
    ok "Charts written under $RESULTS_DIR (benchmark.svg + films/*.svg)"
  else
    warn "Charts failed — is matplotlib installed?  ($PY -m pip install matplotlib)"
  fi
else
  warn "python3 not found — skipping charts."
fi

# ── Markdown summary with speed-ups and throughput (awk pivot) ───────────────
log "Writing markdown summary…"
{
  echo "# Vesta vs subs2srs — benchmark results"
  echo
  echo "_Generated $(date '+%Y-%m-%d %H:%M')_ · CPU cores: ${CORES} · Vesta max workers: ${VESTA_JOBS} · repeats: ${REPEATS}"
  echo
  echo "Wall-clock time of the full deck build (lower is better). Speed-up = subs2srs ÷ Vesta."
  echo "subs2srs runs exactly as written — single-threaded, sequential ffmpeg; Vesta is timed"
  echo "both on its \"max\" plan (cores−1 parallel ffmpeg workers) and with a single worker,"
  echo "the core-for-core control isolating per-card efficiency from the parallelism win."
  echo
  echo "![Combined benchmark](benchmark.svg)"
  echo
  echo "## Per-film results"
  echo
  echo "| Film | Subtitles | Series | Time | Cards/min | Speed-up vs subs2srs |"
  echo "|---|--:|---|--:|--:|--:|"
} > "$MD"

awk -F, 'NR>1 {
    title=$1; subc[title]=$2
    if ($3=="subs2srs") label="subs2srs"
    else { core=($11=="1")?" (1 core)":""; label="Vesta " toupper($5) core }
    ms[title SUBSEP label] = $6
    if ($3=="subs2srs") base[title]=$6
    if (!(label in labelset)) { labels[++ln_]=label; labelset[label]=1 }
    if (!(title in orderset)) { titles[++tn]=title; orderset[title]=1 }
}
END {
    # Sort labels to match the requested order: subs2srs, Vesta APKG (1 core), Vesta TSV (1 core), Vesta APKG, Vesta TSV
    rank["subs2srs"] = 1
    rank["Vesta APKG (1 core)"] = 2
    rank["Vesta TSV (1 core)"] = 3
    rank["Vesta APKG"] = 4
    rank["Vesta TSV"] = 5

    for (x=1; x<=ln_; x++) {
        for (y=x+1; y<=ln_; y++) {
            rx = rank[labels[x]] ? rank[labels[x]] : 99
            ry = rank[labels[y]] ? rank[labels[y]] : 99
            if (rx > ry) {
                tmp = labels[x]
                labels[x] = labels[y]
                labels[y] = tmp
            }
        }
    }

    for (i=1;i<=tn;i++) {
        t=titles[i]
        for (j=1;j<=ln_;j++) {
            lbl=labels[j]; v=ms[t SUBSEP lbl]
            if (v=="") continue
            b=base[t]
            cpm=(v>0)? sprintf("%.0f", subc[t]/(v/60000.0)) : "—"
            sp=(lbl!="subs2srs" && b!="" && v>0) ? sprintf("**%.2f×**", b/v) : "—"
            tdisp=t; gsub(/\./," ",tdisp)
            printf("| %s | %s | %s | %.1f s | %s | %s |\n", tdisp, subc[t], lbl, v/1000.0, cpm, sp)
        }
        # accumulate totals (per label, and the matching subs2srs base over the
        # same films only, so partial series get a fair overall speed-up)
        for (j=1;j<=ln_;j++) {
            lbl=labels[j]
            if (ms[t SUBSEP lbl]=="") continue
            tot[lbl]+=ms[t SUBSEP lbl]
            if (base[t]!="") tot_base[lbl]+=base[t]
        }
    }

    print ""
    print "## Aggregate (all films)"
    print ""
    print "| Series | Total time | Overall speed-up |"
    print "|---|--:|--:|"
    for (j=1;j<=ln_;j++) {
        lbl=labels[j]
        if (tot[lbl]=="") continue
        sp=(lbl!="subs2srs" && tot_base[lbl]>0 && tot[lbl]>0)? sprintf("**%.2f×**", tot_base[lbl]/tot[lbl]) : "—"
        printf("| %s | %.1f min | %s |\n", lbl, tot[lbl]/60000.0, sp)
    }

    print ""
    print "## Reading the numbers"
    print ""
    print "- **Vesta (1 core) vs subs2srs** measures raw per-card efficiency: same"
    print "  single-ffmpeg-at-a-time pipeline, so any gain is architectural (batched"
    print "  extraction, no per-card process re-spawn overhead), not parallelism."
    print "- **Vesta (max) vs Vesta (1 core)** isolates the parallelism win of the"
    print "  cores−1 worker pool."
    print "- **subs2srs cannot be run multi-core as written**: it is a single-threaded"
    print "  .NET WinForms pipeline that invokes one ffmpeg process at a time,"
    print "  sequentially, per card. Parallelizing it would require modifying its"
    print "  source, which this benchmark deliberately never does."
    print ""
    print "## Per-film charts"
    print ""
    for (i=1;i<=tn;i++) {
        t=titles[i]
        slug=tolower(t); gsub(/[^A-Za-z0-9]+/,"-",slug); gsub(/^-|-$/,"",slug)
        tdisp=t; gsub(/\./," ",tdisp)
        printf("### %s\n\n![%s](films/%s.svg)\n\n", tdisp, tdisp, slug)
    }
}' "$RESULTS_CSV" >> "$MD"

ok "Summary: $MD"
echo
ok "Done. Files under $RESULTS_DIR:"
ls -1 "$RESULTS_DIR" "$RESULTS_DIR/films" 2>/dev/null | sed 's/^/   /'
