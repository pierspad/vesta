/**
 * Definizione modelli AI disponibili per la traduzione
 * 
 * ARCHITETTURA:
 * - Local: per Ollama, LM Studio e altri server locali (API OpenAI-compatible)
 * - Google: API Google Gemini native (richiede chiave API Google AIza...)
 * - OpenRouter: gateway unificato (DISABILITATO per ora)
 */

export interface FamilyInfo {
  type: "proprietary" | "open-weights" | "open-source";
  badge: string; // es: "Newest", "Open", "Stable", "Budget"
}

export interface ModelInfo {
  id: string;
  name: string;
  provider: string;
  family: string;
  familyInfo?: FamilyInfo;
  contextWindow?: number;
  description?: string;
  recommended?: boolean;
}

export interface ProviderInfo {
  id: string;
  name: string;
  icon: string;
  color: string;
  description: string;
  requiresApiKey: boolean;
  requiresApiUrl: boolean;
  defaultApiUrl?: string;
  enabled: boolean; // Nuovo campo per abilitare/disabilitare provider
}

// Provider disponibili - solo Local e Google sono abilitati
export const providers: Record<string, ProviderInfo> = {
  local: {
    id: "local",
    name: "Local LLM",
    icon: "local", // Use icon ID for custom SVG icons
    color: "from-purple-500 to-pink-500",
    description: "Ollama, LM Studio, local models",
    requiresApiKey: false,
    requiresApiUrl: true,
    defaultApiUrl: "http://localhost:11434/v1",
    enabled: true,
  },
  google: {
    id: "google",
    name: "Google Gemini",
    icon: "google", // Use icon ID for custom SVG icons
    color: "from-blue-500 to-cyan-500",
    description: "Gemini 3.1 Pro, 3 Flash (requires AIza... key)",
    requiresApiKey: true,
    requiresApiUrl: false,
    defaultApiUrl: "https://generativelanguage.googleapis.com/v1beta",
    enabled: true,
  },
  openai: {
    id: "openai",
    name: "OpenAI GPT",
    icon: "openai", // Use icon ID for custom SVG icons
    color: "from-emerald-500 to-teal-500",
    description: "GPT-5.2, GPT-5, GPT-4o (Coming Soon)",
    requiresApiKey: true,
    requiresApiUrl: false,
    defaultApiUrl: "https://api.openai.com/v1",
    enabled: false, // Coming soon
  },
  anthropic: {
    id: "anthropic",
    name: "Anthropic Claude",
    icon: "anthropic", // Use icon ID for custom SVG icons
    color: "from-orange-500 to-amber-500",
    description: "Opus 4.6, Sonnet 4.6 (Coming Soon)",
    requiresApiKey: true,
    requiresApiUrl: false,
    defaultApiUrl: "https://api.anthropic.com/v1",
    enabled: false, // Coming soon
  },
  custom: {
    id: "custom",
    name: "Provider Personalizzato",
    icon: "custom",
    color: "from-gray-500 to-gray-600",
    description: "Qualsiasi endpoint compatibile OpenAI (URL + API key opzionale)",
    requiresApiKey: false,
    requiresApiUrl: true,
    defaultApiUrl: "",
    enabled: true,
  },
  groq: {
    id: "groq",
    name: "Groq API",
    icon: "groq",
    color: "from-orange-400 to-red-500",
    description: "Ultra-fast inference on LPU (Llama, GPT-OSS, Qwen)",
    requiresApiKey: true,
    requiresApiUrl: false,
    defaultApiUrl: "https://api.groq.com/openai/v1",
    enabled: true,
  },
  openrouter: {
    id: "openrouter",
    name: "OpenRouter",
    icon: "openrouter",
    color: "from-indigo-500 to-purple-600",
    description: "Unified access to GPT, Claude, Gemini, Mistral and more (DISABLED)",
    requiresApiKey: true,
    requiresApiUrl: false,
    defaultApiUrl: "https://openrouter.ai/api/v1",
    enabled: false, // Disabilitato
  },
};

// Solo provider abilitati
export const providerOrder = ["local", "custom", "google", "groq"];

