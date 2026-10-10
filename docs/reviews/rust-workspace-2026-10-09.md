# Rust workspace idiomaticity and ownership review — 2026-10-09

This follow-up covers all 18 first-party Cargo packages: the core crates, eight
engines, five CLIs and both desktop apps. The inventory contains 82 Rust files,
including build scripts and tests. Manual review concentrates on ownership,
process/task lifetime, fallible I/O, duplicated implementations and module
responsibility; workspace tests and strict Clippy cover every target. This is
not a proof of correctness for every line, platform or GPU backend.

The [Rust 1.98–1.99 review](rust-1.99-2026-10-09.md) records the release-specific
choices and earlier decoder/audio changes. Idiomatic Rust here chiefly means
using ownership and existing safe APIs correctly, rather than adopting a new
standard-library API merely because it exists.

## Coverage and changes

| Area | Review outcome |
| --- | --- |
| `srt-parser` | Earlier BOM-less UTF-16 regression fix and encoding contracts retained. No reason to replace legacy charset support wholesale. |
| `srt-apkg` | Extraction flush errors propagate. Archive creation finishes, flushes and synchronizes a temporary sibling before replacing the destination. Failed input preserves an existing archive; archives inside their source directory exclude themselves. |
| `srt-download` (new) | Font/model HTTP streaming, cache policy, temporary ownership, progress, cancellation and publication have one implementation. Concurrent downloads use independent partial files and publish without overwriting a completed result. |
| `srt-ankiconnect` | Check HTTP status before parsing; reject missing `result` and overflowing API version; move JSON results out instead of cloning. A legitimate null result remains accepted by actions that allow it. |
| `srt-autosync` | Shared cancellable FFmpeg segment extraction; finite positive duration validation; preparation workers owned by `JoinSet`; preserve segment order. Cancellation interrupts process preparation; extraction, decoding and worker join failures no longer become silent empty success. |
| `srt-transcribe` | Retain the shared FFmpeg runner and streaming WAV decoder from the earlier patch; expose segment extraction for auto-sync; delegate catalog downloads; reject unknown model IDs when constructing paths. Whisper math/FFI and cloud timing policy remain unchanged. |
| `srt-extract` | Pure FFprobe parsing and bounded owned lossy UTF-8 preview retained. Existing real FFmpeg embedded-track tests verify extraction. |
| `srt-flashcards` | Font downloads delegate to the shared helper; one font-status serializer serves library and GUI. Duplicate filtering uses the result of `HashSet::insert`, avoiding a second lookup. Keep existing media batching and matching algorithms, validated by integration and golden tests. |
| `srt-refine` | Move TSV/APKG loading, analysis and saving to `cards.rs`, separate from LLM orchestration. Analyze filtered TSV rows through a cloned iterator rather than deep-cloned strings. APKG saving uses the shared atomic archive helper. |
| `srt-translate` | Legacy translation delegates to the tiered worker engine. Scheduler policy lives separately in `scheduler.rs`. Owned `JoinSet` workers, child cancellation token, interruptible HTTP requests, checked persistence/join errors and batch-ID validation. Standalone repair runs bounded futures directly, reports failed repairs and preserves existing translations instead of replacing failures with original text. Repair context handles maximum `u32` IDs without overflow. |
| `srt-sync` | Session export preserves the actual sampling strategy. Ignore invalid/duplicate checked indices, preventing underflow and corrupt sample counts. Existing interpolation, matching and playback tests retained. |
| Five CLIs | Reviewed argument-to-config mapping, terminal output and Ctrl-C adapters; keep the existing library boundary. Compile and test all targets. CLI process-scoped signal listeners do not justify a new framework or a shared crate solely to remove a few lines. |
| Vesta desktop | One RAII `OperationGuard` replaces duplicated running/token lifecycle code across translation, refinement, flashcards, transcription, downloads and auto-sync. Shared auto-sync/transcription reservations eliminate the previous check-then-start race. Move custom media protocol serving out of startup into `media_stream.rs`; read through one opened file handle, honor bounded ranges and propagate read failures. |
| whisper-bench | Worker child ownership uses `kill_on_drop`; stdout read errors propagate; a success JSON message followed by nonzero exit is an error. Hardware/sample/update policy is separate from font/model catalog downloading and remains separate. |

