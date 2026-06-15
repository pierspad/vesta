#!/usr/bin/env python3
"""Render the Vesta vs subs2srs benchmark chart from the results CSV.

Usage: plot.py <results.csv> <output.png>

CSV columns:
    title,subtitle_count,tool,variant,format,elapsed_ms,lines,audio,snapshots,video,jobs

Draws one grouped bar per medium: subs2srs alongside each Vesta series — the
"export" plan (cores-1 parallel ffmpeg) and the single-core variant that matches
subs2srs core-for-core. Lower is better; the y-axis is log-scaled because Vesta
and subs2srs can differ by an order of magnitude. Requires matplotlib.
"""
import csv
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402

# subs2srs · Vesta TSV/APKG (max) · Vesta TSV/APKG (1 core)
PALETTE = ["#7f8c8d", "#e8770f", "#c0392b", "#f6b26b", "#e6a0a0"]


def series_label(row):
    """Human-readable legend label for a result row."""
    if row["tool"] == "subs2srs":
        return "subs2srs"
    core = " (1 core)" if row.get("jobs") == "1" else ""
    return f"Vesta export {row['format'].upper()}{core}"


def series_order(label):
    """Sort key: subs2srs first, then Vesta max (TSV, APKG), then single-core."""
    if label.startswith("subs2srs"):
        return (0, 0, 0, label)
    single = 1 if "1 core" in label else 0
    fmt = 0 if "TSV" in label else 1
    return (1, single, fmt, label)


def main():
    if len(sys.argv) != 3:
        sys.exit("usage: plot.py <results.csv> <output.png>")
    csv_path, out_path = sys.argv[1], sys.argv[2]

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

    x = np.arange(len(media))
    n = len(series)
    width = min(0.8 / max(n, 1), 0.16)

    fig, ax = plt.subplots(figsize=(max(9, 1.7 * len(media) + 3), 6))
    for i, label in enumerate(series):
        vals = [seconds.get((m, label), np.nan) for m in media]
        offset = (i - (n - 1) / 2) * width
        bars = ax.bar(x + offset, vals, width, label=label, color=PALETTE[i % len(PALETTE)])
        texts = ["" if v != v else f"{v:.1f}" for v in vals]  # v != v skips NaN
        ax.bar_label(bars, labels=texts, padding=2, fontsize=6, rotation=90)

    ax.set_yscale("log")
    ax.set_ylabel("Elapsed time — seconds (log scale, lower is better)")
    ax.set_title("Vesta vs subs2srs — flashcard generation", fontweight="bold")
    ax.set_xticks(x, [f"{m}\n{subcount[m]} subs" for m in media])
    ax.legend(frameon=False, ncol=min(n, 3), loc="upper center", bbox_to_anchor=(0.5, -0.10))
    ax.grid(axis="y", linestyle="--", alpha=0.3)
    fig.tight_layout()
    fig.savefig(out_path, dpi=130, bbox_inches="tight")
    print(out_path)


if __name__ == "__main__":
    main()
