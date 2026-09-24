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
  (`download_vad_model`, `vad_model_installed`, `uninstall_vad_model`)
- `audio` — FFmpeg helpers: `convert_to_wav`, `segment_to_wav_chunks`,
  `read_wav_to_f32`
- `transcribe` — whisper.cpp via `whisper-rs`: `transcribe_full` (greedy or
  beam-search decoding, threads defaulting to the physical cores; with a VAD
  model set it runs Silero through whisper.cpp's standalone VAD API, then
  transcribes only the detected speech spans with timestamps mapped back to
  the original timeline — `whisper_full_params.vad` is deliberately unused, as
  it is a silent no-op through the `whisper_full_with_state` entry point),
  plus `text_similarity` / `normalize_text` (fuzzy text matching, also used
  by [srt-autosync](srt-autosync.md))
- `cloud` — chunked uploads to Groq / OpenAI / Deepgram / AssemblyAI / custom
  OpenAI-compatible endpoints
- `pipeline` — the orchestration: model download → WAV conversion → local or
  cloud transcription → segment post-processing (merge tiny / split overlong
  at sentence boundaries) → SRT writing with language-suffix handling

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
./target/release/srt-transcribe download vad           # Silero VAD model (~23 MB)
./target/release/srt-transcribe run movie.mp4 -o movie.srt \
  --model base --language auto                          # local whisper.cpp
./target/release/srt-transcribe run movie.mp4 -o movie.srt \
  --model small --vad --quality --gpu                   # VAD + beam 5 + GPU
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

## Extract it standalone

Copy `lib/srt-transcribe/` — it has no internal dependencies. External deps:
`whisper-rs` (compiles whisper.cpp — needs cmake and a C++ toolchain; the
repo pins a vendored `whisper-rs-sys` via `[patch.crates-io]`, see the root
`Cargo.toml`), `reqwest`, `hound`, `tokio`, `tokio-util`, `tempfile`,
`futures`, `dirs`, `serde`, `num_cpus`. FFmpeg is a runtime requirement.
The optional `vulkan`, `cuda`, `rocm`, and `sycl` features forward one selected
backend to `whisper-rs`. The official Linux desktop build uses Vulkan and the
runtime flag defaults to GPU enabled; CPU is the safe fallback.