Unused direct HTTP/streaming dependencies were removed from flashcards and
transcription, and unused ZIP/hash dependencies from refinement. Public
translation entry points and Tauri command signatures remain available.

## Observable contract changes

- Cancellation requests retain their operation slot until the owning future
  exits. A second operation cannot start merely because the token was cancelled.
  Aborting a future releases the slot and cancels owned work. Already-running
  synchronous inference remains cooperatively cancellable; Drop does not wait
  synchronously for blocking computations to stop.
- Both translation APIs now use the tiered concurrency bound (maximum 16).
  The legacy API previously allowed endpoint count to define a larger bound.
  Provider failures still use the tiered engine's partial-result/resume policy;
  persistence and worker failures now return errors. Standalone repair reports
  failed requests instead of reporting all requested repairs as successful.
- Empty/malformed audio, invalid media durations, unexpected translation IDs,
  invalid model IDs and nonempty-file cache violations fail explicitly.
- A cached resource must be a regular nonempty file. This is not a checksum or
  authenticity check; callers still choose trusted URLs and resource identity.
- APKG replacement occurs after archive completion. This protects an existing
  file from failures while constructing its replacement; it does not claim
  directory-fsync guarantees against power loss.

## Regression contracts

Added tests cover GUI slot ownership and future abort; download HTTP failures,
empty/truncated bodies, stalled headers/body cancellation and deterministic
concurrent publication; ZIP flush failure, self-exclusion and preserved output;
translation API equivalence, zero batch/empty providers, failed persistence,
in-flight request cancellation, worker panic, wrong IDs and failed repair;
sampler invalid indices and strategy round trip; auto-sync invalid durations,
failed extraction and cancellation of fake FFmpeg processes; bounded media
ranges with Unicode paths and empty/missing files; AnkiConnect HTTP/schema/
version failures; worker exit/read errors; and model-path validation.

The real audio integration checks one-second segment extraction in addition to
conversion and segmentation. Existing media integration and golden subtitle
regression hashes remain the output-equivalence checks. Loopback HTTP tests need
no external API key, and auto-sync lifecycle tests need no real Whisper model.

## Further work and performance judgement

The useful improvements in this patch remove redundant lookup/allocation work
and duplicated lifecycle code. No end-to-end throughput claim follows from them.
For further optimization, prioritize measured FFmpeg seek/preparation workloads,
worker-count versus RSS scaling, Whisper throughput and large WAV downmix inputs;
compare output/timing equivalence before adopting numerical reassociation or
aggressive batching. Keep the existing benchmark evidence and conservative media
fallbacks until new corpus measurements justify a change.

Remaining architectural candidates need a concrete contract before another large
refactor: structured provider errors instead of string-based rate-limit detection;
a shared atomic subtitle/TSV persistence helper (ordinary subtitle saves still use
`fs::write`); and a common synchronous process utility for playback/media probes,
which have different timeout/cancellation semantics from async transcription.
Do not make all engines depend on Whisper merely to share a process launcher.
Splitting more crates or replacing safe code with unsafe APIs is not justified by
Rust 1.99 itself.

## Validation

- Toolchain: Rust/Cargo 1.99.0 on Linux.
- `cargo test --workspace`: passed, including both desktop targets, engine/CLI
  tests, real FFmpeg integration, golden media/subtitle regressions and doc tests.
- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `git diff --check` and internal crate version consistency: passed.

`npm test` in the Vesta desktop frontend: 40 test files and 210 tests passed.
No real GPU inference, installed bundle, Windows/macOS or release performance
validation was performed.


The [2026-10-10 follow-up](release-readiness-2026-10-10.md) verifies actual
RX 7800 XT inference, corrects cancellation error classification found during
that test, and requires successful commit-specific CI before publication.
