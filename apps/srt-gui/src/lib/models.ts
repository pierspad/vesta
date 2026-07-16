import { languages } from "./languages";

// LLM provider/model catalog (FamilyInfo, ModelInfo, ProviderInfo, providers,
// providerOrder, modelsByProvider, getModelsForProvider) lives in llmProviders.ts.
// saveCustomModel/deleteCustomModel/getCustomModels were dropped here: dead code,
// no import site anywhere in the app (getModelsForProvider's own custom-model
// merge logic is the only live reader of the "srt-tools-custom-models" key).

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


// Shortcut predefinite
export interface ShortcutDefinition {
  id: string;
  action: string;
  description: string; // i18n key, resolved via t() in components
  defaultKey: string;
  category: "global" | "translate" | "sync" | "flashcards" | "alignment" | "transcribe";
}

export const defaultShortcuts: ShortcutDefinition[] = [
  // Global — tab navigation follows sidebar order (Flashcards first)
  { id: "tab-flashcards", action: "switchToFlashcards", description: "shortcuts.action.goToFlashcards", defaultKey: "Alt+1", category: "global" },
  { id: "tab-refine", action: "switchToRefine", description: "shortcuts.action.goToRefine", defaultKey: "Alt+2", category: "global" },
  { id: "tab-translate", action: "switchToTranslate", description: "shortcuts.action.goToTranslation", defaultKey: "Alt+3", category: "global" },
  { id: "tab-sync", action: "switchToSync", description: "shortcuts.action.goToSync", defaultKey: "Alt+4", category: "global" },
  { id: "tab-align", action: "switchToAlign", description: "shortcuts.action.goToAlign", defaultKey: "Alt+5", category: "global" },
  { id: "tab-transcribe", action: "switchToTranscribe", description: "shortcuts.action.goToTranscribe", defaultKey: "Alt+6", category: "global" },
  { id: "tab-settings", action: "switchToSettings", description: "shortcuts.action.goToSettings", defaultKey: "Alt+7", category: "global" },
  { id: "tab-shortcuts", action: "switchToShortcuts", description: "shortcuts.action.goToShortcuts", defaultKey: "Alt+8", category: "global" },
  { id: "settings-add-key", action: "addApiKey", description: "shortcuts.action.addApiKey", defaultKey: "Ctrl+N", category: "global" },
  { id: "show-help", action: "showShortcutHelp", description: "shortcuts.action.showHelp", defaultKey: "Shift+?", category: "global" },

  // Flashcards
  { id: "flashcards-generate", action: "generateFlashcards", description: "shortcuts.action.generateFlashcards", defaultKey: "Ctrl+Enter", category: "flashcards" },
  { id: "flashcards-cancel", action: "cancelGeneration", description: "shortcuts.action.cancelGeneration", defaultKey: "Escape", category: "flashcards" },
  { id: "flashcards-preview", action: "previewCards", description: "shortcuts.action.previewCards", defaultKey: "Ctrl+P", category: "flashcards" },

  // Translation
  { id: "translate-open-file", action: "openInputFile", description: "shortcuts.action.openSrt", defaultKey: "Ctrl+O", category: "translate" },
  { id: "translate-start", action: "startTranslation", description: "shortcuts.action.startTranslation", defaultKey: "Ctrl+Enter", category: "translate" },
  { id: "translate-cancel", action: "cancelTranslation", description: "shortcuts.action.cancelTranslation", defaultKey: "Escape", category: "translate" },
  { id: "translate-clear-logs", action: "clearLogs", description: "shortcuts.action.clearLogs", defaultKey: "Ctrl+L", category: "translate" },

  // Synchronization
  { id: "sync-open-file", action: "openSrt", description: "shortcuts.action.openSrt", defaultKey: "Ctrl+O", category: "sync" },
  { id: "sync-auto", action: "startAutoSync", description: "shortcuts.action.autoSync", defaultKey: "Ctrl+A", category: "sync" },
  { id: "sync-new", action: "newSync", description: "shortcuts.action.newSync", defaultKey: "Ctrl+N", category: "sync" },
  { id: "sync-load-session", action: "loadSession", description: "shortcuts.action.loadSession", defaultKey: "Ctrl+L", category: "sync" },
  { id: "sync-save-session", action: "saveSession", description: "shortcuts.action.saveSession", defaultKey: "Ctrl+Shift+S", category: "sync" },
  { id: "sync-play-pause", action: "playPause", description: "shortcuts.action.playPause", defaultKey: "Space", category: "sync" },
  { id: "sync-seek-back", action: "seekBack", description: "shortcuts.action.back01s", defaultKey: "ArrowLeft", category: "sync" },
  { id: "sync-seek-forward", action: "seekForward", description: "shortcuts.action.forward01s", defaultKey: "ArrowRight", category: "sync" },
  { id: "sync-seek-back-fast", action: "seekBackFast", description: "shortcuts.action.back1s", defaultKey: "Shift+ArrowLeft", category: "sync" },
  { id: "sync-seek-forward-fast", action: "seekForwardFast", description: "shortcuts.action.forward1s", defaultKey: "Shift+ArrowRight", category: "sync" },
  { id: "sync-offset-up", action: "offsetUp", description: "shortcuts.action.offsetUp", defaultKey: "ArrowUp", category: "sync" },
  { id: "sync-offset-down", action: "offsetDown", description: "shortcuts.action.offsetDown", defaultKey: "ArrowDown", category: "sync" },
  { id: "sync-offset-up-fast", action: "offsetUpFast", description: "shortcuts.action.offsetUpFast", defaultKey: "Shift+ArrowUp", category: "sync" },
  { id: "sync-offset-down-fast", action: "offsetDownFast", description: "shortcuts.action.offsetDownFast", defaultKey: "Shift+ArrowDown", category: "sync" },
  { id: "sync-undo", action: "syncUndo", description: "shortcuts.action.syncUndo", defaultKey: "Ctrl+Z", category: "sync" },
  { id: "sync-confirm", action: "confirmAnchor", description: "shortcuts.action.confirmAnchor", defaultKey: "Enter", category: "sync" },
  { id: "sync-next-sub", action: "nextSubtitle", description: "shortcuts.action.nextSubtitle", defaultKey: "Tab", category: "sync" },
  { id: "sync-prev-sub", action: "prevSubtitle", description: "shortcuts.action.prevSubtitle", defaultKey: "Shift+Tab", category: "sync" },
  { id: "sync-prev-anchor", action: "prevAnchor", description: "shortcuts.action.prevAnchor", defaultKey: "Ctrl+ArrowUp", category: "sync" },
  { id: "sync-next-anchor", action: "nextAnchor", description: "shortcuts.action.nextAnchor", defaultKey: "Ctrl+ArrowDown", category: "sync" },
  { id: "sync-go-suggested", action: "goToSuggested", description: "shortcuts.action.goToSuggested", defaultKey: "Ctrl+G", category: "sync" },
  { id: "sync-save", action: "saveFile", description: "shortcuts.action.saveFile", defaultKey: "Ctrl+S", category: "sync" },

  // Alignment
  { id: "align-open-file", action: "openInputFile", description: "shortcuts.action.openSrt", defaultKey: "Ctrl+O", category: "alignment" },
  { id: "align-next-page", action: "alignNextPage", description: "shortcuts.action.alignNextPage", defaultKey: "Tab", category: "alignment" },
  { id: "align-prev-page", action: "alignPrevPage", description: "shortcuts.action.alignPrevPage", defaultKey: "Shift+Tab", category: "alignment" },
  { id: "align-swap-files", action: "alignSwapFiles", description: "shortcuts.action.alignSwapFiles", defaultKey: "Ctrl+Shift+S", category: "alignment" },
  { id: "align-undo", action: "alignUndo", description: "shortcuts.action.alignUndo", defaultKey: "Ctrl+Z", category: "alignment" },
  { id: "align-save", action: "alignSave", description: "shortcuts.action.alignSave", defaultKey: "Ctrl+S", category: "alignment" },
  { id: "align-cycle-per-page", action: "alignCyclePerPage", description: "shortcuts.action.alignCyclePerPage", defaultKey: "Ctrl+Shift+P", category: "alignment" },

  // Transcribe
  { id: "transcribe-open-file", action: "openInputFile", description: "shortcuts.action.openSrt", defaultKey: "Ctrl+O", category: "transcribe" },
  { id: "transcribe-start", action: "startTranscription", description: "shortcuts.action.startTranscription", defaultKey: "Ctrl+Enter", category: "transcribe" },
  { id: "transcribe-cancel", action: "cancelTranscription", description: "shortcuts.action.cancelTranscription", defaultKey: "Escape", category: "transcribe" },
];

