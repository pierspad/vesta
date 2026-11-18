# External Provider API Compatibility

This reference lists the public HTTP contracts and endpoint conventions used by Vesta's cloud transcription, translation, and annotation integrations.

Account permissions, rate limits, regional availability, and model lifecycles are controlled by each provider. Where supported, Vesta queries the provider's model discovery endpoint at runtime and falls back to static catalog defaults on network or authentication errors.

Model discovery follows Google `nextPageToken` / `pageToken` and Anthropic
`has_more` / `last_id` / `after_id` cursors on the configured origin. A shared
request deadline, cursor-cycle detection and catalog/page limits prevent an
unbounded discovery run. Failed later pages reject discovery rather than
publishing an incomplete list.

| Integration | Endpoint & Protocol Contract | Reference Documentation |
| --- | --- | --- |
| **OpenAI Transcription** | `POST /v1/audio/transcriptions` (multipart). Whisper models request `verbose_json` with timestamps. `gpt-transcribe` uses default JSON and plural `languages[]`, without Whisper-only parameters. Legacy GPT-4o transcription models request `json`. Text-only models are excluded from synchronization. | [OpenAI Audio Transcriptions](https://developers.openai.com/api/reference/resources/audio/subresources/transcriptions/methods/create) |
| **Groq Speech-to-Text** | OpenAI-compatible `POST /openai/v1/audio/transcriptions` (`whisper-large-v3`, `whisper-large-v3-turbo`). | [Groq Speech-to-Text](https://console.groq.com/docs/speech-to-text) |
| **Deepgram Speech-to-Text** | `POST /v1/listen` with pre-recorded audio payload and utterance/word timing parameters. | [Deepgram Pre-recorded Audio](https://developers.deepgram.com/reference/speech-to-text/listen-pre-recorded) |
| **AssemblyAI Speech-to-Text** | `POST /v2/upload` followed by `POST /v2/transcript` creation and status polling with `speech_models` selection. | [AssemblyAI Transcripts API](https://www.assemblyai.com/docs/pre-recorded-audio/api-reference/transcripts/submit) |
| **Anthropic Claude** | Native `POST /v1/messages` with `x-api-key` + `anthropic-version: 2023-06-01` and `GET /v1/models` discovery. | [Anthropic Messages API](https://docs.anthropic.com/en/api/messages) |
| **Google Gemini** | Native `v1beta` `models/{model}:generateContent` and `GET /v1beta/models` capability discovery. | [Gemini Models API](https://ai.google.dev/gemini-api/docs/models) |
| **OpenRouter** | OpenAI-compatible `POST /api/v1/chat/completions` and `GET /api/v1/models`. | [OpenRouter API Reference](https://openrouter.ai/docs/api_reference/overview) |
| **Mistral** | OpenAI-compatible `POST /v1/chat/completions` and `GET /v1/models`. | [Mistral Chat API](https://docs.mistral.ai/api) |
| **NVIDIA NIM** | OpenAI-compatible `POST /v1/chat/completions` and `GET /v1/models`. | [NVIDIA NIM LLM APIs](https://docs.api.nvidia.com/nim/reference/llm-apis) |
| **Ollama / Local LLM** | Local OpenAI-compatible `/v1/chat/completions` and `/v1/models` (or native `/api/tags`). | [Ollama OpenAI Compatibility](https://docs.ollama.com/api/openai-compatibility) |
| **Local Whisper** | Native `whisper.cpp` inference via `whisper-rs` with GGML model weights and Silero VAD (`ggml-silero-v6.2.0.bin`). | [ggml-org/whisper.cpp](https://github.com/ggml-org/whisper.cpp) |

Custom OpenAI-compatible endpoints (`custom`, `custom_whisper`) accept user-supplied base URLs and optional API keys. To verify documentation and model-discovery URLs against live endpoints, run `python3 build-scripts/check_provider_links.py`.

Translation, annotation and refinement share the Rust translator's HTTP retry
path. It honors `Retry-After` delta-seconds and HTTP dates for 429/5xx responses;
waits longer than the 120-second retry budget return an error for failover
instead of sending an early retry. Known billing/spend-limit error codes return
immediately. Ordinary transient failures retain the bounded retry count,
redacted errors and optional diagnostics. See [Groq rate-limit headers](https://console.groq.com/docs/rate-limits),
[OpenAI error codes](https://developers.openai.com/api/docs/guides/error-codes)
and [Claude rate limits](https://platform.claude.com/docs/en/api/rate-limits).

OpenAI original GPT-5, GPT-5.6/GPT-6 and Claude 4.7+ requests omit custom
`temperature`; supported defaults such as GPT-5.1 retain the existing sampling
value. Claude requests keep API version
`2023-06-01` for existing integrations: adopting `2026-10-21` globally would remove
sampling controls even for older models. See [OpenAI migration guidance](https://developers.openai.com/api/docs/guides/latest-model)
and [Claude version migration](https://platform.claude.com/docs/en/api/migration-2026-10-21).

The 2026-11-18 review checked official provider documentation and unauthenticated
model discovery, without uploading user audio or running paid inference.
OpenAI's legacy `whisper-1` and GPT-4o transcription models remain available but
are scheduled for retirement on 2027-02-26. Existing timestamp/translation flows
retain them for compatibility; `gpt-transcribe` is an additional text-only
transcription option, not a replacement for timestamp-based alignment or audio
translation. See [file transcription](https://developers.openai.com/api/docs/guides/speech-to-text)
and [deprecations](https://developers.openai.com/api/docs/deprecations).

The NVIDIA fallback uses `openai/gpt-oss-20b`, present in the public catalog and
[documented Chat Completions example](https://build.nvidia.com/openai/gpt-oss-20b).
Unavailable static DeepSeek V4.1 and old Nemotron entries were removed; saved
user selections are not migrated. The fallback catalogs also include current
Gemini 4, GPT-6.2 Sol and Claude 5.5 choices. Live account discovery remains
authoritative; new catalog entries do not change existing saved model choices.
