#!/usr/bin/env python3
"""
generate_full_report.py — Generate comprehensive benchmark charts and markdown reports
for Vesta vs subs2srs suite across all test variants.
"""

import csv
import re
import sys
import os
import shutil
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

# Ordered variants and palette
SERIES_ORDER = [
    ("subs2srs", "subs2srs", "tsv", "subs2srs (TSV)", "#e8770f"),
    ("vesta", "single_direct", "tsv", "Vesta 1c Direct (TSV)", "#f6b26b"),
    ("vesta", "single_direct", "apkg", "Vesta 1c Direct (APKG)", "#e69138"),
    ("vesta", "single_gpu", "tsv", "Vesta 1c GPU (TSV)", "#4ecdc4"),
    ("vesta", "single_gpu", "apkg", "Vesta 1c GPU (APKG)", "#2ab7ca"),
    ("vesta", "multi_direct", "tsv", "Vesta Multi Direct (TSV)", "#b3aadd"),
    ("vesta", "multi_direct", "apkg", "Vesta Multi Direct (APKG)", "#8073c9"),
    ("vesta", "multi_gpu", "tsv", "Vesta Multi GPU (TSV)", "#12a184"),
    ("vesta", "multi_gpu", "apkg", "Vesta Multi GPU (APKG)", "#0c7c65"),
]

VARIANT_LOOKUP = { (tool, var, fmt): (lbl, col) for tool, var, fmt, lbl, col in SERIES_ORDER }
BG_COLOR = "#f8f9fb"

def get_series_key(row):
    tool = row.get("tool", "")
    var = row.get("variant", "")
    fmt = row.get("format", "")
    # Backward compatibility mappings
    if tool == "subs2srs":
        return ("subs2srs", "subs2srs", "tsv")
    if var == "single":
        var = "single_direct"
    elif var == "max":
        var = "multi_direct"
    elif var == "gpu":
        var = "multi_gpu"
    return (tool, var, fmt)

def slugify(title):
    return re.sub(r"[^A-Za-z0-9]+", "-", title).strip("-").lower() or "film"

def pretty(title):
    return title.replace(".", " ").strip()

def style_axes(ax):
    ax.set_facecolor(BG_COLOR)
    for spine in ("top", "right"):
        ax.spines[spine].set_visible(False)
    ax.grid(axis="y", linestyle="--", alpha=0.35)
    ax.set_axisbelow(True)

def load_data(csv_path):
    with open(csv_path, newline="") as f:
        rows = list(csv.DictReader(f))
    if not rows:
        return [], {}, {}, {}

    media = []
    subcount = {}
    series_present = []
    seconds = {}

    for r in rows:
        title = r["title"]
        if title not in subcount:
            media.append(title)
        subcount[title] = int(r["subtitle_count"])

        s_key = get_series_key(r)
        if s_key not in VARIANT_LOOKUP:
            continue
        lbl, _ = VARIANT_LOOKUP[s_key]
        if lbl not in series_present:
            series_present.append(lbl)

        sec = float(r["elapsed_ms"]) / 1000.0
        seconds[(title, lbl)] = sec

    # Sort series according to SERIES_ORDER
    ordered_labels = [item[3] for item in SERIES_ORDER]
    series_present.sort(key=lambda s: ordered_labels.index(s) if s in ordered_labels else 99)
    # Sort media descending by subtitle count
    media.sort(key=lambda m: subcount[m], reverse=True)

    return media, subcount, series_present, seconds

