# Vesta vs subs2srs — Comprehensive Benchmark Report

### System & Hardware Specifications
- **CPU**: AMD Ryzen 7 5800X (8c-16t)
- **GPU**: AMD Radeon RX 7800 XT
- **Pipeline Modes Tested**:
  - **Direct**: Direct stream cutting without pre-transcoding (identical to subs2srs).
  - **GPU**: Hardware-accelerated (VA-API) pre-transcoding into an intermediate low resolution stream.
- **Formats Tested**: Raw TSV + media folder vs ready-to-import Anki `.apkg` packages.
- **Threading Tested**: 1t (single-thread control matching subs2srs) vs 16t (all 16 CPU threads).

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
| **subs2srs (TSV)** | 127.9 min | — (baseline) | — | — | — |
| **Vesta 1t Direct (TSV)** | 99.5 min | **1.29×** | **1.29×** | 1.03× | 1.89× |
| **Vesta 1t Direct (APKG)** | 99.5 min | **1.29×** | **1.29×** | 1.03× | 1.89× |
| **Vesta 1t GPU (TSV)** | 83.4 min | **1.53×** | **1.51×** | 1.20× | 1.86× |
| **Vesta 1t GPU (APKG)** | 83.4 min | **1.53×** | **1.51×** | 1.20× | 1.86× |
| **Vesta 16t Direct (TSV)** | 43.3 min | **2.96×** | **3.52×** | 1.72× | 6.22× |
| **Vesta 16t Direct (APKG)** | 43.4 min | **2.95×** | **3.51×** | 1.72× | 6.20× |
| **Vesta 16t GPU (TSV)** | 34.2 min | **3.74×** | **3.74×** | 3.09× | 5.00× |
| **Vesta 16t GPU (APKG)** | 33.7 min | **3.80×** | **3.78×** | 3.22× | 4.99× |

## Per-Movie Detailed Results

