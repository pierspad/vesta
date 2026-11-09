# Native Snapshot Batching Benchmark Suite

The native snapshot batching implementation lives in `lib/srt-flashcards/src/snapshot_batch.rs` and is shared by both the CLI and desktop application. `verify_native.py` is an external benchmark and verification harness that executes the compiled Rust binary (`srt-flashcards`) and validates output equivalence across A/B runs.

## Running the Benchmark Suite

From the repository root on an idle machine:

```bash
./benchmarking/snapshot_batch/run-benchmark.sh --dry-run
./benchmarking/snapshot_batch/run-benchmark.sh
```

The second command builds the release binary before starting measurements. Cargo, Python 3, FFmpeg, and FFprobe are required (`psutil` is optional for sampling RSS and process counts).

The harness recursively scans `Test_Subs/FILM` and resolves `<FilmName>-en.srt` or `<FilmName>.srt` (including `Detour(1945).mp4` with `Detour-en.srt`). Films without matching subtitles are listed as excluded.

The default benchmark matrix evaluates, for each film:

- `25`, `100`, `300`, and `all` cards using actual subtitle timestamps;
- `100` sparse cards distributed across the full film duration;
- WebP snapshots at `256×144` (application default) and `640×360`;
- Three alternating A/B pairs: baseline (`--no-optimize`) vs. optimized, reversing execution order on the second repetition to balance warm OS filesystem cache effects;
- Audio MP3 `128k` / `44.1 kHz` / stereo, track `0`, loudness normalization enabled, WebP quality `80`, zero crop/padding, and `15` requested workers (clamped to available logical cores).

Across 8 films, the full matrix comprises **480 generations**. Each generation has a default two-hour timeout (`--timeout`, where `0` disables the limit).

## Results and Resumption

Outputs are written outside the repository under `../vesta-benchmark-results/YYYYMMDD-HHMMSS/`:

- `summary.md`: median timings, speedups or regressions, completed pairs, status, and exclusions;
- `results.json`: exact commands, FFmpeg/FFprobe versions, binary and subtitle hashes, video sizes/timestamps, wall times, and SHA-256 hashes of every generated media file and note field;
- selected subtitle subsets and `stdout`/`stderr` logs per generation.

Every generated `.apkg` is checked for ZIP and SQLite integrity, card and media counts, media reference completeness, non-empty media files, and byte-for-byte identity of MP3/WebP media and Anki note fields between baseline and optimized runs. Any mismatch halts the suite with a non-zero exit code and retains the failing deck. Verified decks are deleted to conserve disk space unless `--keep-decks` is passed.

Pressing `Ctrl+C` or sending `SIGTERM` terminates active child processes and saves progress. To resume an interrupted run, pass the same arguments and output directory:

```bash
./benchmarking/snapshot_batch/run-benchmark.sh --resume --output /path/to/results
```

## Custom Profiles and Runner Tests

Run full films only at default dimensions without sparse samples:

```bash
./benchmarking/snapshot_batch/run-benchmark.sh --sizes all --profiles default --sparse 0
```

Specify a custom input directory, language, worker count, and output path:

```bash
./benchmarking/snapshot_batch/run-benchmark.sh --input /path/to/films --language it --jobs 8 --output /path/to/new-results
```

Run the harness unit tests without executing the full benchmark:

```bash
python3 -m unittest discover -s benchmarking/snapshot_batch -p test_runner.py
```

## Generating Charts from Completed Results

To render SVG and PNG charts from a completed `results.json` file (requires `matplotlib` in `.venv` or system Python):

```bash
./benchmarking/snapshot_batch/plot-results.sh /path/to/results/results.json
```

See [`docs/BENCHMARK_NATIVE.md`](../../docs/BENCHMARK_NATIVE.md) for the recorded 480-generation A/B equivalence report.
