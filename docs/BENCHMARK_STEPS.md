# Benchmarking Methodology & Step-by-Step Reproduction Guide

This document explains the technical methodology, fairness controls, and step-by-step reproduction instructions for the benchmark suite comparing **Vesta** against **subs2srs**.

---

## 1. How the Comparison is Kept 100% Fair

Benchmarking media-processing software is notoriously prone to methodology bias (e.g. comparing a GUI against a CLI, using different codecs, or benchmarking on artificial short clips). To ensure an **apples-to-apples, scientifically rigorous comparison**, the following controls are strictly enforced:

### A. Real Headless CLIs with Zero GUI Overhead
- **subs2srs**: Historically ships only as a Windows .NET 3.5 WinForms GUI application without a command-line interface. Testing it through a GUI would unfairly penalize it with window-manager rendering and message-pump latency. Instead, we compile a **headless CLI harness** directly from the genuine subs2srs C# source code (see details below).
- **Vesta**: Runs from its headless release binary (`srt-flashcards-cli`), which executes the identical processing engine used by the GUI application.

### B. The Identical System FFmpeg Engine
Both tools rely on `ffmpeg` under the hood:
- On Linux, subs2srs looks for `Utils/ffmpeg/ffmpeg.exe`.
- The compilation script creates a symlink pointing `Utils/ffmpeg/ffmpeg.exe` to `/usr/bin/ffmpeg` (the host system's FFmpeg binary).
- **Both tools execute the exact same FFmpeg executable** with the exact same system codecs, eliminating any discrepancies in encoder versions or CPU optimizations.

### C. A Core-for-Core Single-Threaded Control
- subs2srs was architected sequentially: 1 card at a time, 1 FFmpeg process at a time.
- To measure Vesta's pure per-card algorithmic efficiency separately from its multi-core parallelism, Vesta is benchmarked in a **single-core control mode** (`-j 1`).
- This proves that even on a single core, Vesta runs **~1.3× to 1.9× faster** than subs2srs due to efficient stream selection and modern pipeline design.

### D. Multi-Core Scaling (All Logical Cores)
- Vesta is also tested in its standard default configuration with parallel worker threads (`-j 16`, matching all logical cores of the host CPU).
- The speedup accurately reflects the real-world architectural advantage of modern multi-threaded execution.

### E. Hardware GPU Acceleration (VA-API / NVENC)
- When transcoding heavy 1080p and HEVC videos, Vesta offers an optional hardware-accelerated pipeline.
- The GPU pre-transcodes intermediate streams at 50–100× realtime in ~1–2 minutes, after which workers cut audio, snapshots, and video clips from the lightweight stream, avoiding repeated high-overhead decoding of massive video files.

### F. Identical Inputs & Outputs
- **8 Feature-Length Films** (~12,000+ total dialogue lines) spanning various codecs, containers (MP4, MKV, AVI), resolutions, and bitrates.
- **Identical Outputs Produced**:
  - Audio snippets (MP3)
  - Snapshots (JPEG/WebP)
  - Video clips (MP4)
  - Tab-delimited TSV cards and self-contained Anki `.apkg` SQLite decks.
- Verification checks count and assert that both tools produced the exact same number of media files for every film.

---

## 2. The subs2srs Headless Harness (`benchmarking_against_subs2srs/`)

All source code for the subs2srs harness is **fully vendored in this repository** under `benchmarking_against_subs2srs/`:

```
benchmarking_against_subs2srs/
├── subs2srs_source_code/      # Genuine, untouched C# source of subs2srs (GPLv3)
│   └── subs2srs/              # WorkerSubs, WorkerAudio, WorkerSnapshot, WorkerVideo...
├── subs2srs-headless/         # Headless CLI entry point & GUI stubs
│   ├── Program.cs             # Replicates SubsProcessor.bw_DoWork execution flow
│   └── Stubs.cs               # No-op stubs for WinForms GUI dialogs
├── 1_compile_subs2srs.sh      # Compiles the harness using Mono (mcs)
├── 2_compile_vesta.sh         # Compiles Vesta in release mode
├── 3_run_benchmarks.sh        # Standard benchmark runner
├── config.sh                  # Media discovery & test configuration
└── report/
    ├── generate_full_report.py# Generates SVGs and Markdown reports
    ├── plot.py                # Grouped CPU benchmark plotter
    └── plot_gpu.py            # GPU vs CPU benchmark plotter
```

### How the Headless Harness Works:
1. **Unchanged Compute Logic**: All genuine subs2srs generation classes (`WorkerSubs`, `WorkerAudio`, `WorkerSnapshot`, `WorkerVideo`, `Utils*`, `Settings`) are compiled directly.
2. **WinForms GUI Replacement**: Exactly three UI-dependent types are replaced with no-op implementations in [`Stubs.cs`](file:///home/ribben/Desktop/pierspad/github/vesta/benchmarking_against_subs2srs/subs2srs-headless/Stubs.cs):
   - `DialogProgress`: Progress dialog methods become no-ops.
   - `UtilsMsg`: Message boxes route silently to `stderr`.
   - `TagLib`: Audio ID3 tagging shim.
3. **Execution Sequence**: [`Program.cs`](file:///home/ribben/Desktop/pierspad/github/vesta/benchmarking_against_subs2srs/subs2srs-headless/Program.cs) invokes the exact workflow defined in `SubsProcessor.bw_DoWork`:
   ```csharp
   combineAllSubs();
   inactivateLines();
   genSrs();
   genAudioClip();
   genSnapshots();
   genVideoClip();
   ```

---

## 3. Step-by-Step Reproduction Guide

To run the complete benchmark suite on your own machine:

### Prerequisites
On Ubuntu/Debian:
```bash
sudo apt update
sudo apt install -y mono-mcs ffmpeg python3 python3-matplotlib
```
Rust toolchain:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Step 1: Compile the Headless subs2srs Harness
```bash
./benchmarking_against_subs2srs/1_compile_subs2srs.sh
```
This produces `benchmarking_against_subs2srs/subs2srs-headless/subs2srs-headless.exe` and sets up the FFmpeg symlink.

### Step 2: Compile Vesta
```bash
cargo build --release -p srt-flashcards-cli
```
The optimized release binary is located at `target/release/srt-flashcards`.

### Step 3: Place Media Files
Drop your test films in `Test_Subs/FILM/` following the naming convention:
```
Test_Subs/FILM/
  ├── <FilmTitle>-en.srt               # Target subtitles (e.g. English)
  ├── <FilmTitle>-it.srt               # Native subtitles (e.g. Italian)
  └── <FilmTitle>.(mp4|mkv|avi|webm)   # Video file
```
Any film with all three matching files is automatically discovered.

### Step 4: Run the Benchmark Suite
To run the automated suite with sleep inhibition (preventing system suspend during long runs):
```bash
./run_overnight_benchmarks.sh
```
You can select:
- `1` (or enter): Full run from scratch across all films and variants (~5–6 hours).
- `2`: Run only missing test combinations, preserving existing results.

### Step 5: Generate Visual Charts & Reports
```bash
python3 benchmarking_against_subs2srs/report/generate_full_report.py \
  benchmarking_against_subs2srs/results/results_full.csv \
  benchmarking_against_subs2srs/results
```
All generated charts are automatically synced into `docs/` and `docs/films/`.

---

## 4. Summary of Measured Results

Testing on an AMD Ryzen 7 5800X (16 threads) and AMD Radeon RX 7800 XT (VA-API):

| Configuration | Total Suite Time | Average Speedup | Peak Speedup | Best Use Case |
|---|---:|---:|---:|---|
| **subs2srs (baseline)** | 127.9 min | 1.00× | 1.00× | Legacy baseline |
| **Vesta 1-Core Control** | 99.5 min | **1.29×** | **1.89×** | Single-core parity |
| **Vesta Multi-Core Direct** | 43.3 min | **3.52×** | **6.22×** | Fast seeks / standard bitrates |
| **Vesta Multi-Core GPU** | 33.7 min | **3.78×** | **5.00×** | Heavy 1080p & HEVC videos |

For per-film measurements and charts, see the [Full Benchmark Report](BENCHMARK_REPORT.md).
