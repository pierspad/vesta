# srt-transcribe — transcription pipeline (media → SRT)

`lib/srt-transcribe` contains everything Whisper-related, from low-level
primitives to a complete, GUI-agnostic **media → SRT pipeline**
(`pipeline` module — the engine behind Vesta's Transcribe tab and the
`srt-transcribe` CLI).

**Modules**

- `model` — Whisper ggml model catalogue: `list_models()`,
  `download_model(id, on_progress, cancel)` (Hugging Face →
  `~/.cache/whisper/`), `model_file_path(id)`, `uninstall_model(id)`; plus the
  optional Silero VAD add-on managed with the same lifecycle
  (`list_vad_models`, `download_vad_model`, `vad_model_installed`, `vad_model_path`, `uninstall_vad_model`)
- `audio` — FFmpeg helpers: `convert_to_wav`, `extract_wav_segment`,
  `segment_to_wav_chunks`, `read_wav_to_f32`
- `transcribe` — `whisper.cpp` via `whisper-rs`: `transcribe_full` and
  `transcribe_full_with_state` (greedy or beam-search decoding, threads
  defaulting to physical cores capped at eight; supports `webrtc_vad`,
  `no_context`, `single_temperature`, `suppress_nst`, and `split_on_word` in
  `TranscribeOptions`; when a VAD model is configured, it runs Silero through
  `whisper.cpp`'s standalone VAD API (`vad_speech_spans_ms`) and transcribes only
  detected speech spans with timestamps mapped back to the original timeline),
  plus `text_similarity` and `normalize_text` (`NormalizedText`)
- `cloud` — chunked uploads (`transcribe_cloud`, `transcribe_timed_chunk`) to
  Groq, OpenAI, Deepgram, AssemblyAI, and custom OpenAI-compatible endpoints
- `pipeline` — full orchestration: model verification/download → WAV conversion →
  local or cloud transcription (`run_local`, `run_cloud`) → segment post-processing
  (`postprocess_segments`) → atomic SRT publication (`write_srt`) with automatic
  language-suffix handling (`apply_language_suffix_to_srt_path`)

`pipeline::transcribe_to_srt` retains its automatic local model download
behavior. Servers that must keep downloads disabled can call
`pipeline::transcribe_to_srt_with_download_policy(..., false)`; local
transcription then requires a valid, already-installed, non-empty catalog
model file. The policy does not affect cloud providers.

**Decode options (local backend)**

- `quality` — beam-search decoding (width 5) instead of greedy: ~2-3× slower,
  sometimes more accurate on difficult audio.
- `vad` — native whisper.cpp Silero VAD: transcribes only detected speech,
  skipping silence/music and reducing hallucinations. Requires the VAD model
  (`srt-transcribe download vad`, or Settings → Whisper in the GUI).
- `use_gpu` — offload inference to the backend compiled into the binary.
  `srt_transcribe::gpu_supported()` and `gpu_backend_name()` report build
  capability; whisper.cpp falls back to CPU when no usable device exists.

## Use as a binary

```bash
cargo build --release -p srt-transcribe-cli

./target/release/srt-transcribe models                 # list models (+ VAD add-on)
./target/release/srt-transcribe download vad           # Silero VAD model (~0.9 MB)
./target/release/srt-transcribe run movie.mp4 -o movie.srt \
  --model base --language auto                          # local whisper.cpp (GPU by default if compiled in)
./target/release/srt-transcribe run movie.mp4 -o movie.srt \
  --model small --vad --quality                        # VAD + beam search (pass --no-gpu to force CPU)
./target/release/srt-transcribe run movie.mp4 -o movie.srt \
  --provider groq --model whisper-large-v3 --api-key $GROQ_API_KEY
```

On Arch Linux, the portable GPU build is:

```bash
sudo pacman -S --needed vulkan-headers vulkan-icd-loader shaderc
cargo build --release -p srt-transcribe-cli --features vulkan
```

Install the Vulkan driver for the actual GPU (`vulkan-radeon`,
`vulkan-intel`, or `nvidia-utils`). Alternative, mutually exclusive worker
builds are available through `--features cuda`, `rocm`, or `sycl`; each needs
its vendor SDK/toolchain. Do not combine all backend features in one binary.

Requires `ffmpeg` on PATH (or `--ffmpeg /path/to/ffmpeg`).

## Use as a Rust dependency

```toml
[dependencies]
srt-transcribe = { git = "https://github.com/pierspad/vesta" }
anyhow = "1"
tokio          = { version = "1", features = ["full"] }
tokio-util     = "0.7"
```

```rust
use srt_transcribe::pipeline::{transcribe_to_srt, PipelineCallbacks, TranscriptionConfig};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = TranscriptionConfig {
        input_path: "movie.mp4".into(),
        output_path: "movie.srt".into(),
        model: "base".into(),
        language: "auto".into(),
        translate_to_english: false,
        word_timestamps: false,
        max_segment_length: 0,
        provider: None, // local whisper.cpp
        api_key: None,
        api_url: None,
        quality: false, // beam search 5 when true
        vad: false,     // Silero VAD (requires the downloaded model)
        vad_model_id: None,
        vad_custom_path: None,
        use_gpu: true,  // compiled GPU backend when usable; otherwise CPU
    };

    let outcome = transcribe_to_srt(
        &config, "ffmpeg", PipelineCallbacks::default(), &CancellationToken::new(),
    ).await?;

    println!("{} subtitles → {}", outcome.subtitle_count, outcome.output_path);
    Ok(())
}
```

Progress/segment streaming: fill `PipelineCallbacks::on_progress` /
`on_segment` with `Arc::new(|…| …)` closures.

## Vendoring & Standalone Integration

Prefer the Cargo Git dependency above. When vendoring into another workspace,
include `lib/srt-transcribe/`, `core/srt-parser/`, and `core/srt-download/`,
along with the vendored `whisper-rs-sys` patch (`[patch.crates-io]`) from the
root `Cargo.toml`; see [`docs/library-integration.md`](../library-integration.md).
The optional `vulkan`, `cuda`, `rocm`, and `sycl` features forward one selected
backend to `whisper-rs`. The official Linux desktop build uses Vulkan and the
runtime flag defaults to GPU enabled; CPU is the safe fallback.

## Cloud timing and verification

The GUI selects a single cloud engine (no translation-tier failover). OpenAI
Whisper supports segment timestamps; GPT-4o transcription is JSON-only here and
uses whole-WAV-chunk timing. Use a timestamp-capable engine for precise subtitles.
Loopback provider tests run with `cargo test -p srt-transcribe`; account-dependent
inference still requires the opt-in release checks in [QUALITY](../QUALITY.md).
