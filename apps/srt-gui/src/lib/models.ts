import { languages } from "./languages";

// LLM provider/model catalog (FamilyInfo, ModelInfo, ProviderInfo, providers,
// providerOrder, modelsByProvider, getModelsForProvider) lives in llmProviders.ts.
// saveCustomModel/deleteCustomModel/getCustomModels were dropped here: dead code,
// no import site anywhere in the app (getModelsForProvider's own custom-model
// merge logic is the only live reader of the "srt-tools-custom-models" key).

// formatContextWindow was dropped here: dead code, zero import sites
// anywhere in the app (it formatted ModelInfo.contextWindow for a UI that
// no longer calls it).

// Field names / note types / card templates (FieldNamesConfig, NoteTypeDef,
// CardTemplateConfig and all their load/save functions) live in noteTypes.ts
// — one cohesive module, see its header comment for why it wasn't split
// further.

// ApiProviderId/ApiKeyConfig stay here (not in llmProviders.ts): shared between
// LLM providers and transcription providers (deepgram/assemblyai/local_whisper/...).
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
  const saved = localStorage.getItem("srt-tools-api-keys");
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

// ─── Translation Tiers (priority list with automatic failover) ────────────────
//
// Una "tier list" è una lista ordinata di tier. tiers[0] è il tier a priorità
// massima. Ogni tier contiene una o più entry; all'interno del tier le entry
// vengono usate in round-robin, e quando un intero tier esaurisce i limiti si
// passa automaticamente al tier successivo (failover, gestito dal backend Rust).
//
// Una entry punta a una API key salvata (apiKeyId) e a un modello specifico:
// così la stessa key può comparire più volte con modelli diversi nello stesso
// tier, sfruttando quote separate per modello con un'unica chiave.

export interface TierEntry {
  id: string;
  provider: ApiProviderId;
  /** Riferimento a ApiKeyConfig.id. Vuoto per provider locali senza key. */
  apiKeyId: string;
  /** Id del modello da chiamare. */
  model: string;
  /** Override opzionale RPM (richieste/minuto). */
  rpm?: number;
  /** Budget opzionale di richieste per run. */
  maxRequests?: number;
}

export interface Tier {
  id: string;
  entries: TierEntry[];
}

const TIERS_KEY = "vesta-translate-tiers";
export const TIERS_UPDATED_EVENT = "vesta:tiers-updated";

export function newTierId(): string {
  return `tier-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
}

export function newTierEntryId(): string {
  return `te-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
}

export function loadTiers(): Tier[] {
  try {
    const raw = localStorage.getItem(TIERS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        return parsed
          .filter((t) => t && Array.isArray(t.entries))
          .map((t) => ({
            id: typeof t.id === "string" ? t.id : newTierId(),
            entries: (t.entries as any[])
              .filter((e) => e && typeof e.provider === "string")
              .map((e) => ({
                id: typeof e.id === "string" ? e.id : newTierEntryId(),
                provider: e.provider as ApiProviderId,
                apiKeyId: typeof e.apiKeyId === "string" ? e.apiKeyId : "",
                model: typeof e.model === "string" ? e.model : "",
                rpm: typeof e.rpm === "number" && e.rpm > 0 ? e.rpm : undefined,
                maxRequests:
                  typeof e.maxRequests === "number" && e.maxRequests > 0
                    ? e.maxRequests
                    : undefined,
              })),
          }));
      }
    }
  } catch {
    /* ignore */
  }
  return [];
}

export function saveTiers(tiers: Tier[]): void {
  localStorage.setItem(TIERS_KEY, JSON.stringify(tiers));
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(TIERS_UPDATED_EVENT));
  }
}

/** True se almeno una entry ha un modello valido (tier configurabili e usabili). */
export function tiersHaveUsableEntries(tiers: Tier[]): boolean {
  return tiers.some((t) => t.entries.some((e) => e.model && e.model.trim().length > 0));
}

// ─── Transcribe Tiers ──────────────────────────────────────────────────────────

export interface TranscribeTierEntry {
  id: string;
  provider: string; // "local" | "groq" | "openai" | "deepgram" | "assemblyai" | "custom"
  apiKeyId: string;
  model: string;
  rpm?: number;
  maxRequests?: number;
}

export interface TranscribeTier {
  id: string;
  entries: TranscribeTierEntry[];
}

const TRANSCRIBE_TIERS_KEY = "vesta-transcribe-tiers";
export const TRANSCRIBE_TIERS_UPDATED_EVENT = "vesta:transcribe-tiers-updated";

