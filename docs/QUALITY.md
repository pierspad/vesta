# Stability and performance checks

Prioritize correct exports, recoverable failures, durable settings, cancellation,
and usable progress. Optimize bottlenecks after measuring them on identical input.
A passing unit suite is evidence about those cases, not proof that every desktop
interaction or third-party API works on every supported machine.

## One command

From the repository root, after `npm ci` in `apps/srt-gui`:

```bash
python3 build-scripts/quality_check.py --desktop --smoke
```

Without `--desktop`, the Rust gate excludes both desktop applications and runs
libraries/CLIs; this avoids requiring Tauri/Vulkan build dependencies. `--desktop`
includes Vesta native tests. The separate Whisper benchmarking app is covered by
`cargo test --workspace`, not this focused gate. FFmpeg and FFprobe are required
for the new media integration tests. Initial dependency/native compilation can
be substantial; warmed tests and the synthetic smoke run are short.

The gate runs Svelte/TypeScript checks, frontend tests, i18n validation, production
frontend build, Rust tests, and internal version consistency. Before pushing,
also run the repository's required formatting and strict Clippy checks:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Short smoke benchmark

```bash
cargo build -p srt-flashcards-cli
python3 build-scripts/quality_smoke.py --samples 3
```

The script creates a 20-second synthetic film with ten subtitle lines, runs
preview and APKG media generation with two workers, and cleans up its temporary
outputs. Every sample checks ZIP integrity, the numbered media manifest, nonempty
media files, SQLite integrity, and exactly ten notes. A missing FFmpeg fails
explicitly. It prints machine/profile information and per-run, minimum, maximum,
and median wall time. Each subprocess has a finite timeout.

Build time and fixture creation are outside the reported pipeline measurements.
Debug results are useful for local regressions, not release-performance claims.
For optimization comparisons use the same machine, FFmpeg, inputs, settings,
cache state and release profile, with no other heavy jobs running:

```bash
cargo build --release -p srt-flashcards-cli
python3 build-scripts/quality_smoke.py --profile release --samples 5
```

Keep correctness assertions. Investigate a repeated median regression before
adding a hard CI timing threshold; shared runners are noisy. This short fixture
does not measure HEVC seek costs, film-length source preparation, Whisper quality,
network latency, GPU fallback, or large-series memory use. Use the
[full-film benchmark guide](BENCHMARK_STEPS.md) for representative throughput
and add a short HEVC/1080p workload before changing preparation policy again.

## HTTP contracts and links

`cargo test -p srt-transcribe` exercises real loopback HTTP requests for OpenAI
JSON transcription, Groq translation, Deepgram and AssemblyAI upload/create/poll,
plus an authorization failure. No credentials, uploaded user media, or billable
inference are involved. Model discovery tests cover URL normalization, Gemini
capability filtering and HTTP errors. A frontend source test verifies that every
literal IPC invocation is registered in the desktop handler; dynamic commands
and argument schemas need additional integration coverage.

Run the optional online link check separately:

```bash
python3 build-scripts/check_provider_links.py > provider-links.json
```

It probes the key/documentation URLs and model-discovery URLs from the checked-in
catalogs, with six workers and 12-second request timeouts. It does not make
inference or upload requests. Login redirects, HTTP 401 and HTTP 403 are recorded
as access requirements, not authenticated successes. Reachability cannot prove
that a model remains enabled for a particular account. Review redirections and
non-success results rather than treating this inventory as a release certificate.

Before release, use explicit test accounts for the enabled cloud providers and a
short, known speech fixture. Verify successful translation/transcription, expected
language/timing, invalid keys, unsupported models, 429, offline/timeout, partial
output and cancellation. Avoid retrying billable requests without bounds. Local
cancellation of an AssemblyAI polling future does not delete its remote job.

## Release smoke matrix

Test installed bundles on Linux and Windows, and on macOS if distributing there:

- Complete setup, restart twice, verify language/media/export defaults persist;
  disk-full/unreadable/corrupt settings must show an error without replacing data.
- Generate audio + snapshots, TSV, APKG, video clips and a two-episode merged APKG.
  Open the package in Anki and play the actual card media. Confirm no dangling
  references, collisions or overwritten episode exports.
- Save embedded SRT tracks with language/title labels. Verify PGS/VobSub are
  clearly marked as requiring OCR; failed extraction preserves existing output.
- Preview common codecs and seek; check zero-length/missing files, byte ranges,
  audio backend/decoder failures and recoverable error messages.
- Exercise manual retiming/session save-load and Whisper-assisted autosync with
  an installed model; compare against a known offset, not only successful return.
- Run local Whisper on CPU and an actual supported GPU; test unavailable GPU
  fallback, interrupted model downloads and model removal.
- Translate/annotate using real configured endpoints; test failover, resume,
  empty/malformed JSON, rate limits and cancellation.
- Open output filenames and their folders; verify `.apkg` association with Anki
  and useful errors when the file/application no longer exists.

A web preview with mocked IPC can check presentation and disabled controls but
cannot validate native dialogs, file associations, WebKit/GStreamer playback or
GPU behavior. Keep this distinction in release notes and test reports.

For development CSS regressions, run `npm run test:dev-css` in `apps/srt-gui`.
It starts temporary Vite servers on ephemeral ports, requests component CSS before
component JavaScript, checks scoped style output, and closes each server. The
standard Python quality gate runs this automatically. CI and semantic release
also run it along with a frontend production build before publication.

The full localization gate also rejects untranslated English prose (`same_as_english`).
All 15 catalogs must provide every English key and preserve its interpolation tokens.
Genuine identical terms are explicitly allowed, with language-specific exceptions
where appropriate. This is stronger than checking only whether a key exists. Setup,
settings copy and accessibility labels now use the same catalogs. Names of products,
file formats and measurement units remain untranslated.

## Measured snapshot integration

The native Rust batching change has a complete 480-generation A/B matrix with
byte-identical media and note fields. See [the evidence and scope](BENCHMARK_NATIVE.md).
The full subs2srs suite uses different product defaults and media counts and
cannot replace the quality-equivalence check.

## Publishing benchmark charts

The README's aggregate chart is generated from the completed suite CSV by
`benchmarking_against_subs2srs/report/generate_full_report.py`. Durations are sums
of per-film medians, not sums of every repetition or averages of speedup ratios.
Time saved is the baseline total minus the measured total; percentage saved uses
the baseline total as its denominator. Keep all films matched across compared
series and label export formats, worker counts, hardware and output differences.
Per-film charts and raw measurements belong in the detailed report.
