#!/usr/bin/env python3
"""Render the Vesta vs subs2srs benchmark charts from the results CSV.

Usage: plot.py <results.csv> <output_dir>

Produces (all SVG — GitHub renders SVG inline in markdown):
    <output_dir>/benchmark.svg          combined grouped-bar chart, log scale
    <output_dir>/films/<slug>.svg       one linear-scale chart per film

CSV columns:
    title,subtitle_count,tool,variant,format,elapsed_ms,lines,audio,snapshots,video,jobs

Series per film: subs2srs (single-threaded), each Vesta variant × format —
the "max" plan (cores-1 parallel ffmpeg) and the single-core control that
matches subs2srs core-for-core. Lower is better. Requires matplotlib.
"""
import csv
import re
import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402

# subs2srs · Vesta APKG/TSV (1 core) · Vesta APKG/TSV (max)
PALETTE = ["#e8770f", "#b3aadd", "#f6b26b", "#8073c9", "#12a184"]
BG = "#f8f9fb"


def series_label(row):
    """Human-readable legend label for a result row."""
    if row["tool"] == "subs2srs":
        return "subs2srs"
    core = " (1 core)" if row.get("jobs") == "1" else ""
    return f"Vesta {row['format'].upper()}{core}"


def series_order(label):
    """Sort key: subs2srs, then vesta single core apkg, vesta single core tsv, vesta multi core apkg, vesta multi core tsv"""
    if label.startswith("subs2srs"):
        return (0, 0, 0, label)
    single = 0 if "1 core" in label else 1
    fmt = 0 if "APKG" in label else 1
    return (1, single, fmt, label)


def slugify(title):
    return re.sub(r"[^A-Za-z0-9]+", "-", title).strip("-").lower() or "film"


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
    width = min(0.8 / max(n, 1), 0.16)

    fig, ax = plt.subplots(figsize=(max(9, 1.9 * len(media) + 3), 6.2))
    for i, label in enumerate(series):
        vals = [seconds.get((m, label), np.nan) for m in media]
        offset = (i - (n - 1) / 2) * width
        bars = ax.bar(x + offset, vals, width, label=label,
                      color=PALETTE[i % len(PALETTE)], zorder=3)
        texts = ["" if v != v else f"{v:.1f}s" for v in vals]  # v != v skips NaN
        ax.bar_label(bars, labels=texts, padding=2, fontsize=6.5, rotation=90)

    ax.set_yscale("log")
    style_axes(ax)
    ax.set_ylabel("Elapsed time — seconds (log scale)")
    ax.set_title("Vesta vs subs2srs — flashcard generation (lower is better)",
                 fontweight="bold", pad=14)
    ax.set_xticks(x, [f"{pretty(m)}\n{subcount[m]:,} subtitles" for m in media],
                  fontsize=8.5)
    ax.legend(frameon=False, ncol=min(n, 5), loc="upper center",
              bbox_to_anchor=(0.5, -0.10), fontsize=8.5)
    fig.tight_layout()
    fig.savefig(out_path, bbox_inches="tight")
    plt.close(fig)
    print(out_path)


def film_chart(title, subcount, series, seconds, out_path):
    labels = [s for s in series if (title, s) in seconds]
    vals = [seconds[(title, s)] for s in labels]
    if not vals:
        return

    x = np.arange(len(labels))
    fig, ax = plt.subplots(figsize=(max(6.5, 1.6 * len(labels) + 2), 4.6))
    bars = ax.bar(x, vals, 0.62,
                  color=[PALETTE[series.index(s) % len(PALETTE)] for s in labels],
                  zorder=3)
    ax.bar_label(bars, labels=[f"{v:.1f}s" for v in vals], padding=3, fontsize=9)

    base = seconds.get((title, "subs2srs"))
    if base:
        # Annotate each Vesta bar with its speed-up vs subs2srs.
        for xi, (label, v) in enumerate(zip(labels, vals)):
            if label != "subs2srs" and v > 0:
                ax.text(xi, v / 2, f"{base / v:.2f}×", ha="center", va="center",
                        fontsize=9, fontweight="bold", color="white")

    style_axes(ax)
    ax.set_ylabel("Elapsed time — seconds")
    ax.set_title(f"{pretty(title)} — {subcount[title]:,} subtitles (lower is better)",
                 fontweight="bold", pad=12)
    ax.set_xticks(x, labels, fontsize=9)
    ax.set_ylim(0, max(vals) * 1.18)
    fig.tight_layout()
    fig.savefig(out_path, bbox_inches="tight")
    plt.close(fig)
    print(out_path)


def main():
    if len(sys.argv) != 3:
        sys.exit("usage: plot.py <results.csv> <output_dir>")
    csv_path, out_dir = sys.argv[1], Path(sys.argv[2])
    films_dir = out_dir / "films"
    films_dir.mkdir(parents=True, exist_ok=True)

    media, subcount, series, seconds = load(csv_path)

    combined_chart(media, subcount, series, seconds, out_dir / "benchmark.svg")
    for title in media:
        film_chart(title, subcount, series, seconds,
                   films_dir / f"{slugify(title)}.svg")


if __name__ == "__main__":
    main()