def plot_overview(media, subcount, series, seconds, out_path):
    x = np.arange(len(media))
    n = len(series)
    width = min(0.85 / max(n, 1), 0.12)

    baseline_per_film = {}
    for m in media:
        b = seconds.get((m, "subs2srs (TSV)"))
        if not b:
            vals = [seconds.get((m, s), 0.0) for s in series if (m, s) in seconds]
            b = max(vals) if vals else 1.0
        baseline_per_film[m] = b

    fig_w = max(13, 2.5 * len(media) + 3)
    fig, ax = plt.subplots(figsize=(fig_w, 7.5))

    for i, label in enumerate(series):
        raw_vals = [seconds.get((m, label), np.nan) for m in media]
        norm_vals = [
            (v / baseline_per_film[m] * 100.0) if v == v else np.nan
            for m, v in zip(media, raw_vals)
        ]
        offset = (i - (n - 1) / 2) * width
        
        # Find color
        color = "#555555"
        for _, _, _, lbl, col in SERIES_ORDER:
            if lbl == label:
                color = col
                break

        bars = ax.bar(x + offset, norm_vals, width, label=label, color=color, zorder=3)
        texts = ["" if v != v else f"{v:,.1f}s" for v in raw_vals]
        ax.bar_label(bars, labels=texts, padding=3, fontsize=7, rotation=-45)

        # In-bar speedup annotations for Vesta variants
        if label != "subs2srs (TSV)":
            for xi, (m, v) in enumerate(zip(media, raw_vals)):
                base = seconds.get((m, "subs2srs (TSV)"))
                if base and v > 0 and (norm_vals[xi] > 18):
                    speedup = base / v
                    ax.text(xi + offset, norm_vals[xi] / 2, f"{speedup:.1f}×",
                            ha="center", va="center", fontsize=7.5,
                            fontweight="bold", color="white", rotation=90)

    style_axes(ax)
    ax.set_ylim(0, 125)
    ax.set_ylabel("Relative generation time (% of subs2srs baseline)", fontsize=10)
    ax.yaxis.set_major_formatter(matplotlib.ticker.PercentFormatter(100))
    ax.set_title("Vesta vs subs2srs — Flashcard Generation Suite Comparison (lower is better)",
                 fontweight="bold", pad=18, fontsize=13)
    ax.set_xticks(x, [f"{pretty(m)}\n({subcount[m]:,} subs)" for m in media], fontsize=9.5)
    ax.legend(frameon=False, ncol=min(n, 5), loc="upper center", bbox_to_anchor=(0.5, -0.12), fontsize=8.5)

    fig.text(0.5, -0.06,
             "* Normalized per film: 100% = subs2srs baseline. Values above bars show elapsed seconds; labels inside show speedup vs subs2srs.",
             ha="center", fontsize=8.5, color="#555555", style="italic")

    fig.tight_layout()
    fig.savefig(out_path, bbox_inches="tight")
    plt.close(fig)

def plot_speedup_summary(media, series, seconds, out_path):
    """Horizontal bar chart showing average speedup vs subs2srs for all Vesta variants."""
    vesta_series = [s for s in series if s != "subs2srs (TSV)"]
    if not vesta_series:
        return

    avg_speedups = []
    labels = []
    colors = []

    for s in vesta_series:
        speedups = []
        for m in media:
            base = seconds.get((m, "subs2srs (TSV)"))
            v = seconds.get((m, s))
            if base and v and v > 0:
                speedups.append(base / v)
        if speedups:
            avg_speedups.append(float(np.mean(speedups)))
            labels.append(s)
            color = "#555555"
            for _, _, _, lbl, col in SERIES_ORDER:
                if lbl == s:
                    color = col
                    break
            colors.append(color)

    y_pos = np.arange(len(labels))
    fig, ax = plt.subplots(figsize=(10, max(5, 0.7 * len(labels) + 2)))
    bars = ax.barh(y_pos, avg_speedups, align="center", color=colors, height=0.6, zorder=3)

    ax.set_yticks(y_pos, labels, fontsize=10)
    ax.invert_yaxis()  # Top-down order
    ax.set_xlabel("Average Speedup vs subs2srs (higher is better)", fontsize=10)
    ax.set_title("Vesta Average Speedup vs subs2srs Across All Test Films", fontweight="bold", pad=14, fontsize=12)

    # Reference line at 1.0x (parity with subs2srs)
    ax.axvline(1.0, color="#d9534f", linestyle="--", linewidth=1.5, alpha=0.8, zorder=2)
    ax.text(1.04, -0.3, "subs2srs parity (1.0×)", color="#d9534f", fontsize=9, fontweight="bold", va="bottom")

    # Bar labels
    for bar, val in zip(bars, avg_speedups):
        ax.text(val + 0.08, bar.get_y() + bar.get_height() / 2, f"{val:.2f}×",
                va="center", fontsize=10, fontweight="bold", color="#333333")

    ax.set_facecolor(BG_COLOR)
    for spine in ("top", "right"):
        ax.spines[spine].set_visible(False)
    ax.grid(axis="x", linestyle="--", alpha=0.35)
    ax.set_axisbelow(True)
    ax.set_ylim(len(labels) - 0.35, -0.65)
    ax.set_xlim(0, max(avg_speedups, default=1.0) * 1.22)

    fig.tight_layout()
    fig.savefig(out_path, bbox_inches="tight", pad_inches=0.15)
    plt.close(fig)

