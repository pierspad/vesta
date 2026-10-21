# Codebase audit follow-up — 22 October 2026

Baseline: `be0f743` (v0.29.0), following `CODEBASE_AUDIT_2026-10-21.md`.
This report describes the working-tree changes; it does not certify every path
in the application. Two Luna subagents investigated the frontend findings;
changes were reviewed and validated together in the main chat.

## Changes

- Commit subjects and bodies must always be in English. Saved in the global
  Codex instructions, repository `AGENTS.md`, and `.cursorrules`.
- A01: isolated tier discovery state from reactive inputs; local endpoints refresh
  on the explicit 15-second timer, with pending-request deduplication. Mounted
  tests cover success, empty results, errors and teardown.
- A02–A04: identities track credentials and URLs; obsolete/removed entries and
  disposed controllers reject late model/error updates and persistent writes.
  Persistent identities use SHA-256 rather than credential prefixes, cache
  records have a version and 24-hour TTL, and malformed records fall through.
  Fingerprinting/storage failures do not prevent live discovery. Forced STT
  refreshes supersede older results; ordinary concurrent loads share work.
- A05: shared pure persisted-tier normalization; storage keys, events, accepted
  provider strings, and distinct translation/STT defaults remain unchanged.
  Malformed configuration fixtures cover both adapters.
- A06: removed the unused STT numeric handler. Preserved `rpm`/`maxRequests`
  fields for compatibility; execution semantics were not invented.
- A07: one local/custom endpoint probe per URL/credential pair within each
  availability check. Different credentials remain separate; existing 404/405
  manual-model fallback is preserved.
- A08: the Refine strip-HTML cache retains only expression/meaning strings from
  the current indexed dataset. Switching decks releases old cached text;
  repeated indexing reuses unchanged text. Regression coverage checks reuse,
  eviction, rebuilding and search results.
- A14 (partial): aligned divergent fallback model sizes. The fallback catalog
  remains duplicated; this is not claimed as a complete single-source solution.

## Repeatable static checks

`npm test` now runs `npm run check:patterns` before Vitest. Existing CI, hooks
that invoke npm test, and `quality_check.py` therefore enforce the gate too.
The checker uses TypeScript ASTs, with the Svelte parser locating instance and
module scripts. It reports path, line, column and rule for:

- async `forEach` callbacks (their returned promises are discarded);
- async `Promise` executors (executor rejections can be lost);
- debugger statements.

Four checker fixtures cover violations, safe alternatives, comments/strings,
Svelte offsets and invalid components. The current source tree has no findings
for these rules. This narrow gate neither detects all floating promises nor
proves the absence of debt. No blanket regex ban on unwrap/any or line-count
threshold was introduced. Exploratory `rg -n` results were read in context;
test assertions and invariant-checked header constructors are not bugs merely
because they contain `unwrap`/`expect`.

## Performance evidence and deferred work

A10: ran the existing checkpoint benchmark in release mode on the repository
filesystem, using synthetic subtitles and batches of 100. One run (not a repeated
end-to-end/provider benchmark):

| Subtitles | Saves | Total growing-checkpoint time | Save p50 | Save p95 | Cumulative output bytes |
|---|---:|---:|---:|---:|---:|
| 1,000 | 10 | 18.328 ms | 1.690 ms | 2.344 ms | 553,332 |
| 10,000 | 100 | 327.523 ms | 3.310 ms | 4.503 ms | 51,802,484 |
| 50,000 | 500 | 3,800.509 ms | 7.603 ms | 12.223 ms | 1,305,017,284 |

Command: `cargo run --locked --release -p srt-translate --example checkpoint_bench -- <repository-directory>`.
Times include filesystem writes but exclude provider latency and lock contention;
reported bytes are serialized file sizes, not physical disk writes. The growth
is real, but this run does not justify changing crash recovery/durability or
checkpoint ordering. Production persistence is unchanged.

A09, A11 and A15–A16 remain profiling/design work: search normalization,
adversarial interval matching, archive compression and merge I/O. A12–A13
(component/presentation splits) and A17–A18 (CI/test-tool deduplication) remain
maintainability opportunities. No arbitrary performance claim or broad rewrite
was used to mark these findings resolved. Public compatibility APIs, vendor
patches, benchmark assets, and the separate headless/desktop CI paths remain.

## Validation

- `npm test`: four static-checker fixtures, zero current pattern findings, and
  98 Vitest files / 474 tests passed.
- `npm run check`: zero errors and warnings; production build passed.
- Both localization gates passed (15 locales, 1,280 English keys, no issues).
- `npm run test:dev-css`: cold CSS requests and dev-server restart passed.
- Git-hook fixtures (5 tests), locked Tauri compatibility and its 5 fixtures,
  internal crate versions, `git diff --check`, and Rust formatting passed.
- `cargo test --locked --workspace --exclude vesta --exclude whisper-bench` passed.
- Strict Clippy with the same workspace exclusions and `--all-targets -- -D warnings` passed.
- `cargo check --locked -p vesta` passed for the native desktop application.

No installed-app/webview session, authenticated provider, GPU inference or new
dependency advisory lookup was performed. Mocked frontend tests do not replace
those release smoke checks. No dependency or public Rust API changes were made.
