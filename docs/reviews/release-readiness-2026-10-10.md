# Local release readiness — 2026-10-10

## Real GPU validation

Tested on the user's AMD Radeon RX 7800 XT (RADV NAVI32), Mesa 26.2.4-arch3.1,
Vulkan device API 1.4.354. Rust/Cargo 1.99.0. The binary is built with
`cargo build -p srt-transcribe-cli --features vulkan` and calls the same
`srt-transcribe` engine as the desktop app. This is an engine smoke, not an
installed desktop/bundle test.

Input: checked-in `Test_Subs/fixtures/detour/detour_clip_90s.mp4`, installed
Whisper `base`, fixed English language, greedy decoding. No user media was
uploaded and no model was downloaded. Optional VAD used cached Silero v6.2.0.

| Case | Result |
| --- | --- |
| CPU (`--no-gpu`) | Success; 7 subtitles with valid bounded timestamps. |
| GPU | Success; 7 subtitles; native logs name RX 7800 XT, allocate `Vulkan0` model buffers and confirm `using Vulkan0 backend`. |
| GPU + VAD | Success; 4 subtitles, with music-only segments suppressed on this fixture. |
| No visible GPU (`GGML_VK_VISIBLE_DEVICES=''`) | Success through CPU fallback; output byte-identical to explicit CPU in this run. |
| Cancellation after first decoded segment | Nonzero exit and terminal JSON `Transcription cancelled`; no final SRT; approximately 0.143 seconds from signal in the recorded run. |

Single debug-profile observations were about 2.797 seconds CPU, 1.123 GPU,
2.780 fallback and 1.177 GPU/VAD. These are diagnostics with cached models, not
controlled release-profile benchmark evidence. CPU and GPU transcripts are not
byte-identical: one proper-name phrase differs while the seven timestamp pairs
match. Neither successful return nor this comparison certifies transcription
accuracy; real speech can remain imperfect with the base model.

The stronger cancellation test first failed: Whisper returned `GenericError(-6)`
when its abort callback stopped encoding, and our code returned that error before
checking the cancellation token. The fix checks cancellation after inference
returns and before converting a native error. Unrequested native failures retain
their error. A unit regression covers both outcomes; the real GPU test then passed.

`build-scripts/gpu_smoke.py` makes these checks repeatable, with subprocess
timeouts, cleanup of its owned process group on Unix, backend assertions and
retained logs/transcripts when requested. See [QUALITY](../QUALITY.md).

## Publication ordering

The existing CI and release workflows both run on push. Before this patch,
`semantic-release` could publish before the independent Rust CI completed.
The release workflow now waits for `ci.yml`'s push run on `github.sha`, requires
successful completion, and explicitly rejects any other conclusion. Missing CI
also blocks publication. This applies to main, dev prereleases and manual release
dispatch; it does not bypass test failures.

Local validation parses the workflow YAML, checks Bash syntax and simulates the
step with a stub `gh`: success and delayed registration pass; failed, skipped and
missing CI fail. These simulations do not claim a GitHub-hosted workflow run.
The first real push must confirm the integration with the repository's Actions
permissions and runner environment.

## Other findings

GTK messages about `colorreload-gtk-module` and `window-decorations-gtk-module`
originate in the desktop user's `~/.config/gtk-3.0/settings.ini` `gtk-modules`
setting. Corresponding modules were not found under `/usr/lib`. No application
or global GTK configuration was changed to suppress them. Those messages alone
do not establish a Vesta runtime failure.

The six frontend i18n heuristic warnings concern names/technical labels such as
AnkiConnect, GStreamer, Silero, Opus and LUFS; there are no blocking translation
errors in the quality gate. No unrelated frontend rewrite was added.

## Checks and next step

- `python3 build-scripts/quality_check.py --desktop --smoke`: passed, including
  Svelte/type checks, 210 frontend tests, development CSS, i18n, production frontend
  build, Rust workspace tests except the explicitly excluded benchmark GUI,
  version consistency and synthetic media/APKG ZIP/SQLite/media validation.
- Final `cargo test --workspace`: passed, including whisper-bench and the new
  cancellation regression. `cargo clippy --workspace --all-targets -- -D warnings`,
  formatting and diff checks: passed after the Rust fix.
- Real Vulkan inference and cancellation: passed; actual release artifacts,
  Windows/macOS, real cloud accounts and Anki playback remain separate release
  smoke checks.

Deliver the reviewed changes to main using Conventional Commits and let the
existing automatic release wait for commit-specific CI. Keep new performance
experiments or broader provider-error/persistence refactors in subsequent patches.
Platform smoke checks are deferred to the maintainer after this delivery; no
manual version bump or release tag is needed for semantic-release.
