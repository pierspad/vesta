# Vesta vs subs2srs — Comprehensive Benchmark Report

### System & Hardware Specifications
- **CPU**: AMD Ryzen 7 5800X 8-Core Processor (16 threads)
- **GPU**: Advanced Micro Devices, Inc. [AMD/ATI] Navi 32 [Radeon RX 7700 XT / 7800 XT] (rev c8)
- **Pipeline Modes Tested**:
  - **Default**: native Rust pipeline, with automatic source preparation when eligible for video clips and nearby WebP snapshot batching.
  - No separately measured GPU variant is included in the 2026-10-08 results; hardware use is not instrumented by this suite.
- **Formats Tested**: Raw TSV + media folder vs ready-to-import Anki `.apkg` packages.
- **Workers requested**: [1, 16]; subs2srs remains sequential. A worker count is not a CPU affinity/thread limit.
- **Dataset date**: 2026-10-08. The published 2026-10-08 run uses three repetitions per cell and median wall-clock times; repetition metadata is retained separately.
- **Scope**: same subtitle/video inputs and requested media types, but product encoding defaults differ (JPEG versus WebP and different clip settings). This is not a byte-identical or controlled equal-quality comparison.
- **Counts**: Vesta multicore TSV produced 12,859 audio clips; subs2srs produced 12,463. See the raw CSV for all media types. The reported lines count is the parsed input count, not proof of exported media completeness.
- **Causality**: use [the native A/B results](https://github.com/pierspad/vesta/blob/main/docs/BENCHMARK_NATIVE.md) to isolate the snapshot technique at identical quality; this full pipeline also includes video preparation and other changes.

## Charts Overview

### 1. Suite Comparison (All Movies & Variants)
![Suite Overview](benchmark_overview.svg)

### 2. Average Speedup vs subs2srs
![Speedup Summary](benchmark_speedup_summary.svg)

### 3. Speedup Range (Min, Average, Max)
![Speedup Range](benchmark_speedup_range.svg)

## Aggregate Performance Summary

| Series | Total Wall-Clock Time | Overall Speed-up | Avg Movie Speed-up | Min Speed-up | Max Speed-up |
|---|---:|---:|---:|---:|---:|
| **subs2srs (TSV)** | 125.5 min | — (baseline) | — | — | — |
| **Vesta 1 worker (TSV)** | 79.5 min | **1.58×** | **1.56×** | 1.30× | 1.89× |
| **Vesta 1 worker (APKG)** | 79.5 min | **1.58×** | **1.56×** | 1.30× | 1.89× |
| **Vesta 16 workers (TSV)** | 30.3 min | **4.14×** | **4.15×** | 3.44× | 5.24× |
| **Vesta 16 workers (APKG)** | 30.4 min | **4.13×** | **4.14×** | 3.44× | 5.21× |

## Per-Movie Detailed Results

| Movie | Subtitles | Series | Time | Cards/min | Speed-up vs subs2srs |
|---|---:|---|---:|---:|---:|
| Uncut Gems | 2,315 | subs2srs (TSV) | 1,307.8 s | 106 | — |
| Uncut Gems | 2,315 | Vesta 1 worker (TSV) | 863.3 s | 161 | **1.51×** |
| Uncut Gems | 2,315 | Vesta 1 worker (APKG) | 863.7 s | 161 | **1.51×** |
| Uncut Gems | 2,315 | Vesta 16 workers (TSV) | 297.2 s | 467 | **4.40×** |
| Uncut Gems | 2,315 | Vesta 16 workers (APKG) | 298.7 s | 465 | **4.38×** |
| Interstellar | 1,996 | subs2srs (TSV) | 1,320.0 s | 91 | — |
| Interstellar | 1,996 | Vesta 1 worker (TSV) | 777.1 s | 154 | **1.70×** |
| Interstellar | 1,996 | Vesta 1 worker (APKG) | 778.2 s | 154 | **1.70×** |
| Interstellar | 1,996 | Vesta 16 workers (TSV) | 317.9 s | 377 | **4.15×** |
| Interstellar | 1,996 | Vesta 16 workers (APKG) | 315.2 s | 380 | **4.19×** |
| Good Will Hunting | 1,755 | subs2srs (TSV) | 1,140.5 s | 92 | — |
| Good Will Hunting | 1,755 | Vesta 1 worker (TSV) | 603.7 s | 174 | **1.89×** |
| Good Will Hunting | 1,755 | Vesta 1 worker (APKG) | 602.1 s | 175 | **1.89×** |
| Good Will Hunting | 1,755 | Vesta 16 workers (TSV) | 217.7 s | 484 | **5.24×** |
| Good Will Hunting | 1,755 | Vesta 16 workers (APKG) | 218.9 s | 481 | **5.21×** |
| Zootopia | 1,698 | subs2srs (TSV) | 955.4 s | 107 | — |
| Zootopia | 1,698 | Vesta 1 worker (TSV) | 642.6 s | 159 | **1.49×** |
| Zootopia | 1,698 | Vesta 1 worker (APKG) | 641.1 s | 159 | **1.49×** |
| Zootopia | 1,698 | Vesta 16 workers (TSV) | 238.4 s | 427 | **4.01×** |
| Zootopia | 1,698 | Vesta 16 workers (APKG) | 238.8 s | 427 | **4.00×** |
| Snatch | 1,522 | subs2srs (TSV) | 950.4 s | 96 | — |
| Snatch | 1,522 | Vesta 1 worker (TSV) | 660.4 s | 138 | **1.44×** |
| Snatch | 1,522 | Vesta 1 worker (APKG) | 661.1 s | 138 | **1.44×** |
| Snatch | 1,522 | Vesta 16 workers (TSV) | 275.9 s | 331 | **3.44×** |
| Snatch | 1,522 | Vesta 16 workers (APKG) | 276.1 s | 331 | **3.44×** |
| Trainspotting | 1,384 | subs2srs (TSV) | 672.5 s | 123 | — |
| Trainspotting | 1,384 | Vesta 1 worker (TSV) | 448.5 s | 185 | **1.50×** |
| Trainspotting | 1,384 | Vesta 1 worker (APKG) | 449.4 s | 185 | **1.50×** |
| Trainspotting | 1,384 | Vesta 16 workers (TSV) | 167.6 s | 495 | **4.01×** |
| Trainspotting | 1,384 | Vesta 16 workers (APKG) | 168.2 s | 494 | **4.00×** |
| In Bruges | 1,171 | subs2srs (TSV) | 822.3 s | 85 | — |
| In Bruges | 1,171 | Vesta 1 worker (TSV) | 497.8 s | 141 | **1.65×** |
| In Bruges | 1,171 | Vesta 1 worker (APKG) | 496.9 s | 141 | **1.65×** |
| In Bruges | 1,171 | Vesta 16 workers (TSV) | 219.6 s | 320 | **3.74×** |
| In Bruges | 1,171 | Vesta 16 workers (APKG) | 218.7 s | 321 | **3.76×** |
| Detour | 1,018 | subs2srs (TSV) | 362.1 s | 169 | — |
| Detour | 1,018 | Vesta 1 worker (TSV) | 279.0 s | 219 | **1.30×** |
| Detour | 1,018 | Vesta 1 worker (APKG) | 279.2 s | 219 | **1.30×** |
| Detour | 1,018 | Vesta 16 workers (TSV) | 86.1 s | 709 | **4.21×** |
| Detour | 1,018 | Vesta 16 workers (APKG) | 88.0 s | 694 | **4.12×** |

## Per-Movie Charts

### Uncut Gems

![Uncut Gems](films/uncut-gems.svg)

### Interstellar

![Interstellar](films/interstellar.svg)

### Good Will Hunting

![Good Will Hunting](films/good-will-hunting.svg)

### Zootopia

![Zootopia](films/zootopia.svg)

### Snatch

![Snatch](films/snatch.svg)

### Trainspotting

![Trainspotting](films/trainspotting.svg)

### In Bruges

![In Bruges](films/in-bruges.svg)

### Detour

![Detour](films/detour.svg)

