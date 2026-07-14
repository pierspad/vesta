# whisper-common — transcription pipeline (media → SRT)

`lib/whisper-common` contains everything Whisper-related, from low-level
primitives to a complete, GUI-agnostic **media → SRT pipeline**
(`pipeline` module — the engine behind Vesta's Transcribe tab and the
`srt-transcribe` CLI).

**Modules**

- `model` — Whisper ggml model catalogue: `list_models()`,
  `download_model(id, on_progress, cancel)` (Hugging Face →
  `~/.cache/whisper/`), `model_file_path(id)`, `uninstall_model(id)`
- `audio` — FFmpeg helpers: `convert_to_wav`, `segment_to_wav_chunks`,
  `read_wav_to_f32`
- `transcribe` — whisper.cpp via `whisper-rs`: `transcribe_full`,
  `transcribe_chunked`, plus `text_similarity` / `normalize_text` (fuzzy text
  matching, also used by [srt-autosync](srt-autosync.md))
- `cloud` — chunked uploads to Groq / OpenAI / Deepgram / AssemblyAI / custom
  OpenAI-compatible endpoints
- `pipeline` — the orchestration: model download → WAV conversion → local or
  cloud transcription → segment post-processing (merge tiny / split overlong
  at sentence boundaries) → SRT writing with language-suffix handling

## Use as a binary

```bash
cargo build --release -p srt-transcribe-cli

./target/release/srt-transcribe models                 # list models
./target/release/srt-transcribe run movie.mp4 -o movie.srt \
  --model base --language auto                          # local whisper.cpp
./target/release/srt-transcribe run movie.mp4 -o movie.srt \
  --provider groq --model whisper-large-v3 --api-key $GROQ_API_KEY
```

Requires `ffmpeg` on PATH (or `--ffmpeg /path/to/ffmpeg`).

## Use as a Rust dependency

```toml
[dependencies]
whisper-common = { git = "https://github.com/pierspad/vesta" }
tokio          = { version = "1", features = ["full"] }
tokio-util     = "0.7"
```

```rust
use whisper_common::pipeline::{transcribe_to_srt, PipelineCallbacks, TranscriptionConfig};
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

Copy `lib/whisper-common/` — it has no internal dependencies. External deps:
`whisper-rs` (compiles whisper.cpp — needs cmake and a C++ toolchain; the
repo pins a vendored `whisper-rs-sys` via `[patch.crates-io]`, see the root
`Cargo.toml`), `reqwest`, `hound`, `tokio`, `tokio-util`, `tempfile`,
`futures`, `dirs`, `serde`. FFmpeg is a runtime requirement.