// Modelli per provider
export const modelsByProvider: Record<string, ModelInfo[]> = {
  local: [
    // --- Meta Llama ---
    {
      id: "llama3.3",
      name: "Llama 3.3 70B",
      provider: "local",
      family: "Meta",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Ultimo modello Meta 70B, eccellente per traduzioni",
      recommended: true,
    },
    {
      id: "llama3.2",
      name: "Llama 3.2 3B",
      provider: "local",
      family: "Meta",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Compatto e veloce, 3B parametri",
    },
    {
      id: "llama3.2:1b",
      name: "Llama 3.2 1B",
      provider: "local",
      family: "Meta",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Ultra-leggero 1B, per risorse limitate",
    },
    {
      id: "llama3.1",
      name: "Llama 3.1 8B",
      provider: "local",
      family: "Meta",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "8B parametri, buon bilanciamento qualità/velocità",
    },
    {
      id: "llama3.1:70b",
      name: "Llama 3.1 70B",
      provider: "local",
      family: "Meta",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "70B parametri, qualità elevata",
    },
    // --- Mistral ---
    {
      id: "mistral-small-3.1",
      name: "Mistral Small 3.1",
      provider: "local",
      family: "Mistral",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Mistral Small 3.1, 24B parametri",
      recommended: true,
    },
    {
      id: "mistral-medium-3",
      name: "Mistral Medium 3",
      provider: "local",
      family: "Mistral",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Mistral Medium 3 locale",
    },
    {
      id: "mistral-nemo",
      name: "Mistral Nemo 12B",
      provider: "local",
      family: "Mistral",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "12B, ottimo per multilingue e traduzioni",
    },
    {
      id: "mixtral-8x7b",
      name: "Mixtral 8x7B",
      provider: "local",
      family: "Mistral",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Mixtral MoE, 46.7B totali (12.9B attivi)",
    },
    {
      id: "codestral",
      name: "Codestral 22B",
      provider: "local",
      family: "Mistral",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Specializzato in codice ma buono anche per testo",
    },
    // --- DeepSeek ---
    {
      id: "deepseek-r1",
      name: "DeepSeek R1",
      provider: "local",
      family: "DeepSeek",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "Ragionamento avanzato, eccellente per traduzioni complesse",
      recommended: true,
    },
    {
      id: "deepseek-r1:8b",
      name: "DeepSeek R1 8B",
      provider: "local",
      family: "DeepSeek",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "Versione compatta 8B del modello R1",
    },
    {
      id: "deepseek-r1:14b",
      name: "DeepSeek R1 14B",
      provider: "local",
      family: "DeepSeek",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "14B parametri, buon bilanciamento",
    },
    {
      id: "deepseek-r1:32b",
      name: "DeepSeek R1 32B",
      provider: "local",
      family: "DeepSeek",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "32B parametri, qualità superiore",
    },
    {
      id: "deepseek-r1:70b",
      name: "DeepSeek R1 70B",
      provider: "local",
      family: "DeepSeek",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "70B parametri, massima qualità",
    },
    {
      id: "deepseek-v3",
      name: "DeepSeek V3",
      provider: "local",
      family: "DeepSeek",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "MoE 671B totali, modello flagship",
    },
    // --- Qwen ---
    {
      id: "qwen2.5",
      name: "Qwen 2.5 7B",
      provider: "local",
      family: "Qwen",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "7B, ottimo per multilingue",
    },
    {
      id: "qwen2.5:14b",
      name: "Qwen 2.5 14B",
      provider: "local",
      family: "Qwen",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "14B, bilanciamento qualità/velocità",
    },
    {
      id: "qwen2.5:32b",
      name: "Qwen 2.5 32B",
      provider: "local",
      family: "Qwen",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "32B, qualità elevata per multilingue",
    },
    {
      id: "qwen2.5:72b",
      name: "Qwen 2.5 72B",
      provider: "local",
      family: "Qwen",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "72B, massima qualità Qwen",
    },
    {
      id: "qwq",
      name: "QwQ 32B",
      provider: "local",
      family: "Qwen",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "Modello di ragionamento Qwen 32B",
    },
    // --- Google Gemma ---
    {
      id: "gemma3:27b",
      name: "Gemma 3 27B",
      provider: "local",
      family: "Google",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Top di gamma open, eccellente ragionamento",
    },
    {
      id: "gemma3:12b",
      name: "Gemma 3 12B",
      provider: "local",
      family: "Google",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Bilanciato, ottime performance",
    },
    {
      id: "gemma3:4b",
      name: "Gemma 3 4B",
      provider: "local",
      family: "Google",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Compatto, veloce per task semplici",
    },
    {
      id: "gemma3:1b",
      name: "Gemma 3 1B",
      provider: "local",
      family: "Google",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Nano modello, massima velocità",
    },
    // --- Microsoft ---
    {
      id: "phi4",
      name: "Phi-4 14B",
      provider: "local",
      family: "Microsoft",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Ultimo modello Microsoft, 14B parametri",
    },
    {
      id: "phi3.5",
      name: "Phi-3.5 Mini",
      provider: "local",
      family: "Microsoft",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "3.8B, compatto e performante",
    },
    {
      id: "phi3",
      name: "Phi-3 3.8B",
      provider: "local",
      family: "Microsoft",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Microsoft compatto locale",
    },
    // --- Cohere ---
    {
      id: "command-r",
      name: "Command R 35B",
      provider: "local",
      family: "Cohere",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Ottimizzato per RAG e conversazione",
    },
    {
      id: "command-r-plus",
      name: "Command R+ 104B",
      provider: "local",
      family: "Cohere",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "104B, massima qualità Cohere",
    },
    // --- 01.AI ---
    {
      id: "yi:34b",
      name: "Yi 1.5 34B",
      provider: "local",
      family: "01.AI",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Eccellente per cinese, inglese e multilingue",
    },
    {
      id: "yi:9b",
      name: "Yi 1.5 9B",
      provider: "local",
      family: "01.AI",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Compatto, buono per multilingue",
    },
    // --- Community Models ---
    {
      id: "nous-hermes2",
      name: "Nous Hermes 2",
      provider: "local",
      family: "Community",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Fine-tuned Llama, ottimo per istruzioni",
    },
    {
      id: "dolphin-mixtral",
      name: "Dolphin Mixtral",
      provider: "local",
      family: "Community",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Mixtral uncensored, versatile",
    },
    {
      id: "solar",
      name: "Solar 10.7B",
      provider: "local",
      family: "Upstage",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Upstage Solar, ottimo per task generali",
    },
    {
      id: "internlm2",
      name: "InternLM2 20B",
      provider: "local",
      family: "Shanghai AI Lab",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "Forte in multilingue e ragionamento",
    },
  ],

  // Google Gemini API nativa - usa direttamente le API di Google
  google: [
    // --- Serie Gemini 3 (Ultima Generazione 2026) ---
    {
      id: "gemini-3-flash",
      name: "Gemini 3 Flash",
      provider: "google",
      family: "Gemini 3",
      familyInfo: { type: "proprietary", badge: "Newest" },
      contextWindow: 1048576,
      description: "Ultimo modello Flash: massima velocità, ragionamento avanzato e capacità agentiche.",
      recommended: true,
    },

    // --- Serie Gemma 3 (Modelli Open Weights) ---
    // Nota: Gemma 3 supporta nativamente 128k di contesto
    {
      id: "gemma-3-27b-it",
      name: "Gemma 3 27B",
      provider: "google",
      family: "Gemma 3",
      familyInfo: { type: "open-weights", badge: "Open" },
      contextWindow: 131072,
      description: "Modello open top di gamma. Eccellente ragionamento e capacità di coding.",
    },
    {
      id: "gemma-3-12b-it",
      name: "Gemma 3 12B",
      provider: "google",
      family: "Gemma 3",
      familyInfo: { type: "open-weights", badge: "Open" },
      contextWindow: 131072,
      description: "Bilanciato: ottime performance mantenendo una dimensione gestibile.",
    },
    {
      id: "gemma-3-4b-it",
      name: "Gemma 3 4B",
      provider: "google",
      family: "Gemma 3",
      familyInfo: { type: "open-weights", badge: "Open" },
      contextWindow: 131072,
      description: "Compatto e veloce, ideale per task semplici e veloci.",
    },
    {
      id: "gemma-3-2b-it",
      name: "Gemma 3 2B",
      provider: "google",
      family: "Gemma 3",
      familyInfo: { type: "open-weights", badge: "Open" },
      contextWindow: 131072,
      description: "Estremamente leggero, per dispositivi con risorse limitate.",
    },
    {
      id: "gemma-3-1b-it",
      name: "Gemma 3 1B",
      provider: "google",
      family: "Gemma 3",
      familyInfo: { type: "open-weights", badge: "Open" },
      contextWindow: 32768,
      description: "Nano modello per la massima velocità di inferenza.",
    },

    // --- Serie Gemini 2.5 (Stabile & Efficiente) ---
    {
      id: "gemini-2.5-flash",
      name: "Gemini 2.5 Flash",
      provider: "google",
      family: "Gemini 2.5",
      familyInfo: { type: "proprietary", badge: "Stable" },
      contextWindow: 1048576,
      description: "Il punto di riferimento per velocità ed efficienza. Ottimo per task quotidiani.",
    },
    {
      id: "gemini-2.5-flash-lite",
      name: "Gemini 2.5 Flash Lite",
      provider: "google",
      family: "Gemini 2.5",
      familyInfo: { type: "proprietary", badge: "Budget" },
      contextWindow: 1048576,
      description: "Versione leggera ed economica, ottimizzata per risposte rapide a basso costo.",
    },
  ],

  // Groq API - Inferenza ultra-veloce su hardware LPU
  groq: [
    {
      id: "llama-3.3-70b-versatile",
      name: "Llama 3.3 70B Versatile",
      provider: "groq",
      family: "Meta",
      familyInfo: { type: "open-weights", badge: "Open" },
      contextWindow: 131072,
      description: "Meta 70B, modello flagship velocissimo su Groq",
      recommended: true,
    },
    {
      id: "llama-3.1-8b-instant",
      name: "Llama 3.1 8B Instant",
      provider: "groq",
      family: "Meta",
      familyInfo: { type: "open-weights", badge: "Open" },
      contextWindow: 131072,
      description: "Ultra-veloce 8B, latenza minima",
    },
    {
      id: "openai/gpt-oss-120b",
      name: "GPT-OSS 120B",
      provider: "groq",
      family: "OpenAI",
      familyInfo: { type: "open-weights", badge: "Open" },
      contextWindow: 131072,
      description: "OpenAI flagship open-weight MoE 120B",
      recommended: true,
    },
    {
      id: "openai/gpt-oss-20b",
      name: "GPT-OSS 20B",
      provider: "groq",
      family: "OpenAI",
      familyInfo: { type: "open-weights", badge: "Open" },
      contextWindow: 131072,
      description: "OpenAI compact MoE 20B, ottimizzato per costi",
    },
    {
      id: "qwen/qwen3-32b",
      name: "Qwen 3 32B",
      provider: "groq",
      family: "Qwen",
      familyInfo: { type: "open-source", badge: "OSS" },
      contextWindow: 131072,
      description: "Qwen 3 multilingue avanzato, ragionamento dual-mode",
    },
  ],

  // OpenRouter è disabilitato ma manteniamo la struttura per future implementazioni
  openrouter: [],
};

