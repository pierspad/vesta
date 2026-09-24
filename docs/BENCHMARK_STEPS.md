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

## Requirements (Arch Linux)

Install the native build, benchmark, media, and reporting tools:

```bash
sudo pacman -Syu --needed base-devel rustup mono ffmpeg python python-matplotlib
rustup default stable
```

The workspace currently requires the Rust version declared by
`workspace.package.rust-version` in the root `Cargo.toml`. Confirm the tools
before starting a long run:

```bash
rustc --version
mcs --version
ffmpeg -version
python -c 'import matplotlib; print(matplotlib.__version__)'
```

Direct variants do not require a GPU. The recorded GPU variant uses VA-API
pre-transcoding. Install the inspection tool plus the driver for the machine:

```bash
sudo pacman -S --needed libva-utils

# AMD (Mesa)
sudo pacman -S --needed libva-mesa-driver

# Intel Gen 8+
sudo pacman -S --needed intel-media-driver
```

Only install the driver matching the hardware. Verify that VA-API and FFmpeg
expose an H.264 encoder before recording GPU results:

```bash
vainfo
ffmpeg -hide_banner -encoders | grep -E 'h264_(vaapi|nvenc|qsv)'
```

If no usable hardware encoder is found, Vesta falls back to `libx264` on the
CPU. Do not label that run as a GPU result.

## Procedure

### 1. Compile both programs

From the repository root:

```bash
./benchmarking_against_subs2srs/1_compile_subs2srs.sh
./benchmarking_against_subs2srs/2_compile_vesta.sh
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
REPEATS=3 ./benchmarking_against_subs2srs/3_run_benchmarks.sh
```

This replaces `results/results.csv`. For the separate VA-API run use
`benchmarking_against_subs2srs/run_vesta_gpu.sh`; use
`run_missing_subs2srs.sh` only to fill missing baseline rows. Do not use the
machine for unrelated CPU/GPU work while recording results. Keep the CPU
governor, power profile, worker count, and thermal conditions stable between
variants.

### 4. Generate the report

```bash
./benchmarking_against_subs2srs/4_generate_report.sh
```

The script records machine metadata and updates the summary/charts under
`benchmarking_against_subs2srs/results/`. To regenerate the checked-in full
report from the historical full matrix, run:

```bash
python benchmarking_against_subs2srs/report/generate_full_report.py \
  benchmarking_against_subs2srs/results/results_full.csv \
  benchmarking_against_subs2srs/results
```

## Test machine results

Recorded on an AMD Ryzen 7 5800X (8 cores/16 threads) and Radeon RX 7800 XT:

| Configuration | Total suite time | Average speedup | Peak speedup |
|---|---:|---:|---:|
| subs2srs TSV baseline | 127.9 min | 1.00× | 1.00× |
| Vesta 1t Direct | 99.5 min | 1.29× | 1.89× |
| Vesta 16t Direct | 43.3 min | 3.52× | 6.22× |
| Vesta 16t GPU | 33.7 min | 3.78× | 5.00× |

These values describe this hardware and dataset; they are not universal performance guarantees. See the [full report](BENCHMARK_REPORT.md) for per-movie timings and all TSV/APKG variants.
