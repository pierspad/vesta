import * as vestaConfig from "$lib/config/vestaConfig";

/**
 * Definizione modelli AI disponibili per la traduzione
 *
 * ARCHITETTURA:
 * - Local: per Ollama, LM Studio e altri server locali (API OpenAI-compatible)
 * - Google: API Google Gemini native (richiede chiave API Google AIza...)
 * - OpenRouter: gateway unificato (DISABILITATO per ora)
 *
 * Estratto da models.ts (che restava un grab-bag di feature scollegate:
 * lingue, shortcut, tier di traduzione/trascrizione, note-type, ecc.) —
 * questo file contiene solo il catalogo statico provider/modelli LLM.
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
  /** Pagina dove ottenere la API key (mostrata come link nella UI). */
  apiKeyUrl?: string;
  /** Prefisso atteso della key (hint/validazione leggera, es: "AIza", "gsk_"). */
  keyPrefix?: string;
  /** Placeholder per il campo key. */
  keyPlaceholder?: string;
  documentationUrl?: string;
}

// Provider disponibili - solo Local e Google sono abilitati
export const providers: Record<string, ProviderInfo> = {
  local: {
    id: "local",
    name: "Local LLM (Open API)",
    icon: "local", // Use icon ID for custom SVG icons
    color: "from-purple-500 to-pink-500",
    description: "Ollama, LM Studio, local models",
    requiresApiKey: false,
    requiresApiUrl: true,
    defaultApiUrl: "http://localhost:11434/v1",
    enabled: true,
    documentationUrl: "https://ollama.com",
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
    apiKeyUrl: "https://aistudio.google.com/apikey",
    keyPrefix: "AIza",
    keyPlaceholder: "AIza...",
    documentationUrl: "https://ai.google.dev/gemini-api/docs",
  },
  openai: {
    id: "openai",
    name: "OpenAI GPT / Whisper",
    icon: "openai", // Use icon ID for custom SVG icons
    color: "from-emerald-500 to-teal-500",
    description: "GPT models and Whisper speech-to-text",
    requiresApiKey: true,
    requiresApiUrl: false,
    defaultApiUrl: "https://api.openai.com/v1",
    enabled: true,
    apiKeyUrl: "https://platform.openai.com/api-keys",
    keyPlaceholder: "sk-...",
    documentationUrl: "https://platform.openai.com/docs",
  },
  deepgram: {
    id: "deepgram",
    name: "Deepgram",
    icon: "deepgram",
    color: "from-violet-500 to-fuchsia-500",
    description: "Speech-to-text API (Nova-3, Nova-2)",
    requiresApiKey: true,
    requiresApiUrl: false,
    defaultApiUrl: "https://api.deepgram.com/v1",
    enabled: true,
    apiKeyUrl: "https://console.deepgram.com",
    keyPlaceholder: "Deepgram API key",
    documentationUrl: "https://developers.deepgram.com",
  },
  assemblyai: {
    id: "assemblyai",
    name: "AssemblyAI",
    icon: "assemblyai",
    color: "from-indigo-500 to-blue-500",
    description: "Speech-to-text API (best, nano)",
    requiresApiKey: true,
    requiresApiUrl: false,
    defaultApiUrl: "https://api.assemblyai.com/v2",
    enabled: true,
    apiKeyUrl: "https://www.assemblyai.com/app/api-keys",
    keyPlaceholder: "AssemblyAI API key",
    documentationUrl: "https://www.assemblyai.com/docs",
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
    name: "Custom Provider",
    icon: "custom",
    color: "from-gray-500 to-gray-600",
    description: "Any OpenAI-compatible endpoint (URL + optional API key)",
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
    apiKeyUrl: "https://console.groq.com/keys",
    keyPrefix: "gsk_",
    keyPlaceholder: "gsk_...",
    documentationUrl: "https://console.groq.com/docs",
  },
  openrouter: {
    id: "openrouter",
    name: "OpenRouter",
    icon: "openrouter",
    color: "from-indigo-500 to-purple-600",
    description: "One key for GPT, Claude, Gemini, Llama, Mistral and 300+ models",
    requiresApiKey: true,
    requiresApiUrl: false,
    defaultApiUrl: "https://openrouter.ai/api/v1",
    enabled: true,
    apiKeyUrl: "https://openrouter.ai/keys",
    keyPrefix: "sk-or-",
    keyPlaceholder: "sk-or-...",
    documentationUrl: "https://openrouter.ai/docs",
  },
  mistral: {
    id: "mistral",
    name: "Mistral AI",
    icon: "mistral",
    color: "from-orange-500 to-rose-500",
    description: "La Plateforme: Mistral Large, Medium, Small, Nemo, Codestral",
    requiresApiKey: true,
    requiresApiUrl: false,
    defaultApiUrl: "https://api.mistral.ai/v1",
    enabled: true,
    apiKeyUrl: "https://console.mistral.ai/api-keys",
    keyPlaceholder: "Mistral API key",
    documentationUrl: "https://docs.mistral.ai",
  },
  github: {
    id: "github",
    name: "GitHub Models",
    icon: "github",
    color: "from-gray-600 to-gray-800",
    description: "Free tier via GitHub PAT (models:read): GPT, Llama, Mistral, DeepSeek",
    requiresApiKey: true,
    requiresApiUrl: false,
    defaultApiUrl: "https://models.github.ai/inference",
    enabled: true,
    apiKeyUrl: "https://github.com/settings/personal-access-tokens",
    keyPrefix: "github_pat_",
    keyPlaceholder: "github_pat_... or ghp_...",
    documentationUrl: "https://docs.github.com",
  },
  nvidia: {
    id: "nvidia",
    name: "NVIDIA NIM",
    icon: "nvidia",
    color: "from-green-500 to-lime-500",
    description: "build.nvidia.com: Llama, Nemotron, DeepSeek, Qwen and more",
    requiresApiKey: true,
    requiresApiUrl: false,
    defaultApiUrl: "https://integrate.api.nvidia.com/v1",
    enabled: true,
    apiKeyUrl: "https://build.nvidia.com",
    keyPrefix: "nvapi-",
    keyPlaceholder: "nvapi-...",
    documentationUrl: "https://docs.nvidia.com",
  },
  local_whisper: {
    id: "local_whisper",
    name: "Local Whisper",
    icon: "local",
    color: "from-purple-500 to-pink-500",
    description: "Modello Whisper locale eseguito sul tuo PC",
    requiresApiKey: false,
    requiresApiUrl: false,
    enabled: true,
    documentationUrl: "https://github.com/ggerganov/whisper.cpp",
  },
  custom_whisper: {
    id: "custom_whisper",
    name: "Custom Whisper",
    icon: "custom",
    color: "from-gray-500 to-gray-600",
    description: "Any OpenAI-compatible endpoint (e.g. /audio/transcriptions)",
    requiresApiKey: false,
    requiresApiUrl: true,
    defaultApiUrl: "http://localhost:8000/v1",
    enabled: true,
  },
};