// Modelli personalizzati (salvati in localStorage)
export interface CustomModel {
  id: string;
  name: string;
  provider: string;
  apiModelId: string; // ID usato nelle chiamate API
  description?: string;
}

export interface ApiKeyConfig {
  id: string;
  name: string;
  apiType: "local" | "google" | "openrouter" | "openai" | "anthropic" | "custom" | "groq";
  apiKey: string;
  apiUrl?: string;
  modelName?: string;  // Nome modello preferito
  isDefault: boolean;
}

// Funzione per ottenere tutti i modelli di un provider (inclusi custom)
export function getModelsForProvider(providerId: string): ModelInfo[] {
  const defaultModels = modelsByProvider[providerId] || [];

  // Carica modelli personalizzati
  const customModelsJson = localStorage.getItem("srt-tools-custom-models");
  if (customModelsJson) {
    try {
      const customModels: CustomModel[] = JSON.parse(customModelsJson);
      const providerCustomModels = customModels
        .filter((m) => m.provider === providerId)
        .map((m) => ({
          id: m.apiModelId,
          name: m.name,
          provider: m.provider,
          family: "Custom", // Default family for custom models
          description: m.description || "Custom model",
        }));
      return [...defaultModels, ...providerCustomModels];
    } catch {
      return defaultModels;
    }
  }

  return defaultModels;
}

