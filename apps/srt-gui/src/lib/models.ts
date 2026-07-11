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
    description: "Modelli GPT e speech-to-text Whisper",
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
    description: "Qualsiasi endpoint compatibile OpenAI (e.g. /audio/transcriptions)",
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

export const languageAliases: Record<string, string[]> = {
  ar: ["ara", "arb", "arabic", "arabo", "arab", "arabia"],
  ca: ["cat", "catalan", "català", "catala", "catalano"],
  zh: ["chi", "zho", "cmn", "zh-cn", "chs", "chinese", "mandarin", "simplified chinese", "cinese", "cinese semplificato"],
  "zh-tw": ["chi", "zho", "cmn", "zh-hant", "zh-tw", "cht", "traditional chinese", "cinese tradizionale", "taiwanese mandarin"],
  cs: ["cze", "ces", "czech", "čeština", "cestina", "ceco"],
  da: ["dan", "danish", "dansk", "danese"],
  nl: ["dut", "nld", "dutch", "nederlands", "olandese", "neerlandese"],
  en: ["eng", "english", "inglese", "ingles", "anglais", "inglés", "en-us", "en-gb"],
  fi: ["fin", "finnish", "suomi", "finlandese"],
  fr: ["fre", "fra", "french", "français", "francais", "francese"],
  de: ["ger", "deu", "german", "deutsch", "tedesco"],
  el: ["gre", "ell", "greek", "ελληνικά", "ellinika", "greco"],
  he: ["heb", "hebrew", "עברית", "ivrit", "ebraico"],
  hi: ["hin", "hindi", "हिंदी"],
  hu: ["hun", "hungarian", "magyar", "ungherese"],
  is: ["ice", "isl", "icelandic", "íslenska", "islenska", "islandese"],
  id: ["ind", "indonesian", "bahasa indonesia", "indonesiano"],
  it: ["ita", "italian", "italiano", "italiana"],
  ja: ["jpn", "japanese", "日本語", "nihongo", "giapponese"],
  ko: ["kor", "korean", "한국어", "hangul", "coreano"],
  ms: ["may", "msa", "malay", "bahasa melayu", "malese"],
  no: ["nor", "norwegian", "norsk", "norvegese", "nb", "nob", "nn", "nno"],
  pl: ["pol", "polish", "polski", "polacco"],
  pt: ["por", "portuguese", "português", "portugues", "portoghese"],
  "pt-br": ["por", "pt-br", "ptbr", "brazilian portuguese", "português brasil", "portugues brasil", "portoghese brasiliano", "brasiliano"],
  ro: ["rum", "ron", "romanian", "română", "romana", "rumeno"],
  ru: ["rus", "russian", "русский", "russkiy", "russo"],
  es: ["spa", "esp", "spanish", "español", "espanol", "espaniol", "castellano", "spagnolo"],
  sv: ["swe", "sved", "swedish", "svenska", "svedese"],
  th: ["tha", "thai", "ไทย", "tailandese"],
  tr: ["tur", "turkish", "türkçe", "turkce", "turco"],
  uk: ["ukr", "ukrainian", "українська", "ukrayinska", "ucraino"],
  vi: ["vie", "vietnamese", "tiếng việt", "tieng viet", "vietnamita"],
};

export function normalizeLanguageText(value: string | null | undefined): string {
  return (value || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[’']/g, "")
    .replace(/[^\p{L}\p{N}]+/gu, " ")
    .trim();
}

export function getLanguageSearchTerms(code: string): string {
  const lang = languages.find((item) => item.code === code);
  return [
    code,
    code.split("-")[0],
    lang?.nameEn,
    lang?.name,
    ...(languageAliases[code] || []),
    ...(languageAliases[code.split("-")[0]] || []),
  ]
    .filter(Boolean)
    .map((term) => String(term))
    .join(" ");
}

function uniqueLanguageTerms(code: string): string[] {
  return [
    code,
    code.split("-")[0],
    getLanguageSearchTerms(code),
  ]
    .join(" ")
    .split(/\s+/)
    .concat(
      (languageAliases[code] || []),
      (languageAliases[code.split("-")[0]] || []),
      languages.find((item) => item.code === code)?.nameEn || "",
      languages.find((item) => item.code === code)?.name || "",
    )
    .map(normalizeLanguageText)
    .filter(Boolean)
    .filter((term, index, arr) => arr.indexOf(term) === index);
}

function levenshteinDistance(a: string, b: string): number {
  if (a === b) return 0;
  if (!a.length) return b.length;
  if (!b.length) return a.length;

  const row = Array.from({ length: b.length + 1 }, (_, index) => index);
  for (let i = 1; i <= a.length; i += 1) {
    let previous = row[0];
    row[0] = i;
    for (let j = 1; j <= b.length; j += 1) {
      const current = row[j];
      row[j] =
        a[i - 1] === b[j - 1]
          ? previous
          : Math.min(row[j - 1] + 1, previous + 1, row[j] + 1);
      previous = current;
    }
  }
  return row[b.length];
}

function similarity(a: string, b: string): number {
  const maxLength = Math.max(a.length, b.length);
  if (maxLength === 0) return 1;
  return 1 - levenshteinDistance(a, b) / maxLength;
}

export function scoreLanguageMatch(value: string, code: string): number {
  const normalized = normalizeLanguageText(value);
  if (!normalized) return 0;

  const tokens = normalized.split(/\s+/).filter(Boolean);
  const tokenSet = new Set(tokens);
  let score = 0;

  for (const term of uniqueLanguageTerms(code)) {
    if (!term) continue;
    if (normalized === term) score = Math.max(score, 100);

    if (term.length <= 3) {
      if (tokenSet.has(term)) score = Math.max(score, 96);
      continue;
    }

    if (` ${normalized} `.includes(` ${term} `)) score = Math.max(score, 92);

    for (const token of tokens) {
      if (token.length < 4) continue;
      if (token === term) score = Math.max(score, 90);
      if (term.startsWith(token) || token.startsWith(term)) score = Math.max(score, 82);
      if (similarity(token, term) >= 0.86) score = Math.max(score, 76);
    }
  }

  return score;
}

export function detectLanguageCode(value: string): string | null {
  let bestCode: string | null = null;
  let bestScore = 0;

  for (const lang of languages) {
    const score = scoreLanguageMatch(value, lang.code);
    if (score > bestScore) {
      bestScore = score;
      bestCode = lang.code;
    }
  }

  return bestScore >= 76 ? bestCode : null;
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
// normalizzati dal backend (whisper-common/cloud.rs) in segmenti SRT.

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