export function loadTranscribeTiers(): TranscribeTier[] {
  try {
    const raw = localStorage.getItem(TRANSCRIBE_TIERS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        return parsed
          .filter((t) => t && Array.isArray(t.entries))
          .map((t) => ({
            id: typeof t.id === "string" ? t.id : newTierId(),
            entries: (t.entries as any[])
              .filter((e) => e && typeof e.provider === "string")
              .map((e) => ({
                id: typeof e.id === "string" ? e.id : newTierEntryId(),
                provider: e.provider,
                apiKeyId: typeof e.apiKeyId === "string" ? e.apiKeyId : "",
                model: typeof e.model === "string" ? e.model : "",
                rpm: typeof e.rpm === "number" && e.rpm > 0 ? e.rpm : undefined,
                maxRequests: typeof e.maxRequests === "number" && e.maxRequests > 0 ? e.maxRequests : undefined,
              })),
          }));
      }
    }
  } catch {
    /* ignore */
  }
  // Default: one tier with local base model
  return [
    {
      id: newTierId(),
      entries: [
        {
          id: newTierEntryId(),
          provider: "local",
          apiKeyId: "",
          model: "base",
        }
      ]
    }
  ];
}

export function saveTranscribeTiers(tiers: TranscribeTier[]): void {
  localStorage.setItem(TRANSCRIBE_TIERS_KEY, JSON.stringify(tiers));
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(TRANSCRIBE_TIERS_UPDATED_EVENT));
  }
}

export function transcribeTiersHaveUsableEntries(tiers: TranscribeTier[]): boolean {
  return tiers.some((t) => t.entries.some((e) => e.model && e.model.trim().length > 0));
}

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
    const raw = localStorage.getItem(TRANSCRIBE_CLOUD_KEY);
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
  localStorage.setItem(TRANSCRIBE_CLOUD_KEY, JSON.stringify(s));
}

// ─── Silero VAD variant selection ───────────────────────────────────────────
// Which VAD model to run when the user has VAD enabled: one of the built-in
// variants (see `srt_transcribe::model::VAD_MODELS`, mirrored here only by
// id) or a custom local path picked via file dialog. Shared between
// SettingsTab (download/select/upload) and TranscribeTab (resolves the
// active choice into `transcribe_start`'s config).

export const DEFAULT_VAD_MODEL_ID = "v5.1.2";

export interface VadSelection {
  /** Id of the selected built-in variant, ignored when `customPath` is set. */
  modelId: string;
  /** Absolute path to a user-provided .bin, overrides `modelId` when set. */
  customPath: string | null;
}

const VAD_SELECTION_KEY = "vesta-transcribe-vad-selection";

export function loadVadSelection(): VadSelection {
  const fallback: VadSelection = { modelId: DEFAULT_VAD_MODEL_ID, customPath: null };
  try {
    const raw = localStorage.getItem(VAD_SELECTION_KEY);
    if (raw) {
      const p = JSON.parse(raw);
      return {
        modelId: typeof p.modelId === "string" ? p.modelId : DEFAULT_VAD_MODEL_ID,
        customPath: typeof p.customPath === "string" ? p.customPath : null,
      };
    }
  } catch {
    /* ignore */
  }
  return fallback;
}

export function saveVadSelection(s: VadSelection): void {
  localStorage.setItem(VAD_SELECTION_KEY, JSON.stringify(s));
}



// ─── Shared Utilities ───────────────────────────────────────────────────────

export function getFileName(path: string): string {
  const normalized = path.replace(/\\/g, "/");
  return normalized.split("/").pop() || path;
}

const knownLangCodes = new Set(languages.map((l) => l.code.toLowerCase()));

export function inferLanguageFromPath(filePath: string): string | null {
  const filename = getFileName(filePath).toLowerCase();
  const base = filename.replace(/\.[^/.]+$/, "");
  const tokens = base.split(/[.\-_]+/).filter(Boolean);
  for (let i = tokens.length - 1; i >= 0; i--) {
    if (knownLangCodes.has(tokens[i])) {
      const lang = languages.find((l) => l.code.toLowerCase() === tokens[i]);
      if (lang) return lang.code;
    }
  }
  return null;
}

export function getFlagForPath(path: string): string {
  const code = inferLanguageFromPath(path);
  if (!code) return "";
  const lang = languages.find((l) => l.code === code);
  return lang?.flag || "";
}



