# Rust 1.98–1.99 review — 2026-10-09

Reviewed the installed Rust 1.99.0 toolchain, workspace manifests, subtitle
encoding/probing, audio preprocessing, snapshot batching, CI and existing
benchmark evidence. This is a focused review, not an exhaustive correctness or
performance audit of every module.

## Release changes relevant to Vesta

Primary sources: [1.98 announcement](https://blog.rust-lang.org/2026/08/20/Rust-1.98.0/),
[1.99 announcement](https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/) and
[complete release notes](https://doc.rust-lang.org/releases.html).

| Change | Assessment / action |
| --- | --- |
| 1.98 integer `NumBuffer` / `format_into` | Candidate if profiling identifies decimal formatting as significant. Timestamp formatting currently needs padding and separators; no wholesale replacement justified. Vesta does not directly depend on `itoa`; transitive users cannot be removed by changing our code. |
| 1.98 UTF-16 LE/BE decoding | Standard alternatives now exist. Keep `encoding_rs` because legacy subtitle detection/decoding still needs it; replacing only UTF-16 would add another decoding path without removing a dependency. Corrected detection precedence instead. |
| 1.98 algebraic floating-point operations | Possible experiment for WAV downmix or energy calculations. Reassociation changes numerical results; require a release-profile microbenchmark plus waveform/VAD/timing comparisons before adoption. No automatic fast-math conversion. |
| 1.98 substring ranges, circumfix stripping, atomic slice helpers | No compelling use found in the reviewed paths. Avoid mechanical syntax changes. |
| 1.99 owned lossy UTF-8 conversion | Adopted for the bounded embedded subtitle preview: the byte vector is no longer needed, and valid UTF-8 can reuse its allocation. No measured end-to-end speedup claimed. Borrowed stderr formatting should remain borrowed. |
| 1.99 C variadic definitions, raw layout / Box / Vec APIs | No need in the reviewed application FFI, which uses a fixed-signature Whisper callback. No new unsafe code justified. |
| 1.99 inclusive-range iterator optimization | Automatic compiler/library benefit where applicable. No exhausted-range state reuse found in the reviewed paths. |
| 1.99 Cargo workspace `default-features` override | No inherited dependency override of this form currently found; no immediate change needed. |
| 1.99 CI incremental compilation disabled by default | CI already explicitly sets `CARGO_INCREMENTAL=0`; retaining that is clear and compatible. The new `debug` profile does not justify changing our customized development profile. |
| Compatibility changes and lints | Full workspace tests, formatting and strict Clippy are the appropriate local checks. Linux validation does not establish Windows FLS behavior or GPU/FFI correctness on every platform. |

## Implemented

- Rust minimum raised from 1.97 to 1.99 in workspace metadata and README.
  Every first-party package now inherits `rust-version`; a workspace declaration
  alone does not apply it to member packages. Vendored code is unchanged.
- BOM-less UTF-16 detection runs before the strict UTF-8 fast path. Pure ASCII
  UTF-16 is also byte-valid UTF-8 and previously leaked NULs into parsed text.
  Detection remains heuristic: ambiguous text containing many NULs may be treated
  as UTF-16; ordinary UTF-8, Unicode and isolated-NUL cases are covered.
- FFprobe JSON interpretation extracted into a pure function, separate from
  timeout/process management; this keeps contract tests fast and deterministic.
- Embedded preview adopts `String::from_utf8_lossy_owned` (Rust 1.99).

Five added tests cover ASCII UTF-16 in both endian orders; BOM-selected endian,
surrogate pairs, malformed surrogates and odd trailing bytes; short/empty/Unicode
UTF-8 and isolated NUL; probe language/title defaults, Unicode tags, bitmap codecs,
invalid/missing/out-of-range indices; empty probe responses and invalid JSON.
The ASCII UTF-16 regression test was run against the original decoder and failed
with the expected NUL-containing output. It passes with the fix.

## Further decoupling and tests worth prioritizing

The follow-up implements the audio findings rather than leaving them as
recommendations:

- Conversion and segmentation share a private async process runner. Child
  ownership uses `kill_on_drop`; explicit cancellation kills and reaps the child.
  Stdin is disabled. Waiting and draining stderr run concurrently with
  `try_join!`, without a detached reader task. Read/wait/cleanup errors are
  propagated, and failed commands report their exit status and stderr tail.
- Stderr is continuously drained but only the last 64 KiB are retained.
- Chunk enumeration propagates directory/metadata errors and excludes directories
  that happen to have chunk-like filenames.
- WAV decoding propagates sample errors with sample index and file context,
  rejects incomplete frames and non-finite samples, and fixes the signed-shift
  normalization bug for 32-bit PCM. A generic streaming downmix replaces six
  duplicated paths and removes the scratch vector for multichannel frames.
  Valid mono/stereo arithmetic order is preserved; malformed files now fail
  explicitly instead of silently shortening the audio.
- The local transcription pipeline retains its `TempPath` owner across awaits;
  conversion, decoding and transcription errors, as well as future cancellation,
  now clean up the temporary WAV through RAII.

Eleven additional audio tests exercise 8/16/24/32-bit PCM and float channel
averaging, empty WAV, truncated samples, malformed headers, missing files,
NaN/infinity, incomplete frames, downmix overflow, bounded stderr and read errors,
spawn failure, pre-cancellation, real FFmpeg conversion/segmentation/failure,
large stderr on nonzero exit, and killing/reaping a child on token cancellation
or task abort. Unix-specific process lifecycle tests ran on Linux; they do not
establish the same lifecycle behavior on Windows.

The nearby production `unwrap` cases in subtitle normalization and segment
merging follow explicit nonempty checks; the `NonZeroU32` construction in the
rate limiter clamps to at least one. Those invariants justify the calls.
Defaults for optional cloud metadata and best-effort cleanup are not automatically
errors: each fallback needs a contract-specific assessment. Strict Clippy across
all targets complements this focused manual review; it is not an exhaustive
idiomaticity audit of the whole repository.

The existing crate boundaries already separate engines from GUI adapters. Avoid
splitting more crates solely by file size; prefer narrow pure-function seams
such as the FFprobe parser when they enable useful tests.

## Optimization recommendation

Continue only with a measured bottleneck and an output-equivalence check.
The [native A/B matrix](../BENCHMARK_NATIVE.md) already records 480 generations,
byte-identical media and note fields, and approximately 15.3% less elapsed time
on complete-film aggregates. It also shows cases where batching is neutral.

Next useful experiments are short HEVC/1080p preparation workloads, sparse versus
nearby seeks, worker-count/RSS scaling and WAV preprocessing on large inputs.
Keep the existing conservative CFR/keyframe fallback until a new corpus proves
quality equivalence. Profile FFmpeg and Whisper separately from Rust formatting:
minor string allocation savings should not be presented as media throughput gains.
Do not repeat the entire film matrix for this encoding/probe change.

## Validation

- `cargo test --workspace`: passed, including Vesta, whisper-bench, media
  integration tests, golden subtitle regression hashes and doc tests.
- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.

No frontend code changed. No installed-bundle, real GPU, Windows/macOS or
release-performance validation was performed for this patch.

The subsequent [cross-workspace ownership and idiomaticity review](rust-workspace-2026-10-09.md)
extends this work to every first-party package, consolidates downloads and
translation execution, and adds lifecycle/persistence regression coverage.