// Funzione per salvare un modello personalizzato
export function saveCustomModel(model: CustomModel): void {
  const customModelsJson = localStorage.getItem("srt-tools-custom-models");
  let customModels: CustomModel[] = [];

  if (customModelsJson) {
    try {
      customModels = JSON.parse(customModelsJson);
    } catch {
      customModels = [];
    }
  }

  // Controlla se esiste già
  const existingIndex = customModels.findIndex((m) => m.id === model.id);
  if (existingIndex >= 0) {
    customModels[existingIndex] = model;
  } else {
    customModels.push(model);
  }

  localStorage.setItem("srt-tools-custom-models", JSON.stringify(customModels));
}

// Funzione per eliminare un modello personalizzato
export function deleteCustomModel(modelId: string): void {
  const customModelsJson = localStorage.getItem("srt-tools-custom-models");
  if (!customModelsJson) return;

  try {
    let customModels: CustomModel[] = JSON.parse(customModelsJson);
    customModels = customModels.filter((m) => m.id !== modelId);
    localStorage.setItem("srt-tools-custom-models", JSON.stringify(customModels));
  } catch {
    // Ignora errori
  }
}

// Funzione per ottenere tutti i modelli personalizzati
export function getCustomModels(): CustomModel[] {
  const customModelsJson = localStorage.getItem("srt-tools-custom-models");
  if (!customModelsJson) return [];

  try {
    return JSON.parse(customModelsJson);
  } catch {
    return [];
  }
}