def plot_speedup_range(media, series, seconds, out_path):
    """Chart displaying Min, Average, and Max speedup vs subs2srs for each Vesta variant."""
    vesta_series = [s for s in series if s != "subs2srs (TSV)"]
    if not vesta_series:
        return

    data = []
    for s in vesta_series:
        speedups = []
        for m in media:
            base = seconds.get((m, "subs2srs (TSV)"))
            v = seconds.get((m, s))
            if base and v and v > 0:
                speedups.append(base / v)
        if speedups:
            min_sp = float(np.min(speedups))
            avg_sp = float(np.mean(speedups))
            max_sp = float(np.max(speedups))
            color = "#555555"
            for _, _, _, lbl, col in SERIES_ORDER:
                if lbl == s:
                    color = col
                    break
            data.append({
                "label": s,
                "min": min_sp,
                "avg": avg_sp,
                "max": max_sp,
                "color": color
            })

    if not data:
        return

    fig, ax = plt.subplots(figsize=(max(9, 1.4 * len(data) + 2), 6.5))
    x = np.arange(len(data))
    
    # Plot range bars (from min to max) and points for avg, min, max
    for i, d in enumerate(data):
        # Range line from min to max
        ax.plot([i, i], [d["min"], d["max"]], color="#888888", linewidth=2.5, zorder=2)
        # Min horizontal cap
        ax.plot([i - 0.15, i + 0.15], [d["min"], d["min"]], color="#555555", linewidth=2, zorder=3)
        # Max horizontal cap
        ax.plot([i - 0.15, i + 0.15], [d["max"], d["max"]], color="#555555", linewidth=2, zorder=3)
        # Average center marker
        ax.scatter([i], [d["avg"]], color=d["color"], s=160, edgecolor="black", linewidth=1.2, zorder=4)

        # Labels
        ax.text(i, d["avg"] + 0.15, f"Avg: {d['avg']:.2f}×", ha="center", fontsize=8.5, fontweight="bold")
        ax.text(i + 0.22, d["min"], f"Min: {d['min']:.2f}×", va="center", fontsize=7.5, color="#555555")
        ax.text(i + 0.22, d["max"], f"Max: {d['max']:.2f}×", va="center", fontsize=7.5, color="#555555")

    # Reference parity line
    ax.axhline(1.0, color="#d9534f", linestyle="--", linewidth=1.2, alpha=0.8, zorder=1)
    ax.text(len(data) - 0.5, 1.08, "subs2srs baseline (1.0×)", color="#d9534f", fontsize=8, fontweight="bold")

    style_axes(ax)
    ax.set_xticks(x, [d["label"] for d in data], rotation=25, ha="right", fontsize=9)
    ax.set_ylabel("Speedup vs subs2srs (higher is better)", fontsize=10)
    ax.set_title("Vesta Speedup Range vs subs2srs: Min, Average, and Max",
                 fontweight="bold", pad=16, fontsize=12)
    max_val = max(d["max"] for d in data)
    ax.set_ylim(0, max_val * 1.2)

    fig.tight_layout()
    fig.savefig(out_path, bbox_inches="tight")
    plt.close(fig)

def plot_individual_films(media, subcount, series, seconds, films_dir):
    films_dir.mkdir(parents=True, exist_ok=True)
    for title in media:
        labels = [s for s in series if (title, s) in seconds]
        vals = [seconds[(title, s)] for s in labels]
        if not vals:
            continue

        colors = []
        for s in labels:
            c = "#555555"
            for _, _, _, lbl, col in SERIES_ORDER:
                if lbl == s:
                    c = col
                    break
            colors.append(c)

        x = np.arange(len(labels))
        fig, ax = plt.subplots(figsize=(max(7.5, 1.3 * len(labels) + 2), 5.0))
        bars = ax.bar(x, vals, 0.55, color=colors, zorder=3)
        ax.bar_label(bars, labels=[f"{v:,.1f}s" for v in vals], padding=3, fontsize=8.5, rotation=-35)

        base = seconds.get((title, "subs2srs (TSV)"))
        if base:
            for xi, (label, v) in enumerate(zip(labels, vals)):
                if label != "subs2srs (TSV)" and v > 0:
                    ax.text(xi, v / 2, f"{base / v:.2f}×", ha="center", va="center",
                            fontsize=9, fontweight="bold", color="white")

        style_axes(ax)
        ax.set_ylabel("Elapsed time (seconds)", fontsize=9.5)
        ax.set_title(f"{pretty(title)} — {subcount[title]:,} subtitles (lower is better)",
                     fontweight="bold", pad=12, fontsize=11)
        ax.set_xticks(x, labels, rotation=30, ha="right", fontsize=8.5)
        ax.set_ylim(0, max(vals) * 1.25)
        fig.tight_layout()
        slug = slugify(title)
        fig.savefig(films_dir / f"{slug}.svg", bbox_inches="tight")
        plt.close(fig)

