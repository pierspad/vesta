# srt-translate

GUI-independent subtitle translation engine used by Vesta and `srt-translate-cli`.

It translates overlapping batches so adjacent dialogue remains available as context, writes progress incrementally, resumes partial output, repairs missing lines, and supports cooperative cancellation. Endpoints are arranged into ordered tiers: requests rotate among available entries in the active tier and continue with the next tier when necessary.

Supported configurations include Google Gemini, Groq, OpenRouter, and OpenAI-compatible services such as Mistral, GitHub Models, Ollama, LM Studio, and custom endpoints.

The maintained API is based on `TierEntry`, `build_pool`, and `translate_subtitles_tiered_cancellable`. See [`docs/modules/srt-translate.md`](../../docs/modules/srt-translate.md) for a current Rust example and [`cli/srt-translate-cli/README.md`](../../cli/srt-translate-cli/README.md) for the command-line configuration format.

Licensed GPL-3.0-only.
