<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invokeCommand as invoke } from "$lib/services/tauriClient";
  import { listen } from "@tauri-apps/api/event";
  import { fetch as tauriFetch } from "$lib/services/tauriHttp";
  import CodeEditor from "$lib/components/CodeEditor.svelte";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";
  import FooterActions from "$lib/components/FooterActions.svelte";
  import ToggleRow from "$lib/components/ToggleRow.svelte";
  import ShortcutsTab from "$lib/tabs/ShortcutsTab.svelte";
  import TranslationTiers from "$lib/components/TranslationTiers.svelte";
  import ProviderIcon from "$lib/components/ProviderIcon.svelte";
  import ApiKeysCard from "$lib/panels/ApiKeysCard.svelte";
  import AddApiKeyModal from "$lib/modals/AddApiKeyModal.svelte";
  import { apiKeyEditorStore } from "$lib/stores/apiKeyEditorStore.svelte";
  import AnkiSettingsPanel from "$lib/panels/AnkiSettingsPanel.svelte";
  import { ankiTemplateStore } from "$lib/stores/ankiTemplateStore.svelte";
  import WhisperSettingsPanel from "$lib/panels/WhisperSettingsPanel.svelte";
  import { whisperModelsStore } from "$lib/stores/whisperModelsStore.svelte";
  import OverviewSettingsPanel from "$lib/panels/OverviewSettingsPanel.svelte";
  import DiagnosticsSettingsPanel from "$lib/panels/DiagnosticsSettingsPanel.svelte";
  import { cpuRamStore } from "$lib/stores/cpuRamStore.svelte";
  import { exportFormatStore } from "$lib/stores/exportFormatStore.svelte";
  import { updateCheckerStore } from "$lib/stores/updateCheckerStore.svelte";
  import { smartMatchingStore } from "$lib/stores/smartMatchingStore.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import { ankiStore } from "$lib/stores/ankiStore.svelte";
  import {
    availableUILanguages,
    currentLanguage,
    locale,
    setLanguage,
  } from "$lib/i18n";
  import LanguageSelect from "$lib/components/LanguageSelect.svelte";
  import {
    buildSettingsActionHash,
    publishSettingsActionState,
  } from "$lib/utils/settingsNotifications";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import {
    DEFAULT_REFINEMENT_PROMPT,
    REFINEMENT_PROMPT_STORAGE_KEY,
    loadRefinementPrompt,
  } from "$lib/config/refinementPrompt";
  import {
    fetchModelsFromEndpoint,
    type DiscoveredModel,
  } from "$lib/services/modelDiscovery";
  import { loadAndValidateApiKeys, type ApiKeyConfig } from "$lib/config/apiKeys";
  import {
    loadTiers,
    tiersHaveUsableEntries,
    TIERS_UPDATED_EVENT,
    type Tier,
  } from "$lib/config/translationTiers";
  import {
    loadTranscribeTiers,
    transcribeTiersHaveUsableEntries,
    TRANSCRIBE_TIERS_UPDATED_EVENT,
    type TranscribeTier,
  } from "$lib/config/transcribeTiers";
  import {
    loadTranscribeCloud,
    saveTranscribeCloud,
    transcribeProviders,
    transcribeProviderOrder,
  } from "$lib/config/transcribeProviders";
  import { getModelsForProvider, providers, type ModelInfo } from "$lib/config/llmProviders";
  import { languages } from "$lib/config/languages";
  import * as vestaConfig from "$lib/config/vestaConfig";

  const allProviderIds = ["local", "google", "groq", "openai", "deepgram", "assemblyai", "openrouter", "mistral", "github", "nvidia", "custom"];
  const apiKeyProviderIds = ["google", "groq", "openai", "deepgram", "assemblyai", "openrouter", "mistral", "github", "nvidia", "custom"];
  type EndpointStatus = "idle" | "checking" | "online" | "offline";
  type SettingsSection = "overview" | "llm" | "whisper" | "language" | "anki" | "diagnostics" | "shortcuts";
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
  function loadStoredValue(key: string, fallback = ""): string {
    try {
      return vestaConfig.getItem(key) || fallback;
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
  let defaultTargetLanguage = $state(loadStoredValue(DEFAULT_TARGET_LANGUAGE_KEY, "en"));
  let defaultTranscribeLanguage = $state(loadStoredValue(DEFAULT_TRANSCRIBE_LANGUAGE_KEY, "auto"));
  let defaultFlashcardsLanguage = $state(loadStoredValue(DEFAULT_FLASHCARDS_LANGUAGE_KEY, "en"));
  let defaultNativeLanguage = $state(loadStoredValue(DEFAULT_NATIVE_LANGUAGE_KEY, "en"));
  
  const DEFAULT_REFINEMENT_PROMPT_KEY = REFINEMENT_PROMPT_STORAGE_KEY;
  let defaultRefinementPrompt = $state(loadRefinementPrompt());

  function persistRefinementPrompt() {
    try {
      vestaConfig.setItem(DEFAULT_REFINEMENT_PROMPT_KEY, defaultRefinementPrompt);
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

  let showAddModel = $state(false);

  let translationTiersRef = $state<any>(null);
  let transcribeTiers = $state<TranscribeTier[]>([]);

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

  function s(key: string): string {
    return t(`settings.copy.${key}`);
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
  let configuredApiKeyCount = $derived(apiKeys.filter((key) => key.apiType !== "local").length);
  let hasRemoteApiKey = $derived(configuredApiKeyCount > 0);
  // La traduzione è pronta quando esiste almeno un endpoint tier usabile.
  let isDefaultLlmReady = $derived(tiersHaveUsableEntries(tiers));
  let requiredSetupActions = $derived.by(() => {
    const actions: { section: SettingsSection; title: string; desc: string }[] = [];
    if (whisperModelsStore.downloadedWhisperCount === 0) {
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
    vestaConfig.setItem(DEFAULT_LLM_PROVIDER_KEY, defaultLlmProvider);
    vestaConfig.setItem(DEFAULT_LLM_MODEL_KEY, defaultLlmModel);
    vestaConfig.setItem(DEFAULT_LLM_CUSTOM_PROVIDER_KEY, defaultLlmCustomProviderId);
    vestaConfig.setItem(LOCAL_SERVER_URL_KEY, defaultLocalServerUrl);
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
    vestaConfig.setItem(key, value);
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
      if (!parsed || typeof parsed !== "object") return t("settings.smartMatching.invalidObject");
      return null;
    } catch (e: any) {
      return e.message || t("settings.smartMatching.invalidJson");
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
      snackbar.show(t("settings.smartMatching.saveSuccess"), "success", 1300);
    } catch (e: any) {
      smartMatchingRulesError = e.message || t("settings.smartMatching.saveError");
    }
  }

  // Card template editor
  let showResetConfirm = $state<"style" | "fields" | "overview" | "llm" | "whisper" | "language" | "anki" | null>(null);
  let resetTitle = $derived.by(() => {
    if (showResetConfirm === "overview") {
      return t("settings.overview.resetConfirmTitle");
    }
    if (showResetConfirm === "llm") {
      return t("settings.llm.resetConfirmTitle");
    }
    if (showResetConfirm === "whisper") {
      return t("settings.whisper.resetConfirmTitle");
    }
    if (showResetConfirm === "language") {
      return t("settings.language.resetConfirmTitle");
    }
    if (showResetConfirm === "anki") {
      return t("settings.anki.resetConfirmTitle");
    }
    return t("settings.resetConfirmTitle");
  });
  let resetMessage = $derived.by(() => {
    if (showResetConfirm === "overview") {
      return t("settings.overview.resetConfirmDesc");
    }
    if (showResetConfirm === "llm") {
      return t("settings.llm.resetConfirmDesc");
    }
    if (showResetConfirm === "whisper") {
      return t("settings.whisper.resetConfirmDesc");
    }
    if (showResetConfirm === "language") {
      return t("settings.language.resetConfirmDesc");
    }
    if (showResetConfirm === "anki") {
      return t("settings.anki.resetConfirmDesc");
    }
    return t("settings.resetConfirmDesc");
  });
  // Export format: exportFormatStore is the single source of truth and
  // persists on its own setters — no sync $effects needed here.
  $effect(() => {
    const _ = uiMode.expertMode;
    try {
      const savedCores = vestaConfig.getItem("vesta_cpu_cores");
      if (savedCores) {
        cpuRamStore.cpuCores = parseInt(savedCores);
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

  function resetOverviewSettings() {
    setLanguage("en");
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

  function resetLanguageSettings() {
    defaultTargetLanguage = "en";
    defaultTranscribeLanguage = "auto";
    defaultFlashcardsLanguage = "en";
    defaultNativeLanguage = "en";
    saveDefaultLanguage(DEFAULT_TARGET_LANGUAGE_KEY, "en");
    saveDefaultLanguage(DEFAULT_TRANSCRIBE_LANGUAGE_KEY, "auto");
    saveDefaultLanguage(DEFAULT_FLASHCARDS_LANGUAGE_KEY, "en");
    saveDefaultLanguage(DEFAULT_NATIVE_LANGUAGE_KEY, "en");

    smartMatchingStore.resetRules();
    smartMatchingStore.setEnabled(true);
    smartMatchingRulesDraft = formatSmartMatchingRules(smartMatchingStore.rules);
    smartMatchingRulesError = null;

    snackbar.show(t("settings.language.resetSuccess"), "info", 1300);
  }


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

  onMount(() => {
    void cpuRamStore.init();
    void updateCheckerStore.init();

    loadApiKeys();
    smartMatchingRulesDraft = formatSmartMatchingRules(smartMatchingStore.rules);
    whisperModelsStore.defaultWhisperModel = vestaConfig.getItem("srt-default-whisper-model") || "base";

    whisperModelsStore.refreshModels().catch((e) => console.error("Could not list models:", e));
    void whisperModelsStore.refreshAddons();

    const handleKeydown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (deleteConfirmId) {
          cancelDelete();
        } else if (apiKeyEditorStore.showAddKey) {
          apiKeyEditorStore.close();
        }
      }
    };

    const handleOpenAddKeyModalEvent = () => {
      openAddKeyModal("custom");
    };

    tiers = loadTiers();
    transcribeTiers = loadTranscribeTiers();
    window.addEventListener("keydown", handleKeydown);
    window.addEventListener("vesta-open-settings-section", handleOpenSettingsSectionEvent);
    window.addEventListener("vesta-open-add-key-modal", handleOpenAddKeyModalEvent);
    window.addEventListener(TIERS_UPDATED_EVENT, syncTiersFromStorage);
    window.addEventListener(TRANSCRIBE_TIERS_UPDATED_EVENT, syncTranscribeTiersFromStorage);

    let activeListener = true;
    let unlistenProg: (() => void) | null = null;

    // Real backend subscription (unlike the whisper-model download/uninstall
    // window events, which only ever originate from WhisperSettingsPanel's
    // own child TranscribeTiers) -- stays registered for SettingsTab's whole
    // lifetime so progress tracking survives navigating away from the
    // "whisper" section mid-download. See [[vesta-settings-refactor]].
    listen<{
      stage: string;
      message: string;
      percentage: number;
    }>("transcribe-progress", (event) => {
      const p = event.payload;
      whisperModelsStore.progress = Math.round(p.percentage);
      whisperModelsStore.progressMessage = p.message;
      whisperModelsStore.progressStage = p.stage;
      window.dispatchEvent(new CustomEvent("vesta-whisper-download-progress", {
        detail: {
          modelId: whisperModelsStore.downloadingModelId,
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
      window.removeEventListener(TIERS_UPDATED_EVENT, syncTiersFromStorage);
      window.removeEventListener(TRANSCRIBE_TIERS_UPDATED_EVENT, syncTranscribeTiersFromStorage);
      if (unlistenProg) unlistenProg();
    };
  });

  function syncTiersFromStorage() {
    tiers = loadTiers();
  }

  function syncTranscribeTiersFromStorage() {
    transcribeTiers = loadTranscribeTiers();
  }

  let needsQuickSetup = $derived(whisperModelsStore.downloadedWhisperCount === 0 || !isDefaultLlmReady);

  // Shared with the whisper section's model grid (see WhisperSettingsPanel.svelte)
  // and the llm section's refinement-prompt editor -- stays here since both
  // read it.
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

  $effect(() => {
    if (typeof window === "undefined") return;
    publishSettingsActionState(
      buildSettingsActionHash({
        needsWhisper: whisperModelsStore.downloadedWhisperCount === 0 && !aiStore.killSwitchActive,
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

  function loadApiKeys() {
    apiKeys = loadAndValidateApiKeys();
  }

  function saveApiKeys() {
    vestaConfig.setItem("srt-tools-api-keys", JSON.stringify(apiKeys));
    // Dispatch custom event to notify other tabs in the same window
    window.dispatchEvent(new CustomEvent("apikeys-updated"));
  }

  function generateId(): string {
    return Date.now().toString(36) + Math.random().toString(36).substr(2);
  }

  function openAddKeyModal(providerId?: string) {
    apiKeyEditorStore.openAdd(activeSettingsSection === "whisper" ? "whisper" : "llm", providerId);
  }

  function openEditKeyModal(id: string) {
    const key = apiKeys.find((k) => k.id === id);
    if (!key) return;
    apiKeyEditorStore.openEdit(key, activeSettingsSection === "whisper" ? "whisper" : "llm");
  }

  function addApiKey() {
    const editKeyId = apiKeyEditorStore.editKeyId;
    const newKeyType = apiKeyEditorStore.newKeyType;
    const newKeyName = apiKeyEditorStore.newKeyName;
    const newKeyValue = apiKeyEditorStore.newKeyValue;
    const newKeyUrl = apiKeyEditorStore.newKeyUrl;

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

    apiKeyEditorStore.reset();
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

  function onModelClick(model: ModelInfo) {
    openAddKeyModal(model.provider);
  }

</script>

<div
  class="h-full flex flex-col bg-gray-900 overflow-hidden"
>
  {#if activeSettingsSection === 'shortcuts'}
    <div class="flex-1 flex flex-col min-h-0">
      <ShortcutsTab />
    </div>
  {:else}
    <!-- Scrollable content area -->
    <div class="settings-scroll flex-1 overflow-y-auto p-6">


  {#if activeSettingsSection === "overview"}
    <OverviewSettingsPanel {defaultLanguagesCard} />
  {/if}

  {#if activeSettingsSection === "language"}
  <div class="mb-6 flex flex-col gap-4">
    {@render defaultLanguagesCard()}

    <!-- Smart Matching Card -->
    {#if uiMode.expertMode}
      <div class="glass-card p-6 flex flex-col gap-4">
        <ToggleRow
          label={t("settings.smartMatching.title")}
          checked={smartMatchingEnabled}
          onchange={toggleSmartMatching}
          accent="violet"
          iconPath="M13 10V3L4 14h7v7l9-11h-7z"
        />

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
    <!-- Keep the card lifted so language dropdowns paint over the cards below. -->
    <div class="glass-card p-6 relative z-30">
      <div class="grid grid-cols-1 md:grid-cols-2 2xl:grid-cols-4 gap-5">
        <div class="relative z-40 rounded-xl border border-transparent bg-white/5 p-5">
          <div class="flex items-center justify-between gap-3 mb-4">
            <span class="flex items-center gap-2 text-sm font-semibold text-white"><span class="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-500/15 text-emerald-300"><svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 3a4 4 0 110 8 4 4 0 010-8zM5 21a7 7 0 0114 0"/></svg></span>{s("nativeLanguage")}</span>
            <span class="text-3xl">{languages.find((lang) => lang.code === defaultNativeLanguage)?.flag || "🌐"}</span>
          </div>
          <LanguageSelect
            className="language-select"
            noResultsText={t("common.noResults")}
            value={defaultNativeLanguage}
            onchange={(v) => {
              defaultNativeLanguage = v;
              saveDefaultLanguage(DEFAULT_NATIVE_LANGUAGE_KEY, v);
            }}
            placeholder={t("flashcards.noteTypeLanguagePlaceholder")}
          />
          <p class="mt-3 text-xs leading-relaxed text-gray-500">{s("nativeLanguageHint")}</p>
        </div>
        <div class="relative z-30 rounded-xl border border-transparent bg-white/5 p-5">
          <div class="flex items-center justify-between gap-3 mb-4">
            <span class="flex items-center gap-2 text-sm font-semibold text-white"><span class="flex h-8 w-8 items-center justify-center rounded-lg bg-violet-500/15 text-violet-300">文</span>{s("studyingLanguage")}</span>
            <span class="text-3xl">{languages.find((lang) => lang.code === defaultFlashcardsLanguage)?.flag || "🌐"}</span>
          </div>
          <LanguageSelect
            className="language-select"
            noResultsText={t("common.noResults")}
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
        <div class="relative z-20 rounded-xl border border-transparent bg-white/5 p-5">
          <div class="flex items-center justify-between gap-3 mb-4">
            <span class="flex items-center gap-2 text-sm font-semibold text-white"><span class="flex h-8 w-8 items-center justify-center rounded-lg bg-cyan-500/15 text-cyan-300"><svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M7 7h11m0 0-3-3m3 3-3 3M17 17H6m0 0 3 3m-3-3 3-3"/></svg></span>{s("translationLanguage")}</span>
            <span class="text-3xl">{languages.find((lang) => lang.code === defaultTargetLanguage)?.flag || "🌐"}</span>
          </div>
          <LanguageSelect
            className="language-select"
            noResultsText={t("common.noResults")}
            value={defaultTargetLanguage}
            onchange={(v) => {
              defaultTargetLanguage = v;
              saveDefaultLanguage(DEFAULT_TARGET_LANGUAGE_KEY, v);
            }}
            placeholder={t("translate.targetLang")}
          />
          <p class="mt-3 text-xs leading-relaxed text-gray-500">{s("translationLanguageHint")}</p>
        </div>
        <div class="relative z-10 rounded-xl border border-transparent bg-white/5 p-5">
          <div class="flex items-center justify-between gap-3 mb-4">
            <span class="flex items-center gap-2 text-sm font-semibold text-white"><span class="flex h-8 w-8 items-center justify-center rounded-lg bg-amber-500/15 text-amber-300"><svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 3a3 3 0 00-3 3v6a3 3 0 006 0V6a3 3 0 00-3-3zM5 11a7 7 0 0014 0M12 18v3m-4 0h8"/></svg></span>{s("transcription")}</span>
            <span class="text-3xl">{defaultTranscribeLanguage === "auto" ? "🌐" : languages.find((lang) => lang.code === defaultTranscribeLanguage)?.flag || "🌐"}</span>
          </div>
          <LanguageSelect
            className="language-select"
            noResultsText={t("common.noResults")}
            autoDetect
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

  {#if activeSettingsSection === "llm" && !aiStore.killSwitchActive}
    <ApiKeysCard
      title={s("apiKeysSaved")}
      addButtonLabel={s("addProviderButton")}
      defaultProvider="custom"
      {apiKeys}
      onAddKey={openAddKeyModal}
      onEditKey={openEditKeyModal}
      onDeleteKey={askDeleteApiKey}
      onSetDefault={setDefaultKey}
    />

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
            <h3 class="text-sm font-bold text-white">{t("settings.refinementPromptTitle")}</h3>
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
    <WhisperSettingsPanel
      {s}
      {apiKeys}
      onAddKey={openAddKeyModal}
      onEditKey={openEditKeyModal}
      onDeleteKey={askDeleteApiKey}
      onSetDefault={setDefaultKey}
      {highlightedModelId}
      {whisperEngine}
    />
  {/if}

  {#if activeSettingsSection === "anki"}
    <AnkiSettingsPanel {s} />
  {/if}

  {#if activeSettingsSection === "diagnostics"}
    <DiagnosticsSettingsPanel />
  {/if}
  </div>

  <!-- Fixed Bottom Band: reset è un'azione rara e distruttiva, quindi piccola
       e in un angolo (con conferma) invece di un bottone rosso gigante al
       centro di ogni sezione di Settings. -->
  {#if activeSettingsSection !== "diagnostics"}
  <FooterActions justify="end">
    {#snippet right()}
      <button
        onclick={() => {
          if (activeSettingsSection !== "shortcuts" && activeSettingsSection !== "diagnostics") {
            showResetConfirm = activeSettingsSection;
          }
        }}
        class="px-3.5 py-2 bg-red-500/10 hover:bg-red-500/20 border border-red-500/30 text-red-300 rounded-lg font-semibold text-xs transition-colors flex items-center gap-1.5 cursor-pointer"
      >
        <svg
          class="w-3.5 h-3.5"
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
        {t("settings.resetDefaults")}
      </button>
    {/snippet}
  </FooterActions>
  {/if}
  {/if}

  <!-- Reset Confirmation Dialog -->
  <ConfirmDialog
    show={!!showResetConfirm}
    title={resetTitle}
    message={resetMessage}
    confirmText={t("settings.resetDefaults")}
    cancelText={t("common.cancel")}
    variant="danger"
    on:cancel={() => (showResetConfirm = null)}
    on:confirm={() => {
      if (showResetConfirm === "overview") {
        resetOverviewSettings();
      } else if (showResetConfirm === "llm") {
        resetLlmSettings();
      } else if (showResetConfirm === "whisper") {
        whisperModelsStore.resetAll();
      } else if (showResetConfirm === "language") {
        resetLanguageSettings();
      } else if (showResetConfirm === "anki") {
        ankiTemplateStore.resetAll();
      }
      showResetConfirm = null;
    }}
  />

  <AddApiKeyModal onSave={addApiKey} />

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

  :global(.language-select .searchable-select-input) {
    background: rgba(24, 24, 42, 0.98) !important;
    border: 1px solid rgba(148, 163, 184, 0.22);
  }

  :global(.language-select .searchable-select-dropdown) {
    background: #0f172a !important;
  }

  :global(.language-select .searchable-select-option) {
    background: #0f172a !important;
  }

</style>
