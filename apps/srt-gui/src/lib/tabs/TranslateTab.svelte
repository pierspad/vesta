<script lang="ts">
  import { localShortcutAction } from "$lib/utils/shortcuts";
  import { isInputActive } from "$lib/utils/keyboardFocus";
  import { RequirementFeedback } from "$lib/utils/requirementFeedback.svelte";
  const requirements = new RequirementFeedback<"files" | "options">();
  import { validatePickerPath } from "$lib/utils/pathValidation";
  import { AsyncOperation, LatestRequest } from "$lib/utils/asyncControl.svelte";
  import { listen } from "@tauri-apps/api/event";
  import { guardedOpen, guardedSave } from "$lib/utils/dialogGuard";
  import { setupWebviewDragDrop } from "$lib/utils/dragDrop";
  import { createLogPanelBuffer } from "$lib/utils/logPanelBuffer.svelte";
  import { ManagedModelDiscovery } from "$lib/workflows/managedModelDiscovery.svelte";
  import { onMount, untrack } from "svelte";
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import { errorMessage } from "$lib/utils/errorMessage";
  import { loadAndValidateApiKeys, type ApiKeyConfig } from "$lib/config/apiKeys";
  import { loadTiers, tiersHaveUsableEntries, TIERS_UPDATED_EVENT, type Tier } from "$lib/config/translationTiers";
  import { generateTranslationOutputPath as generateOutputPath } from "$lib/utils/translationPaths";
  import TranslationPreviewPanel from "$lib/panels/TranslationPreviewPanel.svelte";
  import PathPreviewModal from "$lib/modals/PathPreviewModal.svelte";
  import WorkflowWorkspace from "$lib/components/WorkflowWorkspace.svelte";
  import type { LogEntry } from "$lib/panels/LogPanel.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import {
    buildTiersPayload as buildTiersPayloadShared,
    checkTiersAvailability as checkTiersAvailabilityShared,
    tiersUnavailableMessage as tiersUnavailableMessageShared,
    type TierEntryPayload,
    type TiersUnavailableReason,
  } from "$lib/config/llmTiers";
  import * as vestaConfig from "$lib/config/vestaConfig";
  import {
    getLatestTranslatedSubtitles,
    loadSrtForTranslate,
    suggestTranslationContext,
    startTranslation as apiStartTranslation,
    cancelTranslation as apiCancelTranslation,
    type SrtFileInfo,
    type TranslateConfig,
    type TranslateResult,
    type SubtitlePair,
  } from "$lib/services/translate";
  import { transcribeCheckFileExists } from "$lib/services/transcribe";
  import TranslateOptionsPanel from "./translate/TranslateOptionsPanel.svelte";
  import TranslateFilesPanel from "./translate/TranslateFilesPanel.svelte";
  import TranslateProgressPanel from "./translate/TranslateProgressPanel.svelte";
  import TranslateFooterBar from "./translate/TranslateFooterBar.svelte";

  interface Props {
    onGoToSettings?: (section?: "overview" | "llm" | "whisper" | "language" | "anki") => void;
    active?: boolean;
  }

  let { onGoToSettings: _onGoToSettings, active = true }: Props = $props();
  let t = $derived($locale);

  interface TranslateProgressEvent {
    message: string;
    current_batch: number;
    total_batches: number;
    percentage: number;
    eta_seconds: number | null;
  }

  const LAST_PROVIDER_KEY = "vesta-translate-last-provider";
  const LAST_MODEL_KEY = "vesta-translate-last-model";
  const LAST_CUSTOM_PROVIDER_KEY = "vesta-translate-last-custom-provider";
  const LAST_CUSTOM_MODEL_KEY = "vesta-translate-last-custom-model";
  const LAST_TARGET_LANGUAGE_KEY = "vesta-translate-target-language";
  const DEFAULT_LLM_PROVIDER_KEY = "vesta-default-llm-provider";
  const DEFAULT_LLM_MODEL_KEY = "vesta-default-llm-model";
  const DEFAULT_LLM_CUSTOM_PROVIDER_KEY = "vesta-default-llm-custom-provider";
  const DEFAULT_TARGET_LANGUAGE_KEY = "vesta-default-target-language";
  const LOCAL_SERVER_URL_KEY = "vesta-local-server-url";
  const DEFAULT_LOCAL_URL = "http://localhost:11434/v1";

  function loadStoredValue(key: string): string {
    try { return vestaConfig.getItem(key) || ""; } catch { return ""; }
  }

  let inputPath = $state("");
  let outputPath = $state("");
  let automaticOutputPath = $state(true);
  let targetLang = $state(loadStoredValue(LAST_TARGET_LANGUAGE_KEY) || loadStoredValue(DEFAULT_TARGET_LANGUAGE_KEY) || "it");
  let selectedProviderFamily = $state(loadStoredValue(DEFAULT_LLM_PROVIDER_KEY) || loadStoredValue(LAST_PROVIDER_KEY) || "local");
  let providerConfirmed = $state(true);
  let selectedCustomProviderId = $state(loadStoredValue(DEFAULT_LLM_CUSTOM_PROVIDER_KEY) || loadStoredValue(LAST_CUSTOM_PROVIDER_KEY));
  let localCustomModel = $state(loadStoredValue(LAST_CUSTOM_MODEL_KEY));
  let selectedModel = $state(loadStoredValue(DEFAULT_LLM_MODEL_KEY) || loadStoredValue(LAST_MODEL_KEY));
  let localServerUrl = $state(vestaConfig.getItem(LOCAL_SERVER_URL_KEY) || DEFAULT_LOCAL_URL);

  let batchSize = $state(15);
  let resumeOverlap = $state(2);
  let effectiveBatchSize = $derived(uiMode.easyMode ? 15 : batchSize);
  let effectiveResumeOverlap = $derived(uiMode.easyMode ? 2 : resumeOverlap);
  let titleContext = $state("");
  let contextDraft = $state("");
  let contextDraftPath = $state("");
  let contextLoading = $state(false);
  let contextError = $state("");
  let contextRequest = 0;
  let previousContextPath = "";

  $effect(() => {
    void inputPath;
    void targetLang;
    if (previousContextPath && previousContextPath !== inputPath) titleContext = "";
    previousContextPath = inputPath;
    contextDraft = "";
    contextError = "";
    contextDraftPath = "";
    contextRequest += 1;
  });

  const modelDiscovery = new ManagedModelDiscovery(vestaConfig);
  let fileInfo = $state<SrtFileInfo | null>(null);
  const translationOperation = new AsyncOperation();
  const fileRequests = new LatestRequest();
  const previewRequests = new LatestRequest();
  let fileLoading = $state(false);
  let previewPending: Promise<void> | null = null;
  let disposed = false;
  let isTranslating = $derived(translationOperation.busy);
  $effect(() => { aiStore.isTranslating = isTranslating; });

  let progress = $state<TranslateProgressEvent | null>(null);
  const logBuffer = createLogPanelBuffer();
  let error = $state<string | null>(null);
  let result = $state<TranslateResult | null>(null);
  let expandedPathField = $state<string | null>(null);
  let translatedPairs = $state<SubtitlePair[]>([]);
  let previewRefreshInterval: ReturnType<typeof setInterval> | null = null;
  let apiKeys = $state<ApiKeyConfig[]>([]);
  let tiers = $state<Tier[]>([]);
  let useTiers = $derived(tiersHaveUsableEntries(tiers));
  let isDraggingOver = $state(false);
  let canUseFilePanel = $derived(providerConfirmed);
  let canStartTranslation = $derived(
    !!fileInfo?.subtitle_count && !!outputPath && useTiers && !contextLoading && !fileLoading && !aiStore.killSwitchActive
  );
  let translationBlockedReason = $derived(
    !useTiers ? t("translate.blocked.noTiers") : !inputPath || !outputPath ? t("translate.blocked.selectFiles") : ""
  );

  const refreshTiers = () => { tiers = loadTiers(); };
  const loadApiKeys = () => { apiKeys = loadAndValidateApiKeys(); };

  function buildTiersPayload(): TierEntryPayload[][] | null {
    return useTiers ? buildTiersPayloadShared(tiers, apiKeys) : null;
  }
  function tiersUnavailableMessage(reason: TiersUnavailableReason): string {
    return tiersUnavailableMessageShared(reason, "translate.errorTranslating");
  }
  async function checkTiersAvailability(): Promise<{ available: boolean; errorMsg?: string }> {
    if (!useTiers) return { available: false, errorMsg: t("tiers.noneConfigured") };
    const check = await checkTiersAvailabilityShared(tiers, apiKeys);
    return check.available ? { available: true } : { available: false, errorMsg: tiersUnavailableMessage(check.reason) };
  }

  async function suggestContext() {
    if (contextLoading || isTranslating || aiStore.killSwitchActive) return;
    const tiersPayload = buildTiersPayload();
    if (!tiersPayload || !inputPath) return;
    const request = ++contextRequest;
    const path = inputPath;
    const language = targetLang;
    const context = titleContext;
    contextLoading = true;
    contextError = "";
    try {
      const check = await checkTiersAvailability();
      if (request !== contextRequest || path !== inputPath || disposed || aiStore.killSwitchActive) return;
      if (!check.available) { requirements.pulse(["options"]); if (check.errorMsg) snackbar.show(check.errorMsg, "error"); return; }
      const draft = await suggestTranslationContext(path, language, context, tiersPayload);
      if (request === contextRequest && path === inputPath && !aiStore.killSwitchActive && !disposed) {
        contextDraft = draft;
        contextDraftPath = path;
      }
    } catch (e) {
      if (request === contextRequest && !disposed) contextError = errorMessage(e);
    } finally {
      if (request === contextRequest) {
        contextLoading = false;
      }
    }
  }

  function ensureProviderSelectedForFiles(): boolean {
    if (providerConfirmed) return true;
    error = t("translate.selectProviderFirst");
    return false;
  }

  async function handleFileDrop(paths: string[]) {
    if (isTranslating || contextLoading || !ensureProviderSelectedForFiles()) return;
    const srtFile = paths.find((p) => p.toLowerCase().endsWith(".srt"));
    if (srtFile) {
      inputPath = srtFile;
      await loadFileInfo();
      if (!outputPath) outputPath = generateOutputPath(inputPath, targetLang);
    }
  }

  $effect(() => {
    if (automaticOutputPath && inputPath) outputPath = generateOutputPath(inputPath, targetLang);
  });

  $effect(() => {
    vestaConfig.setItem(LAST_PROVIDER_KEY, selectedProviderFamily || "");
    vestaConfig.setItem(LAST_MODEL_KEY, selectedModel || "");
    vestaConfig.setItem(LAST_TARGET_LANGUAGE_KEY, targetLang || "");
    vestaConfig.setItem(LAST_CUSTOM_PROVIDER_KEY, selectedCustomProviderId || "");
    vestaConfig.setItem(LAST_CUSTOM_MODEL_KEY, localCustomModel || "");
  });

  $effect(() => {
    const family = providerConfirmed ? selectedProviderFamily : "";
    const key = apiKeys.find((k) => k.apiType === family && k.apiKey?.trim())?.apiKey.trim() || "";
    untrack(() => { void modelDiscovery.load(family, key); });
  });

  $effect(() => {
    if (!active) stopPreviewRefresh();
    else if (isTranslating) startPreviewRefresh();
  });

  function loadDefaultLlmSettings() {
    selectedProviderFamily = loadStoredValue(DEFAULT_LLM_PROVIDER_KEY) || selectedProviderFamily || "local";
    selectedModel = loadStoredValue(DEFAULT_LLM_MODEL_KEY) || selectedModel;
    selectedCustomProviderId = loadStoredValue(DEFAULT_LLM_CUSTOM_PROVIDER_KEY) || selectedCustomProviderId;
    localServerUrl = vestaConfig.getItem(LOCAL_SERVER_URL_KEY) || DEFAULT_LOCAL_URL;
    providerConfirmed = true;
  }

  function handleStorageChange(e: StorageEvent) {
    if (e.key === "srt-tools-api-keys") loadApiKeys();
    else if (
      e.key === DEFAULT_LLM_PROVIDER_KEY || e.key === DEFAULT_LLM_MODEL_KEY ||
      e.key === DEFAULT_LLM_CUSTOM_PROVIDER_KEY || e.key === LOCAL_SERVER_URL_KEY
    ) loadDefaultLlmSettings();
    else if (e.key === DEFAULT_TARGET_LANGUAGE_KEY && !vestaConfig.getItem(LAST_TARGET_LANGUAGE_KEY)) {
      targetLang = loadStoredValue(DEFAULT_TARGET_LANGUAGE_KEY) || targetLang;
    }
  }

  onMount(() => {
    loadApiKeys();
    refreshTiers();
    window.addEventListener("storage", handleStorageChange);
    window.addEventListener("apikeys-updated", loadApiKeys);
    window.addEventListener("vesta-llm-default-updated", loadDefaultLlmSettings);
    window.addEventListener(TIERS_UPDATED_EVENT, refreshTiers);

    let activeListener = true;
    let unlistenProg: (() => void) | null = null;
    const cleanupDragDrop = setupWebviewDragDrop({
      isActive: () => active && !isTranslating && !contextLoading,
      setDraggingOver: (v) => (isDraggingOver = v),
      onDrop: handleFileDrop,
    });

    listen<TranslateProgressEvent>("translate-progress", (event) => {
      if (!isTranslating || translationOperation.cancelRequested || disposed) return;
      progress = event.payload;
      addLog(event.payload.message);
    }).then((fn) => {
      if (!activeListener) fn();
      else unlistenProg = fn;
    }).catch(console.error);

    return () => {
      activeListener = false;
      disposed = true;
      modelDiscovery.dispose();
      requirements.clear();
      translationOperation.dispose();
      fileRequests.invalidate();
      previewRequests.invalidate();
      contextRequest += 1;
      window.removeEventListener("storage", handleStorageChange);
      window.removeEventListener("apikeys-updated", loadApiKeys);
      window.removeEventListener("vesta-llm-default-updated", loadDefaultLlmSettings);
      window.removeEventListener(TIERS_UPDATED_EVENT, refreshTiers);
      cleanupDragDrop();
      if (unlistenProg) unlistenProg();
      stopPreviewRefresh();
    };
  });

  async function refreshTranslatedPreview(waitForCurrent = false) {
    if (previewPending) {
      if (!waitForCurrent) return;
      await previewPending;
    }
    if (!inputPath || !outputPath || translationOperation.cancelRequested || disposed) return;
    const request = previewRequests.begin();
    const previewInput = inputPath;
    const previewOutput = outputPath;
    previewPending = (async () => {
      try {
        const pairs = await getLatestTranslatedSubtitles(previewInput, previewOutput, 10);
        if (request.isCurrent() && previewInput === inputPath && previewOutput === outputPath && !translationOperation.cancelRequested && !disposed) {
          translatedPairs = pairs;
        }
      } catch (e) { console.debug("Preview refresh:", e); }
    })().finally(() => { previewPending = null; });
    await previewPending;
  }

  function startPreviewRefresh() {
    if (!previewRefreshInterval) previewRefreshInterval = setInterval(refreshTranslatedPreview, 2000);
  }
  function stopPreviewRefresh() {
    previewRequests.invalidate();
    if (previewRefreshInterval) { clearInterval(previewRefreshInterval); previewRefreshInterval = null; }
  }
  function addLog(message: string, type: LogEntry["type"] = "info") { logBuffer.addLog(message, type); }
  function clearLogs() { logBuffer.clearLogs(); }

  async function selectInputFile() {
    if (isTranslating || contextLoading || !ensureProviderSelectedForFiles()) return;
    try {
      const selected = await guardedOpen({ multiple: false, filters: [{ name: t("translate.inputFile"), extensions: ["srt"] }] });
      if (selected && !isTranslating && !contextLoading && !disposed) {
        inputPath = selected as string;
        await loadFileInfo();
        if (!outputPath) outputPath = generateOutputPath(inputPath, targetLang);
        addLog(`📥 ${t("translate.inputFile")} · ${getFileName(inputPath)}`);
        addLog(`📤 ${t("translate.outputFile")} · ${getFileName(outputPath)}`);
      }
    } catch (e) { error = `${t("translate.errorSelectingFile")} ${errorMessage(e)}`; }
  }

  async function selectOutputFile() {
    if (isTranslating || contextLoading || !ensureProviderSelectedForFiles()) return;
    try {
      const selected = await guardedSave({ filters: [{ name: t("translate.inputFile"), extensions: ["srt"] }], defaultPath: outputPath || undefined });
      if (selected && !isTranslating && !contextLoading && !disposed) {
        automaticOutputPath = false;
        outputPath = selected;
        addLog(`📤 ${t("translate.outputFile")} · ${getFileName(outputPath)}`);
      }
    } catch (e) { error = `${t("translate.errorSelectingFile")} ${errorMessage(e)}`; }
  }

  async function saveInputPath(newPath: string): Promise<boolean> {
    if (isTranslating || contextLoading) return false;
    const cleaned = await validatePickerPath(newPath, "input", transcribeCheckFileExists);
    if (!cleaned || isTranslating || contextLoading) return false;
    inputPath = cleaned;
    await loadFileInfo();
    return true;
  }

  async function saveOutputPath(newPath: string): Promise<boolean> {
    if (isTranslating || contextLoading) return false;
    const cleaned = await validatePickerPath(newPath, "output", transcribeCheckFileExists);
    if (!cleaned || isTranslating || contextLoading) return false;
    automaticOutputPath = false;
    outputPath = cleaned;
    return true;
  }

  async function loadFileInfo() {
    if (!inputPath) return;
    const path = inputPath;
    const request = fileRequests.begin();
    fileLoading = true;
    outputPath = generateOutputPath(path, targetLang);
    automaticOutputPath = true;
    fileInfo = null;
    translatedPairs = [];
    result = null;
    progress = null;
    error = null;
    stopPreviewRefresh();
    try {
      const info = await loadSrtForTranslate(path);
      if (!request.isCurrent() || path !== inputPath || disposed) return;
      fileInfo = info;
      addLog(`📄 ${t("translate.loadedFile", { count: fileInfo.subtitle_count })}`);
    } catch (e) {
      if (!request.isCurrent() || path !== inputPath || disposed) return;
      error = `${t("translate.errorLoading")} ${errorMessage(e)}`;
      fileInfo = null;
    } finally { if (request.isCurrent()) fileLoading = false; }
  }

  async function startTranslation() {
    if (contextLoading || isTranslating || fileLoading || aiStore.killSwitchActive) return;
    if (!fileInfo || !inputPath || !outputPath || !fileInfo.subtitle_count) {
      requirements.pulse(["files"]);
      error = t("translate.selectFileAndKey");
      return;
    }

    const run = translationOperation.begin();
    if (!run) return;
    try {
      const check = await checkTiersAvailability();
      if (run.isCancelled() || disposed) return;
      if (!check.available) { requirements.pulse(["options"]); if (check.errorMsg) snackbar.show(check.errorMsg, "error"); return; }
      const tiersPayload = buildTiersPayload();
      if (!tiersPayload) { error = t("tiers.noneConfigured"); return; }
      error = null;
      result = null;
      progress = null;
      translatedPairs = [];
      addLog(`🚀 ${t("translate.starting")}`);
      addLog(`🌐 ${t("translate.targetLang")} · ${targetLang}`);
      addLog(`⚙️ ${t("translate.batchSize")} · ${effectiveBatchSize} / ${t("translate.resumeOverlapExpert")} · ${effectiveResumeOverlap}`);
      startPreviewRefresh();
      const endpointCount = tiersPayload.reduce((sum, tier) => sum + tier.length, 0);
      addLog(`🪜 ${t("tiers.logActive", { tiers: tiersPayload.length, endpoints: endpointCount })}`);

      const config: TranslateConfig = {
        input_path: inputPath,
        output_path: outputPath,
        target_lang: targetLang,
        batch_size: effectiveBatchSize,
        resume_overlap: effectiveResumeOverlap,
        title_context: titleContext || null,
        tiers: tiersPayload,
      };

      const res = await apiStartTranslation(config);
      if (run.isCancelled() || !run.isCurrent()) return;
      result = res;
      if (res.output_path) addLog(`${t("extract.saved")} ${getFileName(res.output_path)}`);
      addLog(resultMessage(res));
      stopPreviewRefresh();
      await refreshTranslatedPreview(true);
    } catch (e: any) {
      if (run.isCancelled() || !run.isCurrent()) return;
      const msg = errorMessage(e);
      const errorLower = msg.toLowerCase();
      let userMsg = msg;
      if (msg.includes("ERR_ALREADY_RUNNING")) userMsg = t("common.error.alreadyRunning");
      else if (errorLower.includes("429") || errorLower.includes("quota") || errorLower.includes("rate limit")) userMsg = t("translate.error.ratelimit");
      else if (errorLower.includes("401") || errorLower.includes("unauthorized") || errorLower.includes("api key")) userMsg = t("translate.error.apikey");
      else if (errorLower.includes("json") || errorLower.includes("parse")) userMsg = t("translate.error.json");
      error = `${t("translate.errorTranslating")} ${userMsg}`;
      addLog(`❌ ${t("translate.error")}: ${msg}`);
    } finally {
      stopPreviewRefresh();
      progress = null;
      run.finish();
    }
  }

  async function cancelTranslation() {
    try {
      const cancelled = await translationOperation.cancel(apiCancelTranslation);
      if (cancelled && !disposed) { progress = null; stopPreviewRefresh(); addLog(t("translate.cancelled")); }
    } catch (e) {
      error = `${t("translate.errorCancelling")} ${errorMessage(e)}`;
      snackbar.show(error, "error");
    }
  }

  function resetTranslation() {
    if (isTranslating || contextLoading) return;
    fileRequests.invalidate();
    fileLoading = false;
    stopPreviewRefresh();
    result = null;
    error = null;
    progress = null;
    logBuffer.clearLogs();
    translatedPairs = [];
    inputPath = "";
    outputPath = "";
    automaticOutputPath = true;
    fileInfo = null;
  }

  function resultMessage(r: TranslateResult): string {
    const raw = (r.message || "").toLowerCase();
    if (raw.includes("annullat") || raw.includes("cancel")) return t("translate.cancelled");
    if (r.success) return t("translate.resultDone", { count: String(r.translated_count) });
    if (r.translated_count === 0) return t("translate.resultNone");
    return r.message;
  }

  function formatEta(seconds: number | null): string {
    if (seconds === null) return "...";
    return `${Math.floor(seconds / 60)}m ${Math.floor(seconds % 60)}s`;
  }

  function handleShortcut(event: KeyboardEvent) {
    if (!active || expandedPathField || isInputActive()) return;
    const action = localShortcutAction(event, "translate");
    if (action === "openInputFile" && !isTranslating && !fileLoading) {
      event.preventDefault();
      void selectInputFile();
    } else if (action === "startTranslation" && !isTranslating && canStartTranslation) {
      event.preventDefault();
      void startTranslation();
    } else if (action === "cancelTranslation" && isTranslating) {
      event.preventDefault();
      void cancelTranslation();
    } else if (action === "clearLogs") {
      event.preventDefault();
      clearLogs();
    }
  }

  // Keep reference to strings checked by check-i18n
  void (() => [t("provider.local"), t("translate.configureLlmFirst")]);
