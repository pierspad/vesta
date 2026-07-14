#!/usr/bin/env python3
"""Score transcribed SRTs against a reference SRT and merge timing data.

Usage:
    score.py --reference ref.srt --timings timings.csv \
             --out-csv results.csv --out-md summary.md \
             name1=hyp1.srt name2=hyp2.srt …

Metrics per variant
-------------------
- WER          word error rate vs the reference text (lowercased, no
                punctuation, [bracketed]/(parenthesised) cues removed).
                The reference is a human subtitle track, not a verbatim
                transcript, so treat absolute WER as a proxy; the
                *relative* ranking between variants is what matters.
- hit_rate     fraction of reference cues temporally covered (≥50% of the
                cue's span overlapped by some hypothesis segment).
- timing_mae   mean |start offset| (s) between each hypothesis segment and
                the reference cue it overlaps most.
- score        0–100 composite: 70% text accuracy + 30% timing.
                accuracy = max(0, 1 − WER); timing = hit_rate scaled by a
                1-second-tolerance factor on timing_mae.

The timings CSV (from 2_run_benchmark.sh) contributes wall/CPU/RSS/size
columns; medians across repeats are reported.
"""

from __future__ import annotations

import argparse
import csv
import re
import statistics
import sys
from dataclasses import dataclass
from pathlib import Path

# ── SRT parsing ───────────────────────────────────────────────────────────────

TS = re.compile(r"(\d{2}):(\d{2}):(\d{2})[,.](\d{3})")


@dataclass
class Cue:
    start: float
    end: float
    text: str


def parse_srt(path: Path) -> list[Cue]:
    cues: list[Cue] = []
    block_text: list[str] = []
    start = end = None
    raw = path.read_text(encoding="utf-8-sig", errors="replace")
    for line in raw.splitlines() + [""]:
        line = line.strip("﻿").rstrip()
        m = re.match(rf"^{TS.pattern}\s*-->\s*{TS.pattern}", line)
        if m:
            g = [int(x) for x in m.groups()]
            start = g[0] * 3600 + g[1] * 60 + g[2] + g[3] / 1000
            end = g[4] * 3600 + g[5] * 60 + g[6] + g[7] / 1000
            block_text = []
        elif line == "":
            if start is not None and block_text:
                cues.append(Cue(start, end, " ".join(block_text)))
            start = end = None
            block_text = []
        elif start is not None:
            block_text.append(line)
    return cues


# ── Text normalisation + WER ─────────────────────────────────────────────────

DROP = re.compile(r"\[[^\]]*\]|\([^)]*\)|♪[^♪]*♪|<[^>]+>")
NONWORD = re.compile(r"[^a-z0-9' ]+")


def norm_words(text: str) -> list[str]:
    text = DROP.sub(" ", text.lower())
    text = text.replace("’", "'")
    return NONWORD.sub(" ", text).split()


def words(cues: list[Cue]) -> list[str]:
    return norm_words(" ".join(c.text for c in cues))


def edit_distance(ref: list[str], hyp: list[str]) -> int:
    """Two-row Levenshtein (ins+del+sub)."""
    if not ref:
        return len(hyp)
    prev = list(range(len(hyp) + 1))
    for i, rw in enumerate(ref, 1):
        cur = [i] + [0] * len(hyp)
        for j, hw in enumerate(hyp, 1):
            cur[j] = min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (rw != hw))
        prev = cur
    return prev[-1]


