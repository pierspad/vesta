# Provider compatibility review

Reviewed 2026-10-19 against the providers' public documentation. This is a contract and documentation-link review; it does not claim authenticated inference was tested with users' API keys. Account permissions, regional availability and individual model retirement still depend on the provider. The app's model discovery is preferable to hardcoded lists where supported.

| Integration | Public contract checked | Result |
| --- | --- | --- |
| OpenAI transcription | [Transcriptions](https://developers.openai.com/api/reference/resources/audio/subresources/transcriptions/methods/create) | `/v1/audio/transcriptions` remains supported. Whisper uses verbose JSON with timestamps; GPT transcription uses JSON and does not request Whisper-only word timestamp options. Updated documentation links to developers.openai.com. |
| Groq | [Speech to text](https://console.groq.com/docs/speech-to-text) | OpenAI-compatible `/openai/v1/audio/transcriptions`, large-v3 and large-v3-turbo remain supported. |
| Gemini | [Models](https://ai.google.dev/gemini-api/docs/models) | Native v1beta generateContent integration and model discovery retained. |
| AssemblyAI | [Submit transcript](https://www.assemblyai.com/docs/pre-recorded-audio/api-reference/transcripts/submit) | v2 upload/transcript polling and speech_models selection retained. |
| Deepgram | [Pre-recorded audio](https://developers.deepgram.com/reference/speech-to-text/listen-pre-recorded) | v1 listen integration retained; modal now links to the actual endpoint reference. |
| OpenRouter | [API reference](https://openrouter.ai/docs/api_reference/overview) | `/api/v1/chat/completions` and OpenAI-compatible response parsing retained. |
| Mistral | [Chat API](https://docs.mistral.ai/api) | `/v1/chat/completions` integration retained. |
| NVIDIA NIM | [LLM APIs](https://docs.api.nvidia.com/nim/reference/llm-apis) | OpenAI-compatible integration retained; replaced generic NVIDIA documentation home with NIM API reference. |
| Ollama | [OpenAI compatibility](https://docs.ollama.com/api/openai-compatibility) | Local `/v1` endpoint retained; replaced marketing homepage with compatibility documentation. |
| Local Whisper | [whisper.cpp](https://github.com/ggml-org/whisper.cpp) | Updated repository link to ggml-org. |

Custom endpoints are user supplied and cannot be universally validated against a provider's public contract. Disabled integrations are not advertised as supported merely because their API still exists.