// Funzione per ottenere le shortcut (con override utente)
export function getShortcuts(): ShortcutDefinition[] {
  const overridesJson = localStorage.getItem("srt-tools-shortcut-overrides");
  if (!overridesJson) return defaultShortcuts;

  try {
    const overrides: Record<string, string> = JSON.parse(overridesJson);
    return defaultShortcuts.map((shortcut) => ({
      ...shortcut,
      defaultKey: overrides[shortcut.id] || shortcut.defaultKey,
    }));
  } catch {
    return defaultShortcuts;
  }
}

// Funzione per salvare override shortcut
export function saveShortcutOverride(shortcutId: string, newKey: string): void {
  const overridesJson = localStorage.getItem("srt-tools-shortcut-overrides");
  let overrides: Record<string, string> = {};

  if (overridesJson) {
    try {
      overrides = JSON.parse(overridesJson);
    } catch {
      overrides = {};
    }
  }

  overrides[shortcutId] = newKey;
  localStorage.setItem("srt-tools-shortcut-overrides", JSON.stringify(overrides));
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("shortcuts-updated"));
  }
}

// Funzione per resettare le shortcut
export function resetShortcuts(): void {
  localStorage.removeItem("srt-tools-shortcut-overrides");
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("shortcuts-updated"));
  }
}

