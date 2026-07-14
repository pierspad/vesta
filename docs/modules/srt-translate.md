# srt-translate — LLM subtitle translation

`lib/srt-translate` translates SRT files with LLMs, batching subtitles with
overlap for context, with resume support (partial output files are continued,
holes are repaired), rate limiting, and a **multi-tier failover pool**: you
declare an ordered list of endpoints (provider + model + key) and the engine
round-robins inside a tier, then falls to the next tier when entries exhaust
their quota or hit rate limits.

Supported endpoints: Google (Gemini), Groq, OpenRouter, and any
OpenAI-compatible API (Mistral, GitHub Models, NVIDIA, Ollama/local, custom).

**Library API highlights**

- `TierEntry` + `build_pool(&tiers)` → `TranslatorPool` (module `pool`:
  per-provider defaults for base URL, RPM and model live here)
- `translate_subtitles_tiered_cancellable(pool, subs, target_lang, batch_size,
  overlap, title_context, output_path, on_progress, cancel_token)`
- `Translator::generate_response(prompt)` — raw single-prompt call (used by
  [srt-refine](srt-refine.md))
- Progress via a plain `Fn(TranslationProgress)` callback; cancellation via
  `CancellationToken`.

## Use as a binary

```bash
cargo build --release -p srt-translate-cli
./target/release/srt-translate --help
```

## Use as a Rust dependency

```toml
[dependencies]
srt-parser    = { git = "https://github.com/pierspad/vesta" }
srt-translate = { git = "https://github.com/pierspad/vesta" }
tokio         = { version = "1", features = ["full"] }
tokio-util    = "0.7"
```

```rust
use srt_parser::SrtParser;
use srt_translate::{build_pool, TierEntry, translate_subtitles_tiered_cancellable};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut subs = SrtParser::parse_file("movie.en.srt")?;
    SrtParser::normalize_subtitles(&mut subs);

    let tiers = vec![vec![TierEntry {
        provider: "google".into(),
        model: "gemini-2.5-flash".into(),
        api_key: Some(std::env::var("GEMINI_API_KEY")?),
        api_url: None,
        rpm: None,          // provider default
        max_requests: None, // unlimited
    }]];
    let pool = build_pool(&tiers).map_err(anyhow::Error::msg)?;

    let translated = translate_subtitles_tiered_cancellable(
        pool, subs, "it", 30, 2, Some("Detour (1945)"),
        std::path::Path::new("movie.it.srt"),
        |p| eprintln!("{}", p.message),
        CancellationToken::new(),
    ).await?;

    println!("Translated {} subtitles", translated.len());
    Ok(())
}
```

## Extract it standalone

Copy `lib/srt-translate/` + `core/srt-parser/`. External deps: `anyhow`,
`tokio`, `tokio-util`, `reqwest` (rustls), `serde`, `serde_json`, `governor`.
