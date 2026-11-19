<script lang="ts">
  import { settledScroll } from "$lib/utils/settledScroll";
  import { onMount } from "svelte";
  import { diagnosticsStore } from "$lib/stores/diagnosticsStore.svelte";
  import { listen } from "@tauri-apps/api/event";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";
  import FooterActions from "$lib/components/FooterActions.svelte";
  import ShortcutsTab from "$lib/tabs/ShortcutsTab.svelte";
  import AddApiKeyModal from "$lib/modals/AddApiKeyModal.svelte";
  import { apiKeyEditorStore } from "$lib/stores/apiKeyEditorStore.svelte";
  import AnkiSettingsPanel from "$lib/panels/AnkiSettingsPanel.svelte";
  import { ankiTemplateStore } from "$lib/stores/ankiTemplateStore.svelte";
  import WhisperSettingsPanel from "$lib/panels/WhisperSettingsPanel.svelte";
  import IttSettingsPanel from "$lib/panels/IttSettingsPanel.svelte";
  import { whisperModelsStore } from "$lib/stores/whisperModelsStore.svelte";
  import { ocrModelsStore } from "$lib/stores/ocrModelsStore.svelte";
  import { saveIttTiers } from "$lib/config/ittTiers";
  import OverviewSettingsPanel from "$lib/panels/OverviewSettingsPanel.svelte";
  import DiagnosticsSettingsPanel from "$lib/panels/DiagnosticsSettingsPanel.svelte";
  import { cpuRamStore } from "$lib/stores/cpuRamStore.svelte";
  import { exportFormatStore } from "$lib/stores/exportFormatStore.svelte";
  import { updateCheckerStore } from "$lib/stores/updateCheckerStore.svelte";
  import { smartMatchingStore } from "$lib/stores/smartMatchingStore.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import { ankiStore } from "$lib/stores/ankiStore.svelte";
  import { locale, setLanguage } from "$lib/i18n";
  import { buildSettingsActionHash, publishSettingsActionState } from "$lib/utils/settingsNotifications";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import { DEFAULT_REFINEMENT_PROMPT, REFINEMENT_PROMPT_STORAGE_KEY, loadRefinementPrompt } from "$lib/config/refinementPrompt";
  import { isBuiltinEndpointId, loadAndValidateApiKeys, normalizeApiKeyDefaults, apiKeyDefaultGroup, supportsCapability, type ApiKeyConfig } from "$lib/config/apiKeys";
  import { loadTiers, tiersHaveUsableEntries, TIERS_UPDATED_EVENT, type Tier } from "$lib/config/translationTiers";
  import { loadTranscribeCloud } from "$lib/config/transcribeProviders";
  import * as vestaConfig from "$lib/config/vestaConfig";
  import { DefaultLlmProviderSettings } from "$lib/workflows/defaultLlmProviderSettings.svelte";
  import { fetchModelsFromEndpoint } from "$lib/services/modelDiscovery";
  import { SmartMatchingRulesDraft } from "$lib/workflows/smartMatchingRulesDraft.svelte";
  import { validateAndApplyApiKey } from "./settings/apiKeyValidation";
  import DefaultLanguagesCard from "./settings/DefaultLanguagesCard.svelte";
  import LanguageSettingsPanel from "./settings/LanguageSettingsPanel.svelte";
  import LlmSettingsPanel from "./settings/LlmSettingsPanel.svelte";

  type SettingsSection = "overview" | "llm" | "whisper" | "itt" | "language" | "anki" | "diagnostics" | "shortcuts";
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
    try { return vestaConfig.getItem(key) || fallback; } catch { return fallback; }
  }

  const defaultLlmSettings = new DefaultLlmProviderSettings(
    {
      provider: loadStoredValue(DEFAULT_LLM_PROVIDER_KEY, "local"),
      model: loadStoredValue(DEFAULT_LLM_MODEL_KEY, ""),
      customProviderId: loadStoredValue(DEFAULT_LLM_CUSTOM_PROVIDER_KEY, ""),
      localServerUrl: loadStoredValue(LOCAL_SERVER_URL_KEY, DEFAULT_LOCAL_URL),
    },
    (settings) => {
      vestaConfig.setItem(DEFAULT_LLM_PROVIDER_KEY, settings.provider);
      vestaConfig.setItem(DEFAULT_LLM_MODEL_KEY, settings.model);
      vestaConfig.setItem(DEFAULT_LLM_CUSTOM_PROVIDER_KEY, settings.customProviderId);
      vestaConfig.setItem(LOCAL_SERVER_URL_KEY, settings.localServerUrl);
      window.dispatchEvent(new CustomEvent("vesta-llm-default-updated"));
    },
    fetchModelsFromEndpoint,
  );

  let t = $derived($locale);
  let apiKeys = $state<ApiKeyConfig[]>([]);
  let tiers = $state<Tier[]>([]);
  let activeSettingsSection = $state<SettingsSection>("overview");
  let lastRequestedSection = $state<SettingsSection>("overview");
  let defaultTargetLanguage = $state(loadStoredValue(DEFAULT_TARGET_LANGUAGE_KEY, "en"));
  let defaultTranscribeLanguage = $state(loadStoredValue(DEFAULT_TRANSCRIBE_LANGUAGE_KEY, "auto"));
  let defaultFlashcardsLanguage = $state(loadStoredValue(DEFAULT_FLASHCARDS_LANGUAGE_KEY, "en"));
  let defaultNativeLanguage = $state(loadStoredValue(DEFAULT_NATIVE_LANGUAGE_KEY, "en"));
  let defaultRefinementPrompt = $state(loadRefinementPrompt());
  let whisperEngine = $state(loadTranscribeCloud().engine || "local");

  function persistRefinementPrompt() {
    try { vestaConfig.setItem(REFINEMENT_PROMPT_STORAGE_KEY, defaultRefinementPrompt); } catch {}
  }
  function resetRefinementPrompt() {
    defaultRefinementPrompt = DEFAULT_REFINEMENT_PROMPT;
    persistRefinementPrompt();
  }
  function showSnackbar(message: string, type: "error" | "success" = "success") {
    snackbar.show(message, type === "error" ? "error" : "success");
  }
  function s(key: string): string {
    return t(`settings.copy.${key}`);
  }

  // Keep reference to copy/status keys checked by check-i18n while settings panels share translations
  void (() => [
    s("transcription"), s("setupWhisperDesc"), s("translation"), s("setupLlmDesc"),
    s("macroArea"), s("llmTitle"), s("llmDesc"), s("whisperTitle"), s("whisperDesc"),
    s("languageTitle"), s("languageDesc"), s("ankiTitle"), s("ankiDesc"),
    t("settings.providerStatus.available"), t("settings.providerStatus.checking"),
    t("settings.providerStatus.offline"), t("settings.providerStatus.requiresKey"),
    t("settings.endpointStatus.checking"),
  ]);

  let savedCustomProviders = $derived(
    apiKeys.filter((k) => k.apiType === "custom" && supportsCapability(k, "llm") && k.apiUrl && k.apiUrl.trim().length > 0),
  );
  let selectedDefaultCustomProvider = $derived(
    savedCustomProviders.find((key) => key.id === defaultLlmSettings.customProviderId),
  );
  let activeDefaultEndpointUrl = $derived.by(() => {
    if (defaultLlmSettings.provider === "local") return defaultLlmSettings.localServerUrl;
    if (defaultLlmSettings.provider === "custom") return selectedDefaultCustomProvider?.apiUrl || "";
    return "";
  });
  let activeDefaultEndpointApiKey = $derived(
    defaultLlmSettings.provider === "custom" ? selectedDefaultCustomProvider?.apiKey || "" : "",
  );
  let isDefaultLlmReady = $derived(tiersHaveUsableEntries(tiers));

  function openSettingsSection(section: SettingsSection) {
    activeSettingsSection = section;
    requestedSection = section;
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

  function hasDefaultProviderAccess(providerId: string) {
    if (providerId === "local") return true;
    if (providerId === "custom") return savedCustomProviders.length > 0;
    return apiKeys.some((key) => key.apiType === providerId);
  }

  async function refreshDefaultEndpointModels() {
    await defaultLlmSettings.refreshEndpointModels({
      endpointUrl: activeDefaultEndpointUrl.trim(),
      apiKey: activeDefaultEndpointApiKey,
      messages: {
        idle: t("settings.endpointStatus.idle"),
        checking: t("settings.endpointStatus.checking"),
        offline: t("settings.endpointStatus.offline"),
        available: (count) => t("settings.modelsAvailable", { count }),
      },
    });
  }

  async function refreshLocalProviderStatus() {
    await defaultLlmSettings.refreshLocalProviderStatus(defaultLlmSettings.localServerUrl);
  }

  $effect(() => {
    if (!hasDefaultProviderAccess(defaultLlmSettings.provider)) defaultLlmSettings.reconcileAccess(false);
  });
  $effect(() => {
    defaultLlmSettings.reconcileAccess(true, savedCustomProviders[0]?.id);
  });

  $effect(() => {
    if (!active) return;
    const provider = defaultLlmSettings.provider;
    const endpointUrl = activeDefaultEndpointUrl;
    const endpointApiKey = activeDefaultEndpointApiKey;
    if (provider !== "local" && provider !== "custom") {
      defaultLlmSettings.invalidateEndpointCheck();
      defaultLlmSettings.endpointStatus = "idle";
      defaultLlmSettings.endpointMessage = t("settings.endpointStatus.idle");
      defaultLlmSettings.discoveredModels = [];
      defaultLlmSettings.checkingEndpoint = false;
      return;
    }
    if (provider === "local" && defaultLlmSettings.localProviderStatus !== "idle" && defaultLlmSettings.localProviderStatus !== "checking") {
      defaultLlmSettings.endpointStatus = defaultLlmSettings.localProviderStatus;
      defaultLlmSettings.endpointMessage = defaultLlmSettings.localProviderStatus === "online" ? t("settings.endpointStatus.online") : t("settings.endpointStatus.offline");
    }
    const timeout = setTimeout(() => { void refreshDefaultEndpointModels(); }, 300);
    const interval = setInterval(() => { void refreshDefaultEndpointModels(); }, 15000);
    return () => {
      void endpointUrl;
      void endpointApiKey;
      clearTimeout(timeout);
      clearInterval(interval);
      defaultLlmSettings.invalidateEndpointCheck();
      defaultLlmSettings.checkingEndpoint = false;
    };
  });

  $effect(() => {
    if (!active) return;
    const endpointUrl = defaultLlmSettings.localServerUrl;
    const timeout = setTimeout(() => { void refreshLocalProviderStatus(); }, 300);
    const interval = setInterval(() => { void refreshLocalProviderStatus(); }, 15000);
    return () => {
      void endpointUrl;
      clearTimeout(timeout);
      clearInterval(interval);
      defaultLlmSettings.invalidateLocalCheck();
    };
  });

  function saveDefaultLanguage(key: string, value: string) {
    vestaConfig.setItem(key, value);
    window.dispatchEvent(new CustomEvent("vesta-language-defaults-updated"));
  }

  let smartMatchingEnabled = $derived(smartMatchingStore.enabled);
  const smartMatchingRulesDraft = new SmartMatchingRulesDraft(
    smartMatchingStore,
    {
      invalidObject: () => t("settings.smartMatching.invalidObject"),
      invalidJson: () => t("settings.smartMatching.invalidJson"),
      saveSuccess: () => t("settings.smartMatching.saveSuccess"),
      saveError: () => t("settings.smartMatching.saveError"),
    },
    (message) => snackbar.show(message, "success"),
  );

  let showResetConfirm = $state<"style" | "fields" | "overview" | "llm" | "whisper" | "itt" | "language" | "anki" | null>(null);
  let resetTitle = $derived(
    showResetConfirm && ["overview", "llm", "whisper", "itt", "language", "anki"].includes(showResetConfirm)
      ? t(`settings.${showResetConfirm}.resetConfirmTitle`)
      : t("settings.resetConfirmTitle")
  );
  let resetMessage = $derived(
    showResetConfirm && ["overview", "llm", "whisper", "itt", "language", "anki"].includes(showResetConfirm)
      ? t(`settings.${showResetConfirm}.resetConfirmDesc`)
      : t("settings.resetConfirmDesc")
  );

  $effect(() => {
    void uiMode.expertMode;
    try {
      const savedCores = vestaConfig.getItem("vesta_cpu_cores");
      if (savedCores) cpuRamStore.cpuCores = parseInt(savedCores);
    } catch {}
  });

  $effect(() => {
    if (!uiMode.expertMode) {
      if (activeSettingsSection === "language" || activeSettingsSection === "anki") activeSettingsSection = "overview";
      if (requestedSection === "language" || requestedSection === "anki") requestedSection = "overview";
    }
  });

  function resetOverviewSettings() {
    setLanguage("en");
    resetDefaultLanguages();
    cpuRamStore.setCores(cpuRamStore.maxCpuCores);
    cpuRamStore.setRamLimitMb(0);
    exportFormatStore.setExportFormat("apkg");
    snackbar.show(t("settings.overview.resetSuccess"), "info");
  }

  function resetLlmSettings() {
    defaultLlmSettings.resetLocal(DEFAULT_LOCAL_URL);
    resetRefinementPrompt();
    defaultLlmSettings.persist();
    snackbar.show(t("settings.llm.resetSuccess"), "info");
  }

  function resetDefaultLanguages() {
    defaultTargetLanguage = "en";
    defaultTranscribeLanguage = "auto";
    defaultFlashcardsLanguage = "en";
    defaultNativeLanguage = "en";
    saveDefaultLanguage(DEFAULT_TARGET_LANGUAGE_KEY, "en");
    saveDefaultLanguage(DEFAULT_TRANSCRIBE_LANGUAGE_KEY, "auto");
    saveDefaultLanguage(DEFAULT_FLASHCARDS_LANGUAGE_KEY, "en");
    saveDefaultLanguage(DEFAULT_NATIVE_LANGUAGE_KEY, "en");
    saveDefaultLanguage(NOTE_TYPE_LANGUAGE_KEY, "en");
  }

  function resetLanguageSettings() {
    resetDefaultLanguages();
    smartMatchingStore.resetRules();
    smartMatchingStore.setEnabled(true);
    smartMatchingRulesDraft.loadFromStore();
    snackbar.show(t("settings.language.resetSuccess"), "info");
  }

  const syncTiersFromStorage = () => { tiers = loadTiers(); };

  onMount(() => {
    void diagnosticsStore.refresh();
    void cpuRamStore.init();
    void updateCheckerStore.init();
    loadApiKeys();
    smartMatchingRulesDraft.loadFromStore();
    whisperModelsStore.defaultWhisperModel = vestaConfig.getItem("srt-default-whisper-model") || "base";
    whisperModelsStore.refreshModels().catch((e) => console.error("Could not list models:", e));
    void whisperModelsStore.refreshAddons();
    ocrModelsStore.defaultOcrModel = vestaConfig.getItem("vesta-default-ocr-model") || "paddleocr-v4-mobile-multi";
    ocrModelsStore.refreshModels().catch((e) => console.error("Could not list OCR models:", e));

    const handleKeydown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (deleteConfirmId) cancelDelete();
        else if (apiKeyEditorStore.showAddKey) apiKeyEditorStore.close();
      }
    };
    const handleOpenAddKeyModalEvent = (event: Event) => {
      const detail = (event as CustomEvent<{ providerId?: string; context?: "llm" | "whisper" | "itt" }>).detail;
      const providerId = detail?.providerId;
      const context = detail?.context ?? (providerId === "custom_whisper" ? "whisper" : activeSettingsSection === "itt" ? "itt" : "llm");
      apiKeyEditorStore.openAdd(context, providerId === "custom_whisper" ? "whisper" : providerId);
    };

    tiers = loadTiers();
    window.addEventListener("keydown", handleKeydown);
    window.addEventListener("vesta-open-settings-section", handleOpenSettingsSectionEvent);
    window.addEventListener("vesta-open-add-key-modal", handleOpenAddKeyModalEvent);
    window.addEventListener(TIERS_UPDATED_EVENT, syncTiersFromStorage);

    let activeListener = true;
    let unlistenProg: (() => void) | null = null;
    let unlistenOcrProg: (() => void) | null = null;
    listen<{ stage: string; message: string; percentage: number }>("transcribe-progress", (event) => {
      const p = event.payload;
      if (p.stage !== "download" || (!whisperModelsStore.isDownloading && !whisperModelsStore.downloadingVadId)) return;
      whisperModelsStore.progress = Math.round(p.percentage);
      whisperModelsStore.progressMessage = p.message;
      whisperModelsStore.progressStage = p.stage;
      window.dispatchEvent(new CustomEvent("vesta-whisper-download-progress", {
        detail: { modelId: whisperModelsStore.downloadingModelId, progress: Math.round(p.percentage), stage: p.stage, message: p.message },
      }));
    }).then((fn) => { if (!activeListener) fn(); else unlistenProg = fn; }).catch(console.error);

    listen<{ stage: string; message: string; percentage: number }>("ocr-progress", (event) => {
      const p = event.payload;
      if (p.stage !== "download" || !ocrModelsStore.isDownloading) return;
      ocrModelsStore.progress = Math.round(p.percentage);
      ocrModelsStore.progressMessage = p.message;
    }).then((fn) => { if (!activeListener) fn(); else unlistenOcrProg = fn; }).catch(console.error);

    return () => {
      activeListener = false;
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener("vesta-open-settings-section", handleOpenSettingsSectionEvent);
      window.removeEventListener("vesta-open-add-key-modal", handleOpenAddKeyModalEvent);
      window.removeEventListener(TIERS_UPDATED_EVENT, syncTiersFromStorage);
      if (unlistenProg) unlistenProg();
      if (unlistenOcrProg) unlistenOcrProg();
    };
  });

  let highlightedModelId = $state<string | null>(null);
  $effect(() => {
    if (!highlightItemId) return;
    highlightedModelId = highlightItemId;
    const targetId = highlightItemId === "notification-whisper" ? "whisper-models" : highlightItemId;
    const scrollTimer = setTimeout(() => {
      const el = targetId ? document.getElementById(targetId) : null;
      if (el) {
        el.scrollIntoView({ behavior: "smooth", block: "center" });
        if (el.tagName === "TEXTAREA" || el.tagName === "INPUT") el.focus();
      }
    }, 100);
    const timer = setTimeout(() => { highlightedModelId = null; highlightItemId = null; }, 1500);
    return () => { clearTimeout(scrollTimer); clearTimeout(timer); };
  });

  $effect(() => {
    if (typeof window === "undefined") return;
    publishSettingsActionState(buildSettingsActionHash({
      needsWhisper: whisperModelsStore.downloadedWhisperCount === 0 && !aiStore.killSwitchActive,
      needsLlm: !isDefaultLlmReady && !aiStore.killSwitchActive,
    }));
  });

  $effect(() => {
    if (aiStore.killSwitchActive) {
      if (activeSettingsSection === "llm" || activeSettingsSection === "whisper") activeSettingsSection = "overview";
      if (requestedSection === "llm" || requestedSection === "whisper") requestedSection = "overview";
    }
  });

  function loadApiKeys() { apiKeys = loadAndValidateApiKeys(); }
  function saveApiKeys() {
    apiKeys = normalizeApiKeyDefaults(apiKeys);
    vestaConfig.setItem("srt-tools-api-keys", JSON.stringify(apiKeys));
    window.dispatchEvent(new CustomEvent("apikeys-updated"));
  }
  const generateId = () => Date.now().toString(36) + Math.random().toString(36).substr(2);
  const openAddKeyModal = (providerId?: string) => {
    const context = activeSettingsSection === "whisper" ? "whisper" : activeSettingsSection === "itt" ? "itt" : "llm";
    apiKeyEditorStore.openAdd(context, providerId);
  };
  function openEditKeyModal(id: string) {
    const key = apiKeys.find((k) => k.id === id);
    const context = activeSettingsSection === "whisper" ? "whisper" : activeSettingsSection === "itt" ? "itt" : "llm";
    if (key) apiKeyEditorStore.openEdit(key, context);
  }

  function addApiKey() {
    const outcome = validateAndApplyApiKey(
      apiKeys,
      {
        editKeyId: apiKeyEditorStore.editKeyId,
        newKeyType: apiKeyEditorStore.newKeyType,
        newKeyName: apiKeyEditorStore.newKeyName,
        newKeyValue: apiKeyEditorStore.newKeyValue,
        newKeyUrl: apiKeyEditorStore.newKeyUrl,
        newKeyUsage: apiKeyEditorStore.newKeyUsage,
      },
      generateId,
    );
    if (!outcome.ok) { showSnackbar(t(outcome.errorKey), "error"); return; }

    apiKeys = outcome.nextKeys;
    saveApiKeys();
    if (outcome.isEdit) {
      if (apiKeyEditorStore.newKeyType === "custom" && defaultLlmSettings.customProviderId === apiKeyEditorStore.editKeyId) {
        void refreshDefaultEndpointModels();
      }
      showSnackbar(t("settings.keyUpdated"));
    } else {
      showSnackbar(t("settings.keyAdded"));
    }

    if (apiKeyEditorStore.editKeyId === "local" && apiKeyEditorStore.newKeyType === "local" && outcome.resolvedUrl) {
      defaultLlmSettings.localServerUrl = outcome.resolvedUrl;
      defaultLlmSettings.persist();
    }
    apiKeyEditorStore.reset();
  }

  let deleteConfirmId = $state<string | null>(null);
  let deleteConfirmName = $state("");

  function askDeleteApiKey(id: string) {
    const key = apiKeys.find((k) => k.id === id);
    if (!key || isBuiltinEndpointId(id)) return;
    deleteConfirmId = id;
    deleteConfirmName = key.name;
  }
  function cancelDelete() { deleteConfirmId = null; deleteConfirmName = ""; }
  function confirmDeleteApiKey() {
    if (!deleteConfirmId) return;
    const key = apiKeys.find((k) => k.id === deleteConfirmId);
    if (!key) { cancelDelete(); return; }
    if (apiKeyEditorStore.editKeyId === deleteConfirmId) apiKeyEditorStore.reset();
    apiKeys = apiKeys.filter((k) => k.id !== deleteConfirmId);
    saveApiKeys();
    showSnackbar(t("settings.keyDeleted"));
    cancelDelete();
  }
  function setDefaultKey(id: string) {
    const key = apiKeys.find((k) => k.id === id);
    if (!key) return;
    apiKeys = apiKeys.map((k) => ({
      ...k,
      isDefault: apiKeyDefaultGroup(k) === apiKeyDefaultGroup(key) ? k.id === id : k.isDefault,
    }));
    saveApiKeys();
  }
