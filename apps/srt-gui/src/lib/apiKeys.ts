import * as vestaConfig from "./vestaConfig";

// API key management: the provider id union (shared between LLM providers
// and transcription providers — deepgram/assemblyai/local_whisper/...) and
// the ApiKeyConfig shape used to store/validate saved keys. Kept separate
// from llmProviders.ts because ApiProviderId is not LLM-catalog-specific.

export type ApiProviderId =
  | "local"
  | "google"
  | "groq"
  | "openrouter"
  | "mistral"
  | "github"
  | "nvidia"
  | "custom"
  | "deepgram"
  | "assemblyai"
  | "local_whisper"
  | "custom_whisper"
  // legacy, mantenuti per retrocompatibilità nello storage
  | "openai"
  | "anthropic";

export interface ApiKeyConfig {
  id: string;
  name: string;
  apiType: ApiProviderId;
  apiKey: string;
  apiUrl?: string;
  modelName?: string;  // Nome modello preferito
  isDefault?: boolean;
  isValid?: boolean;
}

// Helper per caricare e validare le chiavi API
export function loadAndValidateApiKeys(): ApiKeyConfig[] {
  const saved = vestaConfig.getItem("srt-tools-api-keys");
  if (!saved) return [];

  try {
    const parsed = JSON.parse(saved);
    if (!Array.isArray(parsed)) return [];

    const validTypes = new Set([
      "local",
      "google",
      "groq",
      "custom",
      "openrouter",
      "mistral",
      "github",
      "nvidia",
      "openai",
      "deepgram",
      "assemblyai",
      "local_whisper",
      "custom_whisper",
    ]);

    // Migra i tipi legacy e filtra le chiavi non valide.
    // Regola: una chiave viene riclassificata solo quando la conversione è
    // certa (le chiavi Google iniziano con "AIza"); tutto il resto viene
    // scartato — forzare una chiave sconosciuta a "google" produrrebbe solo
    // errori 401 a runtime difficili da diagnosticare.
    const converted = parsed.map((k: any) => {
      // Se già ha un tipo valido, mantienilo
      if (validTypes.has(k.apiType)) {
        return k;
      }
      // Tipi legacy (gemini, ecc.): le chiavi Google iniziano con "AIza"
      if (k.apiKey && k.apiKey.startsWith("AIza")) {
        return {
          ...k,
          apiType: "google" as const,
          apiUrl: "https://generativelanguage.googleapis.com/v1beta"
        };
      }
      // Chiave legacy non riconoscibile: scartala (l'utente la ri-aggiunge
      // dalle impostazioni con il provider corretto).
      return null;
    }).filter((k: any) => k !== null);

    return converted as ApiKeyConfig[];
  } catch {
    return [];
  }
}
