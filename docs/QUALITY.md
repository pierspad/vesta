# Stability and performance checks

Atomic output regressions run with:

```bash
cargo test -p srt-parser -p srt-transcribe -p srt-flashcards
```

They inject a failed writer, reject read-only targets, verify
symlink/permission behavior on Unix, preserve an existing APKG after a media
streaming failure, and prevent cancelled transcription publication. The helper
tests verify cleanup as well as destination bytes. These are process-level
failure tests, not power-loss or filesystem-specific durability certification.

`cargo test -p srt-flashcards` additionally checks staged TSV media publication:
successful replacement, rollback after a failed TSV write or media swap, refusal
of an overlapping publisher and cleanup after a failed first export. The Unix
pipeline fixture verifies old TSV/media bytes on extraction failure and
cancellation at the export stage, plus media references and byte counts on
success. These do not simulate SIGKILL between filesystem renames; crash recovery
and installed-platform behavior remain separate validation work.

Prioritize correct exports, recoverable failures, durable settings, cancellation,
and usable progress. Optimize bottlenecks after measuring them on identical input.
A passing unit suite is evidence about those cases, not proof that every desktop
interaction or third-party API works on every supported machine.

The separate `headless` CI job selects library and CLI packages explicitly,
without Node, Tauri system libraries or a GPU SDK. This checks the default CPU
dependency graph independently of desktop feature unification. CLI extraction
tests parse stdout as JSON and verify failed operations preserve existing files;
manual sync tests exercise both drift correction and rejected anchor/output ranges.
`cargo test -p srt-flashcards-cli` verifies text summaries, invalid arguments and
inputs, a successful media-free export, and (on Unix) failed media extraction and
SIGINT with previous TSV/media preservation. The cancellation fixture waits for
a child-process handshake before signalling and bounds its exit wait.
For the maintained dependency boundaries and downstream integration contract,
see [architecture](ARCHITECTURE.md) and
[library integration](library-integration.md). This guide is the maintained
source for repeatable checks and release validation.

The [CLI guide](CLI.md) records each command's output contract and coverage.
Subprocess regressions also check help examples, standalone language listing,
conflicting report/track modes, and actionable provider/configuration errors.
MCP tests connect a real stdio SDK client; it is an optional adapter with a
smaller feature set, not a replacement for testing the terminal workflows.

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

## Audio decoding and process lifetime

`cargo test -p srt-transcribe audio::tests` covers integer/float WAV downmix,
truncated and non-finite samples, normalization through 32-bit PCM, and real
FFmpeg conversion/segmentation. Invalid samples fail with file/sample context
instead of being skipped, which would shorten audio and shift transcription
or autosync timing. FFmpeg failures include exit status and the last 64 KiB
of stderr; earlier diagnostics may be discarded while the pipe is still drained.
Unix process tests verify cancellation and dropped-future cleanup with real
children, plus large stderr without a pipe deadlock. Windows/macOS behavior
still needs platform validation.

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

Mounted Svelte regressions cover App's persistent tab containers and changing
`active` props, Align's hidden-tab/duplicate keyboard handling, and Flashcards
preview close/reopen, deactivation, undo and player ownership. App tests use
fixture tabs to verify composition; they do not run a native job. Preview tests
use the real store with simulated IPC and media methods, so successful `play()`
there proves wiring rather than codec support. Run the mounted regressions with
`npm test` in `apps/srt-gui`; use the installed-app checks below for codec,
WebKit/GStreamer and native-window behavior.

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


### Rust lifecycle and persistence regressions

`cargo test --workspace` covers shared download cancellation/concurrent publication,
APKG flush failures and atomic replacement, translation worker failure and save
errors, unexpected translation IDs, GUI operation slot ownership, media byte
ranges, AnkiConnect HTTP/schema/version failures and worker process exit/read
errors. Auto-sync tests use fake tools and a stub model to prove cancellation
stops segment preparation without requiring a downloaded Whisper model.
The audio integration also checks a real one-second FFmpeg extraction.
Unix-only process/script tests require a Unix host; run platform and GPU release
checks separately. `cargo test --workspace` includes the wider crate and
documentation-test coverage beyond the focused CI gate above.


### Opt-in real Vulkan inference

On a machine with a supported Vulkan GPU and an already installed Whisper model:

```bash
cargo build -p srt-transcribe-cli --features vulkan
python3 build-scripts/gpu_smoke.py --device 'RX 7800 XT' --vad --output-dir /tmp/vesta-gpu-check
```

The output directory must be new. Omit `--vad` if Silero is not installed; change
`--device` to a substring of the actual GPU name or omit it. The script makes no
cloud requests and downloads no models. It verifies CPU, actual Vulkan backend
selection, no-visible-GPU CPU fallback, valid bounded SRT output, optional VAD,
and on Unix cancellation after the first decoded segment. Use a sufficiently
long clip for cancellation; the default is the repository's 90-second Detour
fixture. Logs and transcripts are retained only when `--output-dir` is supplied.
These timings are smoke diagnostics, not a release-profile performance benchmark.

