<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { fetch as tauriFetch } from "@tauri-apps/plugin-http";
  import CodeEditor from "./CodeEditor.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import ShortcutsTab from "./ShortcutsTab.svelte";
  import TranslationTiers from "./TranslationTiers.svelte";
import TranscribeTiers from "./TranscribeTiers.svelte";
  import ProviderIcon from "./ProviderIcon.svelte";
  import { smartMatchingStore } from "./smartMatchingStore.svelte";
  import { snackbar } from "./snackbarStore.svelte";
  import { uiMode } from "./uiModeStore.svelte";
  import {
    availableUILanguages,
    currentLanguage,
    locale,
    setLanguage,
  } from "./i18n";
  import SearchableSelect from "./SearchableSelect.svelte";
  import {
    buildSettingsActionHash,
    publishSettingsActionState,
  } from "./settingsNotifications";
  import { aiStore } from "./aiStore.svelte";
  import {
    DEFAULT_REFINEMENT_PROMPT,
    REFINEMENT_PROMPT_STORAGE_KEY,
    loadRefinementPrompt,
  } from "./refinementPrompt";
  import {
    fetchModelsFromEndpoint,
    type DiscoveredModel,
  } from "./modelDiscovery";
  import {
    CARD_TEMPLATES_UPDATED_EVENT,
    FIELD_NAMES_UPDATED_EVENT,
    NOTE_TYPES_UPDATED_EVENT,
    defaultCardTemplates,
    defaultFieldNames,
    getModelsForProvider,
    getLanguageSearchTerms,
    languages,
    limitNoteTypeFieldValue,
    loadAndValidateApiKeys,
    loadCardTemplates,
    loadFieldNames,
    providers,
    resetCardTemplates,
    saveCardTemplates,
    saveFieldNames,
    loadActiveNoteTypeId,
    saveActiveNoteTypeId,
    ACTIVE_NOTE_TYPE_CHANGED_EVENT,
    loadTiers,
    tiersHaveUsableEntries,
    TIERS_UPDATED_EVENT,
    loadTranscribeTiers,
    transcribeTiersHaveUsableEntries,
    TRANSCRIBE_TIERS_UPDATED_EVENT,
    loadTranscribeCloud,
    saveTranscribeCloud,
    transcribeProviders,
    transcribeProviderOrder,
    type ApiKeyConfig,
    type FieldNamesConfig,
    type ModelInfo,
    type Tier,
    type TranscribeTier
  } from "./models";

  const allProviderIds = ["local", "google", "groq", "openai", "deepgram", "assemblyai", "openrouter", "mistral", "github", "nvidia", "custom"];
  const apiKeyProviderIds = ["google", "groq", "openai", "deepgram", "assemblyai", "openrouter", "mistral", "github", "nvidia", "custom"];
  type EndpointStatus = "idle" | "checking" | "online" | "offline";
  type SettingsSection = "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts";
  type TemplateCodeTab = "front" | "back" | "css";
  let {
    requestedSection = $bindable("overview"),
    highlightItemId = $bindable(null),
    active = true,
  }: {
    requestedSection?: SettingsSection;
    highlightItemId?: string | null;
    active?: boolean;
  } = $props();

  const DEFAULT_LLM_PROVIDER_KEY = "vesta-default-llm-provider";
  const DEFAULT_LLM_MODEL_KEY = "vesta-default-llm-model";
  const DEFAULT_LLM_CUSTOM_PROVIDER_KEY = "vesta-default-llm-custom-provider";
  const LOCAL_SERVER_URL_KEY = "vesta-local-server-url";
  const DEFAULT_LOCAL_URL = "http://localhost:11434/v1";
  const DEFAULT_TARGET_LANGUAGE_KEY = "vesta-default-target-language";
  const DEFAULT_TRANSCRIBE_LANGUAGE_KEY = "vesta-default-transcribe-language";
  const DEFAULT_FLASHCARDS_LANGUAGE_KEY = "vesta-default-flashcards-language";
  const DEFAULT_NATIVE_LANGUAGE_KEY = "vesta-default-native-language";
  const NOTE_TYPE_LANGUAGE_KEY = "vesta-flashcards-note-type-language";
  const ANKI_FIELD_PRESETS_KEY = "vesta-anki-field-presets";
  const ACTIVE_ANKI_FIELD_PRESET_KEY = "vesta-active-anki-field-preset";

  type AnkiFieldKey = keyof FieldNamesConfig;
  type AnkiFieldPreset = {
    id: string;
    name: string;
    noteTypeName: string;
    fields: FieldNamesConfig;
  };

  const ankiFieldDefinitions: {
    key: AnkiFieldKey;
    variable: string;
    colorClass: string;
    iconClass: string;
    iconPath: string;
  }[] = [
    {
      key: "expression",
      variable: "{{Expression}}",
      colorClass: "border-sky-400/30 bg-sky-400/10 text-sky-200 hover:bg-sky-400/15",
      iconClass: "text-sky-300",
      iconPath: "M4 6h16M4 12h10M4 18h7",
    },
    {
      key: "meaning",
      variable: "{{Meaning}}",
      colorClass: "border-emerald-400/30 bg-emerald-400/10 text-emerald-200 hover:bg-emerald-400/15",
      iconClass: "text-emerald-300",
      iconPath: "M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10",
    },
    {
      key: "reading",
      variable: "{{Reading}}",
      colorClass: "border-violet-400/30 bg-violet-400/10 text-violet-200 hover:bg-violet-400/15",
      iconClass: "text-violet-300",
      iconPath: "M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5s3.332.477 4.5 1.253v13C19.832 18.477 18.246 18 16.5 18s-3.332.477-4.5 1.253",
    },
    {
      key: "audio",
      variable: "{{Audio}}",
      colorClass: "border-rose-400/30 bg-rose-400/10 text-rose-200 hover:bg-rose-400/15",
      iconClass: "text-rose-300",
      iconPath: "M11 5L6 9H3v6h3l5 4V5zm4.5 4.5a4 4 0 010 5m2.5-7.5a8 8 0 010 10",
    },
    {
      key: "snapshot",
      variable: "{{Snapshot}}",
      colorClass: "border-amber-400/30 bg-amber-400/10 text-amber-200 hover:bg-amber-400/15",
      iconClass: "text-amber-300",
      iconPath: "M3 7h4l2-3h6l2 3h4v13H3V7zm9 10a4 4 0 100-8 4 4 0 000 8z",
    },
    {
      key: "video",
      variable: "{{Video}}",
      colorClass: "border-orange-400/30 bg-orange-400/10 text-orange-200 hover:bg-orange-400/15",
      iconClass: "text-orange-300",
      iconPath: "M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 6h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2z",
    },
    {
      key: "tags",
      variable: "{{Tags}}",
      colorClass: "border-lime-400/30 bg-lime-400/10 text-lime-200 hover:bg-lime-400/15",
      iconClass: "text-lime-300",
      iconPath: "M7 7h.01M3 11l8.586-8.586A2 2 0 0113 2h6a2 2 0 012 2v6a2 2 0 01-.586 1.414L11.828 20a2 2 0 01-2.828 0L3 14a2 2 0 010-3z",
    },
    {
      key: "sequenceMarker",
      variable: "{{SequenceMarker}}",
      colorClass: "border-cyan-400/30 bg-cyan-400/10 text-cyan-200 hover:bg-cyan-400/15",
      iconClass: "text-cyan-300",
      iconPath: "M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01",
    },
    {
      key: "notes",
      variable: "{{Notes}}",
      colorClass: "border-fuchsia-400/30 bg-fuchsia-400/10 text-fuchsia-200 hover:bg-fuchsia-400/15",
      iconClass: "text-fuchsia-300",
      iconPath: "M11 5H6a2 2 0 00-2 2v11a1 1 0 001 1h11a2 2 0 002-2v-5M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z",
    },
  ];

  function fieldVariableName(field: { variable: string }): string {
    return field.variable.replace(/^\{\{|\}\}$/g, "");
  }

  function loadStoredValue(key: string, fallback = ""): string {
    try {
      return localStorage.getItem(key) || fallback;
    } catch {
      return fallback;
    }
  }

  let t = $derived($locale);
  let apiKeys = $state<ApiKeyConfig[]>([]);
  let tiers = $state<Tier[]>([]);
  let selectedProviderType = $state<string>("google"); // "local", "google", or "custom"
  let selectedFamily = $state<string | null>(null);
  let activeSettingsSection = $state<SettingsSection>("overview");
  let lastRequestedSection = $state<SettingsSection>("overview");
  let defaultLlmProvider = $state(loadStoredValue(DEFAULT_LLM_PROVIDER_KEY, "local"));
  let defaultLlmModel = $state(loadStoredValue(DEFAULT_LLM_MODEL_KEY, ""));
  let defaultLlmCustomProviderId = $state(loadStoredValue(DEFAULT_LLM_CUSTOM_PROVIDER_KEY, ""));
  let defaultLocalServerUrl = $state(loadStoredValue(LOCAL_SERVER_URL_KEY, DEFAULT_LOCAL_URL));
  let defaultTargetLanguage = $state(loadStoredValue(DEFAULT_TARGET_LANGUAGE_KEY, "it"));
  let defaultTranscribeLanguage = $state(loadStoredValue(DEFAULT_TRANSCRIBE_LANGUAGE_KEY, "auto"));
  let defaultFlashcardsLanguage = $state(loadStoredValue(DEFAULT_FLASHCARDS_LANGUAGE_KEY, "it"));
  let defaultNativeLanguage = $state(loadStoredValue(DEFAULT_NATIVE_LANGUAGE_KEY, "it"));
  
  const DEFAULT_REFINEMENT_PROMPT_KEY = REFINEMENT_PROMPT_STORAGE_KEY;
  let defaultRefinementPrompt = $state(loadRefinementPrompt());

  function persistRefinementPrompt() {
    try {
      localStorage.setItem(DEFAULT_REFINEMENT_PROMPT_KEY, defaultRefinementPrompt);
    } catch {}
  }

  function resetRefinementPrompt() {
    defaultRefinementPrompt = DEFAULT_REFINEMENT_PROMPT;
    persistRefinementPrompt();
  }
  let discoveredDefaultModels = $state<DiscoveredModel[]>([]);
  let isCheckingDefaultEndpoint = $state(false);
  let defaultEndpointStatus = $state<EndpointStatus>("idle");
  let defaultEndpointMessage = $state("");
  let endpointCheckRequestId = 0;
  let localProviderStatus = $state<EndpointStatus>("idle");
  let localProviderCheckRequestId = 0;

  let showAddKey = $state(false);
  let showAddModel = $state(false);
  let modalContext = $state<"llm" | "whisper">("llm");
  const llmProviderIds = ["google", "groq", "openai", "openrouter", "mistral", "github", "nvidia", "custom"];
  const whisperProviderIds = ["groq", "openai", "deepgram", "assemblyai", "custom"];

  const llmApiKeyUrls: Record<string, string> = {
    google: "https://aistudio.google.com/apikey",
    groq: "https://console.groq.com/keys",
    openai: "https://platform.openai.com/api-keys",
    openrouter: "https://openrouter.ai/keys",
    mistral: "https://console.mistral.ai/api-keys",
    github: "https://github.com/settings/personal-access-tokens",
    nvidia: "https://build.nvidia.com",
  };

  const llmDocUrls: Record<string, string> = {
    google: "https://ai.google.dev/gemini-api/docs",
    groq: "https://console.groq.com/docs",
    openai: "https://platform.openai.com/docs",
    openrouter: "https://openrouter.ai/docs",
    mistral: "https://docs.mistral.ai",
    github: "https://docs.github.com",
    nvidia: "https://docs.nvidia.com",
  };

  const whisperApiKeyUrls: Record<string, string> = {
    groq: "https://console.groq.com/keys",
    openai: "https://platform.openai.com/api-keys",
    deepgram: "https://console.deepgram.com",
    assemblyai: "https://www.assemblyai.com/app/api-keys",
  };

  const whisperDocUrls: Record<string, string> = {
    groq: "https://console.groq.com/docs/speech-to-text",
    openai: "https://platform.openai.com/guides/speech-to-text",
    deepgram: "https://developers.deepgram.com/docs/deepgram-whisper-cloud",
    assemblyai: "https://www.assemblyai.com/docs",
  };

  let translationTiersRef = $state<any>(null);
  let transcribeTiers = $state<TranscribeTier[]>([]);
  let transcribeTiersRef = $state<any>(null);

  // Whisper cloud/online settings
  const cloudSettings = loadTranscribeCloud();
  let whisperEngine = $state(cloudSettings.engine || "local");
  let whisperCloudModel = $state(cloudSettings.model || "");
  let whisperCloudCustomUrl = $state(cloudSettings.customUrl || "");
  let whisperCloudKeys = $state<Record<string, string>>(cloudSettings.keys || {});

  function persistWhisperEngineSettings() {
    saveTranscribeCloud({
      engine: whisperEngine,
      model: whisperCloudModel,
      customUrl: whisperCloudCustomUrl,
      keys: whisperCloudKeys,
    });
    window.dispatchEvent(new CustomEvent("vesta:transcribe-cloud-updated"));
  }

  function selectWhisperEngine(id: string) {
    whisperEngine = id;
    if (id !== "local") {
      const prov = transcribeProviders[id];
      if (!whisperCloudModel || !prov?.models.some((m) => m.id === whisperCloudModel)) {
        whisperCloudModel = prov?.models?.find((m) => m.recommended)?.id || prov?.models?.[0]?.id || "";
      }
    }
    persistWhisperEngineSettings();
  }

  function setWhisperCloudKey(v: string) {
    whisperCloudKeys = { ...whisperCloudKeys, [whisperEngine]: v };
    persistWhisperEngineSettings();
  }

  function setWhisperCloudModel(v: string) {
    whisperCloudModel = v;
    persistWhisperEngineSettings();
  }

  // Snackbar notification system (replaces inline error/success banners)
  // Centralized snackbar store delegation
  function showSnackbar(
    message: string,
    type: "error" | "success" = "success",
  ) {
    snackbar.show(message, type === "error" ? "error" : "success", 1300);
  }

  const settingsCopy = {
    en: {
      macroArea: "Macro area",
      overviewKicker: "Settings",
      overviewTitle: "General",
      overviewDesc: "Interface language and macro configuration areas.",
      llmTitle: "LLM, provider and API key",
      llmDesc: "Choose the engine used for translation and manage credentials.",
      whisperTitle: "Whisper",
      whisperDesc: "Download a local model: without it the Transcribe tab stays disabled.",
      languageTitle: "Languages",
      languageDesc: "Set interface language and defaults used by workflow tabs.",
      ankiTitle: "Anki Templates",
      ankiDesc: "Control the exported flashcard look, fields and note type.",
      overviewLlmDesc: "Provider, default model, endpoint and key rotation.",
      overviewWhisperDesc: "Local transcription model, downloads and default.",
      overviewLanguageDesc: "Interface language and default workflow languages.",
      overviewAnkiDesc: "HTML, CSS, fields and note type for flashcards.",
      statusTitle: "Preference status",
      statusActiveTitle: "Preferences",
      statusDesc: "A quick health check for the defaults used by the workflow tabs.",
      apiKeysSaved: "Saved API keys",
      apiKeysHint: "Remote providers available for Translation. Local models do not need keys.",
      translationLanguage: "Language to translate into",
      translationLanguageHint: "Default language used when creating translations.",
      studyingLanguage: "Language to study",
      studyingLanguageHint: "Used for flashcard expressions and auto-selecting original subtitles.",
      nativeLanguage: "Native language",
      nativeLanguageHint: "Used for flashcard meanings and auto-selecting reference subtitles.",
      transcriptionLanguageHint: "Spoken language to use when transcribing audio. Auto-detect remains available.",
      whisperHint: "Default local model used by the Transcription tab.",
      noteTypeHint: "Anki note type used by exported flashcards.",
      quickSetup: "Quick setup",
      quickSetupTitle: "Finish the essentials",
      actionRequired: "Action required",
      transcription: "Language to transcribe from",
      translation: "Translation",
      configure: "To configure",
      llmMissing: "LLM missing",
      providerCheck: "Check provider",
      setupWhisperDesc: "Download at least one Whisper model to enable the Transcribe tab.",
      setupLlmDesc: "Set the provider, key or default model to unlock the Translation tab.",
      interfaceLanguageDesc: "Choose the interface language.",
      defaultLanguages: "Default languages",
      defaultLanguagesDesc: "Choose your study pair, translation target and transcription source. Tabs still remember the last choice.",
      addProviderKicker: "New configuration",
      addProviderTitle: "Add an LLM provider",
      addProviderDesc: "Save API keys or OpenAI-compatible endpoints for translation and remote models.",
      addProviderButton: "Add configuration",
      fieldPanelKicker: "Fields and note type",
      fieldPanelTitle: "Anki export field preset",
      fieldPanelDesc: "Choose a preset, edit names and save it as a reusable template.",
      savedTemplate: "Saved template",
      templateName: "Template name",
      cardPanelKicker: "Card template",
      cardPanelTitle: "Edit Anki template files",
      cardPanelDesc: "Choose Front HTML, Back HTML or Style CSS to change the file open in the editor.",
      clickToCopy: "Click to copy.",
      llmConfigIncomplete: "LLM configuration incomplete",
      llmConfigIncompleteDescModel: "Default model missing. Select or input a model in the \"Default model\" section below.",
      llmConfigIncompleteDescLocalOffline: "The local LLM server is offline. Please start Ollama/LM Studio or verify the endpoint URL.",
      llmConfigIncompleteDescCustomEmpty: "No custom configurations saved. Please add a custom provider config first.",
      llmConfigIncompleteDescKey: "Missing API key for the selected provider. Add a key in the \"Saved API keys\" or \"Add an LLM provider\" section to unlock it.",
      llmRequiredLabel: "required",
      llmLocalOfflineLabel: "server offline",
      llmConfigMissingLabel: "configuration missing",
      llmConfigIncompleteLabel: "configuration incomplete",
    },
    it: {
      macroArea: "Macro area",
      overviewKicker: "Settings",
      overviewTitle: "Generale",
      overviewDesc: "Lingua dell'interfaccia e macro aree delle preferenze.",
      llmTitle: "LLM, provider e API key",
      llmDesc: "Scegli il motore che usera la traduzione e gestisci le credenziali.",
      whisperTitle: "Whisper",
      whisperDesc: "Scarica un modello locale: senza questo la tab Trascrizione resta disabilitata.",
      languageTitle: "Lingue",
      languageDesc: "Imposta lingua interfaccia e default usati dalle tab operative.",
      ankiTitle: "Template Anki",
      ankiDesc: "Controlla aspetto, campi e note type delle flashcard esportate.",
      overviewLlmDesc: "Provider, modello predefinito, endpoint e rotazione chiavi.",
      overviewWhisperDesc: "Modello locale di trascrizione, download e default.",
      overviewLanguageDesc: "Lingua interfaccia e lingue predefinite di lavoro.",
      overviewAnkiDesc: "HTML, CSS, campi e note type delle flashcard.",
      statusTitle: "Stato preferenze",
      statusActiveTitle: "Preferenze",
      statusDesc: "Un controllo rapido dei default usati dalle tab operative.",
      apiKeysSaved: "API key salvate",
      apiKeysHint: "Provider remoti disponibili per Traduzione. I modelli locali non richiedono chiavi.",
      translationLanguage: "Lingua in cui tradurre",
      translationLanguageHint: "Lingua predefinita usata quando crei traduzioni.",
      studyingLanguage: "Lingua da studiare",
      studyingLanguageHint: "Usata per le frasi delle flashcard e per selezionare i sottotitoli originali.",
      nativeLanguage: "Lingua madre",
      nativeLanguageHint: "Usata per i significati delle flashcard e per selezionare i sottotitoli di riferimento.",
      transcriptionLanguageHint: "Lingua parlata da usare quando trascrivi audio. Resta disponibile il rilevamento automatico.",
      whisperHint: "Modello locale predefinito usato dalla tab Trascrizione.",
      noteTypeHint: "Tipo nota Anki usato dalle flashcard esportate.",
      quickSetup: "Setup rapido",
      quickSetupTitle: "Completa le impostazioni essenziali",
      actionRequired: "Azione richiesta",
      transcription: "Lingua da cui trascrivere",
      translation: "Traduzione",
      configure: "Da configurare",
      llmMissing: "LLM mancante",
      providerCheck: "Provider da verificare",
      setupWhisperDesc: "Scarica almeno un modello Whisper per abilitare la tab Trascrizione.",
      setupLlmDesc: "Imposta provider, chiave o modello predefinito per sbloccare la tab Traduzione.",
      interfaceLanguageDesc: "Scegli la lingua dell'interfaccia.",
      defaultLanguages: "Lingue predefinite",
      defaultLanguagesDesc: "Scegli coppia di studio, lingua di arrivo delle traduzioni e lingua sorgente della trascrizione. Le tab ricordano comunque l'ultima scelta.",
      addProviderKicker: "Nuova configurazione",
      addProviderTitle: "Aggiungi un provider LLM",
      addProviderDesc: "Salva API key o endpoint compatibili OpenAI per traduzione e modelli remoti.",
      addProviderButton: "Aggiungi configurazione",
      fieldPanelKicker: "Campi e tipo nota",
      fieldPanelTitle: "Preset campi esportazione Anki",
      fieldPanelDesc: "Scegli un preset, modifica i nomi e salvalo come template riutilizzabile.",
      savedTemplate: "Template salvato",
      templateName: "Nome template",
      cardPanelKicker: "Template card",
      cardPanelTitle: "Modifica i file del template Anki",
      cardPanelDesc: "Scegli Front HTML, Back HTML o Style CSS per cambiare il file aperto nell'editor.",
      clickToCopy: "Clicca per copiare.",
      llmConfigIncomplete: "Configurazione LLM incompleta",
      llmConfigIncompleteDescModel: "Modello predefinito mancante. Seleziona o inserisci un modello nella sezione \"Modello predefinito\" sotto.",
      llmConfigIncompleteDescLocalOffline: "Il server locale LLM non risponde. Avvia Ollama o LM Studio, oppure verifica l'indirizzo.",
      llmConfigIncompleteDescCustomEmpty: "Nessun provider personalizzato salvato. Aggiungi una configurazione personalizzata per questo provider.",
      llmConfigIncompleteDescKey: "Manca la chiave API per il provider selezionato. Aggiungi una chiave nella sezione \"API Key salvate\" o \"Aggiungi un provider LLM\" per sbloccarlo.",
      llmRequiredLabel: "richiesto",
      llmLocalOfflineLabel: "server offline",
      llmConfigMissingLabel: "configurazione mancante",
      llmConfigIncompleteLabel: "configurazione incompleta",
    },
    zh: {
      macroArea: "宏区域",
      overviewKicker: "设置",
      overviewTitle: "已整理的偏好设置",
      overviewDesc: "选择一个区域，只编辑该区域相关的选项。",
      llmTitle: "LLM、提供商和 API key",
      llmDesc: "选择翻译使用的引擎并管理凭据。",
      whisperTitle: "Whisper",
      whisperDesc: "下载本地模型；没有它，转录标签页会保持禁用。",
      languageTitle: "语言",
      languageDesc: "设置界面语言以及各工作标签页的默认语言。",
      ankiTitle: "Anki 模板",
      ankiDesc: "控制导出闪卡的外观、字段和 note type。",
      overviewLlmDesc: "提供商、默认模型、端点和 key 轮换。",
      overviewWhisperDesc: "本地转录模型、下载和默认设置。",
      overviewLanguageDesc: "界面语言和工作默认语言。",
      overviewAnkiDesc: "闪卡的 HTML、CSS、字段和 note type。",
      statusTitle: "偏好状态",
      statusActiveTitle: "偏好设置",
      statusDesc: "快速检查工作标签页使用的默认设置。",
      apiKeysSaved: "已保存的 API key",
      apiKeysHint: "翻译可用的远程提供商。本地模型不需要 key。",
      translationLanguage: "翻译语言",
      translationLanguageHint: "创建翻译时使用的默认语言。",
      studyingLanguage: "学习语言",
      studyingLanguageHint: "用于闪卡例句，并自动选择原始字幕。",
      nativeLanguage: "你的语言",
      nativeLanguageHint: "用于闪卡释义，并自动选择参考字幕。",
      transcriptionLanguageHint: "转录音频时使用的口语语言；仍可选择自动检测。",
      whisperHint: "转录标签页使用的默认本地模型。",
      noteTypeHint: "导出闪卡时使用的 Anki 笔记类型。",
      quickSetup: "快速设置",
      quickSetupTitle: "完成必要设置",
      actionRequired: "需要操作",
      transcription: "转录",
      translation: "翻译",
      configure: "待配置",
      llmMissing: "缺少 LLM",
      providerCheck: "需要检查提供商",
      setupWhisperDesc: "下载至少一个 Whisper 模型以启用转录标签页。",
      setupLlmDesc: "设置提供商、key 或默认模型以解锁翻译标签页。",
      interfaceLanguageDesc: "选择界面语言。",
      defaultLanguages: "默认语言",
      defaultLanguagesDesc: "选择学习语言组合、翻译目标语言和转录源语言。各标签页仍会记住上一次选择。",
      addProviderKicker: "新配置",
      addProviderTitle: "添加 LLM 提供商",
      addProviderDesc: "保存 API key 或 OpenAI 兼容端点，用于翻译和远程模型。",
      addProviderButton: "添加配置",
      fieldPanelKicker: "字段和 note type",
      fieldPanelTitle: "Anki 导出字段预设",
      fieldPanelDesc: "选择预设、编辑名称，并保存为可复用模板。",
      savedTemplate: "已保存模板",
      templateName: "模板名称",
      cardPanelKicker: "卡片模板",
      cardPanelTitle: "编辑 Anki 模板文件",
      cardPanelDesc: "选择 Front HTML、Back HTML 或 Style CSS 来切换编辑器中的文件。",
      clickToCopy: "点击复制。",
      llmConfigIncomplete: "LLM 配置不完整",
      llmConfigIncompleteDescModel: "缺少默认模型。请在下方“默认模型”部分选择或输入一个模型。",
      llmConfigIncompleteDescLocalOffline: "本地 LLM 服务器已离线。请启动 Ollama/LM Studio 或验证端点 URL。",
      llmConfigIncompleteDescCustomEmpty: "未保存自定义配置。请先添加自定义提供商配置。",
      llmConfigIncompleteDescKey: "所选提供商缺少 API key。请在“已保存的 API key”或“添加 LLM 提供商”部分添加 key 以解锁。",
      llmRequiredLabel: "必填",
      llmLocalOfflineLabel: "服务器离线",
      llmConfigMissingLabel: "缺少配置",
      llmConfigIncompleteLabel: "配置不完整",
    },
  } as const;

  let activeUiLanguage = $derived($currentLanguage);
  function s(key: keyof typeof settingsCopy.en): string {
    return settingsCopy[activeUiLanguage as keyof typeof settingsCopy]?.[key] || settingsCopy.en[key];
  }
  let defaultProviderKeys = $derived(apiKeys.filter((k) => k.apiType === defaultLlmProvider));
  let savedCustomProviders = $derived(
    apiKeys.filter((k) => k.apiType === "custom" && k.apiUrl && k.apiUrl.trim().length > 0),
  );
  let selectedDefaultCustomProvider = $derived(
    savedCustomProviders.find((key) => key.id === defaultLlmCustomProviderId),
  );
  let activeDefaultEndpointUrl = $derived.by(() => {
    if (defaultLlmProvider === "local") return defaultLocalServerUrl;
    if (defaultLlmProvider === "custom") return selectedDefaultCustomProvider?.apiUrl || "";
    return "";
  });
  let activeDefaultEndpointApiKey = $derived(
    defaultLlmProvider === "custom" ? selectedDefaultCustomProvider?.apiKey || "" : "",
  );
  let defaultProviderModels = $derived.by<ModelInfo[]>(() => {
    if (defaultLlmProvider === "local" || defaultLlmProvider === "custom") {
      return discoveredDefaultModels.map((model) => ({
        id: model.id,
        name: model.name,
        provider: defaultLlmProvider,
        family: "Endpoint",
      }));
    }
    return getModelsForProvider(defaultLlmProvider);
  });
  let defaultWorkflowLanguageOptions = $derived(
    languages.map((lang) => ({
      value: lang.code,
      label: lang.nameEn === lang.name ? lang.name : `${lang.nameEn} — ${lang.name}`,
      icon: lang.flag,
      searchTerms: getLanguageSearchTerms(lang.code),
    })),
  );
  let configuredApiKeyCount = $derived(apiKeys.filter((key) => key.apiType !== "local").length);
  let hasRemoteApiKey = $derived(configuredApiKeyCount > 0);
  // La traduzione è pronta quando esiste almeno un endpoint tier usabile.
  let isDefaultLlmReady = $derived(tiersHaveUsableEntries(tiers));
  let requiredSetupActions = $derived.by(() => {
    const actions: { section: SettingsSection; title: string; desc: string }[] = [];
    if (downloadedWhisperCount === 0) {
      actions.push({
        section: "whisper",
        title: s("transcription"),
        desc: s("setupWhisperDesc"),
      });
    }
    if (!isDefaultLlmReady) {
      actions.push({
        section: "llm",
        title: s("translation"),
        desc: s("setupLlmDesc"),
      });
    }
    return actions;
  });
  type ProviderStatus = "available" | "checking" | "offline" | "requiresKey";

  function getProviderStatus(providerId: string): ProviderStatus {
    if (providerId === "local") {
      if (localProviderStatus === "online") return "available";
      if (localProviderStatus === "checking") return "checking";
      return "offline";
    }
    if (providerId === "custom") {
      return savedCustomProviders.length > 0 ? "available" : "requiresKey";
    }
    return apiKeys.some((key) => key.apiType === providerId) ? "available" : "requiresKey";
  }

  function getProviderStatusClasses(providerId: string) {
    const status = getProviderStatus(providerId);
    if (status === "available") {
      return "border-indigo-500/30 bg-indigo-500/10 text-indigo-100";
    }
    if (status === "checking") {
      return "border-cyan-500/25 bg-cyan-500/10 text-cyan-200";
    }
    if (status === "offline") {
      return "border-red-500/25 bg-red-500/10 text-red-200";
    }
    return "border-white/10 bg-white/5 text-gray-500";
  }

  function getProviderStatusText(providerId: string): string {
    return t(`settings.providerStatus.${getProviderStatus(providerId)}`);
  }

  function getEndpointStatusText(status: EndpointStatus): string {
    if (status === "checking") return t("settings.endpointStatus.checking");
    if (status === "online") return t("settings.endpointStatus.online");
    if (status === "offline") return t("settings.endpointStatus.offline");
    return t("settings.endpointStatus.idle");
  }

  function providerDescription(pid: string): string {
    const key = `provider.${pid}.desc`;
    const translated = t(key);
    return translated === key ? providers[pid]?.description || "" : translated;
  }
  let activeSectionMeta = $derived.by(() => {
    if (activeSettingsSection === "llm") {
      return {
        label: s("macroArea"),
        title: s("llmTitle"),
        desc: s("llmDesc"),
        accent: "text-indigo-300",
        iconPath: "M9 3h6m-7 4h8a3 3 0 013 3v7a3 3 0 01-3 3H8a3 3 0 01-3-3v-7a3 3 0 013-3zm4 3v4m-2-2h4",
      };
    }
    if (activeSettingsSection === "whisper") {
      return {
        label: s("macroArea"),
        title: s("whisperTitle"),
        desc: s("whisperDesc"),
        accent: "text-cyan-300",
        iconPath: "M12 18a6 6 0 006-6V7a6 6 0 10-12 0v5a6 6 0 006 6zm0 0v3m-4 0h8",
      };
    }
    if (activeSettingsSection === "language") {
      return {
        label: s("macroArea"),
        title: s("languageTitle"),
        desc: s("languageDesc"),
        accent: "text-emerald-300",
        iconPath: "M3 5h12M9 3v2m1 9a18 18 0 01-4-5m7 12l5-10 5 10m-9-4h8",
      };
    }
    return {
      label: s("macroArea"),
      title: s("ankiTitle"),
      desc: s("ankiDesc"),
      accent: "text-amber-300",
      iconPath: "M4 5a1 1 0 011-1h14a1 1 0 011 1v4H4V5zm0 8h8v7H5a1 1 0 01-1-1v-6zm12 0h4v6a1 1 0 01-1 1h-3v-7z",
    };
  });

  function openSettingsSection(section: SettingsSection) {
    activeSettingsSection = section;
    requestedSection = section;
  }

  function openFirstRequiredAction() {
    const firstAction = requiredSetupActions[0];
    if (firstAction) openSettingsSection(firstAction.section);
  }

  function handleOpenSettingsSectionEvent(event: Event) {
    const section = (event as CustomEvent<SettingsSection>).detail;
    if (section) openSettingsSection(section);
  }

  $effect(() => {
    if (requestedSection && requestedSection !== lastRequestedSection) {
      activeSettingsSection = requestedSection;
      lastRequestedSection = requestedSection;
    }
  });

  function persistDefaultLlmSettings() {
    localStorage.setItem(DEFAULT_LLM_PROVIDER_KEY, defaultLlmProvider);
    localStorage.setItem(DEFAULT_LLM_MODEL_KEY, defaultLlmModel);
    localStorage.setItem(DEFAULT_LLM_CUSTOM_PROVIDER_KEY, defaultLlmCustomProviderId);
    localStorage.setItem(LOCAL_SERVER_URL_KEY, defaultLocalServerUrl);
    window.dispatchEvent(new CustomEvent("vesta-llm-default-updated"));
  }

  function hasDefaultProviderAccess(providerId: string) {
    if (providerId === "local") return true;
    if (providerId === "custom") return savedCustomProviders.length > 0;
    return apiKeys.some((key) => key.apiType === providerId);
  }

  function setDefaultLlmProvider(providerId: string) {
    if (!hasDefaultProviderAccess(providerId)) {
      selectedProviderType = providerId;
      openAddKeyModal(providerId);
      return;
    }
    defaultLlmProvider = providerId;
    selectedProviderType = providerId;
    if (providerId !== "custom") {
      defaultLlmCustomProviderId = "";
    } else if (!defaultLlmCustomProviderId && savedCustomProviders.length > 0) {
      defaultLlmCustomProviderId = savedCustomProviders[0].id;
    }
    const models = getModelsForProvider(providerId);
    if (providerId !== "local" && providerId !== "custom" && models.length > 0 && !models.some((model) => model.id === defaultLlmModel)) {
      defaultLlmModel = models.find((model) => model.recommended)?.id || models[0].id;
    } else if (providerId === "local" || providerId === "custom") {
      defaultLlmModel = "";
    }
    persistDefaultLlmSettings();
  }

  async function refreshDefaultEndpointModels() {
    const provider = defaultLlmProvider;
    const endpointUrl = activeDefaultEndpointUrl.trim();
    const endpointApiKey = activeDefaultEndpointApiKey;
    const requestId = ++endpointCheckRequestId;

    if (provider !== "local" && provider !== "custom") {
      defaultEndpointStatus = "idle";
      defaultEndpointMessage = t("settings.endpointStatus.idle");
      discoveredDefaultModels = [];
      return;
    }

    if (!endpointUrl) {
      defaultEndpointStatus = "offline";
      defaultEndpointMessage = t("settings.endpointStatus.offline");
      discoveredDefaultModels = [];
      return;
    }

    if (defaultEndpointStatus === "idle") {
      defaultEndpointStatus = "checking";
      defaultEndpointMessage = t("settings.endpointStatus.checking");
    }
    isCheckingDefaultEndpoint = true;

    try {
      const models = await fetchModelsFromEndpoint(endpointUrl, endpointApiKey, 6500);
      if (requestId !== endpointCheckRequestId) return;
      discoveredDefaultModels = models;
      defaultEndpointStatus = "online";
      defaultEndpointMessage = t("settings.modelsAvailable", { count: models.length });
      if (!defaultLlmModel || !models.some((model) => model.id === defaultLlmModel)) {
        defaultLlmModel = models[0]?.id || "";
        persistDefaultLlmSettings();
      }
    } catch (error) {
      if (requestId !== endpointCheckRequestId) return;
      discoveredDefaultModels = [];
      defaultEndpointStatus = "offline";
      defaultEndpointMessage = t("settings.endpointStatus.offline");
    } finally {
      if (requestId === endpointCheckRequestId) {
        isCheckingDefaultEndpoint = false;
      }
    }
  }

  async function refreshLocalProviderStatus() {
    const endpointUrl = defaultLocalServerUrl.trim();
    const requestId = ++localProviderCheckRequestId;

    if (!endpointUrl) {
      localProviderStatus = "offline";
      return;
    }

    if (localProviderStatus === "idle") {
      localProviderStatus = "checking";
    }

    try {
      await fetchModelsFromEndpoint(endpointUrl, "", 6500);
      if (requestId !== localProviderCheckRequestId) return;
      localProviderStatus = "online";
    } catch {
      if (requestId !== localProviderCheckRequestId) return;
      localProviderStatus = "offline";
    }
  }

  $effect(() => {
    if (!hasDefaultProviderAccess(defaultLlmProvider)) {
      defaultLlmProvider = "local";
      defaultLlmCustomProviderId = "";
      defaultLlmModel = "";
      persistDefaultLlmSettings();
    }
  });

  $effect(() => {
    if (defaultLlmProvider === "custom" && !defaultLlmCustomProviderId && savedCustomProviders.length > 0) {
      defaultLlmCustomProviderId = savedCustomProviders[0].id;
      persistDefaultLlmSettings();
    }
  });

  $effect(() => {
    if (!active) return;
    const provider = defaultLlmProvider;
    const endpointUrl = activeDefaultEndpointUrl;
    const endpointApiKey = activeDefaultEndpointApiKey;

    if (provider !== "local" && provider !== "custom") {
      defaultEndpointStatus = "idle";
      defaultEndpointMessage = t("settings.endpointStatus.idle");
      discoveredDefaultModels = [];
      return;
    }

    if (provider === "local" && localProviderStatus !== "idle" && localProviderStatus !== "checking") {
      defaultEndpointStatus = localProviderStatus;
      defaultEndpointMessage = localProviderStatus === "online" ? t("settings.endpointStatus.online") : t("settings.endpointStatus.offline");
    }

    const timeout = setTimeout(() => {
      void refreshDefaultEndpointModels();
    }, 300);
    const interval = setInterval(() => {
      void refreshDefaultEndpointModels();
    }, 15000);

    return () => {
      void endpointUrl;
      void endpointApiKey;
      clearTimeout(timeout);
      clearInterval(interval);
    };
  });

  $effect(() => {
    if (!active) return;
    const endpointUrl = defaultLocalServerUrl;
    const timeout = setTimeout(() => {
      void refreshLocalProviderStatus();
    }, 300);
    const interval = setInterval(() => {
      void refreshLocalProviderStatus();
    }, 15000);

    return () => {
      void endpointUrl;
      clearTimeout(timeout);
      clearInterval(interval);
    };
  });

  function saveDefaultLanguage(key: string, value: string) {
    localStorage.setItem(key, value);
    window.dispatchEvent(new CustomEvent("vesta-language-defaults-updated"));
  }

  $effect(() => {
    if (!defaultLlmModel && defaultProviderModels.length > 0) {
      defaultLlmModel =
        defaultProviderModels.find((model) => model.recommended)?.id ||
        defaultProviderModels[0].id;
      persistDefaultLlmSettings();
    }
  });

  // CPU Cores Settings
  let systemCpuCount = $state(4);
  let cpuCores = $state(2);
  let minCpuCores = $derived(2);
  let maxCpuCores = $derived(Math.max(2, systemCpuCount - 1));

  let cpuPresets = $derived([
    { id: "eco", threads: minCpuCores },
    {
      id: "balanced",
      threads: minCpuCores + Math.ceil((maxCpuCores - minCpuCores) / 3),
    },
    {
      id: "performance",
      threads: minCpuCores + Math.ceil(((maxCpuCores - minCpuCores) * 2) / 3),
    },
    { id: "full", threads: maxCpuCores },
  ] as const);

  let activeCpuPreset = $derived(
    cpuPresets.find((p) => p.threads === cpuCores)?.id ?? null,
  );

  function setCpuPreset(presetId: string) {
    const preset = cpuPresets.find((p) => p.id === presetId);
    if (preset) {
      cpuCores = preset.threads;
      localStorage.setItem("vesta_cpu_cores", cpuCores.toString());
      window.dispatchEvent(new CustomEvent("vesta-cpu-cores-changed", { detail: cpuCores }));
    }
  }

  // Smart Matching Settings
  let smartMatchingEnabled = $derived(smartMatchingStore.enabled);
  let smartMatchingRulesDraft = $state("");
  let smartMatchingRulesError = $state<string | null>(null);

  function formatSmartMatchingRules(rules: any): string {
    return JSON.stringify(rules, null, 2);
  }

  function toggleSmartMatching() {
    smartMatchingStore.setEnabled(!smartMatchingStore.enabled);
    if (smartMatchingStore.enabled) {
      smartMatchingRulesDraft = formatSmartMatchingRules(smartMatchingStore.rules);
      smartMatchingRulesError = null;
    }
  }

  function getSmartMatchingRulesDraftError(): string | null {
    try {
      const parsed = JSON.parse(smartMatchingRulesDraft.replace(/\\"|"(?:\\"|[^"])*"|(\/\/.*|\/\*[\s\S]*?\*\/)/g, (m, g) => g ? "" : m));
      if (!parsed || typeof parsed !== "object") return "Must be a valid JSON object";
      return null;
    } catch (e: any) {
      return e.message || "Invalid JSON";
    }
  }

  function saveSmartMatchingRules() {
    const err = getSmartMatchingRulesDraftError();
    smartMatchingRulesError = err;
    if (err) return;
    try {
      const cleanJson = smartMatchingRulesDraft.replace(/\\"|"(?:\\"|[^"])*"|(\/\/.*|\/\*[\s\S]*?\*\/)/g, (m, g) => g ? "" : m);
      const parsed = JSON.parse(cleanJson);
      smartMatchingStore.saveRules(parsed);
      snackbar.show("Regole smart matching salvate con successo!", "success", 1300);
    } catch (e: any) {
      smartMatchingRulesError = e.message || "Salvataggio fallito";
    }
  }

  function resetSmartMatchingRules() {
    showResetConfirm = "smartMatching";
  }



  // Card template editor
  let showResetConfirm = $state<"style" | "fields" | "smartMatching" | "overview" | "llm" | "whisper" | "language" | "anki" | null>(null);
  let resetTitle = $derived.by(() => {
    if (showResetConfirm === "smartMatching") {
      return "Ripristinare regole di default?";
    }
    if (showResetConfirm === "overview") {
      return "Ripristinare lingua predefinita?";
    }
    if (showResetConfirm === "llm") {
      return "Ripristinare impostazioni LLM predefinite?";
    }
    if (showResetConfirm === "whisper") {
      return "Ripristinare modello Whisper predefinito?";
    }
    if (showResetConfirm === "language") {
      return "Ripristinare impostazioni lingue e matching?";
    }
    if (showResetConfirm === "anki") {
      return "Ripristinare template e campi Anki?";
    }
    return t("settings.resetConfirmTitle") || "Ripristinare i valori predefiniti?";
  });
  let resetMessage = $derived.by(() => {
    if (showResetConfirm === "smartMatching") {
      return "Tutte le personalizzazioni delle regole dello Smart Matching andranno perse.";
    }
    if (showResetConfirm === "overview") {
      return "La lingua dell'interfaccia verrà ripristinata all'italiano.";
    }
    if (showResetConfirm === "llm") {
      return "Tutte le configurazioni LLM e URL locali verranno ripristinate ai valori predefiniti.";
    }
    if (showResetConfirm === "whisper") {
      return "Il modello Whisper predefinito verrà reimpostato su 'base'.";
    }
    if (showResetConfirm === "language") {
      return "Tutte le preferenze di lingua e le regole dello Smart Matching verranno ripristinate.";
    }
    if (showResetConfirm === "anki") {
      return "Tutti i template HTML/CSS e i preset dei campi Anki verranno ripristinati.";
    }
    return t("settings.resetConfirmDesc") || "Tutte le personalizzazioni correnti andranno perse.";
  });
  let activeTemplateCodeTab = $state<TemplateCodeTab>("front");
  const templateCodeTabs: { id: TemplateCodeTab; label: string; hint: string }[] = [
    { id: "front", label: "Front HTML", hint: "Modifica il file front.html di questo template" },
    { id: "back", label: "Back HTML", hint: "Modifica il file back.html di questo template" },
    { id: "css", label: "Style CSS", hint: "Modifica il file style.css condiviso da questo template" },
  ];
  const initTemplates = loadCardTemplates();
  let templateFrontHtml = $state(initTemplates.frontHtml);
  let templateBackHtml = $state(initTemplates.backHtml);
  let templateCss = $state(initTemplates.css);
  let noteTypeName = $state(initTemplates.noteTypeName);

  // Field names
  const initFieldNames = loadFieldNames();
  let fieldExpression = $state(initFieldNames.expression);
  let fieldMeaning = $state(initFieldNames.meaning);
  let fieldReading = $state(initFieldNames.reading);
  let fieldAudio = $state(initFieldNames.audio);
  let fieldSnapshot = $state(initFieldNames.snapshot);
  let fieldVideo = $state(initFieldNames.video);
  let fieldTags = $state(initFieldNames.tags);
  let fieldSequenceMarker = $state(initFieldNames.sequenceMarker);
  let fieldNotes = $state(initFieldNames.notes);
  const initAnkiFieldPresets = loadAnkiFieldPresets();
  const initAnkiFieldPresetId = loadStoredValue(ACTIVE_ANKI_FIELD_PRESET_KEY, "default");
  const initSelectedAnkiFieldPreset = initAnkiFieldPresets.find(
    (preset) => preset.id === initAnkiFieldPresetId,
  );
  let savedAnkiFieldPresets = $state<AnkiFieldPreset[]>(initAnkiFieldPresets);
  let selectedAnkiFieldPresetId = $state(initSelectedAnkiFieldPreset?.id || "default");
  let ankiFieldPresetName = $state(initSelectedAnkiFieldPreset?.name || "");
  let activeNoteTypeId = $state(loadActiveNoteTypeId());

  $effect(() => {
    const handler = () => {
      activeNoteTypeId = loadActiveNoteTypeId();
    };
    window.addEventListener(ACTIVE_NOTE_TYPE_CHANGED_EVENT, handler);
    return () => {
      window.removeEventListener(ACTIVE_NOTE_TYPE_CHANGED_EVENT, handler);
    };
  });
  let allAnkiFieldPresets = $derived<AnkiFieldPreset[]>([
    {
      id: "default",
      name: "Default_Vesta",
      noteTypeName: defaultCardTemplates.noteTypeName,
      fields: defaultFieldNames,
    },
    ...savedAnkiFieldPresets,
  ]);

  const EXPORT_FORMAT_KEY = "vesta-export-format";
  let exportFormat = $state<"apkg" | "tsv">(
    (() => {
      try {
        const saved = localStorage.getItem(EXPORT_FORMAT_KEY);
        return saved === "tsv" ? "tsv" : "apkg";
      } catch { return "apkg"; }
    })()
  );
  $effect(() => {
    try { localStorage.setItem(EXPORT_FORMAT_KEY, exportFormat); } catch {}
  });

  $effect(() => {
    if (active) {
      try {
        const saved = localStorage.getItem(EXPORT_FORMAT_KEY);
        exportFormat = saved === "tsv" ? "tsv" : "apkg";
      } catch {}
    }
  });

  $effect(() => {
    const _ = uiMode.expertMode;
    try {
      const saved = localStorage.getItem(EXPORT_FORMAT_KEY);
      exportFormat = saved === "tsv" ? "tsv" : "apkg";
    } catch {}
    try {
      const savedCores = localStorage.getItem("vesta_cpu_cores");
      if (savedCores) {
        cpuCores = parseInt(savedCores);
      }
    } catch {}
  });

  $effect(() => {
    if (!uiMode.expertMode) {
      if (activeSettingsSection === "language" || activeSettingsSection === "anki") {
        activeSettingsSection = "overview";
      }
      if (requestedSection === "language" || requestedSection === "anki") {
        requestedSection = "overview";
      }
    }
  });

  function saveTemplates() {
    saveCardTemplates({
      frontHtml: templateFrontHtml,
      backHtml: templateBackHtml,
      css: templateCss,
      noteTypeName: noteTypeName,
    });
  }

  function getCurrentFieldNames(): FieldNamesConfig {
    return {
      expression: fieldExpression,
      meaning: fieldMeaning,
      reading: fieldReading,
      audio: fieldAudio,
      snapshot: fieldSnapshot,
      video: fieldVideo,
      tags: fieldTags,
      sequenceMarker: fieldSequenceMarker,
      notes: fieldNotes,
    };
  }

  function setCurrentFieldNames(fields: FieldNamesConfig) {
    fieldExpression = fields.expression;
    fieldMeaning = fields.meaning;
    fieldReading = fields.reading;
    fieldAudio = fields.audio;
    fieldSnapshot = fields.snapshot;
    fieldVideo = fields.video;
    fieldTags = fields.tags;
    fieldSequenceMarker = fields.sequenceMarker;
    fieldNotes = fields.notes;
  }

  function getFieldValue(key: AnkiFieldKey): string {
    return getCurrentFieldNames()[key];
  }

  function getFieldVariable(field: (typeof ankiFieldDefinitions)[number]): string {
    const fieldName = getFieldValue(field.key).trim() || field.variable.slice(2, -2);
    return `{{${fieldName}}}`;
  }

  function setFieldValue(key: AnkiFieldKey, value: string) {
    const fields = getCurrentFieldNames();
    fields[key] = value;
    setCurrentFieldNames(fields);
  }



  function sanitizeAnkiFieldPreset(raw: Partial<AnkiFieldPreset>): AnkiFieldPreset | null {
    if (!raw.id || !raw.name || !raw.fields) return null;
    const fields = raw.fields as Partial<FieldNamesConfig>;
    return {
      id: raw.id,
      name: limitNoteTypeFieldValue(raw.name),
      noteTypeName: limitNoteTypeFieldValue(raw.noteTypeName || defaultCardTemplates.noteTypeName),
      fields: {
        expression: limitNoteTypeFieldValue(fields.expression !== undefined && fields.expression !== "" ? fields.expression : defaultFieldNames.expression),
        meaning: limitNoteTypeFieldValue(fields.meaning !== undefined ? fields.meaning : defaultFieldNames.meaning),
        reading: limitNoteTypeFieldValue(fields.reading !== undefined ? fields.reading : defaultFieldNames.reading),
        audio: limitNoteTypeFieldValue(fields.audio !== undefined ? fields.audio : defaultFieldNames.audio),
        snapshot: limitNoteTypeFieldValue(fields.snapshot !== undefined ? fields.snapshot : defaultFieldNames.snapshot),
        video: limitNoteTypeFieldValue(fields.video !== undefined ? fields.video : defaultFieldNames.video),
        tags: limitNoteTypeFieldValue(fields.tags !== undefined ? fields.tags : defaultFieldNames.tags),
        sequenceMarker: limitNoteTypeFieldValue(fields.sequenceMarker !== undefined ? fields.sequenceMarker : defaultFieldNames.sequenceMarker),
        notes: limitNoteTypeFieldValue(fields.notes !== undefined ? fields.notes : defaultFieldNames.notes),
      },
    };
  }

  function loadAnkiFieldPresets(): AnkiFieldPreset[] {
    try {
      const raw = localStorage.getItem(ANKI_FIELD_PRESETS_KEY);
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      return parsed
        .map((preset) => sanitizeAnkiFieldPreset(preset))
        .filter((preset): preset is AnkiFieldPreset => Boolean(preset));
    } catch {
      return [];
    }
  }

  function persistAnkiFieldPresets() {
    localStorage.setItem(ANKI_FIELD_PRESETS_KEY, JSON.stringify(savedAnkiFieldPresets));
  }

  function applyAnkiFieldPreset(presetId: string) {
    const preset = allAnkiFieldPresets.find((item) => item.id === presetId);
    if (!preset) return;
    selectedAnkiFieldPresetId = preset.id;
    ankiFieldPresetName = preset.id === "default" ? "" : preset.name;
    localStorage.setItem(ACTIVE_ANKI_FIELD_PRESET_KEY, preset.id);
    noteTypeName = preset.noteTypeName;
    setCurrentFieldNames(preset.fields);
    saveTemplates();
    saveFields();
    window.dispatchEvent(new CustomEvent(NOTE_TYPES_UPDATED_EVENT));
  }

  function saveCurrentAnkiFieldPreset() {
    const existingPreset = savedAnkiFieldPresets.find((preset) => preset.id === selectedAnkiFieldPresetId);
    const fallbackName = existingPreset?.name || noteTypeName || "Template Anki";
    const presetName = limitNoteTypeFieldValue((ankiFieldPresetName || fallbackName).trim());
    const preset: AnkiFieldPreset = {
      id: existingPreset?.id || `custom-${Date.now().toString(36)}`,
      name: presetName,
      noteTypeName: limitNoteTypeFieldValue(noteTypeName || defaultCardTemplates.noteTypeName),
      fields: getCurrentFieldNames(),
    };

    if (existingPreset) {
      savedAnkiFieldPresets = savedAnkiFieldPresets.map((item) =>
        item.id === existingPreset.id ? preset : item,
      );
    } else {
      savedAnkiFieldPresets = [...savedAnkiFieldPresets, preset];
    }

    selectedAnkiFieldPresetId = preset.id;
    ankiFieldPresetName = preset.name;
    localStorage.setItem(ACTIVE_ANKI_FIELD_PRESET_KEY, preset.id);
    persistAnkiFieldPresets();
    window.dispatchEvent(new CustomEvent(NOTE_TYPES_UPDATED_EVENT));
    showSnackbar(t("settings.anki.presetSaved"));
  }

  function deleteCurrentAnkiFieldPreset() {
    if (selectedAnkiFieldPresetId === "default") return;
    const deletedId = selectedAnkiFieldPresetId;
    savedAnkiFieldPresets = savedAnkiFieldPresets.filter((preset) => preset.id !== deletedId);
    persistAnkiFieldPresets();

    // Reset activeNoteTypeId to default if it was deleted
    const currentActiveId = loadActiveNoteTypeId();
    const formattedDeletedId = deletedId.startsWith("custom:") ? deletedId : `custom:${deletedId}`;
    const formattedActiveId = currentActiveId.startsWith("custom:") ? currentActiveId : `custom:${currentActiveId}`;
    if (formattedActiveId === formattedDeletedId) {
      saveActiveNoteTypeId("default");
      activeNoteTypeId = "default";
    }

    applyAnkiFieldPreset("default");
    window.dispatchEvent(new CustomEvent(NOTE_TYPES_UPDATED_EVENT));
    showSnackbar(t("settings.anki.presetDeleted"));
  }

  function resetAnkiFieldsToDefault() {
    selectedAnkiFieldPresetId = "default";
    ankiFieldPresetName = "";
    localStorage.setItem(ACTIVE_ANKI_FIELD_PRESET_KEY, "default");
    noteTypeName = defaultCardTemplates.noteTypeName;
    setCurrentFieldNames(defaultFieldNames);
    saveTemplates();
    saveFields();
    window.dispatchEvent(new CustomEvent(NOTE_TYPES_UPDATED_EVENT));
  }

  function resetOverviewSettings() {
    setLanguage("it");
    snackbar.show(t("settings.overview.resetSuccess"), "info", 1300);
  }

  function resetLlmSettings() {
    defaultLlmProvider = "local";
    defaultLocalServerUrl = DEFAULT_LOCAL_URL;
    defaultLlmCustomProviderId = "";
    defaultLlmModel = "";
    resetRefinementPrompt();
    persistDefaultLlmSettings();
    snackbar.show(t("settings.llm.resetSuccess"), "info", 1300);
  }

  function resetWhisperSettings() {
    const baseModel = whisperModels.find((m) => m.id === "base");
    if (baseModel && baseModel.downloaded) {
      defaultWhisperModel = "base";
      localStorage.setItem("srt-default-whisper-model", "base");
      snackbar.show(t("settings.whisper.resetSuccess"), "info", 2000);
    } else {
      // Find if there is another downloaded model
      const alternate = whisperModels.find((m) => m.downloaded);
      if (alternate) {
        defaultWhisperModel = alternate.id;
        localStorage.setItem("srt-default-whisper-model", alternate.id);
        snackbar.show(t("settings.whisper.resetBaseNotDownloaded", { name: alternate.name }), "warning", 3000);
      } else {
        defaultWhisperModel = "base";
        localStorage.setItem("srt-default-whisper-model", "base");
        snackbar.show(t("settings.whisper.resetBaseDownloadWarning"), "warning", 3000);
      }
    }
  }

  function resetLanguageSettings() {
    defaultTargetLanguage = "it";
    defaultTranscribeLanguage = "auto";
    defaultFlashcardsLanguage = "it";
    defaultNativeLanguage = "it";
    saveDefaultLanguage(DEFAULT_TARGET_LANGUAGE_KEY, "it");
    saveDefaultLanguage(DEFAULT_TRANSCRIBE_LANGUAGE_KEY, "auto");
    saveDefaultLanguage(DEFAULT_FLASHCARDS_LANGUAGE_KEY, "it");
    saveDefaultLanguage(DEFAULT_NATIVE_LANGUAGE_KEY, "it");

    smartMatchingStore.resetRules();
    smartMatchingStore.setEnabled(true);
    smartMatchingRulesDraft = formatSmartMatchingRules(smartMatchingStore.rules);
    smartMatchingRulesError = null;

    snackbar.show(t("settings.language.resetSuccess"), "info", 1300);
  }

  function resetAnkiSettings() {
    const defaults = resetCardTemplates();
    templateFrontHtml = defaults.frontHtml;
    templateBackHtml = defaults.backHtml;
    templateCss = defaults.css;
    noteTypeName = defaults.noteTypeName;
    resetAnkiFieldsToDefault();
    snackbar.show(t("settings.anki.resetSuccess"), "info", 1300);
  }

  function saveFields() {
    saveFieldNames({
      expression: fieldExpression,
      meaning: fieldMeaning,
      reading: fieldReading,
      audio: fieldAudio,
      snapshot: fieldSnapshot,
      video: fieldVideo,
      tags: fieldTags,
      sequenceMarker: fieldSequenceMarker,
      notes: fieldNotes,
    });
  }

  function syncLimitedInput(
    event: Event,
    applyValue: (value: string) => void,
    save: () => void,
  ) {
    const target = event.currentTarget;
    if (!(target instanceof HTMLInputElement)) return;

    const limitedValue = limitNoteTypeFieldValue(target.value);
    if (target.value !== limitedValue) {
      target.value = limitedValue;
    }

    applyValue(limitedValue);
    save();
  }

  let newKeyName = $state("");
  let newKeyType = $state<ApiKeyConfig["apiType"]>("google");
  let newKeyValue = $state("");
  let newKeyUrl = $state("");
  let showNewKeyPassword = $state(false);
  let editKeyId = $state<string | null>(null);

  let currentProviderModels = $derived(
    getModelsForProvider(selectedProviderType),
  );

  let families = $derived.by(() => {
    const fams = new Set<string>();
    currentProviderModels.forEach((m) => {
      if (m.family) fams.add(m.family);
    });
    return Array.from(fams).sort();
  });

  $effect(() => {
    if (
      families.length > 0 &&
      (!selectedFamily || !families.includes(selectedFamily))
    ) {
      selectedFamily = families[0];
    }
  });

  let filteredModels = $derived(
    selectedFamily
      ? currentProviderModels.filter((m) => m.family === selectedFamily)
      : [],
  );

  // Update section states
  type UpdateStatus = "idle" | "checking" | "available" | "current" | "error" | "disabled" | "offline";
  let automaticUpdateChecks = $state<boolean>(true);
  let updateStatus = $state<UpdateStatus>("idle");
  let latestVersion = $state<string>("");
  let releaseUrl = $state<string>("https://github.com/pierspad/VESTA/releases");
  let appVersionNum = $state<string>("");
  let updateError = $state<string>("");

  const RELEASE_API_URL = "https://api.github.com/repos/pierspad/Vesta/releases/latest";

  function normalizeVersion(version: string): string {
    return version.trim().replace(/^v/i, "").split(/[+-]/)[0];
  }

  function compareVersions(left: string, right: string): number {
    const leftParts = normalizeVersion(left).split(".").map((part) => Number.parseInt(part, 10) || 0);
    const rightParts = normalizeVersion(right).split(".").map((part) => Number.parseInt(part, 10) || 0);
    const length = Math.max(leftParts.length, rightParts.length);

    for (let i = 0; i < length; i += 1) {
      const diff = (leftParts[i] || 0) - (rightParts[i] || 0);
      if (diff !== 0) return diff;
    }

    return 0;
  }

  async function checkForUpdates(source: "auto" | "manual" = "manual") {
    if (source === "auto" && !automaticUpdateChecks) {
      updateStatus = "disabled";
      return;
    }

    if (typeof navigator !== "undefined" && navigator.onLine === false) {
      updateStatus = "offline";
      if (source === "manual") {
        snackbar.show($currentLanguage === "it" ? "Connessione assente o GitHub non raggiungibile" : "No connection or GitHub is unreachable", "error");
      }
      return;
    }

    updateStatus = "checking";
    updateError = "";

    // 1. Primary Strategy: GitHub official API via CORS-free tauriFetch
    try {
      const response = await tauriFetch(RELEASE_API_URL, {
        method: "GET",
        headers: {
          "Accept": "application/vnd.github+json",
          "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
        }
      });

      if (!response.ok) {
        throw new Error(`GitHub API returned status ${response.status}`);
      }

      const latest = await response.json() as {
        tag_name?: string;
        name?: string;
        html_url?: string;
      };
      
      const tag = latest.tag_name || latest.name || "";
      if (!tag) {
        throw new Error("Empty version tag in API response");
      }

      latestVersion = tag.startsWith("v") || tag.startsWith("V") ? tag : `v${tag}`;
      releaseUrl = latest.html_url || "https://github.com/pierspad/VESTA/releases";
      
      processUpdateResult(source);
      return;
    } catch (apiError) {
      console.warn("Vesta update check: GitHub API failed, trying package.json fallback:", apiError);
    }

    // 2. Secondary Strategy: Raw package.json via CORS-free tauriFetch (rate-limit free!)
    try {
      const response = await tauriFetch("https://raw.githubusercontent.com/pierspad/Vesta/main/apps/srt-gui/package.json", {
        method: "GET",
        headers: {
          "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
        }
      });

      if (!response.ok) {
        throw new Error(`Raw package.json fetch returned status ${response.status}`);
      }

      const pkg = await response.json() as { version?: string };
      const tag = pkg.version || "";
      if (!tag) {
        throw new Error("Empty version field in package.json");
      }

      latestVersion = tag.startsWith("v") || tag.startsWith("V") ? tag : `v${tag}`;
      releaseUrl = "https://github.com/pierspad/VESTA/releases";

      processUpdateResult(source);
      return;
    } catch (pkgError) {
      console.warn("Vesta update check: Raw package.json fallback failed, trying redirect fallback:", pkgError);
    }

    // 3. Tertiary Strategy: Redirect check via tauriFetch with redirect: "manual"
    try {
      const response = await tauriFetch("https://github.com/pierspad/Vesta/releases/latest", {
        method: "GET",
        redirect: "manual",
        headers: {
          "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
        }
      });

      let tag = "";
      let finalUrl = "";

      const location = response.headers.get("location");
      if ((response.status >= 300 && response.status < 400) && location) {
        finalUrl = location;
        tag = location.substring(location.lastIndexOf("/") + 1);
      } else if (response.ok) {
        finalUrl = response.url || "";
        tag = finalUrl.substring(finalUrl.lastIndexOf("/") + 1);
      }

      if (!tag || tag === "latest") {
        throw new Error("Could not parse redirect version tag");
      }

      latestVersion = tag.startsWith("v") || tag.startsWith("V") ? tag : `v${tag}`;
      releaseUrl = finalUrl || "https://github.com/pierspad/VESTA/releases";

      processUpdateResult(source);
      return;
    } catch (redirectError) {
      console.error("Vesta update check: All strategies failed:", redirectError);
      updateStatus = "error";
      updateError = $currentLanguage === "it" ? "Impossibile controllare gli aggiornamenti" : "Could not check for updates";
      if (source === "manual") {
        snackbar.show(updateError, "error");
      }
    }
  }

  function processUpdateResult(source: "auto" | "manual") {
    if (appVersionNum) {
      if (compareVersions(latestVersion, appVersionNum) > 0) {
        updateStatus = "available";
        if (source === "manual") {
          snackbar.show(($currentLanguage === "it" ? "Nuova versione disponibile: " : "New version available: ") + latestVersion, "info");
        }
      } else {
        updateStatus = "current";
        if (source === "manual") {
          snackbar.show($currentLanguage === "it" ? "Il software è aggiornato" : "Software is up to date", "success");
        }
      }
    } else {
      updateStatus = "current";
    }
  }

  function toggleAutomaticUpdateChecks() {
    automaticUpdateChecks = !automaticUpdateChecks;
    localStorage.setItem("vesta-automatic-update-checks", automaticUpdateChecks.toString());

    if (automaticUpdateChecks) {
      void checkForUpdates("manual");
    } else {
      updateStatus = "disabled";
    }
  }

  onMount(() => {
    invoke<number>("flashcard_get_cpu_count").then((count) => {
      systemCpuCount = count;
      const savedCores = localStorage.getItem("vesta_cpu_cores");
      if (savedCores) {
        cpuCores = parseInt(savedCores);
      } else {
        cpuCores = Math.max(2, systemCpuCount - 1);
      }
    }).catch(() => {
      systemCpuCount = 4;
      const savedCores = localStorage.getItem("vesta_cpu_cores");
      if (savedCores) {
        cpuCores = parseInt(savedCores);
      } else {
        cpuCores = Math.max(2, systemCpuCount - 1);
      }
    });

    loadApiKeys();
    const savedAutoCheck = localStorage.getItem("vesta-automatic-update-checks");
    automaticUpdateChecks = savedAutoCheck !== "false";

    invoke<{ version: string }>("get_app_info")
      .then((info) => {
        appVersionNum = `v${info.version}`;
        if (automaticUpdateChecks) {
          void checkForUpdates("auto");
        } else {
          updateStatus = "disabled";
        }
      })
      .catch(() => {
        appVersionNum = "v0.13.0";
        if (automaticUpdateChecks) {
          void checkForUpdates("auto");
        } else {
          updateStatus = "disabled";
        }
      });
    smartMatchingRulesDraft = formatSmartMatchingRules(smartMatchingStore.rules);
    defaultWhisperModel = localStorage.getItem("srt-default-whisper-model") || "base";

    refreshModels().catch((e) => console.error("Could not list models:", e));

    const handleKeydown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (deleteConfirmId) {
          cancelDelete();
        } else if (showAddKey) {
          showAddKey = false;
        }
      }
    };

    const handleOpenAddKeyModalEvent = () => {
      openAddKeyModal("custom");
    };

    const handleDownloadWhisperModelEvent = (e: Event) => {
      const customEvent = e as CustomEvent<{ modelId: string }>;
      const modelId = customEvent.detail?.modelId;
      if (modelId) {
        void downloadModel(modelId, true);
      }
    };

    const handleUninstallWhisperModelEvent = (e: Event) => {
      const customEvent = e as CustomEvent<{ modelId: string }>;
      const modelId = customEvent.detail?.modelId;
      if (modelId) {
        void uninstallModel(modelId);
      }
    };

    tiers = loadTiers();
    transcribeTiers = loadTranscribeTiers();
    window.addEventListener("keydown", handleKeydown);
    window.addEventListener("vesta-open-settings-section", handleOpenSettingsSectionEvent);
    window.addEventListener("vesta-open-add-key-modal", handleOpenAddKeyModalEvent);
    window.addEventListener(CARD_TEMPLATES_UPDATED_EVENT, syncTemplateStateFromStorage);
    window.addEventListener(FIELD_NAMES_UPDATED_EVENT, syncFieldStateFromStorage);
    window.addEventListener(TIERS_UPDATED_EVENT, syncTiersFromStorage);
    window.addEventListener(TRANSCRIBE_TIERS_UPDATED_EVENT, syncTranscribeTiersFromStorage);
    window.addEventListener("vesta-download-whisper-model", handleDownloadWhisperModelEvent);
    window.addEventListener("vesta-uninstall-whisper-model", handleUninstallWhisperModelEvent);

    let activeListener = true;
    let unlistenProg: (() => void) | null = null;

    listen<{
      stage: string;
      message: string;
      percentage: number;
    }>("transcribe-progress", (event) => {
      const p = event.payload;
      progress = Math.round(p.percentage);
      progressMessage = p.message;
      progressStage = p.stage;
      window.dispatchEvent(new CustomEvent("vesta-whisper-download-progress", {
        detail: {
          modelId: downloadingModelId,
          progress: Math.round(p.percentage),
          stage: p.stage,
          message: p.message
        }
      }));
    }).then((fn) => {
      if (!activeListener) fn();
      else unlistenProg = fn;
    }).catch(console.error);

    return () => {
      activeListener = false;
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener("vesta-open-settings-section", handleOpenSettingsSectionEvent);
      window.removeEventListener("vesta-open-add-key-modal", handleOpenAddKeyModalEvent);
      window.removeEventListener(CARD_TEMPLATES_UPDATED_EVENT, syncTemplateStateFromStorage);
      window.removeEventListener(FIELD_NAMES_UPDATED_EVENT, syncFieldStateFromStorage);
      window.removeEventListener(TIERS_UPDATED_EVENT, syncTiersFromStorage);
      window.removeEventListener(TRANSCRIBE_TIERS_UPDATED_EVENT, syncTranscribeTiersFromStorage);
      window.removeEventListener("vesta-download-whisper-model", handleDownloadWhisperModelEvent);
      window.removeEventListener("vesta-uninstall-whisper-model", handleUninstallWhisperModelEvent);
      if (unlistenProg) unlistenProg();
    };
  });

  function syncTiersFromStorage() {
    tiers = loadTiers();
  }

  function syncTranscribeTiersFromStorage() {
    transcribeTiers = loadTranscribeTiers();
  }

  // Whisper Model Management
  let whisperModels = $state<{
    id: string;
    name: string;
    size: string;
    speed: string;
    downloaded: boolean;
  }[]>([
    { id: "tiny", name: "Tiny", size: "~75MB", speed: "~32x", downloaded: false },
    { id: "base", name: "Base", size: "~150MB", speed: "~16x", downloaded: false },
    { id: "small", name: "Small", size: "~500MB", speed: "~6x", downloaded: false },
    { id: "medium", name: "Medium", size: "~1.5GB", speed: "~2x", downloaded: false },
    { id: "large", name: "Large", size: "~3GB", speed: "~1x", downloaded: false },
  ]);
  let downloadedWhisperCount = $derived(whisperModels.filter((model) => model.downloaded).length);
  let needsQuickSetup = $derived(downloadedWhisperCount === 0 || !isDefaultLlmReady);

  let isDownloading = $state(false);
  let isCancellingDownload = $state(false);
  let downloadingModelId = $state<string | null>(null);
  let pendingDefaultModelId = $state<string | null>(null);
  let progress = $state(0);
  let progressMessage = $state("");
  let progressStage = $state("");
  let defaultWhisperModel = $state("base");
  let highlightedModelId = $state<string | null>(null);

  $effect(() => {
    if (highlightItemId) {
      highlightedModelId = highlightItemId;
      const targetId = highlightItemId;
      
      // Scroll to the element if it exists in the DOM
      setTimeout(() => {
        if (targetId) {
          const el = document.getElementById(targetId);
          if (el) {
            el.scrollIntoView({ behavior: 'smooth', block: 'center' });
            if (el.tagName === 'TEXTAREA' || el.tagName === 'INPUT') {
              el.focus();
            }
          }
        }
      }, 100);

      const timer = setTimeout(() => {
        highlightedModelId = null;
        highlightItemId = null;
      }, 2000);
      return () => clearTimeout(timer);
    }
  });

  function formatWhisperModelName(modelId: string): string {
    const matchedModel = whisperModels.find((model) => model.id === modelId);
    if (matchedModel?.name) return matchedModel.name;
    return modelId ? modelId.charAt(0).toUpperCase() + modelId.slice(1) : "";
  }

  function whisperModelIconPath(modelId: string): string {
    const paths: Record<string, string> = {
      tiny: "M13 3L4 14h7l-1 7 9-12h-7l1-6z",
      base: "M12 4a8 8 0 100 16 8 8 0 000-16zm0 3v5l3 2",
      small: "M6 20V10m6 10V4m6 16v-7M4 10h4m2-6h4m2 9h4",
      medium: "M4 13h3l2-6 4 12 2-6h5",
      large: "M12 3l8 4-8 4-8-4 8-4zm-8 8l8 4 8-4M4 15l8 4 8-4",
    };
    return paths[modelId] || "M9 3h6m-7 4h8a3 3 0 013 3v7a3 3 0 01-3 3H8a3 3 0 01-3-3v-7a3 3 0 013-3zm4 3v4m-2-2h4";
  }

  function whisperModelAccent(modelId: string): string {
    const accents: Record<string, string> = {
      tiny: "from-amber-500/25 to-yellow-500/10 text-amber-200",
      base: "from-sky-500/25 to-cyan-500/10 text-sky-200",
      small: "from-emerald-500/25 to-teal-500/10 text-emerald-200",
      medium: "from-indigo-500/25 to-violet-500/10 text-indigo-200",
      large: "from-fuchsia-500/25 to-rose-500/10 text-fuchsia-200",
    };
    return accents[modelId] || "from-cyan-500/20 to-blue-500/10 text-cyan-200";
  }

  $effect(() => {
    if (typeof window === "undefined") return;
    publishSettingsActionState(
      buildSettingsActionHash({
        needsWhisper: downloadedWhisperCount === 0 && !aiStore.killSwitchActive,
        needsLlm: !isDefaultLlmReady && !aiStore.killSwitchActive,
      }),
    );
  });

  $effect(() => {
    if (aiStore.killSwitchActive) {
      if (activeSettingsSection === "llm" || activeSettingsSection === "whisper") {
        activeSettingsSection = "overview";
      }
      if (requestedSection === "llm" || requestedSection === "whisper") {
        requestedSection = "overview";
      }
    }
  });




  function setDefaultWhisperModel(modelId: string, notify = true) {
    defaultWhisperModel = modelId;
    localStorage.setItem("srt-default-whisper-model", modelId);
    if (notify) {
      showSnackbar(t("settings.whisper.defaultSet", { model: modelId }));
    }
    // Dispatch event so other tabs can pick up the change if needed
    window.dispatchEvent(new CustomEvent("whisper-model-updated", { detail: modelId }));
  }

  function handleWhisperModelClick(model: { id: string; downloaded: boolean }) {
    if (model.downloaded) {
      setDefaultWhisperModel(model.id);
      return;
    }
    void downloadModel(model.id, true);
  }

  async function refreshModels() {
    try {
      const models = await invoke<typeof whisperModels>("transcribe_list_models");
      whisperModels = models;
      window.dispatchEvent(new CustomEvent("vesta-whisper-models-updated", { detail: { models } }));
    } catch (e) {
      console.error("Could not list models:", e);
    }
  }

  async function downloadModel(modelId: string, setAsDefaultAfterDownload = false) {
    if (isDownloading) return;
    isDownloading = true;
    isCancellingDownload = false;
    downloadingModelId = modelId;
    pendingDefaultModelId = setAsDefaultAfterDownload ? modelId : null;
    try {
      await invoke<boolean>("transcribe_download_model", { modelId });
      await refreshModels();

      const downloaded = whisperModels.find((m) => m.id === modelId)?.downloaded;
      if (downloaded && pendingDefaultModelId === modelId) {
        setDefaultWhisperModel(modelId, false);
        showSnackbar(t("settings.whisper.downloadAndSetSuccess", { model: modelId }));
      } else if (downloaded) {
        showSnackbar(t("settings.whisper.downloadSuccess", { model: modelId }));
      }
    } catch (e) {
      const message = String(e).toLowerCase();
      if (message.includes("cancelled") || message.includes("canceled")) {
        showSnackbar(
          t("settings.modelDownloadCancelled", { model: modelId }) || `Download cancelled for model ${modelId}`,
        );
      } else {
        showSnackbar(t("settings.whisper.downloadFailed", { model: modelId, error: String(e) }), "error");
      }
    } finally {
      isDownloading = false;
      isCancellingDownload = false;
      downloadingModelId = null;
      pendingDefaultModelId = null;
      progress = 0;
      progressMessage = "";
      progressStage = "";
      window.dispatchEvent(new CustomEvent("vesta-whisper-download-progress", {
        detail: { modelId: null, progress: 0 }
      }));
    }
  }

  async function cancelModelDownload() {
    if (!isDownloading || isCancellingDownload) return;
    isCancellingDownload = true;
    try {
      await invoke("transcribe_cancel");
    } catch (e) {
      showSnackbar(t("settings.whisper.cancelFailed", { error: String(e) }), "error");
      isCancellingDownload = false;
    }
  }

  async function uninstallModel(modelId: string) {
    if (isDownloading) return;
    try {
      await invoke<boolean>("transcribe_uninstall_model", { modelId });
      showSnackbar(t("settings.whisper.uninstallSuccess", { model: modelId }));
      await refreshModels();
    } catch (e) {
      showSnackbar(t("settings.whisper.uninstallFailed", { model: modelId, error: String(e) }), "error");
    }
  }

  let contextMenu = $state<{
    x: number;
    y: number;
    kind: "model" | "panel";
    modelId?: string;
    downloaded?: boolean;
  } | null>(null);

  function openContextMenu(e: MouseEvent, model: { id: string; downloaded: boolean }) {
    e.preventDefault();
    e.stopPropagation();
    contextMenu = {
      x: e.clientX,
      y: e.clientY,
      kind: "model",
      modelId: model.id,
      downloaded: model.downloaded,
    };
  }

  function openWhisperPanelContextMenu(e: MouseEvent) {
    e.preventDefault();
    contextMenu = {
      x: e.clientX,
      y: e.clientY,
      kind: "panel",
    };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function handleModelDblClick(model: { id: string; downloaded: boolean }) {
    if (!model.downloaded && !isDownloading) {
      void downloadModel(model.id, true);
    } else if (model.downloaded) {
      setDefaultWhisperModel(model.id);
    }
  }

  function syncTemplateStateFromStorage() {
    const templates = loadCardTemplates();
    noteTypeName = templates.noteTypeName;
  }

  function syncFieldStateFromStorage() {
    const fieldNames = loadFieldNames();
    fieldExpression = fieldNames.expression;
    fieldMeaning = fieldNames.meaning;
    fieldReading = fieldNames.reading;
    fieldAudio = fieldNames.audio;
    fieldSnapshot = fieldNames.snapshot;
    fieldVideo = fieldNames.video;
    fieldTags = fieldNames.tags;
    fieldSequenceMarker = fieldNames.sequenceMarker;
    fieldNotes = fieldNames.notes;
  }




  function loadApiKeys() {
    apiKeys = loadAndValidateApiKeys();
  }

  function saveApiKeys() {
    localStorage.setItem("srt-tools-api-keys", JSON.stringify(apiKeys));
    // Dispatch custom event to notify other tabs in the same window
    window.dispatchEvent(new CustomEvent("apikeys-updated"));
  }

  function generateId(): string {
    return Date.now().toString(36) + Math.random().toString(36).substr(2);
  }

  function openAddKeyModal(providerId?: string) {
    editKeyId = null;
    modalContext = activeSettingsSection === "whisper" ? "whisper" : "llm";
    const allowedIds = modalContext === "whisper" ? whisperProviderIds : llmProviderIds;
    const defaultProviderId = modalContext === "whisper" ? "groq" : "google";
    
    const normalizedProviderId =
      providerId && allowedIds.includes(providerId) ? providerId : defaultProviderId;
      
    if (normalizedProviderId) {
      newKeyType = normalizedProviderId as ApiKeyConfig["apiType"];
      newKeyName = normalizedProviderId === "openai" ? "Open AI" : (providers[normalizedProviderId]?.name || "");
    }
    newKeyValue = "";
    newKeyUrl =
      newKeyType === "local" ? providers.local.defaultApiUrl || "" : "";
    if (newKeyType === "custom") newKeyName = "";
    showAddKey = true;
  }

  function openEditKeyModal(id: string) {
    const key = apiKeys.find((k) => k.id === id);
    if (!key) return;
    editKeyId = id;
    modalContext = activeSettingsSection === "whisper" ? "whisper" : "llm";
    newKeyType = key.apiType;
    newKeyName = key.name;
    newKeyValue = key.apiKey;
    newKeyUrl = key.apiUrl || "";
    showNewKeyPassword = false;
    showAddKey = true;
  }

  function addApiKey() {
    if (!newKeyName.trim()) {
      showSnackbar(t("settings.errorNameRequired"), "error");
      return;
    }

    if (
      (newKeyType === "local" || newKeyType === "custom") &&
      !newKeyUrl.trim()
    ) {
      showSnackbar(t("settings.errorEndpointRequired"), "error");
      return;
    }

    if (
      newKeyType !== "local" &&
      newKeyType !== "custom" &&
      !newKeyValue.trim()
    ) {
      showSnackbar(t("settings.errorKeyRequired"), "error");
      return;
    }

    if (newKeyType === "google" && !newKeyValue.trim().startsWith("AIza")) {
      showSnackbar(t("settings.errorInvalidGoogleKey"), "error");
      return;
    }

    if (newKeyType === "groq" && !newKeyValue.trim().startsWith("gsk_")) {
      showSnackbar(t("settings.errorInvalidGroqKey"), "error");
      return;
    }

    // Enforce unique names for custom providers
    if (newKeyType === "custom" && !editKeyId) {
      const existingCustom = apiKeys.find(
        (k) => k.apiType === "custom" && k.name.trim().toLowerCase() === newKeyName.trim().toLowerCase()
      );
      if (existingCustom) {
        showSnackbar(t("settings.errorDuplicateCustomName"), "error");
        return;
      }
    }

    // Auto-set API URL for known providers (custom/local keep the typed URL)
    let resolvedUrl = newKeyUrl.trim() || undefined;
    if (newKeyType !== "custom" && newKeyType !== "local") {
      resolvedUrl = providers[newKeyType]?.defaultApiUrl || resolvedUrl;
    }

    if (editKeyId) {
      // Edit existing key
      apiKeys = apiKeys.map((k) =>
        k.id === editKeyId
          ? {
              ...k,
              name: newKeyName.trim(),
              apiType: newKeyType,
              apiKey: newKeyValue.trim(),
              apiUrl: resolvedUrl,
            }
          : k,
      );
      saveApiKeys();
      if (newKeyType === "custom" && defaultLlmCustomProviderId === editKeyId) {
        void refreshDefaultEndpointModels();
      }
      showSnackbar(t("settings.keyUpdated"));
    } else {
      // Add new key
      const newKey: ApiKeyConfig = {
        id: generateId(),
        name: newKeyName.trim(),
        apiType: newKeyType,
        apiKey: newKeyValue.trim(),
        apiUrl: resolvedUrl,
        isDefault: apiKeys.filter((k) => k.apiType === newKeyType).length === 0,
      };
      apiKeys = [...apiKeys, newKey];
      saveApiKeys();
      showSnackbar(t("settings.keyAdded"));
    }

    newKeyName = "";
    newKeyValue = "";
    newKeyUrl = "";
    editKeyId = null;
    showAddKey = false;
  }

  let deleteConfirmId = $state<string | null>(null);
  let deleteConfirmName = $state<string>("");

  function askDeleteApiKey(id: string) {
    const key = apiKeys.find((k) => k.id === id);
    if (!key) return;
    deleteConfirmId = id;
    deleteConfirmName = key.name;
  }

  function cancelDelete() {
    deleteConfirmId = null;
    deleteConfirmName = "";
  }

  function confirmDeleteApiKey() {
    if (!deleteConfirmId) return;

    const key = apiKeys.find((k) => k.id === deleteConfirmId);
    if (!key) {
      cancelDelete();
      return;
    }

    const wasDefault = key.isDefault;
    const keyType = key.apiType;
    apiKeys = apiKeys.filter((k) => k.id !== deleteConfirmId);

    if (wasDefault) {
      const sameTypeKeys = apiKeys.filter((k) => k.apiType === keyType);
      if (sameTypeKeys.length > 0) {
        sameTypeKeys[0].isDefault = true;
      }
    }

    saveApiKeys();
    showSnackbar(t("settings.keyDeleted"));
    cancelDelete();
  }

  function setDefaultKey(id: string) {
    const key = apiKeys.find((k) => k.id === id);
    if (!key) return;

    apiKeys = apiKeys.map((k) => ({
      ...k,
      isDefault: k.apiType === key.apiType ? k.id === id : k.isDefault,
    }));
    saveApiKeys();
  }

  let visibleKeyIds = $state<Set<string>>(new Set());

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    snackbar.show(t("settings.keyCopied"), "success", 1300);
  }

  function copyApiKey(key: string) {
    copyToClipboard(key);
  }

  function toggleKeyVisibility(keyId: string) {
    const newSet = new Set(visibleKeyIds);
    if (newSet.has(keyId)) {
      newSet.delete(keyId);
    } else {
      newSet.add(keyId);
    }
    visibleKeyIds = newSet;
  }

  function maskApiKey(key: string): string {
    if (!key || key.length <= 8) return "••••••••";
    return key.substring(0, 4) + "••••" + key.substring(key.length - 4);
  }

  function formatApiKeyForDisplay(key: string, isVisible: boolean): string {
    if (!key) return "—";
    if (isVisible) {
      return key
        .split("")
        .map((char) => {
          if (char === " ") return "␣"; // Space indicator
          if (char === "\t") return "→"; // Tab indicator
          if (char === "\n") return "↵"; // Newline indicator
          return char;
        })
        .join("");
    }
    return maskApiKey(key);
  }

  function hasSpecialChars(key: string): boolean {
    return /[\s\t\n]/.test(key);
  }

  function onModelClick(model: ModelInfo) {
    openAddKeyModal(model.provider);
  }