// Funzione per resettare una singola shortcut con cascade dei conflitti
export function resetSingleShortcut(shortcutId: string): void {
  const overridesJson = localStorage.getItem("srt-tools-shortcut-overrides");
  let overrides: Record<string, string> = {};
  if (overridesJson) {
    try {
      overrides = JSON.parse(overridesJson);
    } catch {
      overrides = {};
    }
  }

  const resetQueue: string[] = [shortcutId];
  const processed = new Set<string>();

  while (resetQueue.length > 0) {
    const currentId = resetQueue.shift()!;
    if (processed.has(currentId)) continue;
    processed.add(currentId);

    // Rimuovi l'override per far tornare la scorciatoia al valore predefinito
    delete overrides[currentId];

    // Trova il tasto predefinito di questa scorciatoia
    const defaultDef = defaultShortcuts.find((s) => s.id === currentId);
    if (!defaultDef) continue;
    const revertedKey = defaultDef.defaultKey;

    // Coda per il reset tutte le ALTRE scorciatoie che attualmente risolvono a revertedKey
    for (const s of defaultShortcuts) {
      if (s.id === currentId || processed.has(s.id)) continue;

      const currentKey = overrides[s.id] || s.defaultKey;
      if (currentKey === revertedKey) {
        resetQueue.push(s.id);
      }
    }
  }

  localStorage.setItem("srt-tools-shortcut-overrides", JSON.stringify(overrides));
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("shortcuts-updated"));
  }
}