### Recorded hardware smoke (10 October 2026)

On an AMD Radeon RX 7800 XT (RADV NAVI32, Mesa 26.2.4, Vulkan 1.4.354), the
90-second Detour fixture completed with CPU, Vulkan, Vulkan plus cached Silero
VAD, and CPU fallback when Vulkan devices were hidden. The Vulkan log confirmed
`Vulkan0`; fallback output matched the explicit CPU output byte-for-byte for
that run. Cancellation after the first decoded segment returned a nonzero exit,
reported `Transcription cancelled`, and published no final SRT (about 0.143 s
from signal in that run). This was an engine smoke using an installed base model,
not an installed desktop test or a controlled performance benchmark. CPU/GPU
transcripts differed in one proper-name phrase, so this evidence does not
establish transcription accuracy. Reproduce the checks with the commands above.

Publication now waits for the successful push CI run on the exact release commit,
including workspace tests and strict Clippy; a missing/failed/skipped run blocks
`semantic-release`. Manual dispatch also requires that successful push CI evidence.

## Git hooks and localization policy

Install the tracked hook dispatchers once in each clone:

```bash
bash build-scripts/install-hooks.sh
```

The pre-commit hook audits the actual staged source and locale snapshot. An
unstaged fix cannot hide a broken commit. The pre-push hook audits each outgoing
tip before running frontend type checks, tests/build and the existing native
version, formatting, Clippy and test gates. Remaining frontend/native checks run
in the working checkout; commit or stash unrelated changes for a reproducible
full gate. Dispatchers use the current tracked scripts, support Git worktrees
and `core.hooksPath`, and refuse to replace unrelated hooks.

The translation checker's strict default is shared by CI, release builds, local
hooks and `quality_check.py`: missing locales/keys, empty values, mismatched
placeholders and unintended English copies all block. Approved identical labels
remain in the checker's explicit allowlist. Do not weaken the gate to avoid
translating UI text. Run the same fast localization gate manually with
`bash build-scripts/check-i18n.sh`. Regression tests for staged/outgoing snapshots
and hook installation run with `python3 build-scripts/test_git_hooks.py`.

## Dependency advisory checks

CI runs a separate `security` job against both checked-in lockfiles:

```bash
cargo install cargo-audit --locked --version 0.22.2
cargo audit
(cd apps/srt-gui && npm audit)
```

Review informational `unmaintained` and `unsound` advisories as well as the
vulnerability count. A zero vulnerability count can still include informational
advisories. The current GTK/Tauri graph retains GLib's VariantStrIter unsoundness
warning and maintenance warnings. Repeat these checks before release;
a database lookup does not prove exploitability or absence of unknown defects.


## Frontend workflow regression checks

`quality_check.py` first validates the locked Tauri JS/Rust pairs and runs the
checker fixtures without network. To run just those checks:

```bash
python3 build-scripts/check_tauri_versions.py
python3 build-scripts/test_tauri_versions.py
```

The frontend suite includes pending native-listener cleanup, refinement run IDs,
provider changes during model discovery, manual versus automatic audio-track
selection, matching-rule JSONC parsing, grouped alignment undo, and obsolete
subtitle-output lookups. Mounted tests cover Align shortcuts and Extract lookup
errors. These tests preserve behavior across controller extraction; native
webview, installer and authenticated-provider smoke checks remain separate.

## Static code patterns

Run `npm run check:patterns` from `apps/srt-gui`. `npm test` runs it before
Vitest, so CI and `quality_check.py` also enforce it. It parses TypeScript and both Svelte script blocks and
reports file, line, column and rule for async `forEach` callbacks, async Promise
executors and debugger statements. Tests and application code are both scanned.
Comments, string literals and component markup do not trigger these AST rules.

These are narrow regression guards, not a replacement for Svelte type checking,
strict Clippy or behavioral tests for cancellation, stale responses and persistence.
The checker fixtures cover positive cases, safe alternatives, source offsets and
malformed components. Add a rule only when it identifies a concrete failure mode;
avoid blanket bans on `any`, `unwrap`, file size or duplication. Use `rg -n` for
exploratory searches, then read the surrounding code before classifying a finding.

## MCP processing and release policy checks

`cargo test --locked -p vesta-mcp` exercises all eight tool schemas through a real
stdio client. Local HTTP fixtures cover successful translation, incomplete
translation rejection, translation/transcription cancellation, server-only
credentials, shared job exclusion and real FFmpeg audio conversion. Profile
unit tests cover disabled defaults, environment references, invalid providers,
local model IDs and URLs. `srt-transcribe` separately tests the no-download
policy before invoking FFmpeg. No authenticated provider access is implied.

`python3 build-scripts/test_release_workflows.py` guards release queueing, all
platform prerequisites and tag/version agreement. It also executes the tag
validation and fallback-note shell blocks in a temporary Git fixture, including
a literal tag containing shell syntax. CI and `quality_check.py` run this check.