</script>

<div
  class="h-full flex flex-col bg-gray-900"
>
  {#if activeSettingsSection === 'shortcuts'}
    <div class="flex-1 flex flex-col min-h-0">
      <ShortcutsTab />
    </div>
  {:else}
    <!-- Scrollable content area -->
    <div class="flex-1 overflow-y-auto p-6">


  {#if activeSettingsSection === "overview"}
    <div
      class="glass-card p-6 mb-6 flex flex-col gap-5 shrink-0"
    >
      <div class="ui-language-grid w-full">
        {#each availableUILanguages as lang}
          <button
            onclick={() => setLanguage(lang.code)}
            class="ui-language-button flex items-center gap-3 px-4 py-3 rounded-lg transition-all duration-200 border text-left min-w-0
              {$currentLanguage === lang.code
              ? 'bg-gradient-to-r from-indigo-500/20 to-purple-500/20 border-indigo-500/50 text-white shadow-sm'
              : 'bg-white/5 hover:bg-white/10 text-gray-400 hover:text-gray-200 border-transparent hover:border-white/10'}"
          >
            <span class="text-2xl leading-none shrink-0">{lang.flag}</span>
            <span class="min-w-0 flex flex-col leading-tight">
              <span class="block truncate text-sm font-bold text-white">{lang.name}</span>
              <span class="block truncate text-[11px] font-medium text-gray-400 opacity-80">{lang.nativeName}</span>
            </span>
          </button>
        {/each}
      </div>
    </div>

    {#if uiMode.easyMode}
      <div class="mb-6">
        {@render defaultLanguagesCard()}
      </div>
    {/if}

    <!-- Export Format Card -->
    {#if uiMode.expertMode}
      <div class="glass-card p-6 mb-6">
        <div class="flex items-center gap-3 mb-4">
          <div class="w-9 h-9 rounded-lg bg-sky-500/15 text-sky-300 flex items-center justify-center shrink-0">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
              />
            </svg>
          </div>
          <div>
            <h3 class="text-sm font-bold text-white">{t("settings.anki.exportFormat")}</h3>
          </div>
        </div>
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <!-- APKG option -->
          <button
            type="button"
            onclick={() => (exportFormat = "apkg")}
            class="flex items-start gap-3 p-3.5 rounded-xl border text-left transition-all cursor-pointer
              {exportFormat === 'apkg'
                ? 'border-emerald-500/50 bg-emerald-500/10'
                : 'border-white/10 bg-white/5 hover:border-white/20 hover:bg-white/8'}"
          >
            <div class="mt-0.5 w-4 h-4 rounded-full border-2 flex items-center justify-center shrink-0
              {exportFormat === 'apkg' ? 'border-emerald-400' : 'border-gray-500'}">
              {#if exportFormat === "apkg"}
                <div class="w-2 h-2 rounded-full bg-emerald-400"></div>
              {/if}
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="text-sm font-semibold text-white">{t("settings.anki.exportAPKG")}</span>
                <span class="text-[10px] px-1.5 py-0.5 rounded-full font-bold
                  {exportFormat === 'apkg'
                    ? 'bg-emerald-500/30 text-emerald-300 border border-emerald-500/40'
                    : 'bg-gray-700/60 text-gray-400 border border-gray-700'}">{t("flashcards.exportAPKGBadge")}</span>
              </div>
              <p class="text-xs text-gray-400 mt-1 leading-relaxed">{t("flashcards.exportAPKGDesc")}</p>
            </div>
          </button>

          <!-- TSV option -->
          <button
            type="button"
            onclick={() => (exportFormat = "tsv")}
            class="flex items-start gap-3 p-3.5 rounded-xl border text-left transition-all cursor-pointer
              {exportFormat === 'tsv'
                ? 'border-sky-500/50 bg-sky-500/10'
                : 'border-white/10 bg-white/5 hover:border-white/20 hover:bg-white/8'}"
          >
            <div class="mt-0.5 w-4 h-4 rounded-full border-2 flex items-center justify-center shrink-0
              {exportFormat === 'tsv' ? 'border-sky-400' : 'border-gray-500'}">
              {#if exportFormat === "tsv"}
                <div class="w-2 h-2 rounded-full bg-sky-400"></div>
              {/if}
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="text-sm font-semibold text-white">{t("settings.anki.exportTSV")}</span>
                <span class="text-[10px] px-1.5 py-0.5 rounded-full font-bold
                  {exportFormat === 'tsv'
                    ? 'bg-sky-500/30 text-sky-300 border border-sky-500/40'
                    : 'bg-gray-700/60 text-gray-400 border border-gray-700'}">{t("flashcards.exportTSVBadge")}</span>
              </div>
              <p class="text-xs text-gray-400 mt-1 leading-relaxed">{t("flashcards.exportTSVDesc")}</p>
            </div>
          </button>
        </div>
      </div>
    {/if}

    <div class="{uiMode.expertMode ? 'grid grid-cols-1 lg:grid-cols-2 gap-6' : 'block'} mb-6 items-stretch">
      <!-- CPU Cores Card -->
      {#if uiMode.expertMode}
        <div class="glass-card p-6 flex flex-col justify-between h-full">
          <div>
            <div class="flex items-center gap-3 mb-4">
              <div class="w-9 h-9 rounded-lg bg-orange-500/20 text-orange-300 flex items-center justify-center shrink-0">
                <svg
                  class="w-5 h-5"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"
                  />
                </svg>
              </div>
              <div>
                <h3 class="text-sm font-bold text-white">{t("flashcards.cpuCores")}</h3>
              </div>
            </div>
            <div class="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-2 xl:grid-cols-4 gap-2.5 mb-4">
              <button
                onclick={() => setCpuPreset("eco")}
                class="p-3 rounded-xl text-center transition-all duration-200 border text-xs cursor-pointer {activeCpuPreset ===
                'eco'
                  ? 'bg-orange-500/20 border-orange-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
              >
                <span class="block mb-1 text-white">
                  <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M5 14c0-5.523 4.477-10 10-10h4v4c0 5.523-4.477 10-10 10H5v-4z" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M7 17c2.5-2.5 5.5-4.5 9-6" />
                  </svg>
                </span>
                <span class="font-semibold block">{t("flashcards.cpuEco")}</span>
              </button>
              <button
                onclick={() => setCpuPreset("balanced")}
                class="p-3 rounded-xl text-center transition-all duration-200 border text-xs cursor-pointer {activeCpuPreset ===
                'balanced'
                  ? 'bg-orange-500/20 border-orange-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
              >
                <span class="block mb-1 text-white">
                  <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 4v16" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M6 7h12" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M8 7l-3 5h6L8 7z" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M16 7l-3 5h6l-3-5z" />
                  </svg>
                </span>
                <span class="font-semibold block"
                  >{t("flashcards.cpuBalanced")}</span
                >
              </button>
              <button
                onclick={() => setCpuPreset("performance")}
                class="p-3 rounded-xl text-center transition-all duration-200 border text-xs cursor-pointer {activeCpuPreset ===
                'performance'
                  ? 'bg-orange-500/20 border-orange-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
              >
                <span class="block mb-1 text-white">
                  <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M5 16l5-5 3 3 6-7" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M14 7h5v5" />
                  </svg>
                </span>
                <span class="font-semibold block"
                  >{t("flashcards.cpuPerformance")}</span
                >
              </button>
              <button
                onclick={() => setCpuPreset("full")}
                class="p-3 rounded-xl text-center transition-all duration-200 border text-xs cursor-pointer {activeCpuPreset ===
                'full'
                  ? 'bg-orange-500/20 border-orange-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
              >
                <span class="block mb-1 text-white">
                  <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M11 3L6 13h5l-1 8 8-12h-5l2-6h-4z" />
                  </svg>
                </span>
                <span class="font-semibold block"
                  >{t("flashcards.cpuFullPower")}</span
                >
              </button>
            </div>
          </div>
          <div class="flex items-center justify-between text-sm px-1 mt-auto pt-4 border-t border-white/5">
            <span class="text-gray-400">{t("flashcards.cpuCoresUsage")}</span>
            <span
              class="text-white font-mono bg-white/10 px-2.5 py-1 rounded-lg text-sm"
              >{cpuCores} / {systemCpuCount}</span
            >
          </div>
        </div>
      {/if}

      <!-- Aggiornamenti Card -->
      <div class="glass-card p-6 flex flex-col justify-between h-full">
        <div>
          <div class="flex items-center gap-3 mb-4">
            <div class="w-9 h-9 rounded-lg bg-indigo-500/20 text-indigo-300 flex items-center justify-center">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
            </div>
            <div>
              <h3 class="text-sm font-bold text-white">{$currentLanguage === 'it' ? 'Aggiornamenti' : 'Updates'}</h3>
            </div>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mt-4">
            <button
              type="button"
              onclick={toggleAutomaticUpdateChecks}
              class="rounded-xl border p-4 text-left transition-all duration-200 flex flex-row items-center justify-between gap-3 cursor-pointer
                {automaticUpdateChecks
                  ? 'bg-indigo-500/10 border-indigo-500/30 text-white hover:bg-indigo-500/15 hover:border-indigo-500/40 shadow-md shadow-indigo-500/5'
                  : 'bg-white/5 border-white/10 text-gray-400 hover:text-white hover:bg-white/10 hover:border-white/20'}"
            >
              <span class="text-xs font-semibold">
                {$currentLanguage === 'it' ? 'Controlli automatici' : 'Automatic checks'}
              </span>
              {#if automaticUpdateChecks}
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5 text-indigo-400 shrink-0">
                  <path fill-rule="evenodd" d="M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75 9.75S2.25 17.385 2.25 12zm13.36-1.814a.75.75 0 10-1.22-.872l-3.236 4.53L9.53 12.22a.75.75 0 00-1.06 1.06l2.25 2.25a.75.75 0 001.14-.094l3.75-5.25z" clip-rule="evenodd" />
                </svg>
              {:else}
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-5 h-5 text-gray-500 shrink-0">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              {/if}
            </button>

            {#if updateStatus === "available"}
              <a
                href={releaseUrl}
                target="_blank"
                class="rounded-xl border border-amber-500/40 bg-amber-500/15 p-4 text-left transition-all duration-200 hover:border-amber-500/60 hover:bg-amber-500/25 active:scale-[0.98] flex flex-row items-center justify-between gap-3 cursor-pointer shadow-md shadow-amber-900/20"
              >
                <span class="text-xs font-bold text-amber-200 flex items-center gap-2">
                  <span class="w-2 h-2 rounded-full bg-amber-400 animate-pulse ring-4 ring-amber-500/20 shrink-0"></span>
                  {$currentLanguage === 'it' ? `Aggiorna a ${latestVersion}` : `Update to ${latestVersion}`}
                </span>
                <svg class="w-5 h-5 text-amber-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                </svg>
              </a>
            {:else}
              <button
                type="button"
                onclick={() => checkForUpdates("manual")}
                disabled={updateStatus === 'checking'}
                class="rounded-xl border border-white/10 bg-white/5 p-4 text-left transition-all duration-200 hover:border-white/20 hover:bg-white/10 active:scale-[0.98] disabled:opacity-60 flex flex-row items-center justify-between gap-3 cursor-pointer"
              >
                <span class="text-xs font-semibold text-white">
                  {updateStatus === 'checking'
                    ? ($currentLanguage === 'it' ? 'Controllo in corso…' : 'Checking…')
                    : updateStatus === 'current'
                      ? ($currentLanguage === 'it' ? 'Aggiornato ✓' : 'Up to date ✓')
                      : ($currentLanguage === 'it' ? 'Verifica ora' : 'Check now')}
                </span>
                {#if updateStatus === 'checking'}
                  <svg class="w-5 h-5 text-indigo-400 shrink-0 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                  </svg>
                {:else}
                  <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5 text-gray-400 shrink-0" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992m0 0v-.001M21.015 9.348l-3.181-3.182a8.25 8.25 0 00-13.803 3.7M7.977 14.652H2.985m0 0v.001m0-.001l3.181 3.182a8.25 8.25 0 0013.803-3.7" />
                  </svg>
                {/if}
              </button>
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/if}

  {#if activeSettingsSection === "language"}
  <div class="mb-6 flex flex-col gap-4">
    {@render defaultLanguagesCard()}

    <!-- Smart Matching Card -->
    {#if uiMode.expertMode}
      <div class="glass-card p-6 flex flex-col gap-4">
        <div class="flex items-center justify-between gap-4">
          <div class="flex items-center gap-3">
            <div class="w-9 h-9 rounded-lg bg-violet-500/20 text-violet-300 flex items-center justify-center">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
            </div>
            <div>
              <h3 class="text-sm font-bold text-white">Smart Matching</h3>
            </div>
          </div>
          <!-- Toggle Switch -->
          <button
            type="button"
            class="relative h-6 w-11 shrink-0 rounded-full transition-colors {smartMatchingEnabled ? 'bg-violet-500/60' : 'bg-gray-700'}"
            onclick={toggleSmartMatching}
            role="switch"
            aria-checked={smartMatchingEnabled}
            aria-label="Attiva/disattiva smart matching"
          >
            <span
              class="absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition-transform {smartMatchingEnabled ? 'translate-x-5' : 'translate-x-0'}"
            ></span>
          </button>
        </div>

        {#if smartMatchingEnabled}
          <!-- Rules Editor -->
          <div class="mt-4 pt-4 border-t border-white/5 space-y-3">
            <div class="relative">
              <CodeEditor
                bind:value={smartMatchingRulesDraft}
                language="jsonc"
                heightClass="h-[520px]"
                onchange={saveSmartMatchingRules}
              />
            </div>
            {#if smartMatchingRulesError}
              <p class="mt-2 rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-200">
                {smartMatchingRulesError}
              </p>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </div>
  {/if}

  {#snippet defaultLanguagesCard()}
    <div class="glass-card p-6">
      <div class="flex items-center gap-3 mb-4">
        <div class="w-9 h-9 rounded-lg bg-emerald-500/20 text-emerald-300 flex items-center justify-center">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1 9a18 18 0 01-4-5m7 12l5-10 5 10m-9-4h8" />
          </svg>
        </div>
        <div>
          <h3 class="text-sm font-bold text-white">{s("defaultLanguages")}</h3>
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 2xl:grid-cols-4 gap-5">
        <div class="rounded-xl border border-white/10 bg-white/5 p-5">
          <div class="flex items-center justify-between gap-3 mb-4">
            <span class="block text-sm font-semibold text-white">{s("studyingLanguage")}</span>
            <span class="text-4xl">{languages.find((lang) => lang.code === defaultFlashcardsLanguage)?.flag || "🌐"}</span>
          </div>
          <SearchableSelect
            className="language-select"
            noResultsText={t("common.noResults")}
            options={defaultWorkflowLanguageOptions}
            value={defaultFlashcardsLanguage}
            onchange={(v) => {
              defaultFlashcardsLanguage = v;
              saveDefaultLanguage(DEFAULT_FLASHCARDS_LANGUAGE_KEY, v);
              saveDefaultLanguage(NOTE_TYPE_LANGUAGE_KEY, v);
            }}
            placeholder={t("flashcards.noteTypeLanguagePlaceholder")}
          />
          <p class="mt-3 text-xs leading-relaxed text-gray-500">{s("studyingLanguageHint")}</p>
        </div>
        <div class="rounded-xl border border-white/10 bg-white/5 p-5">
          <div class="flex items-center justify-between gap-3 mb-4">
            <span class="block text-sm font-semibold text-white">{s("nativeLanguage")}</span>
            <span class="text-4xl">{languages.find((lang) => lang.code === defaultNativeLanguage)?.flag || "🌐"}</span>
          </div>
          <SearchableSelect
            className="language-select"
            noResultsText={t("common.noResults")}
            options={defaultWorkflowLanguageOptions}
            value={defaultNativeLanguage}
            onchange={(v) => {
              defaultNativeLanguage = v;
              saveDefaultLanguage(DEFAULT_NATIVE_LANGUAGE_KEY, v);
            }}
            placeholder={t("flashcards.noteTypeLanguagePlaceholder")}
          />
          <p class="mt-3 text-xs leading-relaxed text-gray-500">{s("nativeLanguageHint")}</p>
        </div>
        <div class="rounded-xl border border-white/10 bg-white/5 p-5">
          <div class="flex items-center justify-between gap-3 mb-4">
            <span class="block text-sm font-semibold text-white">{s("translationLanguage")}</span>
            <span class="text-4xl">{languages.find((lang) => lang.code === defaultTargetLanguage)?.flag || "🌐"}</span>
          </div>
          <SearchableSelect
            className="language-select"
            noResultsText={t("common.noResults")}
            options={defaultWorkflowLanguageOptions}
            value={defaultTargetLanguage}
            onchange={(v) => {
              defaultTargetLanguage = v;
              saveDefaultLanguage(DEFAULT_TARGET_LANGUAGE_KEY, v);
            }}
            placeholder={t("translate.targetLang")}
          />
          <p class="mt-3 text-xs leading-relaxed text-gray-500">{s("translationLanguageHint")}</p>
        </div>
        <div class="rounded-xl border border-white/10 bg-white/5 p-5">
          <div class="flex items-center justify-between gap-3 mb-4">
            <span class="block text-sm font-semibold text-white">{s("transcription")}</span>
            <span class="text-4xl">{defaultTranscribeLanguage === "auto" ? "🌐" : languages.find((lang) => lang.code === defaultTranscribeLanguage)?.flag || "🌐"}</span>
          </div>
          <SearchableSelect
            className="language-select"
            noResultsText={t("common.noResults")}
            options={[
              { value: "auto", label: t("transcribe.autoDetect"), icon: "🌐", searchTerms: "auto detect" },
              ...defaultWorkflowLanguageOptions,
            ]}
            value={defaultTranscribeLanguage}
            onchange={(v) => {
              defaultTranscribeLanguage = v;
              saveDefaultLanguage(DEFAULT_TRANSCRIBE_LANGUAGE_KEY, v);
            }}
            placeholder={t("transcribe.sourceLanguage")}
          />
          <p class="mt-3 text-xs leading-relaxed text-gray-500">{s("transcriptionLanguageHint")}</p>
        </div>
      </div>
    </div>
  {/snippet}



  {#snippet apiKeysCard(defaultProvider: string)}
    <div class="glass-card flex flex-col h-auto">
      <div class="p-4 border-b border-white/5 flex items-center justify-between gap-3 w-full">
        <div class="flex items-center gap-3">
          <div class="w-9 h-9 rounded-lg bg-violet-500/20 text-violet-300 flex items-center justify-center shrink-0">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m-2 4a2 2 0 012 2m-3-4a3 3 0 11-6 0 3 3 0 016 0zM8 21a4 4 0 014-4h4a4 4 0 014 4H8z" />
            </svg>
          </div>
          <div>
            <h3 class="text-sm font-bold text-white tracking-wide">
              API Keys
            </h3>
          </div>
        </div>
        <button
          type="button"
          onclick={() => openAddKeyModal(defaultProvider)}
          class="inline-flex items-center justify-center gap-2 rounded-lg bg-indigo-500 px-3.5 py-2 text-xs font-bold text-white shadow-lg shadow-indigo-500/20 hover:bg-indigo-400 transition-colors cursor-pointer"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
          </svg>
          {s("addProviderButton")}
        </button>
      </div>

      <div
        class="p-2 space-y-2 overflow-y-auto"
        style={apiKeys.length <= 1
          ? "height: 78px;"
          : apiKeys.length === 2
            ? "height: 164px;"
            : "height: 250px;"}
      >
        {#each apiKeys as key}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            role="button"
            tabindex="0"
            onclick={() => openEditKeyModal(key.id)}
            onkeydown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                openEditKeyModal(key.id);
              }
            }}
            class="p-3 rounded-xl border transition-all group cursor-pointer bg-white/[0.035] opacity-90 hover:opacity-100 hover:bg-white/[0.075] hover:border-white/30
              {key.isDefault
              ? 'ring-1 ring-indigo-500/50 border-indigo-400/30 bg-indigo-500/10'
              : 'border-white/10'}"
          >
            <div class="flex items-start gap-3">
              <ProviderIcon provider={key.apiType} />

              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 mb-0.5">
                  <span class="font-medium text-gray-200 text-sm truncate"
                    >{key.name}</span
                  >
                </div>
                <div class="flex items-center gap-1.5">
                  <button
                    onclick={(event) => {
                      event.stopPropagation();
                      toggleKeyVisibility(key.id);
                    }}
                    class="text-[10px] text-gray-500 font-mono truncate hover:text-gray-300 transition-colors flex items-center gap-1"
                    title={t("settings.toggleVisibility")}
                  >
                    {#if visibleKeyIds.has(key.id)}
                      <svg
                        class="w-3 h-3 flex-shrink-0"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"
                        />
                      </svg>
                    {:else}
                      <svg
                        class="w-3 h-3 flex-shrink-0"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                        />
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
                        />
                      </svg>
                    {/if}
                    <span class="truncate"
                      >{formatApiKeyForDisplay(
                        key.apiKey,
                        visibleKeyIds.has(key.id),
                      )}</span
                    >
                  </button>
                  <button
                    onclick={(event) => {
                      event.stopPropagation();
                      copyApiKey(key.apiKey);
                    }}
                    class="p-1 text-gray-500 hover:text-gray-300 transition-colors flex-shrink-0"
                    title={t("common.copy")}
                  >
                    <svg
                      class="w-3 h-3"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                      />
                    </svg>
                  </button>
                  {#if hasSpecialChars(key.apiKey)}
                    <span
                      class="text-[9px] bg-amber-500/20 text-amber-400 px-1 py-0.5 rounded flex-shrink-0"
                      title={t("settings.hasSpecialChars")}
                    >
                      ⚠
                    </span>
                  {/if}
                </div>
              </div>

              <div class="flex items-center gap-1.5">
                <button
                  onclick={(event) => {
                    event.stopPropagation();
                    if (!key.isDefault) setDefaultKey(key.id);
                  }}
                  class="p-1.5 rounded transition-colors {key.isDefault
                    ? 'text-indigo-300 bg-indigo-500/15 cursor-default'
                    : 'text-gray-500 hover:text-indigo-400 hover:bg-white/10'}"
                  title={key.isDefault ? t("settings.default") : t("settings.setAsDefault")}
                  aria-pressed={key.isDefault}
                >
                  <svg
                    class="w-4 h-4"
                    fill={key.isDefault ? "currentColor" : "none"}
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 5a2 2 0 012-2h10a2 2 0 012 2v4.38l1.71 1.71c.18.18.29.43.29.7V14a2 2 0 01-2 2h-5v4l-2 2-2-2v-4H5a2 2 0 01-2-2v-2.21c0-.27.11-.52.29-.71L5 9.38V5z"
                    />
                  </svg>
                </button>
                <button
                  onclick={(event) => {
                    event.stopPropagation();
                    openEditKeyModal(key.id);
                  }}
                  class="p-2.5 text-amber-400 hover:text-amber-300 hover:bg-amber-500/10 rounded-lg transition-colors"
                  title={t("settings.edit")}
                >
                  <svg
                    class="w-5 h-5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                    />
                  </svg>
                </button>
                <button
                  onclick={(event) => {
                    event.stopPropagation();
                    askDeleteApiKey(key.id);
                  }}
                  class="p-2.5 text-red-400 hover:text-red-300 hover:bg-red-500/10 rounded-lg transition-colors"
                  title={t("settings.delete")}
                >
                  <svg
                    class="w-5 h-5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                    />
                  </svg>
                </button>
              </div>
            </div>
          </div>
        {/each}

        {#if apiKeys.length === 0}
          <div
            class="h-full flex flex-col items-center justify-center text-gray-500 p-2 text-center opacity-50"
          >
            <svg
              class="w-6 h-6 mb-1"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"
              />
            </svg>
            <p class="text-xs">{t("settings.noApiKeys")}</p>
          </div>
        {/if}
      </div>
    </div>
  {/snippet}

  {#if activeSettingsSection === "llm" && !aiStore.killSwitchActive}
    {@render apiKeysCard("custom")}

      <!-- Translation Tiers (priority list + failover) -->
      <div class="glass-card flex flex-col mb-6 mt-10 relative z-20">
        <div class="p-4 border-b border-white/5 flex items-center justify-between gap-3 w-full">
          <div class="flex items-center gap-3">
            <div class="w-9 h-9 rounded-lg bg-indigo-500/20 text-indigo-300 flex items-center justify-center shrink-0">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h10M4 18h6" />
              </svg>
            </div>
            <div>
              <h3 class="text-sm font-bold text-white">{t("tiers.cardTitle")}</h3>
            </div>
          </div>
          <button
            type="button"
            onclick={() => translationTiersRef?.triggerAddTier()}
            class="inline-flex items-center justify-center gap-2 rounded-lg bg-indigo-500 px-3.5 py-2 text-xs font-bold text-white shadow-lg shadow-indigo-500/20 hover:bg-indigo-400 transition-colors cursor-pointer"
          >
            + {t("tiers.addTier")}
          </button>
        </div>
        <div class="p-4">
          <TranslationTiers bind:this={translationTiersRef} />
        </div>
      </div>

      <!-- Default Flashcard Refinement Prompt -->
      <div class="glass-card p-5 mt-10 mb-6 relative z-10">
        <div class="flex items-center gap-3 mb-4">
          <div class="w-9 h-9 rounded-lg bg-indigo-500/20 text-indigo-300 flex items-center justify-center shrink-0">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
            </svg>
          </div>
          <div>
            <h3 class="text-sm font-bold text-white">{t("settings.refinementPromptTitle") || "Default Flashcard Refinement Prompt"}</h3>
          </div>
        </div>
        <div class="space-y-3">
          <CodeEditor
            id="default-refinement-prompt"
            bind:value={defaultRefinementPrompt}
            onchange={persistRefinementPrompt}
            language="prompt"
            placeholder={DEFAULT_REFINEMENT_PROMPT}
            heightClass="h-[450px]"
            class={highlightedModelId === "default-refinement-prompt" ? "editor-highlight-pulse" : ""}
          />
        </div>
      </div>
  {/if}

  <!-- Whisper Models -->
  {#if activeSettingsSection === "whisper" && !aiStore.killSwitchActive}
    {@render apiKeysCard("groq")}

  <!-- Whisper Tiers (priority list + failover) -->
  <div class="glass-card flex flex-col mb-6 mt-10">
    <div class="p-4 border-b border-white/5 flex items-center justify-between gap-3 w-full">
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-lg bg-indigo-500/20 text-indigo-300 flex items-center justify-center shrink-0">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h10M4 18h6" />
          </svg>
        </div>
        <div>
          <h3 class="text-sm font-bold text-white">{t("transcribe.tiers.cardTitle") || "Tier di precedenza per la trascrizione"}</h3>
        </div>
      </div>
      <button
        type="button"
        onclick={() => transcribeTiersRef?.triggerAddTier()}
        class="inline-flex items-center justify-center gap-2 rounded-lg bg-indigo-500 px-3.5 py-2 text-xs font-bold text-white shadow-lg shadow-indigo-500/20 hover:bg-indigo-400 transition-colors cursor-pointer"
      >
        + {t("tiers.addTier")}
      </button>
    </div>
    <div class="p-4">
      <TranscribeTiers bind:this={transcribeTiersRef} />
    </div>
  </div>



  {#if whisperEngine === "local"}
  <div class="mt-6 glass-card p-5 {downloadedWhisperCount === 0 ? 'border-glow-amber-slow' : ''}" role="group" oncontextmenu={openWhisperPanelContextMenu}>
    <div class="flex items-center gap-3 mb-4">
      <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-cyan-500 to-blue-600 flex items-center justify-center text-white shadow-lg">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
        </svg>
      </div>
      <div class="flex-1">
        <h3 class="text-sm font-bold text-white">{t("transcribe.whisperModel")}</h3>
      </div>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-3 gap-3 mb-4">
      <div class="p-4 rounded-xl bg-cyan-500/10 border border-cyan-500/25">
        <p class="text-xs uppercase tracking-wide text-cyan-300/70 mb-1">{t("settings.modelsDownloadedLocally")}</p>
        <p class="text-2xl font-bold text-white">{downloadedWhisperCount}/{whisperModels.length}</p>
      </div>
      <div class="p-4 rounded-xl bg-white/5 border border-white/10">
        <p class="text-xs uppercase tracking-wide text-gray-500 mb-1">{t("settings.default")}</p>
        <p class="text-2xl font-bold text-white">{defaultWhisperModel ? (t(`transcribe.model${defaultWhisperModel.charAt(0).toUpperCase()}${defaultWhisperModel.slice(1)}`) || defaultWhisperModel) : ""}</p>
      </div>
      <div class="p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/25">
        <p class="text-xs uppercase tracking-wide text-emerald-300/70 mb-1">{t("settings.ready")}</p>
        <p class="text-2xl font-bold text-white">{downloadedWhisperCount > 0 ? t("common.yes") : t("common.no")}</p>
      </div>
    </div>

    {#if isDownloading && downloadingModelId}
      <div class="mb-3 text-xs text-gray-400">
        {t("settings.modelDownloading", { model: downloadingModelId }) || `Downloading model: ${downloadingModelId}`}
        {#if progress > 0}
          <span class="text-cyan-300 ml-1">{progress}%</span>
        {/if}
      </div>
    {/if}
    
    <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-5 gap-3">
      {#each whisperModels as model}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          onclick={() => handleWhisperModelClick(model)}
          ondblclick={() => handleModelDblClick(model)}
          oncontextmenu={(e) => openContextMenu(e, model)}
          onkeydown={(e) => {
            if (e.key === "Enter" || e.key === " ") handleWhisperModelClick(model);
          }}
          role="radio"
          aria-checked={defaultWhisperModel === model.id}
          tabindex="0"
          class="relative min-h-[8.5rem] p-4 rounded-xl text-center transition-all duration-200 border cursor-pointer
            {defaultWhisperModel === model.id && model.downloaded
            ? 'bg-cyan-500/20 border-cyan-500/50 text-white shadow-[0_0_15px_rgba(6,182,212,0.15)]'
            : model.downloaded
              ? 'bg-white/10 hover:bg-white/20 border-white/20 text-gray-200'
              : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-500 opacity-60'}
            {highlightedModelId === model.id ? 'model-highlight-flash' : ''}"
          title={model.downloaded ? t("settings.whisperDownloadedHint") : t("settings.whisperNotDownloadedHint")}
        >
          <div class="absolute top-1.5 right-1.5 pointer-events-none">
            {#if !model.downloaded}
              {#if downloadingModelId === model.id}
                <button
                   onclick={(e) => { e.stopPropagation(); void cancelModelDownload(); }}
                  disabled={isCancellingDownload}
                  class="text-red-400 hover:text-red-300 transition-colors pointer-events-auto p-1 bg-red-500/10 hover:bg-red-500/20 rounded-md border border-red-500/30 flex items-center justify-center cursor-pointer"
                  title={t("settings.stopModelDownload") || "Ferma download"}
                >
                  {#if isCancellingDownload}
                    <svg class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24">
                      <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                      <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                    </svg>
                  {:else}
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 6h12v12H6z" />
                    </svg>
                  {/if}
                </button>
              {:else}
                <button
                  onclick={(e) => { e.stopPropagation(); void downloadModel(model.id, true); }}
                  class="text-amber-400 hover:text-cyan-400 transition-colors animate-pulse pointer-events-auto p-1 hover:bg-white/5 rounded-md"
                  title={t("transcribe.clickToDownload")}
                  disabled={isDownloading}
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
                  </svg>
                </button>
              {/if}
            {/if}
          </div>
          <div class="mx-auto mb-2 flex h-11 w-11 items-center justify-center rounded-xl border border-white/10 bg-gradient-to-br {whisperModelAccent(model.id)} shadow-sm">
            <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={whisperModelIconPath(model.id)} />
            </svg>
          </div>
          <div class="font-bold text-sm">
            {t(`transcribe.model${model.id.charAt(0).toUpperCase()}${model.id.slice(1)}`) || model.name}
          </div>
          <div class="text-[10px] text-gray-500 mt-1">{model.size}</div>
          {#if !model.downloaded}
            <div class="text-[9px] text-amber-400/70 mt-0.5">
              {#if downloadingModelId === model.id}
                {t("settings.downloading")} {progress > 0 ? `${progress}%` : ""}
              {:else}
                {t("settings.notDownloaded")}
              {/if}
            </div>
          {:else if defaultWhisperModel === model.id}
            <div class="text-[9px] text-cyan-400 mt-0.5 font-bold">{t("settings.default")}</div>
          {/if}
        </div>
      {/each}
    </div>
  </div>
  {/if}

  {#if contextMenu}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50"
      onmousedown={closeContextMenu}
      oncontextmenu={(e) => { e.preventDefault(); closeContextMenu(); }}
    >
      <div
        class="absolute bg-gray-900/98 border border-white/10 rounded-xl shadow-2xl py-1 min-w-[190px] animate-fade-in overflow-hidden"
        style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
        onmousedown={(e) => e.stopPropagation()}
      >
        <div class="px-3 py-1.5 border-b border-white/5 bg-white/5 mb-1">
          <span class="text-xs font-bold text-gray-400 uppercase tracking-wide">
            {contextMenu.kind === "model" ? `Whisper: ${contextMenu.modelId}` : "Whisper"}
          </span>
        </div>
        {#if contextMenu.kind === "panel"}
          <button
            class="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-white/10 hover:text-white flex items-center gap-2 transition-colors"
            onclick={() => {
              void refreshModels();
              closeContextMenu();
            }}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9M20 20v-5h-.581m0 0a8.003 8.003 0 01-15.357-2" /></svg>
            {t("settings.refreshStatus")}
          </button>
          {#if whisperModels.some((model) => !model.downloaded)}
            <button
              class="w-full text-left px-4 py-2 text-sm text-cyan-300 hover:bg-cyan-500/10 hover:text-cyan-200 flex items-center gap-2 transition-colors"
              onclick={() => {
                const nextModel = whisperModels.find((model) => !model.downloaded);
                if (nextModel) void downloadModel(nextModel.id, true);
                closeContextMenu();
              }}
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" /></svg>
              {t("settings.downloadNext")}
            </button>
          {/if}
        {:else if contextMenu.downloaded && contextMenu.modelId}
          <button
            class="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-white/10 hover:text-white flex items-center gap-2 transition-colors"
            onclick={() => {
              if (contextMenu?.modelId) setDefaultWhisperModel(contextMenu.modelId);
              closeContextMenu();
            }}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" /></svg>
            {t("settings.setAsDefault")}
          </button>
          <button
            class="w-full text-left px-4 py-2 text-sm text-red-400 hover:bg-red-500/10 hover:text-red-300 flex items-center gap-2 transition-colors"
            onclick={() => {
              if (contextMenu?.modelId) void uninstallModel(contextMenu.modelId);
              closeContextMenu();
            }}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
            {t("settings.remove")}
          </button>
        {:else if contextMenu.modelId}
          <button
            class="w-full text-left px-4 py-2 text-sm text-cyan-300 hover:bg-cyan-500/10 hover:text-cyan-200 flex items-center gap-2 transition-colors"
            onclick={() => {
              if (contextMenu?.modelId) void downloadModel(contextMenu.modelId, true);
              closeContextMenu();
            }}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" /></svg>
            {t("settings.downloadAndSet")}
          </button>
        {/if}
      </div>
    </div>
  {/if}
  {/if}

  <!-- Card Template Editor -->
  {#if activeSettingsSection === "anki"}
  <div class="mt-6 space-y-4">

      <div class="glass-card p-5">
        <div class="flex flex-col lg:flex-row lg:items-start lg:justify-between gap-4 mb-5">
          <div class="flex items-center gap-3">
            <div class="w-9 h-9 rounded-lg bg-emerald-500/20 text-emerald-300 flex items-center justify-center shrink-0">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v4H4V5zm0 8h8v7H5a1 1 0 01-1-1v-6zm12 0h4v6a1 1 0 01-1 1h-3v-7z" />
              </svg>
            </div>
            <div>
              <h3 class="text-sm font-bold text-white">{s("fieldPanelKicker")}</h3>
            </div>
          </div>
          <div class="flex flex-wrap gap-2">
            <button
              type="button"
              onclick={saveCurrentAnkiFieldPreset}
              disabled={!noteTypeName.trim() || !getFieldValue("expression").trim()}
              class="px-3 py-2 rounded-lg border border-emerald-500/30 bg-emerald-500/10 text-emerald-200 hover:bg-emerald-500/20 transition-colors text-xs font-semibold flex items-center gap-2 disabled:opacity-40 disabled:cursor-not-allowed"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
              </svg>
              {t("settings.modal.save")}
            </button>
            <button
              type="button"
              onclick={deleteCurrentAnkiFieldPreset}
              disabled={selectedAnkiFieldPresetId === "default"}
              class="px-3 py-2 rounded-lg border border-white/10 bg-white/5 text-gray-300 hover:text-red-300 hover:border-red-500/30 disabled:opacity-40 disabled:hover:text-gray-300 disabled:hover:border-white/10 transition-colors text-xs font-semibold flex items-center gap-2"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
              </svg>
              {t("settings.delete")}
            </button>

          </div>
        </div>

        <div class="grid grid-cols-1 lg:grid-cols-[1fr_1fr_1.2fr] gap-4 mb-5">
          <div>
            <label for="anki-field-preset-select" class="block text-xs font-semibold text-gray-400 mb-2">{s("savedTemplate")}</label>
            <SearchableSelect
              className="settings-template-select"
              noResultsText={t("common.noResults")}
              options={allAnkiFieldPresets.map((preset) => ({
                value: preset.id,
                label: preset.name,
              }))}
              value={selectedAnkiFieldPresetId}
              onchange={applyAnkiFieldPreset}
              placeholder={s("savedTemplate")}
            />
          </div>
          <div>
            <label for="anki-field-preset-name" class="block text-xs font-semibold text-gray-400 mb-2">{s("templateName")}</label>
            <input
              id="anki-field-preset-name"
              type="text"
              bind:value={ankiFieldPresetName}
              maxlength="25"
              class="input-modern w-full text-sm"
              placeholder="vesta_modificato"
            />
          </div>
          <div>
            <label for="active-flashcards-template-select" class="block text-xs font-semibold text-gray-400 mb-2">Template attivo per Flashcard</label>
            <SearchableSelect
              className="settings-active-template-select"
              noResultsText={t("common.noResults")}
              options={allAnkiFieldPresets.map((preset) => ({
                value: preset.id,
                label: preset.id === "default" ? preset.name : `★ ${preset.name}`,
              }))}
              value={activeNoteTypeId}
              onchange={(v) => {
                activeNoteTypeId = v;
                saveActiveNoteTypeId(v);
              }}
              placeholder="Seleziona il template attivo..."
            />
          </div>
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5 gap-3">
          <div>
            <label for="note-type-name-inline" class="mb-1 flex items-center gap-1.5 text-xs font-semibold text-gray-400">
              <svg class="h-3.5 w-3.5 text-emerald-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v4H4V5zm0 8h8v7H5a1 1 0 01-1-1v-6zm12 0h4v6a1 1 0 01-1 1h-3v-7z" />
              </svg>
              <span>Note type</span>
            </label>
            <input
              id="note-type-name-inline"
              type="text"
              bind:value={noteTypeName}
              maxlength="25"
              disabled={selectedAnkiFieldPresetId === "default"}
              oninput={(event) =>
                syncLimitedInput(event, (value) => (noteTypeName = value), saveTemplates)}
              class="input-modern w-full text-sm disabled:opacity-50 disabled:cursor-not-allowed"
              placeholder="Vesta_Default"
            />
          </div>
          {#each ankiFieldDefinitions as field}
            {@const isLocked = field.key === "expression" || field.key === "sequenceMarker"}
            <div>
              <label for={`anki-field-${field.key}`} class="mb-1 flex items-center justify-between gap-1.5 text-xs font-semibold text-gray-400">
                <div class="flex items-center gap-1.5">
                  <svg class={`h-3.5 w-3.5 ${field.iconClass}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={field.iconPath} />
                  </svg>
                  <span>{fieldVariableName(field)}</span>
                </div>
                {#if isLocked}
                  <span class="text-[9px] text-amber-500/80 font-semibold uppercase tracking-wider bg-amber-500/10 px-1.5 py-0.5 rounded border border-amber-500/20 flex items-center gap-1" title={t("settings.essentialFieldLocked")}>
                    <svg class="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                    </svg>
                    <span>LOCKED</span>
                  </span>
                {/if}
              </label>
              <input
                id={`anki-field-${field.key}`}
                aria-label={fieldVariableName(field)}
                type="text"
                value={getFieldValue(field.key)}
                maxlength="25"
                disabled={selectedAnkiFieldPresetId === "default" || isLocked}
                oninput={(event) => syncLimitedInput(event, (value) => setFieldValue(field.key, value), saveFields)}
                class="input-modern w-full text-sm disabled:opacity-50 disabled:cursor-not-allowed {isLocked ? 'border-amber-500/20 bg-amber-500/5 text-amber-200/90' : !getFieldValue(field.key).trim() ? 'opacity-40 border-dashed border-gray-600' : ''}"
                placeholder={fieldVariableName(field)}
              />
            </div>
          {/each}
        </div>
      </div>

      <div class="glass-card p-5">
        <div class="flex flex-col xl:flex-row xl:items-start xl:justify-between gap-4 mb-4">
          <div class="flex items-center gap-3">
            <div class="w-9 h-9 rounded-lg bg-cyan-500/20 text-cyan-300 flex items-center justify-center shrink-0">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 5h14a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2z" />
              </svg>
            </div>
            <div>
              <h3 class="text-sm font-bold text-white">{s("cardPanelKicker")}</h3>
            </div>
          </div>
          <div class="flex flex-wrap gap-2">
            {#each templateCodeTabs as tab}
              <button
                type="button"
                onclick={() => (activeTemplateCodeTab = tab.id)}
                title={tab.hint}
                class="h-9 px-3 rounded-lg border text-xs font-semibold transition-colors {activeTemplateCodeTab === tab.id ? 'bg-cyan-500/20 border-cyan-400/40 text-cyan-100' : 'bg-black/20 border-white/10 text-gray-400 hover:text-white hover:bg-white/10'}"
              >
                {tab.label}
              </button>
            {/each}


          </div>
        </div>

        {#if activeTemplateCodeTab === "front"}
          <CodeEditor bind:value={templateFrontHtml} language="html" onchange={saveTemplates} />
        {:else if activeTemplateCodeTab === "back"}
          <CodeEditor bind:value={templateBackHtml} language="html" onchange={saveTemplates} />
        {:else}
          <CodeEditor bind:value={templateCss} language="css" onchange={saveTemplates} />
        {/if}

        <div class="mt-4 rounded-lg border border-white/10 bg-black/20 p-4">
          <div class="flex flex-col lg:flex-row lg:items-center gap-3">
            <div class="lg:w-48 shrink-0">
              <p class="text-xs uppercase tracking-wide text-cyan-300/80">{t("settings.availableVars")}</p>
              <p class="text-xs text-gray-500 mt-1">{s("clickToCopy")}</p>
            </div>
            <div class="flex flex-wrap gap-2 text-[11px] font-mono">
            {#each ankiFieldDefinitions as field}
              <button
                type="button"
                onclick={() => {
                  navigator.clipboard.writeText(getFieldVariable(field));
                  showSnackbar(t("settings.keyCopied"));
                }}
                class="px-2.5 py-1.5 rounded-lg border transition-colors {field.colorClass}"
                title="Copia variabile"
              >
                {getFieldVariable(field)}
              </button>
            {/each}
            </div>
          </div>
        </div>
      </div>

    </div>
  {/if}
  </div>

  <!-- Fixed Bottom Band with Red Reset Button styled and sized to sidebar bottom -->
  <div class="h-[92px] border-t border-white/10 bg-gray-900 flex items-center justify-center shrink-0">
    <button
      onclick={() => {
        if (activeSettingsSection !== "shortcuts") {
          showResetConfirm = activeSettingsSection;
        }
      }}
      class="px-5 py-2.5 bg-red-600 hover:bg-red-500 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-red-900/30 flex items-center gap-2 hover:scale-[1.02] active:scale-[0.98] cursor-pointer"
    >
      <svg
        class="w-4 h-4"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
        />
      </svg>
      {t("settings.resetDefaults") || "Ripristina predefiniti"}
    </button>
  </div>
  {/if}

  <!-- Reset Confirmation Dialog -->
  <ConfirmDialog
    show={!!showResetConfirm}
    title={resetTitle}
    message={resetMessage}
    confirmText={t("settings.resetDefaults") || "Ripristina predefiniti"}
    cancelText={t("common.cancel") || "Annulla"}
    variant="danger"
    on:cancel={() => (showResetConfirm = null)}
    on:confirm={() => {
      if (showResetConfirm === "overview") {
        resetOverviewSettings();
      } else if (showResetConfirm === "llm") {
        resetLlmSettings();
      } else if (showResetConfirm === "whisper") {
        resetWhisperSettings();
      } else if (showResetConfirm === "language") {
        resetLanguageSettings();
      } else if (showResetConfirm === "anki") {
        resetAnkiSettings();
      }
      showResetConfirm = null;
    }}
  />

  {#if showAddKey}
    <div
      class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-4"
      role="dialog"
      tabindex="-1"
      onmousedown={(e) => {
        if (e.target === e.currentTarget) showAddKey = false;
      }}
    >
      <div
        class="w-full max-w-4xl max-h-[92vh] overflow-hidden animate-fade-in shadow-2xl border border-white/20 bg-gray-900/98 backdrop-blur-xl rounded-xl flex flex-col"
        role="presentation"
        onmousedown={(e) => e.stopPropagation()}
      >
        <div class="p-6 border-b border-white/5 bg-white/5">
          <h3 class="text-xl font-bold text-white flex items-center gap-2">
            {editKeyId
              ? t("settings.modal.editApiKey")
              : t("settings.modal.addCustomApiKey")}
          </h3>
        </div>

        <div class="p-6 flex-1 overflow-hidden flex flex-col">
          <div class="space-y-5 overflow-y-auto custom-scrollbar pr-1">
          <div>
            <span
              class="block text-xs font-bold text-gray-500 uppercase tracking-wide mb-2"
              >{t("settings.modal.provider")}</span
            >
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2.5 mb-3">
              {#each (modalContext === "whisper" ? whisperProviderIds : llmProviderIds) as pid (pid)}
                {@const prov = providers[pid]}
                {@const isCustom = pid === "custom"}
                <button
                  type="button"
                  onclick={() => {
                    newKeyType = pid as ApiKeyConfig["apiType"];
                    newKeyName = isCustom ? "" : (pid === "openai" ? "Open AI" : prov?.name || pid);
                    newKeyUrl = isCustom ? "" : prov?.defaultApiUrl || "";
                    newKeyValue = "";
                  }}
                  class="flex items-center gap-2.5 p-2.5 rounded-lg transition-all duration-200 border text-left
                    {newKeyType === pid
                    ? 'bg-indigo-500/20 border-indigo-500/50 text-white'
                    : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400'}"
                >
                  <ProviderIcon provider={pid} />
                  <div class="flex flex-col min-w-0">
                    <span class="text-sm font-bold truncate"
                      >{pid === "openai" ? "Open AI" : (isCustom ? t("provider.custom") : prov?.name || pid)}</span
                    >
                    <span class="text-[10px] opacity-70 leading-tight line-clamp-2"
                      >{isCustom ? t("provider.custom.desc") : (pid === "openai" ? (modalContext === "whisper" ? ($currentLanguage === "it" ? "API OpenAI speech-to-text (Whisper)" : "OpenAI speech-to-text API (Whisper)") : ($currentLanguage === "it" ? "Modelli OpenAI (GPT-4o, GPT-4...)" : "OpenAI models (GPT-4o, GPT-4...)")) : prov?.description || "")}</span
                    >
                  </div>
                </button>
              {/each}
            </div>
          </div>

          <div class="space-y-4">
            <div>
              <label
                for="key-name"
                class="block text-xs font-bold text-gray-500 uppercase tracking-wide mb-1.5"
                >{t("settings.modal.configName")}</label
              >
              <input
                id="key-name"
                type="text"
                bind:value={newKeyName}
                placeholder={t("settings.modal.configNamePlaceholder")}
                class="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm text-white focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500/50 outline-none transition-all placeholder-gray-600"
              />
            </div>

            {#if newKeyType === "local" || newKeyType === "custom"}
              <div>
                <label
                  for="api-url"
                  class="block text-xs font-bold text-gray-500 uppercase tracking-wide mb-1.5"
                >
                  {t("settings.modal.apiEndpoint")}
                </label>
                <input
                  id="api-url"
                  type="text"
                  bind:value={newKeyUrl}
	                  placeholder={newKeyType === "local"
	                    ? providers[newKeyType]?.defaultApiUrl || "https://..."
	                    : "https://api.example.com/v1"}
                  class="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm text-white focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500/50 outline-none transition-all placeholder-gray-600 font-mono"
                />
              </div>
            {/if}

            {#if newKeyType !== "local"}
              <div>
                <label
                  for="api-key"
                  class="block text-xs font-bold text-gray-500 uppercase tracking-wide mb-1.5"
                  >{t("settings.modal.apiKey")}
                  {#if newKeyType === "custom"}<span
                      class="text-gray-600 normal-case font-normal"
                      >({t("settings.modal.optional")})</span
                    >{/if}</label
                >
                <div class="relative">
                  <input
                    id="api-key"
                    type={showNewKeyPassword ? "text" : "password"}
                    bind:value={newKeyValue}
                    placeholder={newKeyType === "custom"
                      ? t("settings.modal.notRequiredForLocal")
                      : providers[newKeyType]?.keyPlaceholder || "API key"}
                    class="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 pr-20 text-sm text-white focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500/50 outline-none transition-all placeholder-gray-600 font-mono"
                  />
                  <div
                    class="absolute right-2 top-1/2 -translate-y-1/2 flex gap-1"
                  >
                    <button
                      type="button"
                      onclick={() => (showNewKeyPassword = !showNewKeyPassword)}
                      class="p-1.5 text-gray-500 hover:text-gray-300 transition-colors"
                      title={t("settings.toggleVisibility")}
                    >
                      {#if showNewKeyPassword}
                        <svg
                          class="w-4 h-4"
                          fill="none"
                          stroke="currentColor"
                          viewBox="0 0 24 24"
                        >
                          <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"
                          />
                        </svg>
                      {:else}
                        <svg
                          class="w-4 h-4"
                          fill="none"
                          stroke="currentColor"
                          viewBox="0 0 24 24"
                        >
                          <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                          />
                          <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
                          />
                        </svg>
                      {/if}
                    </button>
                    <button
                      type="button"
                      onclick={() => copyToClipboard(newKeyValue)}
                      class="p-1.5 text-gray-500 hover:text-gray-300 transition-colors"
                      title="Copy"
                    >
                      <svg
                        class="w-4 h-4"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                        />
                      </svg>
                    </button>
                  </div>
                </div>

                {#if (modalContext === "whisper" ? whisperApiKeyUrls[newKeyType] : llmApiKeyUrls[newKeyType]) || (modalContext === "whisper" ? whisperDocUrls[newKeyType] : llmDocUrls[newKeyType])}
                  <div class="mt-2.5 flex flex-wrap gap-x-4 gap-y-1 text-xs">
                    {#if modalContext === "whisper" ? whisperApiKeyUrls[newKeyType] : llmApiKeyUrls[newKeyType]}
                      <a
                        href={modalContext === "whisper" ? whisperApiKeyUrls[newKeyType] : llmApiKeyUrls[newKeyType]}
                        target="_blank"
                        rel="noopener noreferrer"
                        class="text-indigo-400 hover:text-indigo-300 hover:underline flex items-center gap-1.5 font-semibold transition-colors"
                      >
                        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m-2 4a2 2 0 012 2m-3-4a3 3 0 11-6 0 3 3 0 016 0zM8 21a4 4 0 014-4h4a4 4 0 014 4H8z" />
                        </svg>
                        {t("settings.modal.getApiKey")}
                      </a>
                    {/if}
                    {#if modalContext === "whisper" ? whisperDocUrls[newKeyType] : llmDocUrls[newKeyType]}
                      <a
                        href={modalContext === "whisper" ? whisperDocUrls[newKeyType] : llmDocUrls[newKeyType]}
                        target="_blank"
                        rel="noopener noreferrer"
                        class="text-cyan-400 hover:text-cyan-300 hover:underline flex items-center gap-1.5 font-semibold transition-colors"
                      >
                        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5s3.332.477 4.5 1.253v13C19.832 18.477 18.246 18 16.5 18s-3.332.477-4.5 1.253" />
                        </svg>
                        {t("settings.modal.documentation")}
                      </a>
                    {/if}
                  </div>
                {/if}
              </div>
            {/if}

          </div>

          </div>

          <div class="flex gap-3 pt-4 mt-auto border-t border-white/5 shrink-0">
            <button
              onclick={() => (showAddKey = false)}
              class="flex-1 py-2.5 rounded-lg border border-white/10 text-gray-400 hover:bg-white/5 hover:text-white transition-all text-sm font-medium"
            >
              {t("settings.modal.cancel")}
            </button>
            <button
              onclick={addApiKey}
              class="flex-1 py-2.5 rounded-lg bg-indigo-500 hover:bg-indigo-400 text-white shadow-lg shadow-indigo-500/20 transition-all text-sm font-bold"
            >
              {t("settings.modal.save")}
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}



  {#if deleteConfirmId}
    <div
      class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-4"
      role="dialog"
      tabindex="-1"
      onmousedown={(e) => {
        if (e.target === e.currentTarget) cancelDelete();
      }}
    >
      <div
        class="w-full max-w-sm overflow-hidden animate-fade-in shadow-2xl border border-white/20 bg-gray-900/98 backdrop-blur-xl rounded-xl"
        role="presentation"
        onmousedown={(e) => e.stopPropagation()}
      >
        <div class="p-6 border-b border-white/5 bg-white/5">
          <h3 class="text-xl font-bold text-white">{t("app.title")}</h3>
        </div>

        <div class="p-6 space-y-4">
          <p class="text-gray-300">
            {t("settings.confirmDeleteKey", { name: deleteConfirmName })}
          </p>

          <div class="flex gap-3 pt-2">
            <button
              onclick={cancelDelete}
              class="flex-1 py-2.5 rounded-lg border border-white/10 text-gray-400 hover:bg-white/5 hover:text-white transition-all text-sm font-medium"
            >
              {t("settings.modal.cancel")}
            </button>
            <button
              onclick={confirmDeleteApiKey}
              class="flex-1 py-2.5 rounded-lg bg-red-500 hover:bg-red-400 text-white shadow-lg shadow-red-500/20 transition-all text-sm font-bold"
            >
              {t("settings.confirmDelete")}
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  ::-webkit-scrollbar {
    width: 6px;
    height: 6px;
  }
  ::-webkit-scrollbar-track {
    background: transparent;
  }
  ::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 3px;
  }
  ::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.2);
  }

  /* Tooltip for Card Style info buttons only */
  :global(.card-style-tooltip) {
    display: none;
    position: absolute;
    bottom: calc(100% + 8px);
    right: 0;
    width: 280px;
    padding: 8px 10px;
    background: #1f2937;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    font-size: 11px;
    line-height: 1.4;
    color: #d1d5db;
    z-index: 50;
    pointer-events: none;
    white-space: normal;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }
  :global(.card-style-tooltip.down) {
    bottom: auto;
    top: calc(100% + 8px);
  }
  :global(.group:hover > .card-style-tooltip) {
    display: block;
  }


  :global(.settings-quick-setup-pulse) {
    animation: settings-quick-setup-pulse 1.45s ease-in-out infinite;
  }

  @keyframes settings-quick-setup-pulse {
    0%,
    100% {
      border-color: rgba(251, 191, 36, 0.28);
      box-shadow: 0 0 0 0 rgba(251, 191, 36, 0);
    }

    50% {
      border-color: rgba(249, 115, 22, 0.78);
      box-shadow:
        0 0 0 1px rgba(249, 115, 22, 0.32),
        0 0 24px rgba(249, 115, 22, 0.24);
    }
  }

  .border-glow-amber-slow {
    animation: settings-glow-pulse-amber 4s ease-in-out infinite;
  }

  @keyframes settings-glow-pulse-amber {
    0%,
    100% {
      border-color: rgba(245, 158, 11, 0.15);
      box-shadow: 0 0 4px 0 rgba(245, 158, 11, 0.05);
    }
    50% {
      border-color: rgba(245, 158, 11, 0.7);
      box-shadow: 
        0 0 16px 1px rgba(245, 158, 11, 0.25),
        inset 0 0 8px 0 rgba(245, 158, 11, 0.15);
    }
  }

	  .ui-language-grid {
	    display: grid;
	    grid-template-columns: repeat(auto-fill, minmax(11.5rem, 1fr));
	    gap: 0.75rem;
	    width: 100%;
	  }

	  @media (min-width: 1280px) {
	    .ui-language-grid {
	      grid-template-columns: repeat(5, minmax(0, 1fr));
	    }
	  }

	  .ui-language-button {
	    min-width: 0;
	  }

	  :global(.language-select .searchable-select-input) {
    min-height: 3.25rem;
    font-size: 1rem;
    padding-block: 0.95rem;
  }

  :global(.language-select .searchable-select-option) {
    min-height: 2.75rem;
    font-size: 0.95rem;
  }

  :global(.language-select .searchable-select-option span:first-child) {
    font-size: 1.35rem;
  }

  @keyframes settings-model-highlight-flash {
    0%, 100% {
      border-color: rgba(255, 255, 255, 0.1);
      box-shadow: none;
    }
    25%, 75% {
      border-color: #f59e0b;
      box-shadow: 0 0 15px rgba(245, 158, 11, 0.6);
      background-color: rgba(245, 158, 11, 0.1);
      opacity: 1;
    }
  }

  .model-highlight-flash {
    animation: settings-model-highlight-flash 1s ease-in-out 2;
  }
</style>
