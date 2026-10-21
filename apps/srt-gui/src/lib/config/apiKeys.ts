import * as vestaConfig from "$lib/config/vestaConfig";

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
  capabilities?: ("llm" | "stt")[];
  modelName?: string;  // Nome modello preferito
  isDefault?: boolean;
  isValid?: boolean;
}

// Helper per caricare e validare le chiavi API
export function loadAndValidateApiKeys(): ApiKeyConfig[] {
  const saved = vestaConfig.getItem("srt-tools-api-keys");
  if (!saved) return withBuiltinEndpoints([]);

  try {
    const parsed = JSON.parse(saved);
    if (!Array.isArray(parsed)) return withBuiltinEndpoints([]);

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

    return withBuiltinEndpoints(converted as ApiKeyConfig[]);
  } catch {
    return withBuiltinEndpoints([]);
  }
}

/** Provider support is explicit; sharing credentials does not imply API compatibility. */
export function supportsCapability(key: ApiKeyConfig, capability: "llm" | "stt"): boolean {
  if (key.capabilities) return key.capabilities.includes(capability);
  return capability === "stt"
    ? ["groq", "openai", "deepgram", "assemblyai", "custom_whisper", "local_whisper"].includes(key.apiType)
    : ["local", "google", "groq", "openai", "openrouter", "mistral", "nvidia", "github", "custom"].includes(key.apiType);
}

function withBuiltinEndpoints(keys: ApiKeyConfig[]): ApiKeyConfig[] {
  const defaults: ApiKeyConfig[] = [
    {id: "local", name: "Local LLM", apiType: "local", apiKey: "", apiUrl: vestaConfig.getItem("vesta-local-server-url") || "http://localhost:11434/v1", capabilities: ["llm"]},
    {id: "local_stt_server", name: "Local STT server", apiType: "custom", apiKey: "", apiUrl: "http://localhost:8000/v1", capabilities: ["stt"]},
  ];
  return [...defaults.filter(item => !keys.some(key => key.id === item.id)), ...keys];
}

/** Names are unique across all providers, including built-ins and both usages. */
export function hasProviderNameConflict(keys: ApiKeyConfig[], name: string, editingId: string | null = null): boolean {
  const normalize = (value: string) => value.normalize("NFKC").trim().replace(/\s+/g, " ").toLowerCase();
  const normalized = normalize(name);
  return Boolean(normalized) && keys.some(key => key.id !== editingId && normalize(key.name) === normalized);
}
