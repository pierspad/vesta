# Reproducing Vesta benchmarks

## Two scopes, two datasets

The [native Rust A/B suite](BENCHMARK_NATIVE.md) isolates nearby snapshot batching
at byte-identical quality: audio + WebP + APKG, without video clips. The
[subs2srs comparison](BENCHMARK_REPORT.md) measures full product pipelines,
including video clips, with their different encoding defaults and output counts.
Do not combine these datasets or attribute the entire product speed ratio to
one optimization. CPU worker limits are concurrent-process limits, not affinity.

## Native snapshot A/B

From the repository root, when the machine is otherwise idle:

```bash
./benchmarking/snapshot_batch/run-benchmark.sh --dry-run
./benchmarking/snapshot_batch/run-benchmark.sh
```

The default matrix uses all paired local films, samples and full subtitles,
three alternating pairs, two resolutions and sparse requests. Python only runs
and validates the native release binary; it does not optimize media. Results
are stored outside the checkout in `../vesta-benchmark-results/`. Generate charts
from a completed result file without repeating the measurements:

```bash
./benchmarking/snapshot_batch/plot-results.sh /path/to/results/results.json
```

See the [runner documentation](../benchmarking/snapshot_batch/README.md) for
resume, timeouts, input selection, quality checks and optional RSS sampling.

## Full product comparison against subs2srs

Requires Rust/Cargo, Mono (`mcs` and `mono`), FFmpeg/FFprobe, Python 3 and
Matplotlib (the report scripts prefer `.venv` when present). Install the native
Tauri requirements only when building the desktop, not for these headless CLIs.

Put `Film-en.srt`, `Film-it.srt` and the matching video in `Test_Subs/FILM/`.
The script discovers complete pairs; media and non-public-domain subtitles
remain local. `8 e mezzo.mp4` was excluded because subtitles were absent.

```bash
./benchmarking_against_subs2srs/1_compile_subs2srs.sh
./benchmarking_against_subs2srs/2_compile_vesta.sh
set -o pipefail
REPEATS=3 ./benchmarking_against_subs2srs/3_run_benchmarks.sh </dev/null 2>&1 | tee benchmark-subs2srs.log
REPEATS=3 ./benchmarking_against_subs2srs/4_generate_report.sh
```

The 2026-10-08 run completed 120 generations in 17 h 16 min, yielding 40
median rows: eight films × subs2srs TSV plus Vesta TSV/APKG with one or 16
requested workers × three repetitions. The runner reports media counts,
including WebP and embedded APKG manifests. This is a count report, not a
byte-identical/equal-quality assertion. Some subs2srs media counts are smaller
than input counts; Vesta produced all 12,859 requested media sets.

`results/results.csv` is overwritten on a new run; preserve old results first.
Logs are local and ignored. Each cell's repetitions are sequential and use
uncontrolled OS cache. Avoid unrelated workload or concurrent benchmarks.
Video preparation may select hardware or CPU automatically; the current suite
does not instrument encoder selection or contain a new separate GPU series.
Historical GPU/full-matrix CSVs are not inputs to the current report.

To generate the published overview, speed ratios and per-film charts from only
the current CSV (Matplotlib required):

```bash
.venv/bin/python3 benchmarking_against_subs2srs/report/generate_full_report.py \
  benchmarking_against_subs2srs/results/results.csv \
  benchmarking_against_subs2srs/results
```

This refreshes `docs/` charts/report and never rewrites the source CSV. If there
is no virtual environment, use a Python interpreter with Matplotlib installed.
[Individual repetition times](benchmarks/subs2srs-repeats-2026-10-08.csv) are
retained separately from the [median CSV](../benchmarking_against_subs2srs/results/results.csv).
The product ratios (4.13× multicore, 1.58× one worker) and native A/B reduction
(15.3%) answer different questions and use different aggregation methods.