// Funzione per formattare context window
export function formatContextWindow(tokens: number): string {
  if (tokens >= 1000000) {
    return `${(tokens / 1000000).toFixed(1)}M`;
  }
  return `${(tokens / 1000).toFixed(0)}K`;
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

// ─── Card Templates ──────────────────────────────────────────────────────────

export interface CardTemplateConfig {
  frontHtml: string;
  backHtml: string;
  css: string;
  noteTypeName: string;
}

export interface FieldNamesConfig {
  expression: string;
  meaning: string;
  reading: string;
  audio: string;
  snapshot: string;
  video: string;
  tags: string;
  sequenceMarker: string;
  notes: string;
}

const CARD_TEMPLATE_KEY = "vesta-card-templates";
const FIELD_NAMES_KEY = "vesta-field-names";

export const NOTE_TYPE_FIELD_SOFT_LIMIT = 25;
export const CARD_TEMPLATES_UPDATED_EVENT = "vesta:card-templates-updated";
export const FIELD_NAMES_UPDATED_EVENT = "vesta:field-names-updated";

export function limitNoteTypeFieldValue(value: string): string {
  return value.slice(0, NOTE_TYPE_FIELD_SOFT_LIMIT);
}

function sanitizeFieldNamesConfig(config: FieldNamesConfig): FieldNamesConfig {
  return {
    expression: "Expression",
    meaning: limitNoteTypeFieldValue(config.meaning),
    reading: limitNoteTypeFieldValue(config.reading),
    audio: limitNoteTypeFieldValue(config.audio),
    snapshot: limitNoteTypeFieldValue(config.snapshot),
    video: limitNoteTypeFieldValue(config.video),
    tags: limitNoteTypeFieldValue(config.tags),
    sequenceMarker: "SequenceMarker",
    notes: limitNoteTypeFieldValue(config.notes),
  };
}

function sanitizeCardTemplateConfig(config: CardTemplateConfig): CardTemplateConfig {
  return {
    ...config,
    noteTypeName: limitNoteTypeFieldValue(config.noteTypeName),
  };
}

function dispatchWindowEvent(eventName: string) {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(eventName));
  }
}

export const defaultFieldNames: FieldNamesConfig = {
  expression: "Expression",
  meaning: "Meaning",
  reading: "Reading",
  audio: "Audio",
  snapshot: "Snapshot",
  video: "Video",
  tags: "Tags",
  sequenceMarker: "SequenceMarker",
  notes: "Notes",
};

export function loadFieldNames(): FieldNamesConfig {
  try {
    const raw = localStorage.getItem(FIELD_NAMES_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return sanitizeFieldNamesConfig({
        expression: parsed.expression !== undefined && parsed.expression !== "" ? parsed.expression : defaultFieldNames.expression,
        meaning: parsed.meaning !== undefined ? parsed.meaning : defaultFieldNames.meaning,
        reading: parsed.reading !== undefined ? parsed.reading : defaultFieldNames.reading,
        audio: parsed.audio !== undefined ? parsed.audio : defaultFieldNames.audio,
        snapshot: parsed.snapshot !== undefined ? parsed.snapshot : defaultFieldNames.snapshot,
        video: parsed.video !== undefined ? parsed.video : defaultFieldNames.video,
        tags: parsed.tags !== undefined ? parsed.tags : defaultFieldNames.tags,
        sequenceMarker: parsed.sequenceMarker !== undefined ? parsed.sequenceMarker : defaultFieldNames.sequenceMarker,
        notes: parsed.notes !== undefined ? parsed.notes : defaultFieldNames.notes,
      });
    }
  } catch { /* ignore */ }
  return sanitizeFieldNamesConfig({ ...defaultFieldNames });
}

export function saveFieldNames(config: FieldNamesConfig): void {
  const sanitized = sanitizeFieldNamesConfig(config);
  localStorage.setItem(FIELD_NAMES_KEY, JSON.stringify(sanitized));
  dispatchWindowEvent(FIELD_NAMES_UPDATED_EVENT);
}

export function resetFieldNames(): FieldNamesConfig {
  localStorage.removeItem(FIELD_NAMES_KEY);
  dispatchWindowEvent(FIELD_NAMES_UPDATED_EVENT);
  return sanitizeFieldNamesConfig({ ...defaultFieldNames });
}

