# Benchmark pipeline

## TL;DR

```bash
./benchmarking_against_subs2srs/1_compile_subs2srs.sh    # builds the headless subs2srs harness (mono/mcs)
./benchmarking_against_subs2srs/2_compile_vesta.sh       # builds the Vesta flashcard CLI (cargo, release)
./benchmarking_against_subs2srs/3_run_benchmarks.sh      # times both over the test media → results/results.csv
./benchmarking_against_subs2srs/4_generate_report.sh     # → results/benchmark.svg + results/films/*.svg + results/summary.md
```

Test media are **auto-discovered** from `Test_Subs/FILM/` (see below); edit
[`config.sh`](config.sh) only to change the Vesta variants, languages or media
folder. Nothing here modifies the apps; everything writes under
`benchmarking_against_subs2srs/results/` and `benchmarking_against_subs2srs/.work/` (both gitignored).

## Test media auto-discovery

`config.sh` scans `Test_Subs/FILM/` (override with `BENCH_MEDIA_DIR`) for films
laid out as:

```
<Title>-en.srt                      # target subtitles   (BENCH_TARGET_LANG, default en)
<Title>-it.srt                      # native subtitles   (BENCH_NATIVE_LANG, default it)
<Title>*.mp4|mkv|avi|m4v|webm       # video (the name just has to start with <Title>)
```

A film is benchmarked **only if all three files exist**; anything incomplete
(e.g. `8 e mezzo.mp4`, which ships without subtitles) is skipped with a warning.
To add a film, drop the three files into the folder and re-run — no config edits.

## How the comparison is kept fair

The goal is to compare **pure execution time for the same work**, so:

* **Two real headless CLIs.** Both tools run from the command line with no GUI.
  Vesta's CLI (`srt-flashcards`) is a thin shell over the real engine. subs2srs
  ships only as a .NET 3.5 WinForms GUI with **no command line**, so we compile a
  headless harness (see below) that drives its *actual* generation classes.
* **No invented parallelism.** subs2srs is run exactly as written —
  single-threaded, sequential ffmpeg calls. We add nothing it didn't have. Vesta
  runs on its "max" plan: `cores-1` parallel ffmpeg workers (the GUI's top
  setting). The speed-up therefore reflects a genuine architectural difference,
  not a handicap.
* **A core-for-core control.** Alongside the "max" plan, Vesta is also timed with
  a **single** ffmpeg worker (`-j 1`), the apples-to-apples match for subs2srs's
  one-at-a-time pipeline. This isolates raw per-card efficiency from the
  parallelism win: each film is charted as 5 series — subs2srs (TSV), Vesta
  export TSV/APKG at max cores, and Vesta export TSV/APKG at one core.
* **Identical inputs and outputs.** Same subtitles, same video, same media types
  (audio + snapshot + video clip). Both produce the same number of cards/clips
  (the runner prints the counts so you can check).
* **Wall-clock timing** of the whole invocation, optionally repeated
  (`REPEATS=3 ./3_run_benchmarks.sh` reports the median).

## The subs2srs headless harness (`subs2srs-headless/`)

The harness compiles from the vendored subs2srs source under
[`subs2srs_source_code/`](subs2srs_source_code) (GPLv3, kept whole with its
copyright notices and `gpl.txt`, so the build is fully reproducible from this
repo). subs2srs's generation logic (`WorkerSubs/Srs/Audio/Snapshot/Video`,
`Utils*`, `Settings`) is cleanly separable from its WinForms UI. The harness:

* compiles those **real** source files unchanged, and
* swaps exactly three GUI types for headless no-op stubs (`Stubs.cs`):
  `DialogProgress` (progress/cancel — already a no-op when not shown),
  `UtilsMsg` (message boxes → stderr), and a tiny `TagLib` shim (ID3 tagging,
  not part of the timed work), and
* replicates `SubsProcessor.bw_DoWork`'s exact sequence in `Program.cs`
  (`combineAllSubs → inactivateLines → genSrs → genAudioClip → genSnapshots →
  genVideoClip`).

On Linux, subs2srs invokes `Utils/ffmpeg/ffmpeg.exe`; `1_compile_subs2srs.sh`
symlinks that to the system ffmpeg, leaving subs2srs's code untouched.

> Note: subs2srs only exports TSV (+ a media folder), so it is benchmarked in
> TSV mode only. Vesta is benchmarked in both TSV and APKG. For APKG the media is
> embedded inside the `.apkg`, so the on-disk media counts read 0 — the timing is
> still the full deck build.

## Comparing Vesta variants

Each variant is `label|binary|jobs`, where `jobs` is `MAX` (the cores-1 plan), a
literal worker count, or `1` for the single-core control. The default config
ships the max and single-core variants of the local build:

```bash
vesta_VARIANTS=(
  "max|target/release/srt-flashcards|MAX"
  "single|target/release/srt-flashcards|1"
  # race a branch/fork too, e.g.:
  # "experiment|/path/to/other-checkout/target/release/srt-flashcards|MAX"
)
```

`3_run_benchmarks.sh` runs every variant × every format and the report charts
them side by side. A local, gitignored `config.local.sh` can override `TEST_MEDIA`
/ `vesta_VARIANTS` without touching the tracked config.

## Files

| File | Role |
|---|---|
| `config.sh` | Media auto-discovery, Vesta variants, formats, parallelism, paths. |
| `lib/common.sh` | Shared bash helpers (logging, timing, median). |
| `1_compile_subs2srs.sh` | Build `subs2srs-headless.exe` + wire ffmpeg. |
| `2_compile_vesta.sh` | `cargo build --release -p srt-flashcards-cli`. |
| `3_run_benchmarks.sh` | Time every tool/variant/format → `results/results.csv`. |
| `4_generate_report.sh` | SVG charts (`report/plot.py`: combined + one per film) + `results/summary.md`. |
| `subs2srs-headless/` | The headless harness sources (`Program.cs`, `Stubs.cs`). |
| `subs2srs_source_code/` | Vendored subs2srs source (GPLv3) the harness compiles. |

## Requirements

`cargo`, `ffmpeg`/`ffprobe`, and Mono (`mcs` + `mono`) for the subs2srs harness.
The chart is rendered with **matplotlib** (`report/plot.py`); `4_generate_report.sh`
prefers the repo's `.venv` (which ships it) and otherwise falls back to system
`python3` — install matplotlib there with `python3 -m pip install matplotlib`.
