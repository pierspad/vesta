# External Provider API Compatibility

This reference lists the public HTTP contracts and endpoint conventions used by Vesta's cloud transcription, translation, and annotation integrations.

Account permissions, rate limits, regional availability, and model lifecycles are controlled by each provider. Where supported, Vesta queries the provider's model discovery endpoint at runtime and falls back to static catalog defaults on network or authentication errors.

| Integration | Endpoint & Protocol Contract | Reference Documentation |
| --- | --- | --- |
| **OpenAI Transcription** | `POST /v1/audio/transcriptions` (multipart). Whisper models request `verbose_json` with segment/word timestamps; GPT transcription models (`gpt-4o-transcribe`, `gpt-4o-mini-transcribe`) request `json` without Whisper-only timestamp flags. | [OpenAI Audio Transcriptions](https://developers.openai.com/api/reference/resources/audio/subresources/transcriptions/methods/create) |
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