// ─── Note Types ──────────────────────────────────────────────────────────────
//
// A "note type" is the unit the user selects when generating flashcards. It is a
// name + the nine canonical fields + which of them are active. There are two
// kinds:
//
//   • predefined — one per study language (e.g. "English_Vesta"). Locked: always
//     all nine fields, with the canonical names. Generated on the fly from the
//     languages list, never stored.
//   • custom — created by the user (typically by forking a predefined one),
//     stored in localStorage. May switch fields off to get a smaller schema.
//
// The Rust exporter keys the Anki model id off the note type *name* and emits
// exactly the active fields, so as long as a given name always carries the same
// active set (predefined are locked; a custom saves its own fixed set) Anki keeps
// merging re-imports into one note type.

export type FieldKey = keyof FieldNamesConfig;

/** Canonical field order — MUST match the schema order in the Rust exporter. */
export const NOTE_TYPE_FIELD_ORDER: FieldKey[] = [
  "expression",
  "meaning",
  "audio",
  "snapshot",
  "video",
  "tags",
  "sequenceMarker",
  "reading",
  "notes",
];

export type FieldToggles = Record<FieldKey, boolean>;

export const allFieldsIncluded: FieldToggles = {
  expression: true,
  meaning: true,
  reading: true,
  audio: true,
  snapshot: true,
  video: true,
  tags: true,
  sequenceMarker: true,
  notes: true,
};

export interface NoteTypeDef {
  /** "predef:<langCode>" for predefined, "custom:<id>" for custom. */
  id: string;
  /** Anki note type name. */
  name: string;
  predefined: boolean;
  /** Study language code this note type maps to (drives subtitle matching). */
  language: string;
  fields: FieldNamesConfig;
  included: FieldToggles;
}

/** Shape sent to the Rust backend as `output_fields`. */
export interface OutputFieldsPayload {
  include_subs1: boolean;
  include_subs2: boolean;
  include_audio: boolean;
  include_snapshot: boolean;
  include_video: boolean;
  include_tag: boolean;
  include_sequence: boolean;
  include_reading: boolean;
  include_notes: boolean;
}

const CUSTOM_NOTE_TYPES_KEY = "vesta-custom-note-types";
const LEGACY_ANKI_FIELD_PRESETS_KEY = "vesta-anki-field-presets";
export const NOTE_TYPES_UPDATED_EVENT = "vesta:note-types-updated";

function sanitizeToggles(raw: Partial<FieldToggles> | undefined): FieldToggles {
  const r = raw || {};
  // Missing keys default to ON, so legacy data (no `included`) becomes all-on.
  const at = (v: unknown) => v !== false;
  return {
    expression: at(r.expression),
    meaning: at(r.meaning),
    reading: at(r.reading),
    audio: at(r.audio),
    snapshot: at(r.snapshot),
    video: at(r.video),
    tags: at(r.tags),
    sequenceMarker: at(r.sequenceMarker),
    notes: at(r.notes),
  };
}

function sanitizeCustomNoteType(raw: any): NoteTypeDef | null {
  if (!raw || !raw.id || !raw.name) return null;
  const id = String(raw.id).startsWith("custom:") ? String(raw.id) : `custom:${raw.id}`;
  return {
    id,
    name: limitNoteTypeFieldValue(String(raw.name)),
    predefined: false,
    language: typeof raw.language === "string" ? raw.language : "",
    fields: sanitizeFieldNamesConfig({ ...defaultFieldNames, ...(raw.fields || {}) }),
    included: sanitizeToggles(raw.included),
  };
}

