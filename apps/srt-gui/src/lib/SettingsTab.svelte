<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import CodeEditor from "./CodeEditor.svelte";
  import {
    availableUILanguages,
    currentLanguage,
    locale,
    setLanguage,
  } from "./i18n";
  import {
    CARD_TEMPLATES_UPDATED_EVENT,
    FIELD_NAMES_UPDATED_EVENT,
    getModelsForProvider,
    limitNoteTypeFieldValue,
    loadAndValidateApiKeys,
    loadCardTemplates,
    loadFieldNames,
    providers,
    resetCardTemplates,
    resetFieldNames,
    saveCardTemplates,
    saveFieldNames,
    type ApiKeyConfig,
    type ModelInfo
  } from "./models";

  const allProviderIds = ["local", "custom", "google", "groq"];

  let apiKeys = $state<ApiKeyConfig[]>([]);
  let selectedProviderType = $state<string>("google"); // "local", "google", or "custom"
  let selectedFamily = $state<string | null>(null);

  let showAddKey = $state(false);
  let showAddModel = $state(false);

  // Snackbar notification system (replaces inline error/success banners)
  let snackbarMessage = $state<string | null>(null);
  let snackbarType = $state<"error" | "success">("success");
  let snackbarTimeout = $state<ReturnType<typeof setTimeout> | null>(null);

  function showSnackbar(
    message: string,
    type: "error" | "success" = "success",
  ) {
    if (snackbarTimeout) clearTimeout(snackbarTimeout);
    snackbarMessage = message;
    snackbarType = type;
    snackbarTimeout = setTimeout(() => {
      snackbarMessage = null;
    }, 3500);
  }

  let t = $derived($locale);

  // Card template editor
  let showTemplateEditor = $state(false);
  let showFieldEditor = $state(false);
  let showResetConfirm = $state<"style" | "fields" | null>(null);
  let helpSection = $state<string | null>(null);
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

  function saveTemplates() {
    saveCardTemplates({
      frontHtml: templateFrontHtml,
      backHtml: templateBackHtml,
      css: templateCss,
      noteTypeName: noteTypeName,
    });
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

  onMount(() => {
    loadApiKeys();

    const handleKeydown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (deleteConfirmId) {
          cancelDelete();
        } else if (showAddKey) {
          showAddKey = false;
        }
      }
    };

    window.addEventListener("keydown", handleKeydown);
    return () => {
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener(CARD_TEMPLATES_UPDATED_EVENT, syncTemplateStateFromStorage);
      window.removeEventListener(FIELD_NAMES_UPDATED_EVENT, syncFieldStateFromStorage);
      unlistenProgress?.();
    };
  });

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

  let isDownloading = $state(false);
  let isCancellingDownload = $state(false);
  let downloadingModelId = $state<string | null>(null);
  let pendingDefaultModelId = $state<string | null>(null);
  let progress = $state(0);
  let progressMessage = $state("");
  let progressStage = $state("");
  let unlistenProgress: (() => void) | null = null;
  let defaultWhisperModel = $state("base");

  onMount(async () => {
    defaultWhisperModel = localStorage.getItem("srt-default-whisper-model") || "base";
    await refreshModels();

    unlistenProgress = await listen<{
      stage: string;
      message: string;
      percentage: number;
    }>("transcribe-progress", (event) => {
      const p = event.payload;
      progress = Math.round(p.percentage);
      progressMessage = p.message;
      progressStage = p.stage;
    });
  });

  function setDefaultWhisperModel(modelId: string, notify = true) {
    defaultWhisperModel = modelId;
    localStorage.setItem("srt-default-whisper-model", modelId);
    if (notify) {
      showSnackbar(`Default Whisper model set to ${modelId}`);
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
        showSnackbar(`Model ${modelId} downloaded and set as default`);
      } else if (downloaded) {
        showSnackbar(`Model ${modelId} downloaded successfully`);
      }
    } catch (e) {
      const message = String(e).toLowerCase();
      if (message.includes("cancelled") || message.includes("canceled")) {
        showSnackbar(
          t("settings.modelDownloadCancelled", { model: modelId }) || `Download cancelled for model ${modelId}`,
        );
      } else {
        showSnackbar(`Failed to download model ${modelId}: ${e}`, "error");
      }
    } finally {
      isDownloading = false;
      isCancellingDownload = false;
      downloadingModelId = null;
      pendingDefaultModelId = null;
      progress = 0;
      progressMessage = "";
      progressStage = "";
    }
  }

  async function cancelModelDownload() {
    if (!isDownloading || isCancellingDownload) return;
    isCancellingDownload = true;
    try {
      await invoke("transcribe_cancel");
    } catch (e) {
      showSnackbar(`Failed to cancel download: ${e}`, "error");
      isCancellingDownload = false;
    }
  }

  async function uninstallModel(modelId: string) {
    if (isDownloading) return;
    try {
      await invoke<boolean>("transcribe_uninstall_model", { modelId });
      showSnackbar(`Model ${modelId} uninstalled`);
      await refreshModels();
    } catch (e) {
      showSnackbar(`Failed to uninstall model ${modelId}: ${e}`, "error");
    }
  }

  let contextMenu = $state<{
    x: number;
    y: number;
    modelId: string;
    downloaded: boolean;
  } | null>(null);

  function openContextMenu(e: MouseEvent, model: { id: string; downloaded: boolean }) {
    e.preventDefault();
    contextMenu = {
      x: e.clientX,
      y: e.clientY,
      modelId: model.id,
      downloaded: model.downloaded,
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

  onMount(() => {
    window.addEventListener(CARD_TEMPLATES_UPDATED_EVENT, syncTemplateStateFromStorage);
    window.addEventListener(FIELD_NAMES_UPDATED_EVENT, syncFieldStateFromStorage);
  });

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
    if (providerId) {
      newKeyType = providerId as ApiKeyConfig["apiType"];
      newKeyName = providers[providerId]?.name || "";
    }
    newKeyValue = "";
    newKeyUrl =
      newKeyType === "local" ? providers.local.defaultApiUrl || "" : "";
    showAddKey = true;
  }

  function openEditKeyModal(id: string) {
    const key = apiKeys.find((k) => k.id === id);
    if (!key) return;
    editKeyId = id;
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

    // Auto-set API URL for known providers
    let resolvedUrl = newKeyUrl.trim() || undefined;
    if (newKeyType === "google") {
      resolvedUrl = "https://generativelanguage.googleapis.com/v1beta";
    } else if (newKeyType === "groq") {
      resolvedUrl = "https://api.groq.com/openai/v1";
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

  let showCopySnackbar = $state(false);
  let copySnackbarTimeout: ReturnType<typeof setTimeout> | null = null;

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    showCopySnackbar = true;
    if (copySnackbarTimeout) clearTimeout(copySnackbarTimeout);
    // Hide after 2 seconds
    copySnackbarTimeout = setTimeout(() => {
      showCopySnackbar = false;
    }, 2000);
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
  class="h-full flex flex-col p-6 overflow-hidden bg-gradient-to-br from-gray-900 via-gray-900 to-gray-950"
>
  <div class="mb-6 flex flex-col gap-4">
    <div
      class="glass-card p-3 flex items-center justify-between gap-4 overflow-x-auto"
    >
      <span
        class="text-xs font-bold text-gray-500 uppercase tracking-wide whitespace-nowrap px-2"
      >
        {t("settings.language")}
      </span>
      <div class="flex gap-2">
        {#each availableUILanguages as lang}
          <button
            onclick={() => setLanguage(lang.code)}
            class="flex items-center gap-2 px-3 py-1.5 rounded-lg transition-all duration-200 border
              {$currentLanguage === lang.code
              ? 'bg-gradient-to-r from-indigo-500/20 to-purple-500/20 border-indigo-500/50 text-white shadow-sm'
              : 'bg-white/5 hover:bg-white/10 text-gray-400 hover:text-gray-200 border-transparent hover:border-white/10'}"
          >
            <span class="text-base">{lang.flag}</span>
            <span class="text-xs font-medium uppercase">{lang.code}</span>
          </button>
        {/each}
      </div>
    </div>
  </div>

  <!-- Snackbar notification at the bottom -->
  {#if snackbarMessage}
    <div
      class="fixed bottom-6 left-1/2 -translate-x-1/2 z-[100] px-5 py-3 rounded-xl shadow-2xl flex items-center gap-3 animate-fade-in
        {snackbarType === 'error'
        ? 'bg-red-500/90 text-white border border-red-400/50'
        : 'bg-emerald-500/90 text-white border border-emerald-400/50'}"
      style="backdrop-filter: blur(12px);"
    >
      <span class="text-sm font-medium">{snackbarMessage}</span>
      <button
        onclick={() => (snackbarMessage = null)}
        class="text-white/70 hover:text-white ml-2">✕</button
      >
    </div>
  {/if}

  <div class="grid grid-cols-1 xl:grid-cols-12 gap-6 flex-1 min-h-0">
    <div class="col-span-1 xl:col-span-4 flex flex-col gap-4">
      <button
        onclick={() => openAddKeyModal(selectedProviderType)}
        class="btn-primary w-full py-3 flex items-center justify-center gap-2 shadow-lg shadow-indigo-500/20"
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
            d="M12 4v16m8-8H4"
          />
        </svg>
        <span>{t("settings.addCustomApiKey")}</span>
      </button>

      <div class="glass-card flex-1 flex flex-col min-h-0">
        <div class="p-4 border-b border-white/5">
          <div class="grid grid-cols-2 gap-2">
            {#each allProviderIds as pid}
              {@const provider = providers[pid]}
              {@const isEnabled = provider?.enabled ?? false}
              {@const isSelected = selectedProviderType === pid}
              <button
                onclick={() => {
                  if (isEnabled) selectedProviderType = pid;
                }}
                disabled={!isEnabled}
                class="relative py-2 px-3 rounded-lg text-xs font-medium transition-all duration-200 flex items-center gap-2 justify-center
                  {isSelected && isEnabled
                  ? 'bg-white/10 text-white shadow-sm border border-white/10'
                  : isEnabled
                    ? 'text-gray-500 hover:text-gray-300 hover:bg-white/5 border border-transparent'
                    : 'text-gray-600 opacity-50 cursor-not-allowed border border-transparent'}"
              >
                {#if pid === "local"}
                  <div class="w-5 h-5 rounded bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center text-white flex-shrink-0">
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
                        d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
                      />
                    </svg>
                  </div>
                {:else if pid === "google"}
                  <div class="w-5 h-5 rounded bg-gradient-to-br from-blue-500 to-cyan-500 flex items-center justify-center text-white flex-shrink-0">
                    <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
                      <path
                        d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
                      />
                      <path
                        d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
                      />
                      <path
                        d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
                      />
                      <path
                        d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
                      />
                    </svg>
                  </div>
                {:else if pid === "groq"}
                  <div class="w-5 h-5 rounded bg-gradient-to-br from-orange-400 to-red-500 flex items-center justify-center text-white flex-shrink-0">
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M13 10V3L4 14h7v7l9-11h-7z"
                      />
                    </svg>
                  </div>
                {:else if pid === "custom"}
                  <div class="w-5 h-5 rounded bg-gradient-to-br from-gray-500 to-gray-600 flex items-center justify-center text-white flex-shrink-0">
                    <svg
                      class="w-3.5 h-3.5"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                      ><path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                      /><path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                      /></svg
                    >
                  </div>
                {/if}
                <span class="truncate"
                  >{t(`provider.${pid}`) || provider?.name || pid}</span
                >
                {#if !isEnabled}
                  <span
                    class="absolute -top-1 -right-1 text-[8px] bg-amber-500/80 text-white px-1 py-0.5 rounded font-bold"
                    >{t("settings.soonBadge")}</span
                  >
                {/if}
              </button>
            {/each}
          </div>
        </div>

        <div class="flex-1 p-4 space-y-3">
          <h3 class="text-xs font-bold text-gray-500 uppercase tracking-wide">
            {providers[selectedProviderType]?.name || selectedProviderType}
          </h3>
          <p class="text-sm text-gray-500 leading-relaxed">
            {t(`provider.${selectedProviderType}.desc`) ||
              providers[selectedProviderType]?.description ||
              ""}
          </p>
          {#if selectedProviderType === "google"}
            <div
              class="p-3 bg-indigo-500/10 border border-indigo-500/20 rounded-lg"
            >
              <p class="text-xs text-indigo-300">
                💡 {t("settings.googleProviderTip")}
              </p>
            </div>
          {:else if selectedProviderType === "local"}
            <div
              class="p-3 bg-emerald-500/10 border border-emerald-500/20 rounded-lg"
            >
              <p class="text-xs text-emerald-300">
                💡 {t("settings.localProviderTip")}
              </p>
            </div>
          {:else if selectedProviderType === "groq"}
            <div
              class="p-3 bg-orange-500/10 border border-orange-500/20 rounded-lg"
            >
              <p class="text-xs text-orange-300">
                ⚡ {t("settings.groqProviderTip")}
              </p>
            </div>
          {:else if selectedProviderType === "custom"}
            <div
              class="p-3 bg-gray-500/10 border border-gray-500/20 rounded-lg"
            >
              <p class="text-xs text-gray-300">
                ⚙️ {t("settings.customProviderTip")}
              </p>
            </div>
          {/if}
        </div>
      </div>
    </div>

    <div class="col-span-1 xl:col-span-8 flex flex-col min-h-0">
      <div class="glass-card flex-1 flex flex-col min-h-0">
        <div class="p-4 border-b border-white/5">
          <h3
            class="text-sm font-semibold text-gray-400 uppercase tracking-wide"
          >
            {t("settings.apiKeys")}
          </h3>
        </div>

        <div class="flex-1 overflow-y-auto p-2 space-y-2">
          {#each apiKeys as key}
            <div
              class="p-3 bg-white/5 rounded-xl border border-white/10 hover:border-white/20 transition-all group
                {key.isDefault
                ? 'ring-1 ring-indigo-500/50 bg-indigo-500/5'
                : ''}"
            >
              <div class="flex items-start gap-3">
                <div
                  class="w-8 h-8 rounded-lg bg-gradient-to-br {providers[
                    key.apiType
                  ]?.color ||
                    'from-gray-500 to-gray-600'} flex items-center justify-center flex-shrink-0 text-white shadow-lg"
                >
                  {#if key.apiType === "local"}
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
                        d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
                      />
                    </svg>
                  {:else if key.apiType === "google"}
                    <svg
                      class="w-4 h-4"
                      viewBox="0 0 24 24"
                      fill="currentColor"
                    >
                      <path
                        d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
                      />
                      <path
                        d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
                      />
                      <path
                        d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
                      />
                      <path
                        d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
                      />
                    </svg>
                  {:else if key.apiType === "openai"}
                    <svg
                      class="w-4 h-4"
                      viewBox="0 0 24 24"
                      fill="currentColor"
                    >
                      <path
                        d="M22.282 9.821a5.985 5.985 0 0 0-.516-4.91 6.046 6.046 0 0 0-6.51-2.9A6.065 6.065 0 0 0 4.981 4.18a5.985 5.985 0 0 0-3.998 2.9 6.046 6.046 0 0 0 .743 7.097 5.98 5.98 0 0 0 .51 4.911 6.051 6.051 0 0 0 6.515 2.9A5.985 5.985 0 0 0 13.26 24a6.056 6.056 0 0 0 5.772-4.206 5.99 5.99 0 0 0 3.997-2.9 6.056 6.056 0 0 0-.747-7.073zM13.26 22.43a4.476 4.476 0 0 1-2.876-1.04l.141-.081 4.779-2.758a.795.795 0 0 0 .392-.681v-6.737l2.02 1.168a.071.071 0 0 1 .038.052v5.583a4.504 4.504 0 0 1-4.494 4.494zM3.6 18.304a4.47 4.47 0 0 1-.535-3.014l.142.085 4.783 2.759a.771.771 0 0 0 .78 0l5.843-3.369v2.332a.08.08 0 0 1-.033.062L9.74 19.95a4.5 4.5 0 0 1-6.14-1.646zM2.34 7.896a4.485 4.485 0 0 1 2.366-1.973V11.6a.766.766 0 0 0 .388.676l5.815 3.355-2.02 1.168a.076.076 0 0 1-.071 0l-4.83-2.786A4.504 4.504 0 0 1 2.34 7.872zm16.597 3.855l-5.833-3.387L15.119 7.2a.076.076 0 0 1 .071 0l4.83 2.791a4.494 4.494 0 0 1-.676 8.105v-5.678a.79.79 0 0 0-.407-.667zm2.01-3.023l-.141-.085-4.774-2.782a.776.776 0 0 0-.785 0L9.409 9.23V6.897a.066.066 0 0 1 .028-.061l4.83-2.787a4.5 4.5 0 0 1 6.68 4.66zm-12.64 4.135l-2.02-1.164a.08.08 0 0 1-.038-.057V6.075a4.5 4.5 0 0 1 7.375-3.453l-.142.08L8.704 5.46a.795.795 0 0 0-.393.681zm1.097-2.365l2.602-1.5 2.607 1.5v2.999l-2.597 1.5-2.607-1.5z"
                      />
                    </svg>
                  {:else if key.apiType === "anthropic"}
                    <svg
                      class="w-4 h-4"
                      viewBox="0 0 24 24"
                      fill="currentColor"
                    >
                      <path
                        d="M17.304 3.541h-3.672l6.696 16.918H24l-6.696-16.918zm-10.608 0L0 20.459h3.744l1.368-3.576h7.056l1.368 3.576h3.744L10.584 3.541H6.696zm.096 10.454l2.4-6.252 2.376 6.252H6.792z"
                      />
                    </svg>
                  {:else}
                    <span class="text-xs">?</span>
                  {/if}
                </div>

                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 mb-0.5">
                    <span class="font-medium text-gray-200 text-sm truncate"
                      >{key.name}</span
                    >
                    {#if key.isDefault}
                      <svg
                        class="w-3.5 h-3.5 text-indigo-400"
                        fill="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          d="M16 4c.55 0 1 .45 1 1v4.38l1.71 1.71c.18.18.29.43.29.7V14c0 .55-.45 1-1 1h-5v5l-1 1-1-1v-5H6c-.55 0-1-.45-1-1v-2.21c0-.27.11-.52.29-.71L7 9.38V5c0-.55.45-1 1-1h8zm-1 2H9v3.62l-2 2V13h10v-1.38l-2-2V6z"
                        />
                      </svg>
                    {/if}
                  </div>
                  <div class="flex items-center gap-1.5">
                    <button
                      onclick={() => toggleKeyVisibility(key.id)}
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
                      onclick={() => copyApiKey(key.apiKey)}
                      class="p-1 text-gray-500 hover:text-gray-300 transition-colors flex-shrink-0"
                      title="Copy"
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
                  {#if !key.isDefault}
                    <button
                      onclick={() => setDefaultKey(key.id)}
                      class="p-1.5 text-gray-500 hover:text-indigo-400 hover:bg-white/10 rounded transition-colors"
                      title={t("settings.setAsDefault")}
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
                          d="M5 5a2 2 0 012-2h10a2 2 0 012 2v4.38l1.71 1.71c.18.18.29.43.29.7V14a2 2 0 01-2 2h-5v4l-2 2-2-2v-4H5a2 2 0 01-2-2v-2.21c0-.27.11-.52.29-.71L5 9.38V5z"
                        />
                      </svg>
                    </button>
                  {/if}
                  <button
                    onclick={() => openEditKeyModal(key.id)}
                    class="p-2.5 text-gray-400 hover:text-indigo-400 hover:bg-indigo-500/10 rounded-lg transition-colors"
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
                    onclick={() => askDeleteApiKey(key.id)}
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
              class="flex-1 flex flex-col items-center justify-center text-gray-500 p-8 text-center opacity-50"
            >
              <svg
                class="w-10 h-10 mb-2"
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
    </div>
  </div>

  <!-- Whisper Models -->
  <div class="mt-6 glass-card p-4">
    <div class="flex items-center gap-3 mb-4">
      <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-cyan-500 to-blue-600 flex items-center justify-center text-white shadow-lg">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
        </svg>
      </div>
      <div class="flex-1">
        <h3 class="text-sm font-bold text-white">{t("transcribe.whisperModel")}</h3>
        <p class="text-xs text-gray-500">{t("transcribe.whisperModelDesc") || "Download models for local transcription and auto-sync. Double click to download or set default. Right click to uninstall."}</p>
      </div>
      {#if isDownloading && downloadingModelId}
        <button
          type="button"
          onclick={cancelModelDownload}
          disabled={isCancellingDownload}
          class="px-3 py-1.5 rounded-lg text-xs font-semibold border transition-colors
            {isCancellingDownload
            ? 'bg-amber-500/10 border-amber-500/30 text-amber-300 cursor-wait'
            : 'bg-red-500/10 border-red-500/30 text-red-300 hover:bg-red-500/20 hover:border-red-500/50'}"
          title={t("settings.stopModelDownload") || "Stop download"}
        >
          {#if isCancellingDownload}
            {t("settings.stoppingModelDownload") || "Stopping..."}
          {:else}
            {t("settings.stopModelDownload") || "Stop download"}
          {/if}
        </button>
      {/if}
    </div>

    {#if isDownloading && downloadingModelId}
      <div class="mb-3 text-xs text-gray-400">
        {t("settings.modelDownloading", { model: downloadingModelId }) || `Downloading model: ${downloadingModelId}`}
        {#if progress > 0}
          <span class="text-cyan-300 ml-1">{progress}%</span>
        {/if}
      </div>
    {/if}
    
    <div class="grid grid-cols-5 gap-2">
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
          class="relative p-3 rounded-lg text-center transition-all duration-200 border cursor-pointer
            {defaultWhisperModel === model.id && model.downloaded
            ? 'bg-cyan-500/20 border-cyan-500/50 text-white shadow-[0_0_15px_rgba(6,182,212,0.15)]'
            : model.downloaded
              ? 'bg-white/10 hover:bg-white/20 border-white/20 text-gray-200'
              : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-500 opacity-60'}"
          title={model.downloaded ? "Click to set as default. Right-click to uninstall." : "Click to download and set as default"}
        >
          <div class="absolute top-1 right-1 pointer-events-none">
            {#if !model.downloaded}
              <button
                onclick={(e) => { e.stopPropagation(); void downloadModel(model.id, true); }}
                class="text-amber-400 hover:text-cyan-400 transition-colors animate-pulse pointer-events-auto"
                title={t("transcribe.clickToDownload")}
                disabled={isDownloading}
              >
                {#if downloadingModelId === model.id}
                  <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                  </svg>
                {:else}
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/></svg>
                {/if}
              </button>
            {/if}
          </div>
          <div class="font-bold text-sm">
            {t(`transcribe.model${model.id.charAt(0).toUpperCase()}${model.id.slice(1)}`) || model.name}
          </div>
          <div class="text-[10px] text-gray-500 mt-1">{model.size}</div>
          {#if !model.downloaded}
            <div class="text-[9px] text-amber-400/70 mt-0.5">
              {#if downloadingModelId === model.id}
                Downloading... {progress > 0 ? `${progress}%` : ""}
              {:else}
                Not downloaded
              {/if}
            </div>
          {:else if defaultWhisperModel === model.id}
            <div class="text-[9px] text-cyan-400 mt-0.5 font-bold">Default</div>
          {/if}
        </div>
      {/each}
    </div>
  </div>

  {#if contextMenu}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50"
      onmousedown={closeContextMenu}
      oncontextmenu={(e) => { e.preventDefault(); closeContextMenu(); }}
    >
      <div
        class="absolute bg-gray-900 border border-white/10 rounded-lg shadow-2xl py-1 min-w-[160px] animate-fade-in"
        style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
        onmousedown={(e) => e.stopPropagation()}
      >
        <div class="px-3 py-1.5 border-b border-white/5 bg-white/5 mb-1">
          <span class="text-xs font-bold text-gray-400 uppercase tracking-wide">Model: {contextMenu.modelId}</span>
        </div>
        {#if contextMenu.downloaded}
          <button
            class="w-full text-left px-4 py-2 text-sm text-red-400 hover:bg-red-500/10 hover:text-red-300 flex items-center gap-2 transition-colors"
            onclick={() => {
              uninstallModel(contextMenu!.modelId);
              closeContextMenu();
            }}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
            Uninstall
          </button>
        {:else}
          <div class="px-4 py-2 text-sm text-gray-500 italic">Not downloaded</div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Card Template Editor -->
  <div class="mt-6">
    <div class="glass-card p-4">
      <div class="flex items-center gap-3 mb-4">
        <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-amber-500 to-orange-600 flex items-center justify-center text-white shadow-lg">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z" />
          </svg>
        </div>
        <div class="flex-1">
          <h3 class="text-sm font-bold text-white">{t("settings.cardTemplates")}</h3>
          <p class="text-xs text-gray-500">{t("settings.cardTemplatesDesc")}</p>
        </div>
      </div>
      <div class="flex gap-3">
        <button
          type="button"
          onclick={() => (showTemplateEditor = true)}
          class="flex-1 flex items-center justify-center gap-2 py-2.5 px-4 rounded-lg border border-amber-500/30 bg-amber-500/10 text-amber-300 hover:bg-amber-500/20 transition-colors text-sm font-medium"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" />
          </svg>
          {t("settings.editCardStyle") || "Card Style"}
        </button>
        <button
          type="button"
          onclick={() => (showFieldEditor = true)}
          class="flex-1 flex items-center justify-center gap-2 py-2.5 px-4 rounded-lg border border-emerald-500/30 bg-emerald-500/10 text-emerald-300 hover:bg-emerald-500/20 transition-colors text-sm font-medium"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 17V7m0 10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h2a2 2 0 012 2m0 10a2 2 0 002 2h2a2 2 0 002-2M9 7a2 2 0 012-2h2a2 2 0 012 2m0 10V7" />
          </svg>
          {t("settings.editFields") || "Fields & Note Type"}
        </button>
      </div>
    </div>

    {#if showTemplateEditor}
      <div
        class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-6"
        role="dialog"
        tabindex="-1"
        onmousedown={(e) => {
          if (e.target === e.currentTarget) showTemplateEditor = false;
        }}
      >
        <div
          class="w-full max-w-5xl max-h-[95vh] overflow-hidden flex flex-col animate-fade-in shadow-2xl border border-white/20 bg-gray-900/98 backdrop-blur-xl rounded-xl"
          role="presentation"
          onmousedown={(e) => e.stopPropagation()}
        >
          <!-- Modal Header -->
          <div class="p-6 border-b border-white/5 bg-white/5 flex items-center justify-between shrink-0">
            <div>
              <h3 class="text-xl font-bold text-white flex items-center gap-2">
                {t("settings.editCardStyle") || "Card Style"}
              </h3>
              <p class="text-sm text-gray-400 mt-1">{t("settings.cardStyleDesc") || "Edit front, back HTML and CSS styling"}</p>
            </div>
            <div class="flex items-center gap-4">
              <button
                type="button"
                onclick={() => (showResetConfirm = "style")}
                class="btn-secondary py-2 px-4 text-sm flex items-center gap-2 hover:bg-amber-500/10 hover:text-amber-500 hover:border-amber-500/50 transition-colors"
                title="Ripristina ai valori di default di Vesta"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
                {t("settings.resetDefaults")}
              </button>
              <button
                type="button"
                onclick={() => (showTemplateEditor = false)}
                class="w-8 h-8 rounded-full flex items-center justify-center hover:bg-white/10 text-gray-400 hover:text-white transition-colors"
                aria-label={t("common.close") || "Close"}
              >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>

          <!-- Modal Body -->
          <div class="p-6 overflow-y-auto custom-scrollbar flex-1 space-y-8">
            <div class="grid grid-cols-2 gap-6">
              <!-- Front HTML -->
              <div class="space-y-2">
                <label for="template-front-html" class="block text-sm font-bold text-amber-500 uppercase tracking-wide flex items-center gap-2">
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6z"/></svg>
                  {t("settings.cardFrontHtml")}
                  <div class="ml-auto flex items-center gap-1">
                    <div class="relative group">
                      <button type="button" class="text-gray-500 hover:text-amber-300 transition-colors" aria-label="Info">
                        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
                      </button>
                  <div class="card-style-tooltip down">{t("settings.cardFrontHtmlInfo")}</div>
                    </div>
                  </div>
                </label>
                <div class="relative">
                  <CodeEditor bind:value={templateFrontHtml} language="html" onchange={saveTemplates} />
                  <button type="button" onclick={() => { navigator.clipboard.writeText(templateFrontHtml); showSnackbar(t("settings.keyCopied")); }} class="absolute top-2 right-2 text-gray-500 hover:text-amber-300 transition-colors bg-gray-800/80 rounded p-0.5 z-10" title="Copy">
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
                  </button>
                </div>
              </div>

              <!-- Back HTML -->
              <div class="space-y-2">
                <label for="template-back-html" class="block text-sm font-bold text-emerald-500 uppercase tracking-wide flex items-center gap-2">
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"/></svg>
                  {t("settings.cardBackHtml")}
                  <div class="ml-auto flex items-center gap-1">
                    <div class="relative group">
                      <button type="button" class="text-gray-500 hover:text-emerald-300 transition-colors" aria-label="Info">
                        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
                      </button>
                  <div class="card-style-tooltip down">{t("settings.cardBackHtmlInfo")}</div>
                    </div>
                  </div>
                </label>
                <div class="relative">
                  <CodeEditor bind:value={templateBackHtml} language="html" onchange={saveTemplates} />
                  <button type="button" onclick={() => { navigator.clipboard.writeText(templateBackHtml); showSnackbar(t("settings.keyCopied")); }} class="absolute top-2 right-2 text-gray-500 hover:text-emerald-300 transition-colors bg-gray-800/80 rounded p-0.5 z-10" title="Copy">
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
                  </button>
                </div>
              </div>
            </div>

            <!-- CSS -->
            <div class="space-y-2">
              <label for="template-css" class="block text-sm font-bold text-blue-500 uppercase tracking-wide flex items-center gap-2">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"/></svg>
                {t("settings.cardCss")}
                <div class="ml-auto flex items-center gap-1">
                  <div class="relative group">
                    <button type="button" class="text-gray-500 hover:text-blue-300 transition-colors" aria-label="Info">
                      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
                    </button>
                  <div class="card-style-tooltip">{t("settings.cardCssInfo")}</div>
                  </div>
                </div>
              </label>
              <div class="relative">
                <CodeEditor bind:value={templateCss} language="css" onchange={saveTemplates} />
                <button type="button" onclick={() => { navigator.clipboard.writeText(templateCss); showSnackbar(t("settings.keyCopied")); }} class="absolute top-2 right-2 text-gray-500 hover:text-blue-300 transition-colors bg-gray-800/80 rounded p-0.5 z-10" title="Copy">
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
                </button>
              </div>
            </div>
            
            <div class="p-4 bg-white/5 border border-white/10 rounded-lg">
              <div class="flex items-center gap-2 mb-2">
                <h4 class="text-xs font-bold text-gray-400 uppercase">{t("settings.availableVars")}</h4>
                <div class="relative group">
                  <button type="button" class="text-gray-500 hover:text-gray-300 transition-colors" aria-label="Info">
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
                  </button>
                  <div class="card-style-tooltip">{t("settings.availableVarsInfo")}</div>
                </div>
              </div>
              <p class="text-xs text-gray-500 font-mono leading-relaxed">
                <code class="text-amber-300">{"{{Expression}}"}</code>, <code class="text-emerald-300">{"{{Reading}}"}</code>, <code class="text-blue-300">{"{{Meaning}}"}</code>, 
                <code class="text-purple-300">{"{{Audio}}"}</code>, <code class="text-pink-300">{"{{Video}}"}</code>, <code class="text-cyan-300">{"{{Snapshot}}"}</code>, 
                <code class="text-orange-300">{"{{Tags}}"}</code>, <code class="text-gray-300">{"{{SequenceMarker}}"}</code>, <code class="text-red-300">{"{{Notes}}"}</code>
              </p>
            </div>
          </div>
        </div>
      </div>
    {/if}

    {#if showFieldEditor}
      <div
        class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-6"
        role="dialog"
        tabindex="-1"
        onmousedown={(e) => {
          if (e.target === e.currentTarget) showFieldEditor = false;
        }}
      >
        <div
          class="w-full max-w-2xl overflow-hidden flex flex-col animate-fade-in shadow-2xl border border-white/20 bg-gray-900/98 backdrop-blur-xl rounded-xl max-h-[90vh]"
          role="presentation"
          onmousedown={(e) => e.stopPropagation()}
        >
          <!-- Modal Header -->
          <div class="p-6 border-b border-white/5 bg-white/5 flex items-center justify-between shrink-0">
            <div>
              <h3 class="text-xl font-bold text-white flex items-center gap-2">
                {t("settings.editFields") || "Fields & Note Type"}
              </h3>
              <p class="text-sm text-gray-400 mt-1">{t("settings.fieldsDesc") || "Customize field names and note type"}</p>
            </div>
            <div class="flex items-center gap-4">
              <button
                type="button"
                onclick={() => (showResetConfirm = "fields")}
                class="btn-secondary py-2 px-4 text-sm flex items-center gap-2 hover:bg-amber-500/10 hover:text-amber-500 hover:border-amber-500/50 transition-colors"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
                {t("settings.resetDefaults")}
              </button>
              <button
                type="button"
                onclick={() => (showFieldEditor = false)}
                class="w-8 h-8 rounded-full flex items-center justify-center hover:bg-white/10 text-gray-400 hover:text-white transition-colors"
                aria-label={t("common.close") || "Close"}
              >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>

          <!-- Modal Body -->
          <div class="p-6 overflow-y-auto custom-scrollbar flex-1 space-y-6">
            <!-- Note Type Name -->
            <div class="glass-card p-5">
              <label for="note-type-name" class="block text-sm font-bold text-gray-300 uppercase tracking-wide mb-2 flex items-center gap-2">
                🏷️ {t("flashcards.noteTypeName") || "Note Type Name"}
              </label>
              <input
                id="note-type-name"
                type="text"
                bind:value={noteTypeName}
                maxlength="25"
                oninput={(event) =>
                  syncLimitedInput(event, (value) => (noteTypeName = value), saveTemplates)}
                class="input-modern w-full text-sm"
                placeholder="es. subs2srs, Vesta Default..."
              />
              <p class="text-xs text-gray-500 mt-2">
                {t("settings.noteTypeNameDesc")}
              </p>
            </div>

            <!-- Field Names -->
            <div class="space-y-3">
              <h4 class="text-sm font-bold text-emerald-400 uppercase tracking-wide flex items-center gap-2">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 17V7m0 10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h2a2 2 0 012 2m0 10a2 2 0 002 2h2a2 2 0 002-2M9 7a2 2 0 012-2h2a2 2 0 012 2m0 10V7" /></svg>
                {t("settings.fieldNames")}
              </h4>
              <div class="grid grid-cols-2 gap-3">
                <div>
                  <label for="field-expression" class="block text-xs text-gray-400 mb-1">{t("settings.fieldExpression")}</label>
                  <input id="field-expression" type="text" bind:value={fieldExpression} maxlength="25" oninput={(event) => syncLimitedInput(event, (value) => (fieldExpression = value), saveFields)} class="input-modern w-full text-sm" placeholder="Expression" />
                </div>
                <div>
                  <label for="field-meaning" class="block text-xs text-gray-400 mb-1">{t("settings.fieldMeaning")}</label>
                  <input id="field-meaning" type="text" bind:value={fieldMeaning} maxlength="25" oninput={(event) => syncLimitedInput(event, (value) => (fieldMeaning = value), saveFields)} class="input-modern w-full text-sm" placeholder="Meaning" />
                </div>
                <div>
                  <label for="field-reading" class="block text-xs text-gray-400 mb-1">{t("settings.fieldReading")}</label>
                  <input id="field-reading" type="text" bind:value={fieldReading} maxlength="25" oninput={(event) => syncLimitedInput(event, (value) => (fieldReading = value), saveFields)} class="input-modern w-full text-sm" placeholder="Reading" />
                </div>
                <div>
                  <label for="field-audio" class="block text-xs text-gray-400 mb-1">{t("settings.fieldAudio")}</label>
                  <input id="field-audio" type="text" bind:value={fieldAudio} maxlength="25" oninput={(event) => syncLimitedInput(event, (value) => (fieldAudio = value), saveFields)} class="input-modern w-full text-sm" placeholder="Audio" />
                </div>
                <div>
                  <label for="field-snapshot" class="block text-xs text-gray-400 mb-1">{t("settings.fieldSnapshot")}</label>
                  <input id="field-snapshot" type="text" bind:value={fieldSnapshot} maxlength="25" oninput={(event) => syncLimitedInput(event, (value) => (fieldSnapshot = value), saveFields)} class="input-modern w-full text-sm" placeholder="Snapshot" />
                </div>
                <div>
                  <label for="field-video" class="block text-xs text-gray-400 mb-1">{t("settings.fieldVideo")}</label>
                  <input id="field-video" type="text" bind:value={fieldVideo} maxlength="25" oninput={(event) => syncLimitedInput(event, (value) => (fieldVideo = value), saveFields)} class="input-modern w-full text-sm" placeholder="Video" />
                </div>
                <div>
                  <label for="field-tags" class="block text-xs text-gray-400 mb-1">{t("settings.fieldTags")}</label>
                  <input id="field-tags" type="text" bind:value={fieldTags} maxlength="25" oninput={(event) => syncLimitedInput(event, (value) => (fieldTags = value), saveFields)} class="input-modern w-full text-sm" placeholder="Tags" />
                </div>
                <div>
                  <label for="field-sequence-marker" class="block text-xs text-gray-400 mb-1">{t("settings.fieldSequenceMarker")}</label>
                  <input id="field-sequence-marker" type="text" bind:value={fieldSequenceMarker} maxlength="25" oninput={(event) => syncLimitedInput(event, (value) => (fieldSequenceMarker = value), saveFields)} class="input-modern w-full text-sm" placeholder="SequenceMarker" />
                </div>
                <div class="col-span-2">
                  <label for="field-notes" class="block text-xs text-gray-400 mb-1">{t("settings.fieldNotes")}</label>
                  <input id="field-notes" type="text" bind:value={fieldNotes} maxlength="25" oninput={(event) => syncLimitedInput(event, (value) => (fieldNotes = value), saveFields)} class="input-modern w-full text-sm" placeholder="Notes" />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Reset Confirmation Dialog -->
    {#if showResetConfirm}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="fixed inset-0 bg-black/70 flex items-center justify-center z-[60] p-6"
        role="dialog"
        aria-modal="true"
        tabindex="-1"
        onmousedown={(e) => {
          if (e.target === e.currentTarget) showResetConfirm = null;
        }}
        onkeydown={(e) => {
          if (e.key === "Escape") showResetConfirm = null;
        }}
      >
        <div
          class="bg-gray-900 border border-gray-700 rounded-xl w-full max-w-sm p-6 animate-fade-in shadow-2xl"
          onmousedown={(e) => e.stopPropagation()}
        >
          <div class="flex items-center gap-3 mb-4">
            <div class="w-10 h-10 rounded-full bg-amber-500/20 flex items-center justify-center">
              <svg class="w-5 h-5 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z" />
              </svg>
            </div>
            <div>
              <h3 class="text-sm font-bold text-white">{t("settings.resetConfirmTitle") || "Reset to defaults?"}</h3>
              <p class="text-xs text-gray-400 mt-1">{t("settings.resetConfirmDesc") || "All customizations will be lost."}</p>
            </div>
          </div>
          <div class="flex justify-end gap-3">
            <button
              type="button"
              onclick={() => (showResetConfirm = null)}
              class="btn-secondary py-2 px-4 text-sm"
            >
              {t("common.cancel") || "Cancel"}
            </button>
            <button
              type="button"
              onclick={() => {
                if (showResetConfirm === "style") {
                  const defaults = resetCardTemplates();
                  templateFrontHtml = defaults.frontHtml;
                  templateBackHtml = defaults.backHtml;
                  templateCss = defaults.css;
                  noteTypeName = defaults.noteTypeName;
                } else if (showResetConfirm === "fields") {
                  const defaults = resetFieldNames();
                  fieldExpression = defaults.expression;
                  fieldMeaning = defaults.meaning;
                  fieldReading = defaults.reading;
                  fieldAudio = defaults.audio;
                  fieldSnapshot = defaults.snapshot;
                  fieldVideo = defaults.video;
                  fieldTags = defaults.tags;
                  fieldSequenceMarker = defaults.sequenceMarker;
                  fieldNotes = defaults.notes;
                  // Also reset note type name
                  const templateDefaults = resetCardTemplates();
                  noteTypeName = templateDefaults.noteTypeName;
                }
                showResetConfirm = null;
              }}
              class="py-2 px-4 text-sm font-medium rounded-lg bg-amber-500/20 border border-amber-500/50 text-amber-300 hover:bg-amber-500/30 transition-colors"
            >
              {t("settings.resetDefaults")}
            </button>
          </div>
        </div>
      </div>
    {/if}
  </div>

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
        class="w-full max-w-lg overflow-hidden animate-fade-in shadow-2xl border border-white/20 bg-gray-900/98 backdrop-blur-xl rounded-xl"
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

        <div class="p-6 space-y-5">
          <div>
            <span
              class="block text-xs font-bold text-gray-500 uppercase tracking-wide mb-2"
              >{t("settings.modal.provider")}</span
            >
            <div class="grid grid-cols-2 gap-3">
              <!-- Server Locale -->
              <button
                type="button"
                onclick={() => {
                  newKeyType = "local";
                  newKeyName = providers.local.name;
                  newKeyUrl = providers.local.defaultApiUrl || "";
                  newKeyValue = "";
                }}
                class="flex items-center gap-3 p-3 rounded-lg transition-all duration-200 border text-left
                  {newKeyType === 'local'
                  ? 'bg-indigo-500/20 border-indigo-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400'}"
              >
                <div
                  class="w-8 h-8 rounded-lg bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center text-white shadow-lg"
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
                      d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
                    />
                  </svg>
                </div>
                <div class="flex flex-col">
                  <span class="text-sm font-bold"
                    >{t("settings.modal.localServer")}</span
                  >
                  <span class="text-[10px] opacity-70 leading-tight"
                    >{t("settings.modal.localServerDesc")}</span
                  >
                </div>
              </button>

              <!-- Provider Personalizzato -->
              <button
                type="button"
                onclick={() => {
                  newKeyType = "custom";
                  newKeyName = "";
                  newKeyUrl = "";
                  newKeyValue = "";
                }}
                class="flex items-center gap-3 p-3 rounded-lg transition-all duration-200 border text-left
                  {newKeyType === 'custom'
                  ? 'bg-indigo-500/20 border-indigo-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400'}"
              >
                <div
                  class="w-8 h-8 rounded-lg bg-gradient-to-br from-gray-500 to-gray-600 flex items-center justify-center text-white shadow-lg"
                >
                  <svg
                    class="w-5 h-5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    ><path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                    /><path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                    /></svg
                  >
                </div>
                <div class="flex flex-col">
                  <span class="text-sm font-bold">{t("provider.custom")}</span>
                  <span class="text-[10px] opacity-70 leading-tight"
                    >{t("provider.custom.desc")}</span
                  >
                </div>
              </button>

              <!-- Google Gemini -->
              <button
                type="button"
                onclick={() => {
                  newKeyType = "google";
                  newKeyName = providers.google.name;
                  newKeyUrl = providers.google.defaultApiUrl || "";
                  newKeyValue = "";
                }}
                class="flex items-center gap-3 p-3 rounded-lg transition-all duration-200 border text-left
                  {newKeyType === 'google'
                  ? 'bg-indigo-500/20 border-indigo-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400'}"
              >
                <div
                  class="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-500 to-cyan-500 flex items-center justify-center text-white shadow-lg"
                >
                  <svg class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                    <path
                      d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
                    />
                    <path
                      d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
                    />
                    <path
                      d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
                    />
                    <path
                      d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
                    />
                  </svg>
                </div>
                <div class="flex flex-col">
                  <span class="text-sm font-bold"
                    >{t("settings.modal.providerGoogle")}</span
                  >
                  <span class="text-[10px] opacity-70 leading-tight"
                    >{t("settings.modal.providerGoogleDesc")}</span
                  >
                </div>
              </button>

              <!-- Groq API -->
              <button
                type="button"
                onclick={() => {
                  newKeyType = "groq";
                  newKeyName = providers.groq.name;
                  newKeyUrl = providers.groq.defaultApiUrl || "";
                  newKeyValue = "";
                }}
                class="flex items-center gap-3 p-3 rounded-lg transition-all duration-200 border text-left
                  {newKeyType === 'groq'
                  ? 'bg-indigo-500/20 border-indigo-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400'}"
              >
                <div
                  class="w-8 h-8 rounded-lg bg-gradient-to-br from-orange-400 to-red-500 flex items-center justify-center text-white shadow-lg"
                >
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M13 10V3L4 14h7v7l9-11h-7z"
                    />
                  </svg>
                </div>
                <div class="flex flex-col">
                  <span class="text-sm font-bold"
                    >{t("settings.modal.providerGroq")}</span
                  >
                  <span class="text-[10px] opacity-70 leading-tight"
                    >{t("settings.modal.providerGroqDesc")}</span
                  >
                </div>
              </button>
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
                    : "https://api.example.com/v1/chat/completions"}
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
                    placeholder={newKeyType === "google"
                      ? "AIza..."
                      : newKeyType === "groq"
                        ? "gsk_..."
                        : newKeyType === "custom"
                          ? t("settings.modal.notRequiredForLocal")
                          : "sk-..."}
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
              </div>
            {/if}

            {#if newKeyType === "google"}
              <p class="text-[10px] text-gray-500 leading-relaxed">
                💡 {t("settings.modal.apiKeyHintGoogle")}
                <a
                  href="https://aistudio.google.com/apikey"
                  target="_blank"
                  class="text-blue-400 hover:text-blue-300 underline"
                  >aistudio.google.com/apikey</a
                >
              </p>
            {/if}
            {#if newKeyType === "groq"}
              <p class="text-[10px] text-gray-500 leading-relaxed">
                ⚡ {t("settings.modal.apiKeyHintGroq")}
                <a
                  href="https://console.groq.com/keys"
                  target="_blank"
                  class="text-orange-400 hover:text-orange-300 underline"
                  >console.groq.com/keys</a
                >
              </p>
            {/if}
          </div>

          <div class="flex gap-3 pt-4 border-t border-white/5">
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

  {#if showCopySnackbar}
    <div
      class="fixed bottom-6 left-1/2 -translate-x-1/2 z-[60] animate-fade-in"
    >
      <div
        class="bg-gray-800 border border-gray-700 text-white px-4 py-2.5 rounded-lg shadow-xl flex items-center gap-2"
      >
        <svg
          class="w-4 h-4 text-green-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M5 13l4 4L19 7"
          />
        </svg>
        <span class="text-sm font-medium">{t("settings.keyCopied")}</span>
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
</style>