| Movie | Subtitles | Series | Time | Cards/min | Speed-up vs subs2srs |
|---|---:|---|---:|---:|---:|
| Uncut Gems | 2,315 | subs2srs (TSV) | 1,348.7 s | 103 | — |
| Uncut Gems | 2,315 | Vesta 1t Direct (TSV) | 899.7 s | 154 | **1.50×** |
| Uncut Gems | 2,315 | Vesta 1t Direct (APKG) | 899.5 s | 154 | **1.50×** |
| Uncut Gems | 2,315 | Vesta 1t GPU (TSV) | 921.3 s | 151 | **1.46×** |
| Uncut Gems | 2,315 | Vesta 1t GPU (APKG) | 921.1 s | 151 | **1.46×** |
| Uncut Gems | 2,315 | Vesta 16t Direct (TSV) | 328.7 s | 423 | **4.10×** |
| Uncut Gems | 2,315 | Vesta 16t Direct (APKG) | 329.3 s | 422 | **4.10×** |
| Uncut Gems | 2,315 | Vesta 16t GPU (TSV) | 351.2 s | 395 | **3.84×** |
| Uncut Gems | 2,315 | Vesta 16t GPU (APKG) | 352.3 s | 394 | **3.83×** |
| Interstellar | 1,996 | subs2srs (TSV) | 1,354.1 s | 88 | — |
| Interstellar | 1,996 | Vesta 1t Direct (TSV) | 715.2 s | 167 | **1.89×** |
| Interstellar | 1,996 | Vesta 1t Direct (APKG) | 716.0 s | 167 | **1.89×** |
| Interstellar | 1,996 | Vesta 1t GPU (TSV) | 795.5 s | 151 | **1.70×** |
| Interstellar | 1,996 | Vesta 1t GPU (APKG) | 795.4 s | 151 | **1.70×** |
| Interstellar | 1,996 | Vesta 16t Direct (TSV) | 217.6 s | 550 | **6.22×** |
| Interstellar | 1,996 | Vesta 16t Direct (APKG) | 218.3 s | 549 | **6.20×** |
| Interstellar | 1,996 | Vesta 16t GPU (TSV) | 332.6 s | 360 | **4.07×** |
| Interstellar | 1,996 | Vesta 16t GPU (APKG) | 332.8 s | 360 | **4.07×** |
| Good Will Hunting | 1,755 | subs2srs (TSV) | 1,156.5 s | 91 | — |
| Good Will Hunting | 1,755 | Vesta 1t Direct (TSV) | 881.4 s | 119 | **1.31×** |
| Good Will Hunting | 1,755 | Vesta 1t Direct (APKG) | 880.7 s | 120 | **1.31×** |
| Good Will Hunting | 1,755 | Vesta 1t GPU (TSV) | 621.8 s | 169 | **1.86×** |
| Good Will Hunting | 1,755 | Vesta 1t GPU (APKG) | 622.7 s | 169 | **1.86×** |
| Good Will Hunting | 1,755 | Vesta 16t Direct (TSV) | 430.6 s | 245 | **2.69×** |
| Good Will Hunting | 1,755 | Vesta 16t Direct (APKG) | 432.1 s | 244 | **2.68×** |
| Good Will Hunting | 1,755 | Vesta 16t GPU (TSV) | 231.3 s | 455 | **5.00×** |
| Good Will Hunting | 1,755 | Vesta 16t GPU (APKG) | 231.9 s | 454 | **4.99×** |
| Zootopia | 1,698 | subs2srs (TSV) | 973.6 s | 105 | — |
| Zootopia | 1,698 | Vesta 1t Direct (TSV) | 843.7 s | 121 | **1.15×** |
| Zootopia | 1,698 | Vesta 1t Direct (APKG) | 843.7 s | 121 | **1.15×** |
| Zootopia | 1,698 | Vesta 1t GPU (TSV) | 687.2 s | 148 | **1.42×** |
| Zootopia | 1,698 | Vesta 1t GPU (APKG) | 687.8 s | 148 | **1.42×** |
| Zootopia | 1,698 | Vesta 16t Direct (TSV) | 334.2 s | 305 | **2.91×** |
| Zootopia | 1,698 | Vesta 16t Direct (APKG) | 333.9 s | 305 | **2.92×** |
| Zootopia | 1,698 | Vesta 16t GPU (TSV) | 279.0 s | 365 | **3.49×** |
| Zootopia | 1,698 | Vesta 16t GPU (APKG) | 279.6 s | 364 | **3.48×** |
| Snatch | 1,522 | subs2srs (TSV) | 955.0 s | 96 | — |
| Snatch | 1,522 | Vesta 1t Direct (TSV) | 917.6 s | 100 | **1.04×** |
| Snatch | 1,522 | Vesta 1t Direct (APKG) | 918.3 s | 99 | **1.04×** |
| Snatch | 1,522 | Vesta 1t GPU (TSV) | 671.7 s | 136 | **1.42×** |
| Snatch | 1,522 | Vesta 1t GPU (APKG) | 665.6 s | 137 | **1.43×** |
| Snatch | 1,522 | Vesta 16t Direct (TSV) | 434.6 s | 210 | **2.20×** |
| Snatch | 1,522 | Vesta 16t Direct (APKG) | 435.9 s | 209 | **2.19×** |
| Snatch | 1,522 | Vesta 16t GPU (TSV) | 309.5 s | 295 | **3.09×** |
| Snatch | 1,522 | Vesta 16t GPU (APKG) | 277.6 s | 329 | **3.44×** |
| Trainspotting | 1,384 | subs2srs (TSV) | 685.0 s | 121 | — |
| Trainspotting | 1,384 | Vesta 1t Direct (TSV) | 663.1 s | 125 | **1.03×** |
| Trainspotting | 1,384 | Vesta 1t Direct (APKG) | 663.2 s | 125 | **1.03×** |
| Trainspotting | 1,384 | Vesta 1t GPU (TSV) | 460.9 s | 180 | **1.49×** |
| Trainspotting | 1,384 | Vesta 1t GPU (APKG) | 460.9 s | 180 | **1.49×** |
| Trainspotting | 1,384 | Vesta 16t Direct (TSV) | 309.2 s | 269 | **2.22×** |
| Trainspotting | 1,384 | Vesta 16t Direct (APKG) | 310.7 s | 267 | **2.20×** |
| Trainspotting | 1,384 | Vesta 16t GPU (TSV) | 176.8 s | 470 | **3.87×** |
| Trainspotting | 1,384 | Vesta 16t GPU (APKG) | 176.6 s | 470 | **3.88×** |
| In Bruges | 1,171 | subs2srs (TSV) | 830.3 s | 85 | — |
| In Bruges | 1,171 | Vesta 1t Direct (TSV) | 748.6 s | 94 | **1.11×** |
| In Bruges | 1,171 | Vesta 1t Direct (APKG) | 748.0 s | 94 | **1.11×** |
| In Bruges | 1,171 | Vesta 1t GPU (TSV) | 539.2 s | 130 | **1.54×** |
| In Bruges | 1,171 | Vesta 1t GPU (APKG) | 540.0 s | 130 | **1.54×** |
| In Bruges | 1,171 | Vesta 16t Direct (TSV) | 481.5 s | 146 | **1.72×** |
| In Bruges | 1,171 | Vesta 16t Direct (APKG) | 481.5 s | 146 | **1.72×** |
| In Bruges | 1,171 | Vesta 16t GPU (TSV) | 257.6 s | 273 | **3.22×** |
| In Bruges | 1,171 | Vesta 16t GPU (APKG) | 258.0 s | 272 | **3.22×** |
| Detour | 1,018 | subs2srs (TSV) | 371.5 s | 164 | — |
| Detour | 1,018 | Vesta 1t Direct (TSV) | 299.6 s | 204 | **1.24×** |
| Detour | 1,018 | Vesta 1t Direct (APKG) | 299.3 s | 204 | **1.24×** |
| Detour | 1,018 | Vesta 1t GPU (TSV) | 309.2 s | 198 | **1.20×** |
| Detour | 1,018 | Vesta 1t GPU (APKG) | 310.5 s | 197 | **1.20×** |
| Detour | 1,018 | Vesta 16t Direct (TSV) | 60.6 s | 1,008 | **6.13×** |
| Detour | 1,018 | Vesta 16t Direct (APKG) | 61.1 s | 1,000 | **6.08×** |
| Detour | 1,018 | Vesta 16t GPU (TSV) | 111.6 s | 547 | **3.33×** |
| Detour | 1,018 | Vesta 16t GPU (APKG) | 112.5 s | 543 | **3.30×** |

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