// Lista lingue disponibili per la traduzione
// nameEn è usato per la ricerca (es: digita "Italian" per trovare "Italiano")
// Ordinate alfabeticamente per nameEn
export const languages = [
  { code: "ar", name: "العربية", nameEn: "Arabic", flag: "🇸🇦" },
  { code: "ca", name: "Català", nameEn: "Catalan", flag: "🇪🇸" },
  { code: "zh", name: "中文 (简体)", nameEn: "Chinese Simplified", flag: "🇨🇳" },
  { code: "zh-tw", name: "中文 (繁體)", nameEn: "Chinese Traditional", flag: "🇹🇼" },
  { code: "cs", name: "Čeština", nameEn: "Czech", flag: "🇨🇿" },
  { code: "da", name: "Dansk", nameEn: "Danish", flag: "🇩🇰" },
  { code: "nl", name: "Nederlands", nameEn: "Dutch", flag: "🇳🇱" },
  { code: "en", name: "English", nameEn: "English", flag: "🇬🇧" },
  { code: "fi", name: "Suomi", nameEn: "Finnish", flag: "🇫🇮" },
  { code: "fr", name: "Français", nameEn: "French", flag: "🇫🇷" },
  { code: "de", name: "Deutsch", nameEn: "German", flag: "🇩🇪" },
  { code: "el", name: "Ελληνικά", nameEn: "Greek", flag: "🇬🇷" },
  { code: "he", name: "עברית", nameEn: "Hebrew", flag: "🇮🇱" },
  { code: "hi", name: "हिंदी", nameEn: "Hindi", flag: "🇮🇳" },
  { code: "hu", name: "Magyar", nameEn: "Hungarian", flag: "🇭🇺" },
  { code: "is", name: "Íslenska", nameEn: "Icelandic", flag: "🇮🇸" },
  { code: "id", name: "Bahasa Indonesia", nameEn: "Indonesian", flag: "🇮🇩" },
  { code: "it", name: "Italiano", nameEn: "Italian", flag: "🇮🇹" },
  { code: "ja", name: "日本語", nameEn: "Japanese", flag: "🇯🇵" },
  { code: "ko", name: "한국어", nameEn: "Korean", flag: "🇰🇷" },
  { code: "ms", name: "Bahasa Melayu", nameEn: "Malay", flag: "🇲🇾" },
  { code: "no", name: "Norsk", nameEn: "Norwegian", flag: "🇳🇴" },
  { code: "pl", name: "Polski", nameEn: "Polish", flag: "🇵🇱" },
  { code: "pt", name: "Português", nameEn: "Portuguese", flag: "🇵🇹" },
  { code: "pt-br", name: "Português (Brasil)", nameEn: "Portuguese Brazil", flag: "🇧🇷" },
  { code: "ro", name: "Română", nameEn: "Romanian", flag: "🇷🇴" },
  { code: "ru", name: "Русский", nameEn: "Russian", flag: "🇷🇺" },
  { code: "es", name: "Español", nameEn: "Spanish", flag: "🇪🇸" },
  { code: "sv", name: "Svenska", nameEn: "Swedish", flag: "🇸🇪" },
  { code: "th", name: "ไทย", nameEn: "Thai", flag: "🇹🇭" },
  { code: "tr", name: "Türkçe", nameEn: "Turkish", flag: "🇹🇷" },
  { code: "uk", name: "Українська", nameEn: "Ukrainian", flag: "🇺🇦" },
  { code: "vi", name: "Tiếng Việt", nameEn: "Vietnamese", flag: "🇻🇳" },
];

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
  { id: "tab-translate", action: "switchToTranslate", description: "shortcuts.action.goToTranslation", defaultKey: "Alt+2", category: "global" },
  { id: "tab-sync", action: "switchToSync", description: "shortcuts.action.goToSync", defaultKey: "Alt+3", category: "global" },
  { id: "tab-settings", action: "switchToSettings", description: "shortcuts.action.goToSettings", defaultKey: "Alt+4", category: "global" },
  { id: "tab-shortcuts", action: "switchToShortcuts", description: "shortcuts.action.goToShortcuts", defaultKey: "Alt+5", category: "global" },
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
}