/** Custom note types saved by the user, loaded directly from the presets store. */
export function loadCustomNoteTypes(): NoteTypeDef[] {
  try {
    const raw = localStorage.getItem("vesta-anki-field-presets");
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        return parsed
          .map((p: any) => {
            if (!p || !p.id) return null;
            const fields = sanitizeFieldNamesConfig({ ...defaultFieldNames, ...(p.fields || {}) });
            const included: FieldToggles = {
              expression: fields.expression.trim() !== "",
              meaning: fields.meaning.trim() !== "",
              reading: fields.reading.trim() !== "",
              audio: fields.audio.trim() !== "",
              snapshot: fields.snapshot.trim() !== "",
              video: fields.video.trim() !== "",
              tags: fields.tags.trim() !== "",
              sequenceMarker: fields.sequenceMarker.trim() !== "",
              notes: fields.notes.trim() !== "",
            };
            return {
              id: p.id.startsWith("custom:") ? p.id : `custom:${p.id}`,
              name: p.name || p.noteTypeName || "Unnamed Template",
              predefined: false,
              language: "",
              fields,
              included,
            };
          })
          .filter((nt): nt is NoteTypeDef => Boolean(nt));
      }
    }
  } catch { /* ignore */ }
  return [];
}

export function saveCustomNoteTypes(list: NoteTypeDef[]): void {
  // Keeping this function signature for backwards compatibility/types,
  // but presets are saved via SettingsTab's persistAnkiFieldPresets.
  dispatchWindowEvent(NOTE_TYPES_UPDATED_EVENT);
}

/** The locked, predefined note type. */
export function predefinedNoteTypeForLanguage(code: string): NoteTypeDef {
  return {
    id: "default",
    name: "Default_Vesta",
    predefined: true,
    language: "",
    fields: { ...defaultFieldNames },
    included: { ...allFieldsIncluded },
  };
}

/** Predefined note types (just returning the default template now). */
export function predefinedNoteTypes(): NoteTypeDef[] {
  return [predefinedNoteTypeForLanguage("")];
}

/** All selectable note types: predefined first, then custom (A→Z). */
export function listNoteTypes(): NoteTypeDef[] {
  const byName = (a: NoteTypeDef, b: NoteTypeDef) => a.name.localeCompare(b.name);
  const custom = loadCustomNoteTypes().slice().sort(byName);
  const defaultNT = predefinedNoteTypeForLanguage("");
  return [defaultNT, ...custom];
}

export function findNoteTypeById(id: string): NoteTypeDef | null {
  if (id === "default" || id.startsWith("predef:")) {
    return predefinedNoteTypeForLanguage("");
  }
  return loadCustomNoteTypes().find((nt) => nt.id === id) ?? null;
}

const ACTIVE_NOTE_TYPE_ID_KEY = "vesta-active-note-type-id";
export const ACTIVE_NOTE_TYPE_CHANGED_EVENT = "vesta:active-note-type-changed";

export function loadActiveNoteTypeId(): string {
  try {
    const saved = localStorage.getItem(ACTIVE_NOTE_TYPE_ID_KEY);
    if (saved) return saved;
  } catch { /* ignore */ }
  return "default";
}

export function saveActiveNoteTypeId(id: string): void {
  try {
    localStorage.setItem(ACTIVE_NOTE_TYPE_ID_KEY, id);
    dispatchWindowEvent(ACTIVE_NOTE_TYPE_CHANGED_EVENT);
  } catch { /* ignore */ }
}

export function newCustomNoteTypeId(): string {
  return `custom:${Date.now().toString(36)}`;
}

/** Map a note type's active fields onto the backend `output_fields` payload. */
export function noteTypeOutputFields(nt: NoteTypeDef): OutputFieldsPayload {
  const i = nt.included;
  return {
    include_subs1: i.expression,
    include_subs2: i.meaning,
    include_audio: i.audio,
    include_snapshot: i.snapshot,
    include_video: i.video,
    include_tag: i.tags,
    include_sequence: i.sequenceMarker,
    include_reading: i.reading,
    include_notes: i.notes,
  };
}

