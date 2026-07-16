# srt-translate-cli

Command-line tool for translating SRT subtitle files using AI/LLM APIs.

## Installation

### From source

```bash
cargo install --path .
```

### Pre-built binaries

Download from the releases page or build using `./build_all.sh` from the project root.

## Usage

```bash
# Basic usage (language required, config.toml read from the working directory)
srt-translate --input movie.srt --language it

# Custom config path and output file
srt-translate -i movie.srt -l es -c my-config.toml -o movie.es.srt

# Check a translated file for missing subtitles or line count mismatches
srt-translate -i movie.srt --check-missing movie.it.srt

# List supported language codes
srt-translate --language-list
```

## Configuration

Create a `config.toml` file in the working directory. Providers are organized in
**tiers**: within a tier, entries are used round-robin; when every entry in a
tier hits a rate limit or quota error, translation automatically fails over to
the next tier. This is the same pool/scheduler used by the Vesta desktop app
(`srt_translate::build_pool` + `translate_subtitles_tiered_cancellable`), so
CLI and GUI share one failover implementation.

```toml
[translation]
batch_size = 25
resume_overlap = 2

[output]
filename_pattern = "{input_name}.{language}.srt"

# Tier 1: primary providers, tried first
[[tiers]]
[[tiers.entries]]
provider = "google"
model = "gemini-2.5-flash"
api_key = "${GOOGLE_API_KEY}"
rpm = 15

[[tiers.entries]]
provider = "groq"
model = "llama-3.3-70b-versatile"
api_key = "${GROQ_API_KEY}"
rpm = 30

# Tier 2: fallback once Tier 1 is exhausted
[[tiers]]
[[tiers.entries]]
provider = "local"
model = "llama3.2"
```

`${VAR_NAME}` placeholders are expanded from environment variables (loaded
from a `.env` file if present, e.g. via `GOOGLE_API_KEY=AIza...`).

## License

MIT