def generate_markdown_summary(media, subcount, series, seconds, out_md):
    # Hardware info
    import subprocess
    cpu_model = "Generic x86_64"
    gpu_model = "AMD Radeon / VA-API"
    try:
        res = subprocess.run("lscpu", capture_output=True, text=True, check=False)
        for line in res.stdout.splitlines():
            if "Model name:" in line:
                cpu_model = line.split(":", 1)[1].strip()
                break
        res = subprocess.run("lspci", capture_output=True, text=True, check=False)
        for line in res.stdout.splitlines():
            if any(k in line.lower() for k in ["vga", "3d", "display"]):
                gpu_model = line.split(":", 2)[-1].strip()
                break
    except Exception:
        pass

    cores = os.cpu_count() or 16

    lines = [
        "# Vesta vs subs2srs — Comprehensive Benchmark Report",
        "",
        "### System & Hardware Specifications",
        f"- **CPU**: {cpu_model} ({cores} logical cores)",
        f"- **GPU**: {gpu_model}",
        "- **Pipeline Modes Tested**:",
        "  - **Direct (No Transcode)**: Direct stream cutting without pre-transcoding (identical methodology to subs2srs).",
        "  - **GPU Pre-Transcoding**: VA-API hardware acceleration generating intermediate scale stream in ~1-2 min.",
        "- **Formats Tested**: Raw TSV + media folder vs self-contained Anki `.apkg` packages.",
        "- **Worker Configurations**: 1-worker (single core control matching subs2srs) vs Multi-core (all logical cores).",
        "",
        "## Charts Overview",
        "",
        "### 1. Suite Comparison (All Films & Variants)",
        "![Suite Overview](benchmark_overview.svg)",
        "",
        "### 2. Average Speedup vs subs2srs",
        "![Speedup Summary](benchmark_speedup_summary.svg)",
        "",
        "### 3. Speedup Range (Min, Average, Max)",
        "![Speedup Range](benchmark_speedup_range.svg)",
        "",
        "## Aggregate Performance Summary",
        "",
        "| Series | Total Wall-Clock Time | Overall Speed-up | Avg Film Speed-up | Min Speed-up | Max Speed-up |",
        "|---|---:|---:|---:|---:|---:|",
    ]

    totals = {}
    matched_totals = {}
    matched_base = {}
    film_speedups = {s: [] for s in series}

    for m in media:
        base = seconds.get((m, "subs2srs (TSV)"))
        for s in series:
            v = seconds.get((m, s))
            if v is not None:
                totals[s] = totals.get(s, 0.0) + v
                if base is not None:
                    matched_totals[s] = matched_totals.get(s, 0.0) + v
                    matched_base[s] = matched_base.get(s, 0.0) + base
                    if s != "subs2srs (TSV)" and v > 0:
                        film_speedups[s].append(base / v)

    for s in series:
        tot_min = totals.get(s, 0.0) / 60.0
        if s == "subs2srs (TSV)":
            lines.append(f"| **{s}** | {tot_min:.1f} min | — (baseline) | — | — | — |")
        else:
            mb = matched_base.get(s, 0.0)
            mt = matched_totals.get(s, 0.0)
            overall_sp = f"**{mb / mt:.2f}×**" if (mt > 0 and mb > 0) else "—"
            sps = film_speedups[s]
            if sps:
                avg_sp = f"**{np.mean(sps):.2f}×**"
                min_sp = f"{np.min(sps):.2f}×"
                max_sp = f"{np.max(sps):.2f}×"
            else:
                avg_sp = min_sp = max_sp = "—"
            lines.append(f"| **{s}** | {tot_min:.1f} min | {overall_sp} | {avg_sp} | {min_sp} | {max_sp} |")

    lines.extend([
        "",
        "## Per-Film Detailed Results",
        "",
        "| Film | Subtitles | Series | Time | Cards/min | Speed-up vs subs2srs |",
        "|---|---:|---|---:|---:|---:|",
    ])

    for m in media:
        base = seconds.get((m, "subs2srs (TSV)"))
        subc = subcount[m]
        p_name = pretty(m)
        for s in series:
            v = seconds.get((m, s))
            if v is None:
                continue
            cpm = f"{int(round(subc / (v / 60.0))):,}" if v > 0 else "—"
            sp = f"**{base / v:.2f}×**" if (s != "subs2srs (TSV)" and base and v > 0) else "—"
            lines.append(f"| {p_name} | {subc:,} | {s} | {v:,.1f} s | {cpm} | {sp} |")

    lines.extend([
        "",
        "## Per-Film Charts",
        ""
    ])

    for m in media:
        slug = slugify(m)
        p_name = pretty(m)
        lines.append(f"### {p_name}\n\n![{p_name}](films/{slug}.svg)\n")

    with open(out_md, "w") as f:
        f.write("\n".join(lines) + "\n")