def wer(ref_cues: list[Cue], hyp_cues: list[Cue], window: float = 60.0) -> float:
    """Time-windowed WER: words are bucketed by their cue's midpoint into
    fixed windows and aligned window-by-window.

    Full-sequence Levenshtein on a feature film (~6k words) is O(n·m) and
    would take minutes in pure Python; per-window it is milliseconds. As a
    bonus the metric penalises text emitted at the wrong time, which for
    subtitles is exactly what we want. Border effects (a word landing one
    window off) inflate all variants equally, so the comparison stays fair.
    """

    def bucket(cues: list[Cue]) -> dict[int, list[str]]:
        buckets: dict[int, list[str]] = {}
        for c in cues:
            key = int(((c.start + c.end) / 2) // window)
            buckets.setdefault(key, []).extend(norm_words(c.text))
        return buckets

    ref_b, hyp_b = bucket(ref_cues), bucket(hyp_cues)
    total_ref = sum(len(v) for v in ref_b.values())
    if total_ref == 0:
        return 0.0
    edits = sum(
        edit_distance(ref_b.get(k, []), hyp_b.get(k, []))
        for k in sorted(set(ref_b) | set(hyp_b))
    )
    return edits / total_ref


# ── Timing metrics ────────────────────────────────────────────────────────────

def overlap(a: Cue, b: Cue) -> float:
    return max(0.0, min(a.end, b.end) - max(a.start, b.start))


def timing_metrics(ref: list[Cue], hyp: list[Cue]) -> tuple[float, float]:
    """(hit_rate, timing_mae). Single sweep — both lists are time-sorted."""
    ref = sorted(ref, key=lambda c: c.start)
    hyp = sorted(hyp, key=lambda c: c.start)
    covered = [0.0] * len(ref)
    offsets: list[float] = []
    j = 0
    for h in hyp:
        while j and ref[j].start > h.start:
            j -= 1
        while j < len(ref) and ref[j].end < h.start:
            j += 1
        best, best_ov = None, 0.0
        for k in range(j, len(ref)):
            if ref[k].start > h.end:
                break
            ov = overlap(ref[k], h)
            if ov > best_ov:
                best, best_ov = k, ov
        if best is not None and best_ov > 0:
            covered[best] += best_ov
            offsets.append(abs(ref[best].start - h.start))
    hits = sum(
        1 for c, r in zip(covered, ref) if r.end > r.start and c / (r.end - r.start) >= 0.5
    )
    hit_rate = hits / len(ref) if ref else 0.0
    mae = statistics.fmean(offsets) if offsets else float("inf")
    return hit_rate, mae


# ── Composite score ───────────────────────────────────────────────────────────

def composite(w: float, hit_rate: float, mae: float) -> float:
    accuracy = max(0.0, 1.0 - w)
    tolerance = 1.0 / (1.0 + max(0.0, mae))  # 1s MAE → 0.5, 0s → 1.0
    timing = hit_rate * tolerance
    return round(100 * (0.7 * accuracy + 0.3 * timing), 1)


# ── Main ─────────────────────────────────────────────────────────────────────

def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--reference", required=True, type=Path)
    ap.add_argument("--timings", type=Path)
    ap.add_argument("--out-csv", required=True, type=Path)
    ap.add_argument("--out-md", required=True, type=Path)
    ap.add_argument("variants", nargs="+", help="name=path.srt")
    args = ap.parse_args()

    ref_cues = parse_srt(args.reference)
    ref_words = words(ref_cues)
    if not ref_cues:
        sys.exit(f"reference vuota o non parsabile: {args.reference}")

    timings: dict[str, dict[str, float]] = {}
    if args.timings and args.timings.exists():
        rows: dict[str, list[dict]] = {}
        with args.timings.open() as fh:
            for row in csv.DictReader(fh):
                rows.setdefault(row["variant"], []).append(row)
        for name, rs in rows.items():
            med = lambda k: statistics.median(float(r[k]) for r in rs)  # noqa: E731
            timings[name] = {
                "wall_s": med("wall_s"),
                "cpu_s": med("user_s") + med("sys_s"),
                "cpu_pct": med("cpu_pct"),
                "max_rss_mib": med("max_rss_kb") / 1024,
                "binary_mib": med("binary_bytes") / 1024 / 1024,
                "model_mib": med("model_bytes") / 1024 / 1024,
            }

    results = []
    for spec in args.variants:
        name, _, path = spec.partition("=")
        hyp_cues = parse_srt(Path(path))
        w = wer(ref_cues, hyp_cues)
        hit_rate, mae = timing_metrics(ref_cues, hyp_cues)
        row = {
            "variant": name,
            "score": composite(w, hit_rate, mae),
            "wer": round(w, 4),
            "hit_rate": round(hit_rate, 4),
            "timing_mae_s": round(mae, 3) if mae != float("inf") else "",
            "segments": len(hyp_cues),
            "ref_segments": len(ref_cues),
            **{k: round(v, 2) for k, v in timings.get(name, {}).items()},
        }
        results.append(row)
        print(f"  {name:<20} score={row['score']:>5}  WER={w:.2%}  "
              f"hit={hit_rate:.1%}  MAE={row['timing_mae_s']}s")

    results.sort(key=lambda r: r["score"], reverse=True)

    cols = ["variant", "score", "wer", "hit_rate", "timing_mae_s", "segments",
            "ref_segments", "wall_s", "cpu_s", "cpu_pct", "max_rss_mib",
            "binary_mib", "model_mib"]
    args.out_csv.parent.mkdir(parents=True, exist_ok=True)
    with args.out_csv.open("w", newline="") as fh:
        wr = csv.DictWriter(fh, fieldnames=cols, extrasaction="ignore")
        wr.writeheader()
        wr.writerows(results)

    lines = [
        "# Whisper benchmark — Vesta (whisper-common) vs whisper-subs",
        "",
        f"Audio: `Detour (1945)` · Reference: `{args.reference.name}` "
        f"({len(ref_cues)} cues, {len(ref_words)} words)",
        "",
        "| variant | score | WER | hit rate | MAE (s) | wall (s) | CPU (s) | RSS (MiB) | bin (MiB) |",
        "|---|---|---|---|---|---|---|---|---|",
    ]
    for r in results:
        lines.append(
            f"| {r['variant']} | **{r['score']}** | {r['wer']:.2%} | "
            f"{r['hit_rate']:.1%} | {r['timing_mae_s']} | {r.get('wall_s', '')} | "
            f"{r.get('cpu_s', '')} | {r.get('max_rss_mib', '')} | {r.get('binary_mib', '')} |"
        )
    lines += [
        "",
        "score = 70% testo (1−WER) + 30% timing (hit-rate × tolleranza sul MAE). "
        "La reference è una traccia umana OpenSubtitles: WER assoluto è un proxy, "
        "il confronto relativo tra varianti è il dato affidabile.",
        "",
    ]
    args.out_md.write_text("\n".join(lines), encoding="utf-8")
    print(f"\n→ {args.out_csv}\n→ {args.out_md}")


if __name__ == "__main__":
    main()
