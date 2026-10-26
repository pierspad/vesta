# Maintenance

Repeatable procedures are documented in [QUALITY](QUALITY.md); engine architecture and contracts in [ARCHITECTURE](ARCHITECTURE.md); provider contract reviews in [API_COMPATIBILITY](API_COMPATIBILITY.md).
This document outlines boundaries to preserve during modifications.

## Frontend

Tabs compose views and synchronize with user preferences. Separately verifiable responsibilities reside in `apps/srt-gui/src/lib/workflows`:

- **Flashcards**: Episode preparation, batch series generation, previewing, and audio track selection. A stale probe must never overwrite the selection of a different file.
- **Translate**: Model discovery scoped by provider/credential identity; obsolete results must never update state or the cache. A failed discovery preserves the static fallback and must not trigger infinite request loops. Local tiers enforce an explicit 15-second timeout; rotating or removing credentials invalidates pending results. Versioned persistent caches have a 24-hour TTL and full SHA-256 identities without secrets in filenames.
- **Transcribe**: Whisper resources and native subscriptions; listeners or subscriptions that resolve after unmounting must be immediately disposed.
- **Refine**: Card indexing/search with cache limited to current dataset texts and automated job protocols. The progress listener must precede command invocation, filter by run ID, and always be unsubscribed on completion.
- **Settings**: Provider preferences and smart-matching JSONC draft. Saved rules and reloaded rules must traverse identical normalization.
- **Sync**: Frame-coalesced timing adjustments must be cleared on reset and unmount.

Do not indiscriminately move reactive side-effects into a shared controller: translation, transcription, and card generation have distinct ownership and cancellation lifecycles. Preserve UI responsiveness, preference events, and teardown cleanup whenever extracting responsibilities.

## Persistence and Compatibility

Translation checkpoints are atomic snapshots serialized under a lock. Saves occur after every batch: releasing the lock before writing can publish out-of-order snapshots. Before altering serialization protocols, measure cumulative bytes, lock contention, and throughput; verify behavior under crashes, cancellation, and full disks.

`get_missing_subtitle_ids` is a backward-compatible public API that inspects only missing IDs; its replacement additionally checks malformed lines. The legacy field `FlashcardConfig::use_timings_from` is accepted, but generation always uses target timings; the GUI provides no control promising differing behavior. Preserve preference migrations and custom adapter fallbacks.

## Verification and Benchmarks

```sh
python3 build-scripts/quality_check.py --desktop --smoke
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo run --locked --release -p srt-translate --example checkpoint_bench -- /path/to/storage
cargo test --locked -p vesta commands::translate::context_tests::preview_cache_debug_timings_for_1k_10k_and_50k_subtitles --lib -- --nocapture
```

The comparative suite, report generators, fixtures, and documentation in `benchmarking_against_subs2srs/` and `benchmarking/` are reproducibility tools, even when not imported by the main application. Do not remove them as disposable reports. Synthetic benchmark figures do not measure remote provider behavior, lock contention, or end-to-end performance, and do not justify arbitrary increases in concurrency defaults.

Pre-release validation requires installed smoke checks as detailed in QUALITY: Anki desktop integration, webview codecs, persistence, cooperative cancellation, authenticated providers, installers, and physical GPU hardware. Re-run lockfile advisory checks periodically; distinguish security vulnerabilities from maintenance notices or GTK/Tauri chain unsoundness without blindly adding ignores.

## Transitive Desktop Dependencies

Tauri 2.12 relies on GTK3 / GLib 0.18 bindings. The official fix for [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429) is in GLib 0.20+, which does not directly replace the types required by GTK3. The advisory concerns `VariantStrIter`: it is neither suppressed nor considered resolved simply because Vesta code does not invoke that iterator directly. A local vendored copy with backports incurs its own testing and maintenance burden; cargo audit advisories may continue flagging the crate version even with a source patch.

Maintenance advisories also remain for `proc-macro-error` (GTK macros) and `zvariant`, `zvariant_derive`, `zbus_names` (the D-Bus dialog and Opener chain). Eliminating them requires upstream consumer updates: introducing an additional direct workspace version does not remove the transitive one. See also the [upstream zbus guidance](https://github.com/z-galaxy/zbus/issues/2043). Do not migrate the production application to alpha releases or add ignores purely to suppress warning counts. Run `cargo audit` and `cargo tree -i CRATE_NAME` when upgrading this stack.
