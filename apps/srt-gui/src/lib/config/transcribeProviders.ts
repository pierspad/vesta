import * as vestaConfig from "$lib/config/vestaConfig";

// ─── Cloud transcription providers (Whisper tab) ──────────────────────────────
//
// Motori di trascrizione cloud, in aggiunta a Whisper locale. Tutti vengono
// normalizzati dal backend (srt-transcribe/cloud.rs) in segmenti SRT.

export interface TranscribeProviderInfo {
  id: string;
  name: string;
  color: string;
  description: string;
  defaultUrl?: string;
  apiKeyUrl?: string;
  keyPlaceholder?: string;
  /** true per il provider "custom" che richiede l'URL. */
  requiresUrl?: boolean;
  /** true se il provider ignora il campo modello (es. AssemblyAI). */
  modelOptional?: boolean;
  models: { id: string; name: string; recommended?: boolean }[];
}

export const transcribeProviders: Record<string, TranscribeProviderInfo> = {
  groq: {
    id: "groq",
    name: "Groq Cloud",
    color: "from-orange-400 to-red-500",
    description: "Whisper large-v3 ultra-veloce su LPU",
    defaultUrl: "https://api.groq.com/openai/v1",
    apiKeyUrl: "https://console.groq.com/keys",
    keyPlaceholder: "gsk_...",
    models: [
      { id: "whisper-large-v3-turbo", name: "Whisper large-v3 turbo", recommended: true },
      { id: "whisper-large-v3", name: "Whisper large-v3" },
    ],
  },
  openai: {
    id: "openai",
    name: "OpenAI API",
    color: "from-emerald-500 to-teal-500",
    description: "Whisper-1 e gpt-4o-transcribe",
    defaultUrl: "https://api.openai.com/v1",
    apiKeyUrl: "https://platform.openai.com/api-keys",
    keyPlaceholder: "sk-...",
    models: [
      { id: "whisper-1", name: "Whisper-1 (con timestamp)", recommended: true },
      { id: "gpt-4o-mini-transcribe", name: "GPT-4o mini transcribe" },
      { id: "gpt-4o-transcribe", name: "GPT-4o transcribe" },
    ],
  },
  deepgram: {
    id: "deepgram",
    name: "Deepgram",
    color: "from-violet-500 to-fuchsia-500",
    description: "Nova-3, timestamp per parola e utterances",
    defaultUrl: "https://api.deepgram.com/v1",
    apiKeyUrl: "https://console.deepgram.com",
    keyPlaceholder: "Deepgram API key",
    models: [
      { id: "nova-3", name: "Nova-3", recommended: true },
      { id: "nova-2", name: "Nova-2" },
    ],
  },
  assemblyai: {
    id: "assemblyai",
    name: "AssemblyAI",
    color: "from-indigo-500 to-blue-500",
    description: "Upload asincrono, timestamp accurati",
    defaultUrl: "https://api.assemblyai.com/v2",
    apiKeyUrl: "https://www.assemblyai.com/app/api-keys",
    keyPlaceholder: "AssemblyAI API key",
    modelOptional: true,
    models: [
      { id: "best", name: "Best", recommended: true },
      { id: "nano", name: "Nano" },
    ],
  },
  custom: {
    id: "custom",
    name: "Custom (OpenAI-compatible)",
    color: "from-gray-500 to-gray-600",
    description: "Qualsiasi endpoint /audio/transcriptions compatibile",
    requiresUrl: true,
    keyPlaceholder: "API key (opzionale)",
    models: [],
  },
};

export const transcribeProviderOrder = ["groq", "openai", "deepgram", "assemblyai", "custom"];

export interface TranscribeCloudSettings {
  /** "local" (Whisper locale) oppure un id provider cloud. */
  engine: string;
  /** Modello cloud selezionato per l'engine corrente. */
  model: string;
  /** URL custom (solo per engine "custom"). */
  customUrl: string;
  /** API key per provider: { groq, openai, deepgram, assemblyai, custom }. */
  keys: Record<string, string>;
}

const TRANSCRIBE_CLOUD_KEY = "vesta-transcribe-cloud";

export function loadTranscribeCloud(): TranscribeCloudSettings {
  const fallback: TranscribeCloudSettings = { engine: "local", model: "", customUrl: "", keys: {} };
  try {
    const raw = vestaConfig.getItem(TRANSCRIBE_CLOUD_KEY);
    if (raw) {
      const p = JSON.parse(raw);
      return {
        engine: typeof p.engine === "string" ? p.engine : "local",
        model: typeof p.model === "string" ? p.model : "",
        customUrl: typeof p.customUrl === "string" ? p.customUrl : "",
        keys: p.keys && typeof p.keys === "object" ? p.keys : {},
      };
    }
  } catch {
    /* ignore */
  }
  return fallback;
}

export function saveTranscribeCloud(s: TranscribeCloudSettings): void {
  vestaConfig.setItem(TRANSCRIBE_CLOUD_KEY, JSON.stringify(s));
}