// Funzione per resettare le shortcut
export function resetShortcuts(): void {
  localStorage.removeItem("srt-tools-shortcut-overrides");
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

    // Converti chiavi con tipi legacy a "google" e filtra quelle non valide
    const converted = parsed.map((k: any) => {
      // Se già ha un tipo valido, mantienilo
      if (k.apiType === "local" || k.apiType === "google" || k.apiType === "groq" || k.apiType === "custom") {
        return k;
      }
      // Converti tipi legacy (openrouter, gemini, openai, anthropic, etc.) a google
      // Le chiavi Google iniziano con "AIza"
      if (k.apiKey && k.apiKey.startsWith("AIza")) {
        return {
          ...k,
          apiType: "google" as const,
          apiUrl: "https://generativelanguage.googleapis.com/v1beta"
        };
      }
      // Altre chiavi legacy (openrouter) - convertile a google se possibile o salta
      if (k.apiType === "openrouter") {
        // Le chiavi OpenRouter non sono compatibili con Google, le ignoriamo
        return null;
      }
      // Converti altri tipi legacy a google
      if (k.apiType && k.apiType !== "local") {
        return {
          ...k,
          apiType: "google" as const,
          apiUrl: "https://generativelanguage.googleapis.com/v1beta"
        };
      }
      return null;
    }).filter((k: any) => k !== null);

    return converted as ApiKeyConfig[];
  } catch {
    return [];
  }
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
    expression: limitNoteTypeFieldValue(config.expression),
    meaning: limitNoteTypeFieldValue(config.meaning),
    reading: limitNoteTypeFieldValue(config.reading),
    audio: limitNoteTypeFieldValue(config.audio),
    snapshot: limitNoteTypeFieldValue(config.snapshot),
    video: limitNoteTypeFieldValue(config.video),
    tags: limitNoteTypeFieldValue(config.tags),
    sequenceMarker: limitNoteTypeFieldValue(config.sequenceMarker),
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
        expression: parsed.expression || defaultFieldNames.expression,
        meaning: parsed.meaning || defaultFieldNames.meaning,
        reading: parsed.reading || defaultFieldNames.reading,
        audio: parsed.audio || defaultFieldNames.audio,
        snapshot: parsed.snapshot || defaultFieldNames.snapshot,
        video: parsed.video || defaultFieldNames.video,
        tags: parsed.tags || defaultFieldNames.tags,
        sequenceMarker: parsed.sequenceMarker || defaultFieldNames.sequenceMarker,
        notes: parsed.notes || defaultFieldNames.notes,
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