export const defaultCardTemplates: CardTemplateConfig = {
  frontHtml: `<div id="tags-container"></div>
<div id="tags-source" style="display: none;">{{Tags}}</div>
<div id="timestamp-source" style="display: none;">{{SequenceMarker}}</div>
<div class='expression'>{{Expression}}</div>
<hr>`,
  backHtml: `<div id="tags-container"></div>
<div id="tags-source" style="display: none;">{{Tags}}</div>
<div id="timestamp-source" style="display: none;">{{SequenceMarker}}</div>
<span class='media'>{{Audio}}</span>
<div class="expression">{{Expression}}</div>
<hr>
<br>
<div class='reading'>{{Reading}}</div>
<div class='meaning'>{{Meaning}}</div>
<br>
<div class='media'>{{Snapshot}}</div>
<span class='media'>{{Video}}</span>
<br />`,
  css: `.card {
  font-family: arial;
  font-size: 20px;
  text-align: center;
  color: black;
  background-color: white;
}
#tags-container {
  text-align: left;
  margin-bottom: 8px;
  min-height: 20px;
}
.tag-pill {
  display: inline-block;
  font-size: 11px;
  font-family: arial, sans-serif;
  font-weight: 600;
  color: #333;
  background-color: #f0f0f0;
  padding: 4px 8px;
  border-radius: 8px;
  margin-right: 4px;
  margin-bottom: 4px;
  border: 1px solid #ddd;
  box-shadow: 0 1px 1px rgba(0,0,0,0.05);
}
.card video,
.card iframe {
  width: 600px;
  height: 400px;
  max-width: 100%;
  display: block;
  margin: 10px auto;
  border: 1px solid #eee;
}`,
  noteTypeName: "Vesta_Default",
};

export function loadCardTemplates(): CardTemplateConfig {
  try {
    const raw = localStorage.getItem(CARD_TEMPLATE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return sanitizeCardTemplateConfig({
        frontHtml: parsed.frontHtml || defaultCardTemplates.frontHtml,
        backHtml: parsed.backHtml || defaultCardTemplates.backHtml,
        css: parsed.css || defaultCardTemplates.css,
        noteTypeName: parsed.noteTypeName || defaultCardTemplates.noteTypeName,
      });
    }
  } catch { /* ignore */ }
  return sanitizeCardTemplateConfig({ ...defaultCardTemplates });
}

export function saveCardTemplates(config: CardTemplateConfig): void {
  const sanitized = sanitizeCardTemplateConfig(config);
  localStorage.setItem(CARD_TEMPLATE_KEY, JSON.stringify(sanitized));
  dispatchWindowEvent(CARD_TEMPLATES_UPDATED_EVENT);
}

export function resetCardTemplates(): CardTemplateConfig {
  localStorage.removeItem(CARD_TEMPLATE_KEY);
  dispatchWindowEvent(CARD_TEMPLATES_UPDATED_EVENT);
  return sanitizeCardTemplateConfig({ ...defaultCardTemplates });
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

export function getSortedKeys(keyStr: string): string[] {
  if (!keyStr) return [];
  const keys = keyStr.split("+").map(k => k.trim());
  const order = ["Ctrl", "Alt", "Shift"];
  keys.sort((a, b) => {
    const idxA = order.indexOf(a);
    const idxB = order.indexOf(b);
    if (idxA !== -1 && idxB !== -1) {
      return idxA - idxB;
    }
    if (idxA !== -1) return -1;
    if (idxB !== -1) return 1;
    return a.localeCompare(b);
  });
  return keys;
}

/**
 * Localized label for a single shortcut key part.
 *
 * Modifier and special keys (Ctrl, Shift, Enter, Space, arrows, ...) are looked
 * up under the `keys.*` namespace so each language can show its own convention
 * (e.g. German `Strg`/`Umschalt`, French `Maj`/`Entrée`). Letters, digits and
 * any key without a `keys.*` entry fall back to the raw part. The translator is
 * injected so this module stays free of any i18n import.
 */
export function formatKeyPart(part: string, translate: (key: string) => string): string {
  const key = `keys.${part.toLowerCase()}`;
  const label = translate(key);
  return label === key ? part : label;
}