</script>

<div class="h-full flex flex-col bg-gray-900 overflow-hidden">
  {#if activeSettingsSection === "shortcuts"}
    <div class="flex-1 flex flex-col min-h-0"><ShortcutsTab /></div>
  {:else}
    <div use:settledScroll data-dense={activeSettingsSection === "llm" || activeSettingsSection === "whisper" || activeSettingsSection === "itt"} class="settings-scroll stable-settings-layout flex-1 min-h-0 overflow-y-auto p-6">
      {#snippet defaultLanguagesCard()}
        <DefaultLanguagesCard
          {defaultNativeLanguage}
          {defaultFlashcardsLanguage}
          {defaultTargetLanguage}
          {defaultTranscribeLanguage}
          onChangeNative={(v) => { defaultNativeLanguage = v; saveDefaultLanguage(DEFAULT_NATIVE_LANGUAGE_KEY, v); }}
          onChangeFlashcards={(v) => {
            defaultFlashcardsLanguage = v;
            saveDefaultLanguage(DEFAULT_FLASHCARDS_LANGUAGE_KEY, v);
            saveDefaultLanguage(NOTE_TYPE_LANGUAGE_KEY, v);
          }}
          onChangeTarget={(v) => { defaultTargetLanguage = v; saveDefaultLanguage(DEFAULT_TARGET_LANGUAGE_KEY, v); }}
          onChangeTranscribe={(v) => { defaultTranscribeLanguage = v; saveDefaultLanguage(DEFAULT_TRANSCRIBE_LANGUAGE_KEY, v); }}
        />
      {/snippet}

      {#if activeSettingsSection === "overview"}
        <OverviewSettingsPanel {defaultLanguagesCard} />
      {:else if activeSettingsSection === "language"}
        <LanguageSettingsPanel
          {defaultLanguagesCard}
          {smartMatchingEnabled}
          bind:rulesDraftValue={smartMatchingRulesDraft.value}
          rulesDraftError={smartMatchingRulesDraft.error}
          onToggleSmartMatching={() => smartMatchingRulesDraft.toggle()}
          onSaveSmartMatchingRules={() => smartMatchingRulesDraft.save()}
        />
      {:else if activeSettingsSection === "llm" && !aiStore.killSwitchActive}
        <LlmSettingsPanel
          {active}
          {apiKeys}
          {highlightedModelId}
          bind:defaultRefinementPrompt
          onAddKey={openAddKeyModal}
          onEditKey={openEditKeyModal}
          onDeleteKey={askDeleteApiKey}
          onSetDefault={setDefaultKey}
          onPersistPrompt={persistRefinementPrompt}
        />
      {:else if activeSettingsSection === "whisper" && !aiStore.killSwitchActive}
        <WhisperSettingsPanel
          {s}
          apiKeys={apiKeys.filter((key) => supportsCapability(key, "stt"))}
          onAddKey={openAddKeyModal}
          onEditKey={openEditKeyModal}
          onDeleteKey={askDeleteApiKey}
          onSetDefault={setDefaultKey}
          {highlightedModelId}
          {whisperEngine}
        />
      {:else if activeSettingsSection === "itt"}
        <IttSettingsPanel
          {active}
          {s}
          apiKeys={apiKeys.filter((key) => supportsCapability(key, "itt"))}
          onAddKey={openAddKeyModal}
          onEditKey={openEditKeyModal}
          onDeleteKey={askDeleteApiKey}
          onSetDefault={setDefaultKey}
          {highlightedModelId}
        />
      {:else if activeSettingsSection === "anki"}
        <AnkiSettingsPanel {s} />
      {:else if activeSettingsSection === "diagnostics"}
        <DiagnosticsSettingsPanel />
      {/if}
    </div>

    <FooterActions justify="end">
      {#snippet right()}
        {#if activeSettingsSection !== "diagnostics"}
          <button
            onclick={() => {
              if (activeSettingsSection !== "shortcuts" && activeSettingsSection !== "diagnostics") {
                showResetConfirm = activeSettingsSection;
              }
            }}
            class="px-3.5 py-2 bg-red-500/10 hover:bg-red-500/20 border border-red-500/30 text-red-300 rounded-lg font-semibold text-xs transition-colors flex items-center gap-1.5 cursor-pointer"
          >
            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            {t("settings.resetDefaults")}
          </button>
        {/if}
      {/snippet}
    </FooterActions>
  {/if}

  <ConfirmDialog
    show={!!showResetConfirm}
    title={resetTitle}
    message={resetMessage}
    confirmText={t("settings.resetDefaults")}
    cancelText={t("common.cancel")}
    variant="danger"
    on:cancel={() => (showResetConfirm = null)}
    on:confirm={() => {
      if (showResetConfirm === "overview") resetOverviewSettings();
      else if (showResetConfirm === "llm") resetLlmSettings();
      else if (showResetConfirm === "whisper") {
        whisperModelsStore.resetAll();
      }
      else if (showResetConfirm === "itt") {
        ocrModelsStore.resetAll();
        saveIttTiers([]);
      }
      else if (showResetConfirm === "language") resetLanguageSettings();
      else if (showResetConfirm === "anki") {
        ankiTemplateStore.resetAll();
        ankiStore.setUrl("http://127.0.0.1:8765");
        ankiStore.setDictionaryLookup(true);
        ankiStore.setAutoCardFont(true);
        ankiStore.setEmbedCardFont(true);
      }
      showResetConfirm = null;
    }}
  />

  <AddApiKeyModal onSave={addApiKey} onDelete={askDeleteApiKey} />

  <ConfirmDialog
    show={!!deleteConfirmId}
    title={t("settings.modal.deleteProvider")}
    message={t("settings.confirmDeleteKey", { name: deleteConfirmName })}
    confirmText={t("settings.confirmDelete")}
    cancelText={t("settings.modal.cancel")}
    variant="danger"
    on:cancel={cancelDelete}
    on:confirm={confirmDeleteApiKey}
  />
</div>

<style>
  :global(.notification-target-pulse) {
    border-radius: 1rem;
    animation: notification-target-pulse .5s ease-in-out 3;
  }
  @keyframes notification-target-pulse {
    0%, 100% { outline: 2px solid transparent; box-shadow: none; }
    50% { outline: 2px solid var(--color-yellow-300); box-shadow: 0 0 20px color-mix(in srgb, var(--color-yellow-300) 30%, transparent); }
  }
  @media (prefers-reduced-motion: reduce) {
    :global(.notification-target-pulse) { animation: none; outline: 2px solid var(--color-yellow-300); }
  }

  ::-webkit-scrollbar { width: 6px; height: 6px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.1); border-radius: 3px; }
  ::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.2); }

  :global(.language-select .searchable-select-input) {
    background: rgba(24, 24, 42, 0.98) !important;
    border: 1px solid rgba(148, 163, 184, 0.22);
  }
  :global(.language-select .searchable-select-dropdown) { background: #0f172a !important; }
  :global(.language-select .searchable-select-option) { background: #0f172a !important; }
</style>