</script>

<svelte:window onkeydown={handleShortcut} />

<div
  role="region"
  aria-label={t("nav.translate")}
  class="h-full flex flex-col bg-[var(--surface-2)] relative overflow-hidden"
  ondragover={(e) => { e.preventDefault(); if (e.dataTransfer) e.dataTransfer.dropEffect = "copy"; }}
  ondrop={(e) => { e.preventDefault(); isDraggingOver = false; }}
  ondragleave={(e) => {
    const rt = e.relatedTarget as HTMLElement | null;
    const ct = e.currentTarget as HTMLElement;
    if (rt && ct.contains(rt)) return;
    isDraggingOver = false;
  }}
>
  {#if isDraggingOver}
    <div class="absolute inset-0 z-50 bg-[var(--done)]/10 border-2 border-dashed border-[var(--done)] rounded-2xl flex items-center justify-center pointer-events-none">
      <div class="text-center">
        <svg class="w-16 h-16 mx-auto mb-3 text-[var(--done)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
        </svg>
        <p class="text-lg font-medium text-[var(--done)]">{t("translate.dropFileHere")}</p>
        <p class="text-sm text-[var(--muted)] mt-1">{t("translate.dropFileHint")}</p>
      </div>
    </div>
  {/if}

  <div class="flex flex-1 flex-col overflow-hidden p-6 min-h-0">
    <WorkflowWorkspace>
      {#snippet files()}
        <TranslateFilesPanel
          {inputPath}
          {outputPath}
          {fileInfo}
          {canUseFilePanel}
          locked={isTranslating || contextLoading}
          highlightClass={requirements.classFor("files")}
          onSelectInput={selectInputFile}
          onSelectOutput={selectOutputFile}
          onExpand={(field) => (expandedPathField = field)}
        />
      {/snippet}
      <div class="flex flex-1 min-h-full flex-col gap-4">
        <TranslateOptionsPanel
          bind:targetLang
          bind:batchSize
          bind:resumeOverlap
          bind:titleContext
          bind:contextDraft
          {contextDraftPath}
          {contextLoading}
          {contextError}
          {inputPath}
          {fileInfo}
          {useTiers}
          {isTranslating}
          highlightClass={requirements.classFor("options")}
          onSuggestContext={suggestContext}
          onApplyContext={() => {
            titleContext = [titleContext.trim(), contextDraft.trim()].filter(Boolean).join("\n\n");
            contextDraft = "";
          }}
        />
        <TranslateProgressPanel
          {isTranslating}
          {progress}
          {result}
          {error}
          {formatEta}
          {resultMessage}
          onClearError={() => (error = null)}
        />
        <TranslationPreviewPanel
          class="flex-1 min-h-0"
          source={fileInfo?.preview_subtitles ?? []}
          pairs={translatedPairs}
          loaded={!!fileInfo}
          translating={isTranslating}
        />
      </div>
    </WorkflowWorkspace>
  </div>

  {#if false}
    <TranslateProgressPanel mode="logs" logs={logBuffer.logs} {formatEta} {resultMessage} onClearLogs={clearLogs} />
  {/if}

  <TranslateFooterBar
    {isTranslating}
    cancelling={translationOperation.cancelling || translationOperation.cancelRequested}
    {progress}
    {result}
    {error}
    canStart={canStartTranslation}
    {useTiers}
    blockedReason={translationBlockedReason}
    {formatEta}
    onCancel={cancelTranslation}
    onStart={startTranslation}
    onReset={resetTranslation}
  />

  <PathPreviewModal
    isOpen={!!expandedPathField}
    title={expandedPathField === "input" ? t("translate.inputFile") : t("translate.outputFile")}
    value={expandedPathField === "input" ? inputPath : outputPath}
    onclose={() => (expandedPathField = null)}
    editable={true}
    secondaryText={t("transcribe.editPath")}
    desc={expandedPathField === "input" ? t("transcribe.inputPathDesc") : t("transcribe.outputPathDesc")}
    onsave={expandedPathField === "input" ? saveInputPath : saveOutputPath}
  />
</div>
