#!/usr/bin/env python3
"""Render the Vesta GPU vs CPU vs subs2srs benchmark charts.

Usage: plot_gpu.py <results_gpu.csv> <output_dir>
"""
import csv
import re
import sys
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

PALETTE = {
    "subs2srs": "#e8770f",
    "Vesta CPU (16 cores)": "#8073c9",
    "Vesta GPU (VA-API / Pre-transcode)": "#12a184",
}
BG = "#f8f9fb"


def series_label(row):
    if row["tool"] == "subs2srs":
        return "subs2srs"
    if row.get("variant") == "gpu":
        return "Vesta GPU (VA-API / Pre-transcode)"
    return "Vesta CPU (16 cores)"


def series_order(label):
    if label.startswith("subs2srs"):
        return 0
    if "CPU" in label:
        return 1
    return 2


def pretty(title):
    return title.replace(".", " ").strip()


def load(csv_path):
    with open(csv_path, newline="") as f:
        rows = list(csv.DictReader(f))
    if not rows:
        sys.exit("no rows in results")

    media, subcount, series, seconds = [], {}, [], {}
    for r in rows:
        title = r["title"]
        if title not in subcount:
            media.append(title)
        subcount[title] = int(r["subtitle_count"])
        label = series_label(r)
        if label not in series:
            series.append(label)
        seconds[(title, label)] = float(r["elapsed_ms"]) / 1000.0
    series.sort(key=series_order)
    media.sort(key=lambda m: subcount[m], reverse=True)
    return media, subcount, series, seconds


def style_axes(ax):
    ax.set_facecolor(BG)
    for spine in ("top", "right"):
        ax.spines[spine].set_visible(False)
    ax.grid(axis="y", linestyle="--", alpha=0.35)
    ax.set_axisbelow(True)


def combined_chart(media, subcount, series, seconds, out_path):
    x = np.arange(len(media))
    n = len(series)
    width = min(0.8 / max(n, 1), 0.22)

    baseline_per_film = {}
    for m in media:
        film_vals = [seconds.get((m, s), 0.0) for s in series if (m, s) in seconds]
        baseline_per_film[m] = max(film_vals) if film_vals else 1.0

    fig, ax = plt.subplots(figsize=(max(11, 2.4 * len(media) + 3), 6.8))
    for i, label in enumerate(series):
        raw_vals = [seconds.get((m, label), np.nan) for m in media]
        norm_vals = [
            (v / baseline_per_film[m] * 100.0) if v == v else np.nan
            for m, v in zip(media, raw_vals)
        ]
        offset = (i - (n - 1) / 2) * width
        color = PALETTE.get(label, "#555555")
        bars = ax.bar(x + offset, norm_vals, width, label=label,
                      color=color, zorder=3)
        texts = ["" if v != v else f"{v:,.1f}s" for v in raw_vals]
        ax.bar_label(bars, labels=texts, padding=4, fontsize=8, rotation=-45)

        # Annotate speedup on Vesta bars
        if "Vesta" in label:
            for xi, (m, v) in enumerate(zip(media, raw_vals)):
                base = seconds.get((m, "subs2srs"))
                if base and v > 0 and norm_vals[xi] > 12:
                    speedup = base / v
                    ax.text(xi + offset, norm_vals[xi] / 2, f"{speedup:.1f}×",
                            ha="center", va="center", fontsize=8,
                            fontweight="bold", color="white")

    style_axes(ax)
    ax.set_ylim(0, 128)
    ax.set_ylabel("Relative generation time (% of slowest baseline)")
    ax.yaxis.set_major_formatter(matplotlib.ticker.PercentFormatter(100))
    ax.set_title("Vesta GPU Acceleration vs Vesta CPU & subs2srs (lower is better)",
                 fontweight="bold", pad=16, fontsize=12)
    ax.set_xticks(x, [f"{pretty(m)}\n({subcount[m]:,} subtitles)" for m in media],
                  fontsize=9.5)
    ax.legend(frameon=False, ncol=3, loc="upper center",
              bbox_to_anchor=(0.5, -0.12), fontsize=9)

    fig.text(0.5, -0.06,
             "* Normalized per film: 100% = subs2srs baseline. Values above bars show elapsed seconds; labels inside show speedup vs subs2srs.",
             ha="center", fontsize=8, color="#555555", style="italic")

    fig.tight_layout()
    fig.savefig(out_path, bbox_inches="tight")
    plt.close(fig)
    print(out_path)


def main():
    if len(sys.argv) < 3:
        sys.exit("usage: plot_gpu.py <results_gpu.csv> <output_svg>")
    csv_path, out_path = sys.argv[1], Path(sys.argv[2])
    out_path.parent.mkdir(parents=True, exist_ok=True)
    media, subcount, series, seconds = load(csv_path)
    combined_chart(media, subcount, series, seconds, out_path)


if __name__ == "__main__":
    main()