// Provider selezionabili per la modalità a tier / aggiunta key (in ordine).
export const providerOrder = [
  "google",
  "groq",
  "openrouter",
  "mistral",
  "github",
  "nvidia",
  "local",
  "custom",
];

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

  // OpenRouter - un'unica key per centinaia di modelli (id "vendor/model")
  openrouter: [
    {
      id: "google/gemini-2.5-flash",
      name: "Gemini 2.5 Flash",
      provider: "openrouter",
      family: "Google",
      familyInfo: { type: "proprietary", badge: "Stable" },
      description: "Velocità ed efficienza Google via OpenRouter",
      recommended: true,
    },
    {
      id: "openai/gpt-4o-mini",
      name: "GPT-4o mini",
      provider: "openrouter",
      family: "OpenAI",
      familyInfo: { type: "proprietary", badge: "Budget" },
      description: "Economico e veloce, ottimo per traduzioni",
    },
    {
      id: "anthropic/claude-3.5-haiku",
      name: "Claude 3.5 Haiku",
      provider: "openrouter",
      family: "Anthropic",
      familyInfo: { type: "proprietary", badge: "Stable" },
      description: "Haiku rapido di Anthropic",
    },
    {
      id: "meta-llama/llama-3.3-70b-instruct",
      name: "Llama 3.3 70B",
      provider: "openrouter",
      family: "Meta",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Meta 70B instruct",
    },
    {
      id: "deepseek/deepseek-chat",
      name: "DeepSeek V3",
      provider: "openrouter",
      family: "DeepSeek",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "DeepSeek flagship MoE",
    },
    {
      id: "mistralai/mistral-small-3.1-24b-instruct",
      name: "Mistral Small 3.1",
      provider: "openrouter",
      family: "Mistral",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Mistral Small 24B via OpenRouter",
    },
  ],

  // Mistral AI - La Plateforme (API nativa OpenAI-compatible)
  mistral: [
    {
      id: "mistral-large-latest",
      name: "Mistral Large",
      provider: "mistral",
      family: "Mistral",
      familyInfo: { type: "proprietary", badge: "Newest" },
      contextWindow: 131072,
      description: "Modello flagship Mistral, massima qualità",
      recommended: true,
    },
    {
      id: "mistral-medium-latest",
      name: "Mistral Medium",
      provider: "mistral",
      family: "Mistral",
      familyInfo: { type: "proprietary", badge: "Stable" },
      contextWindow: 131072,
      description: "Bilanciamento qualità/costo",
    },
    {
      id: "mistral-small-latest",
      name: "Mistral Small",
      provider: "mistral",
      family: "Mistral",
      familyInfo: { type: "proprietary", badge: "Budget" },
      contextWindow: 131072,
      description: "Veloce ed economico, ottimo per traduzioni",
      recommended: true,
    },
    {
      id: "open-mistral-nemo",
      name: "Mistral Nemo",
      provider: "mistral",
      family: "Mistral",
      familyInfo: { type: "open-weights", badge: "Open" },
      contextWindow: 131072,
      description: "12B multilingue, gratuito su molti tier",
    },
  ],

  // GitHub Models - free tier via PAT (models:read). ID con prefisso vendor.
  github: [
    {
      id: "openai/gpt-4o-mini",
      name: "GPT-4o mini",
      provider: "github",
      family: "OpenAI",
      familyInfo: { type: "proprietary", badge: "Budget" },
      description: "OpenAI compatto, free tier GitHub",
      recommended: true,
    },
    {
      id: "openai/gpt-4o",
      name: "GPT-4o",
      provider: "github",
      family: "OpenAI",
      familyInfo: { type: "proprietary", badge: "Stable" },
      description: "OpenAI flagship multimodale",
    },
    {
      id: "meta/Llama-3.3-70B-Instruct",
      name: "Llama 3.3 70B",
      provider: "github",
      family: "Meta",
      familyInfo: { type: "open-weights", badge: "Open" },
      description: "Meta 70B instruct",
    },
    {
      id: "mistral-ai/Mistral-Large-2411",
      name: "Mistral Large 2411",
      provider: "github",
      family: "Mistral",
      familyInfo: { type: "proprietary", badge: "Stable" },
      description: "Mistral Large via GitHub Models",
    },
    {
      id: "deepseek/DeepSeek-V3-0324",
      name: "DeepSeek V3",
      provider: "github",
      family: "DeepSeek",
      familyInfo: { type: "open-source", badge: "OSS" },
      description: "DeepSeek V3 via GitHub Models",
    },
  ],

  // NVIDIA NIM - integrate.api.nvidia.com (OpenAI-compatible)
  nvidia: [
    {
      id: "meta/llama-3.3-70b-instruct",
      name: "Llama 3.3 70B",
      provider: "nvidia",
      family: "Meta",
      familyInfo: { type: "open-weights", badge: "Open" },
      contextWindow: 131072,
      description: "Meta 70B su NVIDIA NIM",
      recommended: true,
    },
    {
      id: "nvidia/llama-3.1-nemotron-70b-instruct",
      name: "Llama 3.1 Nemotron 70B",
      provider: "nvidia",
      family: "NVIDIA",
      familyInfo: { type: "open-weights", badge: "Open" },
      contextWindow: 131072,
      description: "Nemotron tuned da NVIDIA",
    },
    {
      id: "deepseek-ai/deepseek-r1",
      name: "DeepSeek R1",
      provider: "nvidia",
      family: "DeepSeek",
      familyInfo: { type: "open-source", badge: "OSS" },
      contextWindow: 131072,
      description: "Ragionamento avanzato R1",
    },
    {
      id: "qwen/qwen2.5-72b-instruct",
      name: "Qwen 2.5 72B",
      provider: "nvidia",
      family: "Qwen",
      familyInfo: { type: "open-source", badge: "OSS" },
      contextWindow: 131072,
      description: "Qwen 72B multilingue",
    },
  ],
};

// Modelli personalizzati (salvati in localStorage)
export interface CustomModel {
  id: string;
  name: string;
  provider: string;
  apiModelId: string; // ID usato nelle chiamate API
  description?: string;
}

// ApiProviderId/ApiKeyConfig sono in models.ts: sono condivisi con le
// funzionalità di trascrizione (deepgram/assemblyai/local_whisper/...),
// non specifici del solo catalogo LLM.

// Funzione per ottenere tutti i modelli di un provider (inclusi custom)
export function getModelsForProvider(providerId: string): ModelInfo[] {
  const defaultModels = modelsByProvider[providerId] || [];

  // Carica modelli personalizzati
  const customModelsJson = vestaConfig.getItem("srt-tools-custom-models");
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
