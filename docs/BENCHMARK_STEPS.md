# Reproducing the Vesta vs subs2srs benchmark

This procedure compares the two media-generation engines without GUI overhead. Both process the same subtitle/media inputs and invoke the same host `ffmpeg` binary.

## What is measured

- Eight feature-length movies and roughly 12,000 subtitle lines.
- MP3 audio clips, snapshots, MP4 clips, and card output.
- subs2srs sequential TSV generation as the `1.0×` baseline.
- Vesta with one worker and with all 16 test-machine threads.
- Vesta direct extraction and optional VA-API pre-transcoding.
- Vesta TSV and APKG packaging.

The runner verifies card and media counts. The reported time is wall-clock time for the complete command, including packaging.

## Fairness controls

1. The vendored subs2srs processing classes are compiled into a headless harness; only their WinForms presentation dependencies are replaced with no-op adapters.
2. Both programs use `/usr/bin/ffmpeg` on the host.
3. `-j 1` isolates Vesta pipeline overhead from parallelism; `-j 16` measures the configured machine's full throughput.
4. A direct variant matches subs2srs extraction. GPU variants include the pre-transcode time rather than hiding it.
5. Every variant receives the same SRT pairs and media files.

The harness is under `benchmarking_against_subs2srs/`. The original subs2srs source and license notices remain vendored there for reproducibility.

## Requirements

Ubuntu/Debian packages:

```bash
sudo apt update
sudo apt install -y mono-mcs ffmpeg python3 python3-matplotlib
```

Install a current Rust toolchain if needed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

VA-API variants additionally require a working VA-API driver and an encoder supported by the installed FFmpeg build. Direct variants do not require a GPU.

## Procedure

### 1. Compile both programs

From the repository root:

```bash
./benchmarking_against_subs2srs/1_compile_subs2srs.sh
cargo build --release -p srt-flashcards-cli
```

The outputs are:

- `benchmarking_against_subs2srs/subs2srs-headless/subs2srs-headless.exe`
- `target/release/srt-flashcards`

### 2. Prepare inputs

Place one study-language SRT, one reference SRT, and one matching media file in `Test_Subs/FILM/`:

```text
Test_Subs/FILM/
├── Movie-en.srt
├── Movie-it.srt
└── Movie.(mp4|mkv|avi|webm)
```

`benchmarking_against_subs2srs/config.sh` controls discovery and language suffixes. Copyrighted test media is intentionally not stored in the repository.

### 3. Run the suite

```bash
./run_overnight_benchmarks.sh
```

Choose a full run to replace prior measurements, or the missing-only mode to keep completed combinations. Do not use the machine for unrelated CPU/GPU work while recording results.

### 4. Generate the report

```bash
python3 benchmarking_against_subs2srs/report/generate_full_report.py \
  benchmarking_against_subs2srs/results/results_full.csv \
  benchmarking_against_subs2srs/results
```

The generator updates the Markdown summary and SVG charts copied into `docs/` and `docs/films/`.

## Test machine results

Recorded on an AMD Ryzen 7 5800X (8 cores/16 threads) and Radeon RX 7800 XT:

| Configuration | Total suite time | Average speedup | Peak speedup |
|---|---:|---:|---:|
| subs2srs TSV baseline | 127.9 min | 1.00× | 1.00× |
| Vesta 1t Direct | 99.5 min | 1.29× | 1.89× |
| Vesta 16t Direct | 43.3 min | 3.52× | 6.22× |
| Vesta 16t GPU | 33.7 min | 3.78× | 5.00× |

These values describe this hardware and dataset; they are not universal performance guarantees. See the [full report](BENCHMARK_REPORT.md) for per-movie timings and all TSV/APKG variants.