def main():
    if len(sys.argv) < 3:
        sys.exit("usage: generate_full_report.py <results_full.csv> <output_dir>")
    csv_path = Path(sys.argv[1])
    out_dir = Path(sys.argv[2])
    out_dir.mkdir(parents=True, exist_ok=True)

    media, subcount, series, seconds = load_data(csv_path)
    if not media:
        print("No valid data rows found in CSV.")
        return

    print(f"Loaded {len(media)} media entries, {len(series)} series variants.")
    plot_overview(media, subcount, series, seconds, out_dir / "benchmark_overview.svg")
    plot_speedup_summary(media, series, seconds, out_dir / "benchmark_speedup_summary.svg")
    plot_speedup_range(media, series, seconds, out_dir / "benchmark_speedup_range.svg")
    plot_individual_films(media, subcount, series, seconds, out_dir / "films")
    generate_markdown_summary(media, subcount, series, seconds, out_dir / "summary.md")

    # Copy top-level charts and per-film charts to docs/
    docs_dir = Path("docs")
    if docs_dir.exists():
        for svg_name in ["benchmark_overview.svg", "benchmark_speedup_summary.svg", "benchmark_speedup_range.svg"]:
            src = out_dir / svg_name
            if src.exists():
                shutil.copy(src, docs_dir / svg_name)
        if (out_dir / "summary.md").exists():
            shutil.copy(out_dir / "summary.md", docs_dir / "BENCHMARK_REPORT.md")
        films_src = out_dir / "films"
        if films_src.exists():
            films_dst = docs_dir / "films"
            films_dst.mkdir(parents=True, exist_ok=True)
            for f in films_src.glob("*.svg"):
                shutil.copy(f, films_dst / f.name)

    # Sync legacy results.csv and results_gpu.csv for compatibility
    with open(csv_path) as f:
        all_rows = list(csv.DictReader(f))
    
    # 1. results.csv
    res_csv = out_dir / "results.csv"
    with open(res_csv, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=["title","subtitle_count","tool","variant","format","elapsed_ms","lines","audio","snapshots","video","jobs"])
        writer.writeheader()
        for r in all_rows:
            var = r["variant"]
            if var == "subs2srs":
                writer.writerow({k: r[k] for k in writer.fieldnames})
            elif var == "single_direct":
                writer.writerow({**{k: r[k] for k in writer.fieldnames}, "variant": "single"})
            elif var == "multi_direct":
                writer.writerow({**{k: r[k] for k in writer.fieldnames}, "variant": "max"})

    # 2. results_gpu.csv
    res_gpu_csv = out_dir / "results_gpu.csv"
    with open(res_gpu_csv, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=["title","subtitle_count","tool","variant","format","elapsed_ms","lines","audio","snapshots","video","jobs"])
        writer.writeheader()
        for r in all_rows:
            var = r["variant"]
            fmt = r["format"]
            if var == "subs2srs":
                writer.writerow({k: r[k] for k in writer.fieldnames})
            elif var == "multi_direct" and fmt == "apkg":
                writer.writerow({**{k: r[k] for k in writer.fieldnames}, "variant": "max"})
            elif var == "multi_gpu" and fmt == "apkg":
                writer.writerow({**{k: r[k] for k in writer.fieldnames}, "variant": "gpu"})

    # Regenerate benchmark.svg and benchmark_gpu.svg
    report_dir = Path(__file__).resolve().parent
    plot_script = report_dir / "plot.py"
    plot_gpu_script = report_dir / "plot_gpu.py"
    if plot_script.exists():
        import subprocess
        subprocess.run([sys.executable, str(plot_script), str(res_csv), str(docs_dir)], check=False)
        subprocess.run([sys.executable, str(plot_script), str(res_csv), str(out_dir)], check=False)
    if plot_gpu_script.exists():
        import subprocess
        subprocess.run([sys.executable, str(plot_gpu_script), str(res_gpu_csv), str(docs_dir / "benchmark_gpu.svg")], check=False)
        subprocess.run([sys.executable, str(plot_gpu_script), str(res_gpu_csv), str(out_dir / "benchmark_gpu.svg")], check=False)

    print(f"Successfully generated all charts and reports under {out_dir}")

if __name__ == "__main__":
    main()
