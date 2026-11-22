<script lang="ts">
  import { localShortcutAction } from "$lib/utils/shortcuts";
  import { isInputActive } from "$lib/utils/keyboardFocus";
  import WorkflowWorkspace from "$lib/components/WorkflowWorkspace.svelte";
  import { RequirementFeedback } from "$lib/utils/requirementFeedback.svelte";
  const requirements = new RequirementFeedback<"files" | "model">();
  import { validatePickerPath } from "$lib/utils/pathValidation";
  import { localWhisperOperation } from "$lib/utils/asyncControl.svelte";
  import { TranscriptionResources } from "$lib/workflows/transcriptionResources.svelte";
  import TranscriptionSegmentsPanel from "$lib/panels/TranscriptionSegmentsPanel.svelte";
  import { formatElapsedTime } from "$lib/utils/elapsedTime";
  import { generateTranscriptionOutputPath as generateOutputPathFromInput, rewriteTranscriptionOutputLanguage as rewriteOutputPathLanguage } from "$lib/utils/transcriptionPaths";
  import { runTranscription, isTranscribeEndpointReady } from "$lib/workflows/transcription";
  import type { LogEntry } from "$lib/panels/LogPanel.svelte";
  import { whisperModelsStore as modelStore } from "$lib/stores/whisperModelsStore.svelte";
  import { subscribeTranscriptionEvents } from "$lib/workflows/transcriptionEvents";
  import { guardedOpen, guardedSave } from "$lib/utils/dialogGuard";
  import { setupWebviewDragDrop } from "$lib/utils/dragDrop";
  import { createLogPanelBuffer } from "$lib/utils/logPanelBuffer.svelte";
  import { onMount } from "svelte";
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import { errorMessage } from "$lib/utils/errorMessage";
  import { transcribeProviders } from "$lib/config/transcribeProviders";
  import {
    loadTranscribeTiers,
    TRANSCRIBE_TIERS_UPDATED_EVENT,
    type TranscribeTier,
    type TranscribeTierEntry,
  } from "$lib/config/transcribeTiers";
  import { loadAndValidateApiKeys, type ApiKeyConfig } from "$lib/config/apiKeys";
  import { languages as allLanguages } from "$lib/config/languages";
  import PathPreviewModal from "$lib/modals/PathPreviewModal.svelte";
  import { createSnackbarNotifier } from "$lib/stores/snackbarStore.svelte";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import * as vestaConfig from "$lib/config/vestaConfig";
  import { transcribeCheckFileExists, transcribeStart, transcribeCancel } from "$lib/services/transcribe";
  import { downloadFfmpeg } from "$lib/services/media";
  import TranscribeFfmpegBanner from "./transcribe/TranscribeFfmpegBanner.svelte";
  import TranscribeFilesPanel from "./transcribe/TranscribeFilesPanel.svelte";
  import TranscribeOptionsPanel from "./transcribe/TranscribeOptionsPanel.svelte";
  import TranscribeFooterBar from "./transcribe/TranscribeFooterBar.svelte";
  import TranscribeStatusPanels from "./transcribe/TranscribeStatusPanels.svelte";

  const MEDIA_EXTENSIONS = [
    "mp4", "mkv", "avi", "webm", "mov", "wmv", "flv", "m4v", "ts", "3gp",
    "mpeg", "mpg", "m2ts", "vob", "mp3", "wav", "m4a", "flac", "ogg", "aac",
    "wma", "amr", "opus", "aiff", "alac",
  ];
  const VALID_MEDIA_EXT_SET = new Set(MEDIA_EXTENSIONS);
  const LAST_TRANSCRIBE_LANGUAGE_KEY = "vesta-transcribe-source-language";
  const DEFAULT_TRANSCRIBE_LANGUAGE_KEY = "vesta-default-transcribe-language";
  const TRANSCRIBE_QUALITY_KEY = "vesta-transcribe-quality";
  const TRANSCRIBE_VAD_KEY = "vesta-transcribe-vad";

  let { onGoToSettings, active = false } = $props<{
    onGoToSettings?: (section?: "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts", highlightItemId?: string) => void;
    active?: boolean;
  }>();

  let t = $derived($locale);

  let inputPath = $state("");
  let outputPath = $state("");
  const initialTranscribeLanguage =
    vestaConfig.getItem(LAST_TRANSCRIBE_LANGUAGE_KEY) ||
    vestaConfig.getItem(DEFAULT_TRANSCRIBE_LANGUAGE_KEY) ||
    "auto";
  let selectedLanguage = $state(initialTranscribeLanguage);
  let previousLanguageForOutput = initialTranscribeLanguage;
  let translateToEnglish = $state(false);
  let maxSegmentLength = $state(30);

  let qualityMode = $state(vestaConfig.getItem(TRANSCRIBE_QUALITY_KEY) === "true");
  const storedVad = vestaConfig.getItem(TRANSCRIBE_VAD_KEY);
  let vadEnabled = $state(storedVad === null ? true : storedVad === "true");
  const transcriptionResources = new TranscriptionResources();
  let vadInstalled = $derived(transcriptionResources.vadInstalled);
  let vadSelection = $derived(transcriptionResources.vadSelection);
  const refreshAddons = () => transcriptionResources.refreshAddons();
  const refreshModels = () => transcriptionResources.refreshModels();

  function setQualityMode(enabled: boolean) {
    qualityMode = enabled;
    vestaConfig.setItem(TRANSCRIBE_QUALITY_KEY, String(qualityMode));
  }
  function setVadEnabled(enabled: boolean) {
    vadEnabled = enabled;
    vestaConfig.setItem(TRANSCRIBE_VAD_KEY, String(vadEnabled));
  }

  let isTranscribing = $derived(localWhisperOperation.busy && localWhisperOperation.activeAction === "transcription");
  $effect(() => { aiStore.isTranscribing = isTranscribing; });

  let progress = $state(0);
  let progressMessage = $state("");
  let progressStage = $state("");
  let error = $state<string | null>(null);
  let result = $state<{
    success: boolean;
    message: string;
    output_path?: string;
    subtitle_count?: number;
    detected_language?: string;
  } | null>(null);

  const showSnackbar = createSnackbarNotifier();
  const logBuffer = createLogPanelBuffer();
  let expandedPathField = $state<string | null>(null);
  let isDraggingOver = $state(false);
  let showOverwriteConfirm = $state(false);
  let pendingDroppedPaths = $state<string[]>([]);
  let isDownloadingFFmpeg = $state(false);

  let backends = $derived(transcriptionResources.backends);
  let whisperModels = $derived(transcriptionResources.models);
  let transcribeTiers = $state<TranscribeTier[]>([]);
  let selectedEngine = $state<TranscribeTierEntry | null | undefined>(undefined);
  let effectiveTiers = $derived(selectedEngine === null
    ? transcribeTiers
    : selectedEngine ? [{ id: "selected", entries: [selectedEngine] }] : []);
  let apiKeys = $state<ApiKeyConfig[]>([]);
  let transcribedSegments = $state<{ start_ms: number; end_ms: number; text: string }[]>([]);

  const refreshTranscribeTiers = () => { transcribeTiers = loadTranscribeTiers(); };
  const refreshApiKeys = () => { apiKeys = loadAndValidateApiKeys(); };
  let usableEntries = $derived(
    effectiveTiers.flatMap((tier) => tier.entries).filter((entry) => isTranscribeEndpointReady(entry, whisperModels, apiKeys))
  );
  let canStart = $derived(
    !!inputPath && !!outputPath && usableEntries.length > 0 && !localWhisperOperation.busy && !aiStore.killSwitchActive
  );

  let transcribeBlockedReason = $derived.by(() => {
    if (aiStore.killSwitchActive) return t("nav.aiKillSwitchOn");
    if (localWhisperOperation.busy) return t("common.error.alreadyRunning");
    if (!inputPath || !outputPath) return t("transcribe.blocked.selectFiles");
    if (effectiveTiers.length === 0 || effectiveTiers.flatMap((tier) => tier.entries).length === 0) {
      return t("transcribe.blocked.noTiers");
    }
    if (usableEntries.length === 0) {
      const hasLocalConfigured = effectiveTiers.flatMap((tier) => tier.entries).some((e) => e.provider === "local" || e.provider === "local_whisper");
      return hasLocalConfigured ? t("transcribe.blocked.missingModel") : t("transcribe.blocked.noEndpoint");
    }
    return "";
  });

  let transcriptionLanguages = $derived([
    { code: "auto", name: t("transcribe.autoDetect"), nameEn: "Auto-detect", flag: "🌐" },
    ...allLanguages,
  ]);

  function effectiveLanguageCodeForOutput(langCode: string): string {
    if (langCode !== "auto") return langCode;
    return result?.detected_language?.toLowerCase() || "auto";
  }

  function selectedLanguageLabel(code: string): string {
    return transcriptionLanguages.find((l) => l.code === code)?.nameEn ?? code;
  }

  $effect(() => {
    const currentLang = selectedLanguage;
    const effectiveCurrentLang = effectiveLanguageCodeForOutput(currentLang);
    const effectivePrevLang = effectiveLanguageCodeForOutput(previousLanguageForOutput);

    if (currentLang !== previousLanguageForOutput && inputPath) {
      outputPath = outputPath
        ? rewriteOutputPathLanguage(outputPath, effectiveCurrentLang)
        : generateOutputPathFromInput(inputPath, effectiveCurrentLang);
      addLog(`Language set to ${selectedLanguageLabel(currentLang)}; output updated to ${getFileName(outputPath)}`, "info");
      previousLanguageForOutput = currentLang;
      if (effectivePrevLang !== effectiveCurrentLang) result = null;
    }
  });

  $effect(() => { vestaConfig.setItem(LAST_TRANSCRIBE_LANGUAGE_KEY, selectedLanguage); });

  function handleWhisperModelUpdated() {
    refreshTranscribeTiers();
    refreshModels().catch(console.error);
  }

  onMount(() => {
    void modelStore.refreshModels();
    window.addEventListener("keydown", handleKeydown);
    refreshTranscribeTiers();
    refreshApiKeys();
    window.addEventListener(TRANSCRIBE_TIERS_UPDATED_EVENT, refreshTranscribeTiers);
    window.addEventListener("apikeys-updated", refreshApiKeys);
    window.addEventListener("whisper-model-updated", handleWhisperModelUpdated);
    window.addEventListener("vesta-language-defaults-updated", handleLanguageDefaultsUpdated);

    const refreshBackends = () => transcriptionResources.refreshBackends();
    refreshBackends();
    window.addEventListener("vesta-ffmpeg-updated", refreshBackends);
    refreshModels().catch((e) => console.error("Could not list models:", e));
    void refreshAddons();
    window.addEventListener("vesta-whisper-models-updated", refreshModels);
    window.addEventListener("vesta-vad-updated", refreshAddons);

    const cleanupDragDrop = setupWebviewDragDrop({
      isActive: () => active,
      setDraggingOver: (v) => (isDraggingOver = v),
      onDrop: handleDroppedFiles,
      onError: console.error,
    });

    const cleanupEvents = subscribeTranscriptionEvents({
      acceptingResults: () => isTranscribing && !transcriptionCancelled,
      downloadingModel: () => modelStore.isDownloading || !!modelStore.downloadingVadId,
      onDownload: (p) => {
        modelStore.progress = Math.round(p.percentage);
        modelStore.progressMessage = p.message;
      },
      onProgress: (p) => {
        progress = Math.round(p.percentage);
        progressMessage = p.message;
        progressStage = p.stage;
        if (p.stage !== "done") addLog(p.message, "progress");
      },
      onSegment: (p) => { transcribedSegments.push({ ...p }); },
    });

    return () => {
      cleanupEvents();
      transcriptionResources.dispose();
      requirements.clear();
      window.removeEventListener("vesta-whisper-models-updated", refreshModels);
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener(TRANSCRIBE_TIERS_UPDATED_EVENT, refreshTranscribeTiers);
      window.removeEventListener("apikeys-updated", refreshApiKeys);
      window.removeEventListener("whisper-model-updated", handleWhisperModelUpdated);
      window.removeEventListener("vesta-language-defaults-updated", handleLanguageDefaultsUpdated);
      window.removeEventListener("vesta-ffmpeg-updated", refreshBackends);
      window.removeEventListener("vesta-vad-updated", refreshAddons);
      cleanupDragDrop();
    };
  });

  function handleLanguageDefaultsUpdated() {
    if (!vestaConfig.getItem(LAST_TRANSCRIBE_LANGUAGE_KEY)) {
      selectedLanguage = vestaConfig.getItem(DEFAULT_TRANSCRIBE_LANGUAGE_KEY) || "auto";
      previousLanguageForOutput = selectedLanguage;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!active || showOverwriteConfirm || expandedPathField || isInputActive()) return;
    const action = localShortcutAction(e, "transcribe");
    if (action === "openInputFile" && !isTranscribing) {
      e.preventDefault();
      void selectInputFile();
    } else if (action === "startTranscription" && canStart) {
      e.preventDefault();
      void startTranscription();
    } else if (action === "cancelTranscription" && isTranscribing) {
      e.preventDefault();
      void cancelTranscription();
    }
  }

  function addLog(message: string, type: LogEntry["type"] = "info") { logBuffer.addLog(message, type); }
  function clearLogs() { logBuffer.clearLogs(); }

  async function selectInputFile() {
    if (isTranscribing) return;
    try {
      const selected = await guardedOpen({
        multiple: false,
        filters: [{ name: t("transcribe.audioVideoFiles"), extensions: MEDIA_EXTENSIONS }],
      });
      if (selected) loadInputMedia(selected as string);
    } catch (e) {
      error = `${t("transcribe.errorSelectingFile")}: ${errorMessage(e)}`;
    }
  }

  function isAudioVideoFile(path: string) {
    const ext = path.split(".").pop()?.toLowerCase();
    return ext ? VALID_MEDIA_EXT_SET.has(ext) : false;
  }

  function handleDroppedFiles(paths: string[]) {
    if (isTranscribing) return;
    const mediaPaths = paths.filter(isAudioVideoFile).slice(-1);
    if (mediaPaths.length === 0) return;
    if (inputPath) {
      pendingDroppedPaths = mediaPaths;
      showOverwriteConfirm = true;
    } else {
      loadInputMedia(mediaPaths[0]);
    }
  }

  function confirmOverwrite() {
    showOverwriteConfirm = false;
    if (pendingDroppedPaths.length > 0) {
      loadInputMedia(pendingDroppedPaths[0]);
      pendingDroppedPaths = [];
    }
  }

  async function selectOutputFile() {
    try {
      const selected = await guardedSave({
        filters: [{ name: t("transcribe.outputSrtFile"), extensions: ["srt"] }],
        defaultPath: outputPath || undefined,
      });
      if (selected) {
        outputPath = selected;
        addLog(`${t("transcribe.outputSrtFile")} · ${getFileName(outputPath)}`, "file");
      }
    } catch (e) {
      error = `${t("transcribe.errorSelectingFile")}: ${errorMessage(e)}`;
    }
  }

  async function saveInputPath(newPath: string): Promise<boolean> {
    if (isTranscribing) return false;
    const cleaned = await validatePickerPath(newPath, "input", transcribeCheckFileExists);
    if (!cleaned || isTranscribing) return false;
    loadInputMedia(cleaned);
    return true;
  }

  async function saveOutputPath(newPath: string): Promise<boolean> {
    if (isTranscribing) return false;
    const cleaned = await validatePickerPath(newPath, "output", transcribeCheckFileExists);
    if (!cleaned || isTranscribing) return false;
    outputPath = cleaned;
    return true;
  }

  let transcriptionCancelled = false;

  async function startTranscription() {
    if (localWhisperOperation.busy || aiStore.killSwitchActive) return;
    if (!inputPath || !outputPath) {
      requirements.pulse(["files"]);
      error = t("transcribe.selectFilesFirst");
      showSnackbar(t("transcribe.selectFilesFirst"));
      return;
    }
    if (!canStart) {
      requirements.pulse(["model"]);
      error = transcribeBlockedReason;
      showSnackbar(transcribeBlockedReason);
      return;
    }

    error = null;
    result = null;
    progress = 0;
    transcribedSegments = [];
    const run = localWhisperOperation.begin("transcription");
    if (!run) return;

    const startTime = Date.now();
    transcriptionCancelled = false;
    try {
      const outcome = await runTranscription({
        tiers: $state.snapshot(effectiveTiers),
        models: $state.snapshot(whisperModels),
        keys: $state.snapshot(apiKeys),
        settings: {
          input_path: inputPath,
          output_path: outputPath,
          language: selectedLanguage,
          translate_to_english: translateToEnglish,
          word_timestamps: true,
          max_segment_length: maxSegmentLength,
          quality: qualityMode,
          vad: vadEnabled && vadInstalled,
          vad_model_id: vadSelection.customPath ? null : vadSelection.modelId,
          vad_custom_path: vadSelection.customPath,
        },
        start: transcribeStart,
        isCancelled: () => transcriptionCancelled || run.isCancelled(),
        onTier: (index, count) => addLog(`${t("tiers.tier")} ${index + 1} · ${count}`, "info"),
        onAttempt: (entry) => {
          transcribedSegments = [];
          progress = 0;
          progressMessage = "";
          progressStage = "";
          addLog(`${transcribeProviders[entry.provider]?.name || entry.provider} · ${entry.model || "auto"}`, "info");
        },
        onFailure: (entry, message) => addLog(`${entry.provider} · ${entry.model}: ${message}`, "warning"),
      });
      if (outcome.status === "success") {
        result = outcome.result;
        if (result.output_path) outputPath = result.output_path;
        addLog(result.message, "success");
        await refreshModels();
      } else if (outcome.status === "failed") {
        error = `${t("transcribe.error")}: ${outcome.error}`;
        addLog(error, "error");
      }
    } catch (e) {
      if (!transcriptionCancelled) {
        error = `${t("transcribe.error")}: ${errorMessage(e)}`;
        addLog(error, "error");
      }
    } finally {
      run.finish();
      addLog(`⏱ ${formatElapsedTime(Date.now() - startTime)}`, "info");
    }
  }

  async function cancelTranscription() {
    if (!isTranscribing || localWhisperOperation.cancelRequested) return;
    transcriptionCancelled = true;
    try {
      if (await localWhisperOperation.cancel(transcribeCancel)) {
        progress = 0;
        progressMessage = "";
        addLog(t("transcribe.cancelled"), "warning");
      }
    } catch (e) {
      transcriptionCancelled = false;
      error = `${t("transcribe.error")}: ${errorMessage(e)}`;
      showSnackbar(error, "error");
    }
  }

  function loadInputMedia(path: string) {
    if (isTranscribing || path === inputPath) return;
    clearTranscriptionResult();
    inputPath = path;
    outputPath = generateOutputPathFromInput(path, selectedLanguage);
    addLog(`${t("transcribe.fileSelected")}: ${getFileName(path)}`, "file");
    addLog(`${t("transcribe.outputSrtFile")} · ${getFileName(outputPath)}`, "info");
  }

  function resetTranscription() {
    if (isTranscribing) return;
    clearTranscriptionResult();
    inputPath = "";
    outputPath = "";
  }

  function clearTranscriptionResult() {
    transcribedSegments = [];
    transcriptionCancelled = false;
    progress = 0;
    progressMessage = "";
    progressStage = "";
    error = null;
    result = null;
    clearLogs();
  }

  async function handleDownloadFfmpeg() {
    isDownloadingFFmpeg = true;
    try {
      await downloadFfmpeg();
      await transcriptionResources.refreshBackends();
      window.dispatchEvent(new CustomEvent("vesta-ffmpeg-updated"));
    } catch (e) {
      error = `${t("flashcards.ffmpegDownloadFailed")}: ${errorMessage(e)}`;
    } finally {
      isDownloadingFFmpeg = false;
    }
  }
</script>

<div
  role="region"
  aria-label={t("nav.transcribe")}
  class="h-full flex flex-col bg-gray-900 relative overflow-hidden"
  ondragover={(e) => {
    if (!active) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
    isDraggingOver = true;
  }}
  ondrop={(e) => { if (active) { e.preventDefault(); isDraggingOver = false; } }}
  ondragleave={(e) => {
    if (!active) return;
    const rt = e.relatedTarget as HTMLElement | null;
    const ct = e.currentTarget as HTMLElement;
    if (rt && ct.contains(rt)) return;
    isDraggingOver = false;
  }}
>
  {#if isDraggingOver}
    <div class="absolute inset-0 z-50 bg-indigo-500/10 border-2 border-dashed border-indigo-400 rounded-2xl flex items-center justify-center pointer-events-none">
      <div class="text-center">
        <svg class="w-16 h-16 mx-auto mb-3 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
        </svg>
        <p class="text-lg font-semibold text-indigo-300">{t("transcribe.dropFileHere")}</p>
        <p class="text-xs text-indigo-500 mt-1">{t("transcribe.dropFileHint")}</p>
      </div>
    </div>
  {/if}

  {#if backends && !backends.ffmpeg}
    <TranscribeFfmpegBanner downloading={isDownloadingFFmpeg} onDownload={handleDownloadFfmpeg} />
  {/if}

  <div class="flex flex-col flex-1 overflow-hidden p-6 min-h-0 {isTranscribing ? 'pointer-events-none opacity-60 select-none' : ''}">
    <WorkflowWorkspace>
      {#snippet files()}
        <TranscribeFilesPanel
          {inputPath}
          {outputPath}
          disabled={isTranscribing}
          highlightClass={requirements.classFor("files")}
          onSelectInput={selectInputFile}
          onSelectOutput={selectOutputFile}
          onExpand={(field) => (expandedPathField = field)}
        />
      {/snippet}
      <div class="grid grid-cols-1 items-start gap-6 xl:grid-cols-2">
        <div class="min-w-0">
          <TranscribeOptionsPanel
            {active}
            {isTranscribing}
            modelHighlightClass={requirements.classFor("model")}
            bind:selectedEngine
            bind:selectedLanguage
            {vadInstalled}
            {vadEnabled}
            {qualityMode}
            bind:maxSegmentLength
            {onGoToSettings}
            onToggleVad={setVadEnabled}
            onToggleQuality={setQualityMode}
          />
        </div>
        <div class="flex min-h-0 min-w-0 flex-col xl:relative xl:self-stretch">
          <div class="flex max-h-[42rem] min-h-0 flex-col xl:absolute xl:inset-0 xl:max-h-none">
            <TranscriptionSegmentsPanel segments={transcribedSegments} />
          </div>
        </div>
      </div>
    </WorkflowWorkspace>
  </div>

  {#if false}
    <TranscribeStatusPanels mode="logs" logs={logBuffer.logs} onClearLogs={clearLogs} />
  {/if}

  <TranscribeFooterBar
    {isTranscribing}
    cancelling={localWhisperOperation.cancelling || localWhisperOperation.cancelRequested}
    {progress}
    {progressMessage}
    {result}
    {error}
    {canStart}
    blockedReason={transcribeBlockedReason}
    onCancel={cancelTranscription}
    onReset={resetTranscription}
    onStart={startTranscription}
  />

  <PathPreviewModal
    isOpen={!!expandedPathField}
    title={expandedPathField === "input" ? t("transcribe.inputFile") : t("transcribe.outputFile")}
    value={expandedPathField === "input" ? inputPath : outputPath}
    onclose={() => (expandedPathField = null)}
    editable={true}
    secondaryText={t("transcribe.editPath")}
    desc={expandedPathField === "input" ? t("transcribe.inputPathDesc") : t("transcribe.outputPathDesc")}
    onsave={expandedPathField === "input" ? saveInputPath : saveOutputPath}
  />

  <ConfirmDialog
    show={showOverwriteConfirm}
    title={t("common.replaceFilesTitle")}
    message={t("common.replaceFilesMessage")}
    confirmText={t("sync.confirm")}
    cancelText={t("common.cancel")}
    variant="warning"
    on:cancel={() => { showOverwriteConfirm = false; pendingDroppedPaths = []; }}
    on:confirm={confirmOverwrite}
  />
</div>
