<script lang="ts">
  import WorkflowErrorStatus from "$lib/components/WorkflowErrorStatus.svelte";
  import WorkflowWorkspace from "$lib/components/WorkflowWorkspace.svelte";
  import { RequirementFeedback } from "$lib/utils/requirementFeedback.svelte";
  const requirements = new RequirementFeedback<"files" | "model">();
  import { validatePickerPath } from "$lib/utils/pathValidation";
  import { localWhisperOperation } from "$lib/utils/asyncControl.svelte";
  import { TranscriptionResources } from "$lib/workflows/transcriptionResources.svelte";
  import TranscriptionSegmentsPanel from "$lib/panels/TranscriptionSegmentsPanel.svelte";
  import { formatElapsedTime } from "$lib/utils/elapsedTime";
  import { generateTranscriptionOutputPath as generateOutputPathFromInput, rewriteTranscriptionOutputLanguage as rewriteOutputPathLanguage } from "$lib/utils/transcriptionPaths";
  import { runTranscription, isTranscribeEndpointReady, withLocalModelOverride } from "$lib/workflows/transcription";
  import LogPanel, { type LogEntry } from "$lib/panels/LogPanel.svelte";
  import WhisperModelActionsMenu from "$lib/components/WhisperModelActionsMenu.svelte";
  import WhisperModelSelector from "$lib/components/WhisperModelSelector.svelte";
  import MediaIcon from "$lib/components/MediaIcon.svelte";
  import { whisperModelsStore as modelStore } from "$lib/stores/whisperModelsStore.svelte";
  import CopyPathChip from "$lib/components/CopyPathChip.svelte";
  import { subscribeTranscriptionEvents } from "$lib/workflows/transcriptionEvents";
  import { guardedOpen, guardedSave } from "$lib/utils/dialogGuard";
  import { setupWebviewDragDrop } from "$lib/utils/dragDrop";
  import { createLogPanelBuffer } from "$lib/utils/logPanelBuffer.svelte";
  import { onMount } from "svelte";
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import { errorMessage } from "$lib/utils/errorMessage";
  import {
    transcribeProviders,
    transcribeProviderOrder,
  } from "$lib/config/transcribeProviders";
  import {
    loadTranscribeTiers,
    TRANSCRIBE_TIERS_UPDATED_EVENT,
    type TranscribeTier,
    type TranscribeTierEntry,
  } from "$lib/config/transcribeTiers";
  import { loadAndValidateApiKeys, type ApiKeyConfig } from "$lib/config/apiKeys";
  import { languages as allLanguages } from "$lib/config/languages";
  import PathPickerField from "$lib/components/PathPickerField.svelte";
  import PathPreviewModal from "$lib/modals/PathPreviewModal.svelte";
  import LanguageSelect from "$lib/components/LanguageSelect.svelte";
  import ToggleRow from "$lib/components/ToggleRow.svelte";
  import { createSnackbarNotifier } from "$lib/stores/snackbarStore.svelte";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import FooterActions from "$lib/components/FooterActions.svelte";
  import SettingsJumpButton from "$lib/components/SettingsJumpButton.svelte";
  import * as vestaConfig from "$lib/config/vestaConfig";
  import {
    transcribeCheckFileExists,
    transcribeStart,
    transcribeCancel,
  } from "$lib/services/transcribe";
  import { downloadFfmpeg } from "$lib/services/media";

  let { onGoToSettings, active = false } = $props<{
    onGoToSettings?: (section?: "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts", highlightItemId?: string) => void;
    active?: boolean;
  }>();

  let t = $derived($locale);

  let inputPath = $state("");
  let outputPath = $state("");
  let selectedModel = $state("base");
  let localModelOverride = $state<string | null>(null);
  let modelMenu = $state<{ x: number; y: number; id: string } | null>(null);
  let contextModel = $derived(modelStore.whisperModels.find(model => model.id === modelMenu?.id));
  const LAST_TRANSCRIBE_LANGUAGE_KEY = "vesta-transcribe-source-language";
  const DEFAULT_TRANSCRIBE_LANGUAGE_KEY = "vesta-default-transcribe-language";
  const initialTranscribeLanguage =
    vestaConfig.getItem(LAST_TRANSCRIBE_LANGUAGE_KEY) ||
    vestaConfig.getItem(DEFAULT_TRANSCRIBE_LANGUAGE_KEY) ||
    "auto";
  let selectedLanguage = $state(initialTranscribeLanguage);
  let previousLanguageForOutput = initialTranscribeLanguage;
  let translateToEnglish = $state(false);
  let maxSegmentLength = $state(30);

  // ─── Local-whisper add-ons: quality (beam search), VAD ──────────────────────
  const TRANSCRIBE_QUALITY_KEY = "vesta-transcribe-quality";
  const TRANSCRIBE_VAD_KEY = "vesta-transcribe-vad";
  let qualityMode = $state(vestaConfig.getItem(TRANSCRIBE_QUALITY_KEY) === "true");
  // Default ON when installed; users can disable it for quiet speech that
  // the detector may miss. Never enable VAD without an available model.
  const storedVad = vestaConfig.getItem(TRANSCRIBE_VAD_KEY);
  let vadEnabled = $state(storedVad === null ? true : storedVad === "true");
  const transcriptionResources = new TranscriptionResources();
  let vadInstalled = $derived(transcriptionResources.vadInstalled);
  let gpuSupported = $derived(transcriptionResources.gpuSupported);
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

  const segmentPresets = [
    { id: "short", value: 10 },
    { id: "medium", value: 20 },
    { id: "standard", value: 30 },
    { id: "long", value: 60 },
  ] as const;
  let activeSegmentPreset = $derived(
    segmentPresets.find((p) => p.value === maxSegmentLength)?.id ?? null,
  );
  function setSegmentPreset(presetId: string) {
    const preset = segmentPresets.find((p) => p.id === presetId);
    if (preset) maxSegmentLength = preset.value;
  }

  let isTranscribing = $derived(localWhisperOperation.busy && localWhisperOperation.activeAction === "transcription");
  $effect(() => {
    aiStore.isTranscribing = isTranscribing;
  });
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

  $effect(() => { if (!active || !uiMode.expertMode) modelMenu = null; });
  let isDraggingOver = $state(false);
  let showOverwriteConfirm = $state(false);
  let pendingDroppedPaths = $state<string[]>([]);

  let backends = $derived(transcriptionResources.backends);
  let whisperModels = $derived(transcriptionResources.models);
  let isDownloadingFFmpeg = $state(false);

  let isModelDownloaded = $derived(
    whisperModels.find((m) => m.id === selectedModel)?.downloaded ?? false
  );
  let hasAnyWhisperModel = $derived(whisperModels.some((m) => m.downloaded));

  // ─── Transcription engine (Local Whisper vs cloud providers) ────────────────
  let transcribeTiers = $state<TranscribeTier[]>([]);
  let effectiveTiers = $derived(withLocalModelOverride(transcribeTiers, localModelOverride));
  let apiKeys = $state<ApiKeyConfig[]>([]);
  let transcribedSegments = $state<{ start_ms: number; end_ms: number; text: string }[]>([]);

  function refreshTranscribeTiers() {
    transcribeTiers = loadTranscribeTiers();
  }
  function refreshApiKeys() {
    apiKeys = loadAndValidateApiKeys();
  }

  function isTranscribeEntryReady(entry: TranscribeTierEntry): boolean {
    return isTranscribeEndpointReady(entry, whisperModels, apiKeys);
  }

  let usableEntries = $derived(
    effectiveTiers.flatMap((t) => t.entries).filter(isTranscribeEntryReady)
  );

  let canStart = $derived(
    !!inputPath && !!outputPath && usableEntries.length > 0 && !localWhisperOperation.busy && !aiStore.killSwitchActive
  );

  let transcribeBlockedReason = $derived.by(() => {
    if (aiStore.killSwitchActive) return t("nav.aiKillSwitchOn");
    if (localWhisperOperation.busy) return t("common.error.alreadyRunning");
    if (!inputPath || !outputPath) {
      return t("transcribe.blocked.selectFiles");
    }
    if (effectiveTiers.length === 0 || effectiveTiers.flatMap((t) => t.entries).length === 0) {
      return t("transcribe.blocked.noTiers");
    }
    if (usableEntries.length === 0) {
      const hasLocalConfigured = effectiveTiers.flatMap((t) => t.entries).some(e => e.provider === "local" || e.provider === "local_whisper");
      if (hasLocalConfigured) {
        return t("transcribe.blocked.missingModel");
      }
      return t("transcribe.blocked.noEndpoint");
    }
    return "";
  });

  function handleTranscribeCloudUpdated() {
    // Keep empty or placeholder since tiers are used now
  }

  // Languages for transcription - use the same list as translation tab, with auto-detect option
  let transcriptionLanguages = $derived([
    {
      code: "auto",
      name: t("transcribe.autoDetect"),
      nameEn: "Auto-detect",
      flag: "🌐",
    },
    ...allLanguages,
  ]);


  function effectiveLanguageCodeForOutput(langCode: string): string {
    if (langCode !== "auto") return langCode;
    return result?.detected_language?.toLowerCase() || "auto";
  }

  function selectedLanguageLabel(code: string): string {
    const lang = transcriptionLanguages.find((l) => l.code === code);
    return lang ? lang.nameEn : code;
  }

  $effect(() => {
    const currentLang = selectedLanguage;
    const effectiveCurrentLang = effectiveLanguageCodeForOutput(currentLang);
    const effectivePrevLang = effectiveLanguageCodeForOutput(previousLanguageForOutput);

    if (currentLang !== previousLanguageForOutput && inputPath) {
      if (outputPath) {
        outputPath = rewriteOutputPathLanguage(outputPath, effectiveCurrentLang);
      } else {
        outputPath = generateOutputPathFromInput(inputPath, effectiveCurrentLang);
      }
      addLog(
        `Language set to ${selectedLanguageLabel(currentLang)}; output updated to ${getFileName(outputPath)}`,
        "info",
      );
      previousLanguageForOutput = currentLang;
      if (effectivePrevLang !== effectiveCurrentLang) {
        result = null;
      }
    }
  });

  $effect(() => {
    vestaConfig.setItem(LAST_TRANSCRIBE_LANGUAGE_KEY, selectedLanguage);
  });

  function handleWhisperModelUpdated(e: Event) {
    const detail = (e as CustomEvent).detail;
    if (detail) {
      selectedModel = detail;
    }
    refreshModels().catch(console.error);
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
    refreshTranscribeTiers();
    refreshApiKeys();
    window.addEventListener(TRANSCRIBE_TIERS_UPDATED_EVENT, refreshTranscribeTiers);
    window.addEventListener("apikeys-updated", refreshApiKeys);
    selectedModel = vestaConfig.getItem("srt-default-whisper-model") || "base";

    window.addEventListener("whisper-model-updated", handleWhisperModelUpdated);
    window.addEventListener("vesta-language-defaults-updated", handleLanguageDefaultsUpdated);
    window.addEventListener("vesta:transcribe-cloud-updated", handleTranscribeCloudUpdated);

    const refreshBackends = () => transcriptionResources.refreshBackends();
    refreshBackends();
    window.addEventListener("vesta-ffmpeg-updated", refreshBackends);

    refreshModels().catch((e) => console.error("Could not list models:", e));
    void refreshAddons();
    void modelStore.refreshModels();
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
      onDownload: p => {
        modelStore.progress = Math.round(p.percentage);
        modelStore.progressMessage = p.message;
      },
      onProgress: p => {
        progress = Math.round(p.percentage);
        progressMessage = p.message;
        progressStage = p.stage;
        if (p.stage !== "done") addLog(p.message, "progress");
      },
      onSegment: p => { transcribedSegments.push({ ...p }); },
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
      window.removeEventListener("vesta:transcribe-cloud-updated", handleTranscribeCloudUpdated);
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
    if (!active) return;
    if (
      document.activeElement?.tagName === "INPUT" ||
      document.activeElement?.tagName === "TEXTAREA"
    )
      return;

    // Ctrl+O: Open file
    if (e.key === "o" && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
      e.preventDefault();
      selectInputFile();
      return;
    }
    
    // Ctrl+Enter: Start transcription
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      if (!isTranscribing) startTranscription();
      return;
    }

    // Escape: Cancel transcription
    if (e.key === "Escape") {
      if (isTranscribing) {
        e.preventDefault();
        cancelTranscription();
      }
      return;
    }
  }

  function addLog(message: string, type: LogEntry["type"] = "info") {
    logBuffer.addLog(message, type);
  }

  function clearLogs() {
    logBuffer.clearLogs();
  }


  async function selectInputFile() {
    if (isTranscribing) return;
    try {
      const selected = await guardedOpen({
        multiple: false,
        filters: [
          {
            name: t("transcribe.audioVideoFiles"),
            extensions: [
              "mp4",
              "mkv",
              "avi",
              "webm",
              "mov",
              "wmv",
              "flv",
              "m4v",
              "ts",
              "3gp",
              "mpeg",
              "mpg",
              "m2ts",
              "vob",
              "mp3",
              "wav",
              "m4a",
              "flac",
              "ogg",
              "aac",
              "wma",
              "amr",
              "opus",
              "aiff",
              "alac",
            ],
          },
        ],
      });

      if (selected) {
        loadInputMedia(selected as string);
      }
    } catch (e) {
      error = `${t("transcribe.errorSelectingFile")}: ${errorMessage(e)}`;
    }
  }

  const validMediaExtensions = new Set([
    "mp4", "mkv", "avi", "webm", "mov", "wmv", "flv", "m4v", "ts", "3gp",
    "mpeg", "mpg", "m2ts", "vob", "mp3", "wav", "m4a", "flac", "ogg", "aac",
    "wma", "amr", "opus", "aiff", "alac"
  ]);

  function isAudioVideoFile(path: string) {
    const ext = path.split('.').pop()?.toLowerCase();
    return ext ? validMediaExtensions.has(ext) : false;
  }

  async function processFilesToLoad(paths: string[]) {
    if (paths.length > 0) {
      loadInputMedia(paths[0]);
    }
  }

  function handleDroppedFiles(paths: string[]) {
    if (isTranscribing) return;
    const mediaPaths = paths.filter(isAudioVideoFile).slice(-1);
    if (mediaPaths.length === 0) return;

    const hasExistingFile = !!inputPath;
    if (hasExistingFile) {
      pendingDroppedPaths = mediaPaths;
      showOverwriteConfirm = true;
    } else {
      processFilesToLoad(mediaPaths);
    }
  }

  function confirmOverwrite() {
    showOverwriteConfirm = false;
    if (pendingDroppedPaths.length > 0) {
      processFilesToLoad(pendingDroppedPaths);
      pendingDroppedPaths = [];
    }
  }

  async function selectOutputFile() {
    try {
      const selected = await guardedSave({
        filters: [{ name: "SRT Files", extensions: ["srt"] }],
        defaultPath: outputPath || undefined,
      });

      if (selected) {
        outputPath = selected;
        addLog(`Output file set manually: ${getFileName(outputPath)}`, "file");
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
        tiers: $state.snapshot(effectiveTiers), models: $state.snapshot(whisperModels),
        keys: $state.snapshot(apiKeys),
        settings: {
          input_path: inputPath, output_path: outputPath, language: selectedLanguage,
          translate_to_english: translateToEnglish, word_timestamps: true,
          max_segment_length: maxSegmentLength, quality: qualityMode,
          vad: vadEnabled && vadInstalled,
          vad_model_id: vadSelection.customPath ? null : vadSelection.modelId,
          vad_custom_path: vadSelection.customPath,
        },
        start: transcribeStart,
        isCancelled: () => transcriptionCancelled || run.isCancelled(),
        onTier: (index, count) => addLog(`Tier ${index + 1}: ${count}`, "info"),
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
        progress = 0; progressMessage = "";
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
    addLog(`Output file: ${getFileName(outputPath)}`, "info");
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

  const TRANSCRIBE_PANEL_IDS = [
    "options",
    "files",
    "progress",
    "logs",
  ] as const;

  type TranscribePanelId = (typeof TRANSCRIBE_PANEL_IDS)[number];
</script>

<div
  role="region"
  aria-label={t("nav.transcribe")}
  class="h-full flex flex-col bg-gray-900 relative overflow-hidden"
  ondragover={(e) => {
    if (!active) return;
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'copy';
    }
    isDraggingOver = true;
  }}
  ondrop={(e) => {
    if (!active) return;
    e.preventDefault();
    isDraggingOver = false;
  }}
  ondragleave={(e) => {
    if (!active) return;
    const rt = e.relatedTarget as HTMLElement | null;
    const ct = e.currentTarget as HTMLElement;
    if (rt && ct.contains(rt)) return;
    isDraggingOver = false;
  }}
>
  {#if isDraggingOver}
    <div
      class="absolute inset-0 z-50 bg-indigo-500/10 border-2 border-dashed border-indigo-400 rounded-2xl flex items-center justify-center pointer-events-none"
    >
      <div class="text-center">
        <svg
          class="w-16 h-16 mx-auto mb-3 text-indigo-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          ><path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
          /></svg
        >
        <p class="text-lg font-semibold text-indigo-300">{t("transcribe.dropFileHere")}</p>
        <p class="text-xs text-indigo-500 mt-1">{t("transcribe.dropFileHint")}</p>
      </div>
    </div>
  {/if}
  <!-- FFmpeg Warning (whisper-rs is always available natively) -->
  {#if backends && !backends.ffmpeg}
    <div
      class="mx-6 mt-4 mb-0 p-3 bg-white/[0.03] border border-white/10 rounded-xl flex items-center gap-3 shrink-0"
    >
      <svg
        class="w-5 h-5 text-cyan-400 flex-shrink-0"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"
        />
      </svg>
      <div class="flex-1">
        <p class="text-gray-200 text-sm font-medium">
          {t("transcribe.ffmpegMissing")}
        </p>
        <p class="text-gray-400 text-xs mt-1">
          {t("transcribe.ffmpegMissingDesc")}
        </p>
      </div>
      <button
        type="button"
        disabled={isDownloadingFFmpeg}
        onclick={async () => {
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
        }}
        class="flex-shrink-0 px-3 py-1.5 rounded-lg bg-cyan-500/10 border border-cyan-500/30 text-gray-200 text-xs font-semibold hover:bg-cyan-500/20 transition-colors flex items-center gap-1.5 disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {#if isDownloadingFFmpeg}
          <svg class="animate-spin w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
          {t("flashcards.downloading")}
        {:else}
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" /></svg>
          {t("flashcards.downloadAuto")}
        {/if}
      </button>
    </div>
  {/if}





  {#snippet panelContent(panelId: TranscribePanelId)}
    {#if panelId === "options"}
      <div
        class="glass-card p-5"
      >
        <h3 class="text-lg font-semibold mb-4 flex items-center gap-2 text-blue-400">
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
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
            />
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
            />
          </svg>
          {t("transcribe.options")}
        </h3>
        <div class="space-y-4">
          <div>
            <span class="flex items-center gap-1.5 text-sm text-gray-400 mb-1 font-medium">
              <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10" />
              </svg>
              <span>{t("transcribe.sourceLanguage")}</span>
            </span>
            <LanguageSelect
              noResultsText={t("common.noResults")}
              autoDetect
              value={selectedLanguage}
              onchange={(v) => (selectedLanguage = v)}
              placeholder={t("transcribe.sourceLanguage")}
            />
          </div>
          {#if uiMode.expertMode}
          <!-- Silero VAD (local whisper only; requires the model) -->
          <div class="rounded-lg bg-white/5 p-3">
            <ToggleRow label={t("transcribe.vad")} description={vadInstalled ? t("transcribe.vadHelp") : t("transcribe.vadNotInstalled")} checked={vadEnabled && vadInstalled} disabled={!vadInstalled} accent="sky" onchange={setVadEnabled} />
            {#if !vadInstalled}
              <div class="mt-2.5 pt-2 border-t border-white/5 flex items-center justify-between gap-2">
                <span class="text-xs text-amber-300/80">{t("transcribe.vadNotInstalled")}</span>
                <SettingsJumpButton
                  label={t("transcribe.downloadVad")}
                  section="whisper"
                  targetId="silero-vad"
                  disabled={!onGoToSettings}
                  onclick={() => onGoToSettings?.("whisper", "silero-vad")}
                  variant="amber"
                />
              </div>
            {/if}
          </div>
          <!-- Quality mode (beam search, local whisper only) -->
          <div class="rounded-lg bg-white/5 p-3">
            <ToggleRow label={t("transcribe.quality")} checked={qualityMode} accent="sky" onchange={setQualityMode} />
          </div>
          <div>
            <div class="flex items-center justify-between mb-2">
              <span class="text-sm text-gray-400">
                {t("transcribe.maxSegmentLength")}
              </span>
            </div>
            <div class="grid grid-cols-4 gap-2">
              <button
                onclick={() => setSegmentPreset("short")}
                class="p-2 rounded-lg text-center transition-all duration-200 border text-xs {activeSegmentPreset ===
                'short'
                  ? 'bg-cyan-500/20 border-cyan-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
              >
                <span class="block mb-1">
                  <svg
                    class="w-4 h-4 mx-auto"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    aria-hidden="true"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="1.8"
                      d="M7 10h8M7 14h5"
                    />
                  </svg>
                </span>
                <span class="font-semibold block">{t("transcribe.segmentShort")}</span>
              </button>
              <button
                onclick={() => setSegmentPreset("medium")}
                class="p-2 rounded-lg text-center transition-all duration-200 border text-xs {activeSegmentPreset ===
                'medium'
                  ? 'bg-cyan-500/20 border-cyan-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
              >
                <span class="block mb-1">
                  <svg
                    class="w-4 h-4 mx-auto"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    aria-hidden="true"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="1.8"
                      d="M6 9h12M6 13h9M6 17h7"
                    />
                  </svg>
                </span>
                <span class="font-semibold block">{t("transcribe.segmentMedium")}</span>
              </button>
              <button
                onclick={() => setSegmentPreset("standard")}
                class="p-2 rounded-lg text-center transition-all duration-200 border text-xs {activeSegmentPreset ===
                'standard'
                  ? 'bg-cyan-500/20 border-cyan-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
              >
                <span class="block mb-1">
                  <svg
                    class="w-4 h-4 mx-auto"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    aria-hidden="true"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="1.8"
                      d="M5 9h14M5 13h14M5 17h14"
                    />
                  </svg>
                </span>
                <span class="font-semibold block">{t("transcribe.segmentStandard")}</span>
              </button>
              <button
                onclick={() => setSegmentPreset("long")}
                class="p-2 rounded-lg text-center transition-all duration-200 border text-xs {activeSegmentPreset ===
                'long'
                  ? 'bg-cyan-500/20 border-cyan-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
              >
                <span class="block mb-1">
                  <svg
                    class="w-4 h-4 mx-auto"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    aria-hidden="true"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="1.8"
                      d="M4 8h16M4 12h16M4 16h16M4 20h16"
                    />
                  </svg>
                </span>
                <span class="font-semibold block">{t("transcribe.segmentLong")}</span>
              </button>
            </div>
            <div class="mt-2 flex items-center gap-2 text-xs">
              <span class="text-gray-500">{t("transcribe.segmentLengthLabel")}</span>
              <span
                class="text-white font-mono bg-white/10 px-2 py-0.5 rounded text-sm"
                >{maxSegmentLength}s</span
              >
            </div>
          </div>
          {/if}
        </div>
      </div>
    {:else if panelId === "files"}
      <div class="glass-card p-5 {requirements.classFor('files')}">
        <h3 class="text-lg font-semibold mb-4 flex items-center gap-2 panel-title-files-output">
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
              d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
            />
          </svg>
          {t("common.filesAndOutput")}
        </h3>
        <div class="grid grid-cols-1 gap-4 md:grid-cols-2 md:items-end">
          <PathPickerField
            label={t("transcribe.inputMediaFile")}
            labelIcon="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 01-2-2V7a2 2 0 012-2z"
            value={inputPath}
            placeholder={t("transcribe.noInputMediaSelected")}
            browseTitle={t("transcribe.selectFile")}
            onexpand={() => (expandedPathField = "input")}
            disabled={isTranscribing}
            onbrowse={selectInputFile}
            required={true}
          />

          <PathPickerField
            label={t("transcribe.outputSrtFile")}
            labelIcon="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
            value={outputPath}
            placeholder={t("transcribe.noOutputFileSelected")}
            browseTitle={t("transcribe.selectDestination")}
            onexpand={() => (expandedPathField = "output")}
            disabled={isTranscribing}
            onbrowse={selectOutputFile}
            required={true}
          />
          {#if inputPath}
            <div
              class="p-3 bg-cyan-500/10 border border-cyan-500/30 rounded-lg md:col-span-2"
            >
              <div class="flex items-center gap-3">
                <div
                  class="w-10 h-10 rounded-lg bg-cyan-500/20 flex items-center justify-center"
                >
                  <MediaIcon kind="video" class="w-5 h-5 text-cyan-300" />
                </div>
                <div>
                  <p class="font-medium text-white">
                    {getFileName(inputPath)}
                  </p>
                  <p class="text-sm text-gray-400">
                    {t("transcribe.readyToProcess")}
                  </p>
                </div>
              </div>
            </div>
          {/if}
        </div>
      </div>

    {:else if panelId === "progress"}
      <div class="space-y-3">
        {#if isTranscribing || progress > 0}
          <div
            class="glass-card p-4 shrink-0 {isTranscribing
              ? 'animate-pulse-glow'
              : ''}"
          >
            <div class="flex items-center gap-4">
              <div class="flex-1">
                <div class="progress-modern h-2">
                  <div
                    class="progress-modern-bar bg-gradient-to-r from-cyan-500 to-blue-500"
                    style="width: {progress}%"
                  ></div>
                </div>
              </div>
              <span class="text-lg font-bold text-cyan-400">{progress}%</span>
            </div>
            {#if progressMessage}
              <p class="text-gray-400 text-xs mt-2">{progressMessage}</p>
            {/if}
          </div>
        {/if}
        {#if result}
          <div
            class="glass-card p-4 shrink-0 border-l-4 {result.success
              ? 'border-green-500 bg-green-500/5'
              : 'border-red-500 bg-red-500/5'}"
          >
            <div class="flex items-center gap-3">
              {#if result.success}
                <svg
                  class="w-5 h-5 text-green-400"
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
              {:else}
                <svg
                  class="w-5 h-5 text-red-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              {/if}
              <p
                class="{result.success
                  ? 'text-green-400'
                  : 'text-red-400'} font-medium"
              >
                {result.message}
              </p>
              {#if result.output_path}
                <p class="text-xs text-gray-500 mt-1 font-mono truncate">
                  <MediaIcon kind="folder" /> {result.output_path}
                </p>
              {/if}
            </div>
          </div>
        {/if}
        {#if error}
          <div
            class="glass-card p-4 shrink-0 border border-red-500/30 bg-red-500/10"
          >
            <div class="flex items-center gap-3">
              <svg
                class="w-5 h-5 text-red-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <p class="text-red-300 flex-1 text-sm">{error}</p>
              <button
                onclick={() => (error = null)}
                class="text-red-400 hover:text-red-300">✕</button
              >
            </div>
          </div>
        {/if}
      </div>
    {:else if panelId === "logs"}
      <LogPanel
        title={t("transcribe.log")}
        clearLogText={t("transcribe.clearLog")}
        noLogText={t("translate.noLog")}
        logs={logBuffer.logs}
        onclear={clearLogs}
        minHeight="170px"
        maxHeightContent="100%"
      />
    {/if}
  {/snippet}


  <div class="flex flex-col flex-1 overflow-hidden p-6 min-h-0 {isTranscribing ? 'pointer-events-none opacity-60 select-none' : ''}">
    <WorkflowWorkspace>
      {#snippet files()}{@render panelContent("files")}{/snippet}
      <div class="grid grid-cols-1 items-start gap-6 xl:grid-cols-2">
        <div class="min-w-0">
          {@render panelContent("options")}
          {#if uiMode.expertMode}
            <div class="glass-card p-5 mt-4 {requirements.classFor('model')}">
              <h3 class="mb-4 flex items-center gap-2 text-lg font-semibold text-cyan-300">
                <svg aria-hidden="true" class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M9 3v3m6-3v3M9 18v3m6-3v3M3 9h3m-3 6h3m12-6h3m-3 6h3M6 6h12v12H6zM10 10h4v4h-4z" /></svg>
                {t("transcribe.whisperModel")}
              </h3>
              <WhisperModelSelector models={modelStore.whisperModels} value={localModelOverride ?? transcribeTiers.flatMap(tier => tier.entries).find(entry => entry.provider === "local" || entry.provider === "local_whisper")?.model ?? ""} disabled={modelStore.busy}
                oncontextmenu={(event, model) => { event.preventDefault(); event.stopPropagation(); const rect = event.currentTarget instanceof HTMLElement ? event.currentTarget.getBoundingClientRect() : null; modelMenu = { id: model.id, x: event.clientX || rect?.left || 8, y: event.clientY || rect?.bottom || 8 }; }}
                onselect={async (model) => {
                  if (!model.downloaded && !await modelStore.downloadModel(model.id)) return;
                  if (modelStore.whisperModels.find(m => m.id === model.id)?.downloaded) {
                    selectedModel = model.id;
                    localModelOverride = model.id;
                    await refreshModels();
                  }
                }} />
              {#if modelStore.isDownloading}
                <p class="mt-3 text-xs text-cyan-300" role="status">{t("settings.modelDownloading", { model: modelStore.downloadingModelId ?? "" })} · {modelStore.progress}%</p>
                <button class="mt-2 text-xs text-red-300 disabled:opacity-50" disabled={modelStore.isCancellingDownload} onclick={() => void modelStore.cancelModelDownload()}>{t("common.cancel")}</button>
              {/if}
              {#if localModelOverride}
                <div class="mt-3 flex flex-wrap items-center gap-1.5 text-xs text-red-400">
                  <button type="button" disabled={modelStore.busy} class="hover:text-red-200 hover:underline disabled:opacity-50" onclick={() => localModelOverride = null}>{t("transcribe.useTierModels")}</button>
                  <span>{t("transcribe.configuredIn")}</span>
                  <SettingsJumpButton
                    label={t("transcribe.tiersLink")}
                    section="whisper"
                    targetId="transcribe-tiers"
                    disabled={!onGoToSettings}
                    onclick={() => onGoToSettings?.("whisper", "transcribe-tiers")}
                    variant="subtle"
                  />
                </div>
              {/if}
              {#if !vadInstalled}
                <div class="mt-3">
                  <SettingsJumpButton
                    label={t("transcribe.downloadVad")}
                    section="whisper"
                    targetId="silero-vad"
                    disabled={!onGoToSettings}
                    onclick={() => onGoToSettings?.("whisper", "silero-vad")}
                    variant="amber"
                  />
                </div>
              {/if}
            </div>
          {/if}
        </div>
        <div class="min-w-0"><TranscriptionSegmentsPanel segments={transcribedSegments} /></div>
      </div>
    </WorkflowWorkspace>
  </div>

  <!-- Fixed Bottom Band with Action Buttons -->
  <FooterActions>
    {#snippet background()}
      {#if isTranscribing}
        <div
          class="absolute inset-y-0 left-0 bg-gradient-to-r from-cyan-500/15 to-blue-500/20 transition-all duration-300 ease-out z-0 pointer-events-none"
          style="width: {progress}%"
        ></div>
        <div class="absolute inset-0 bg-shimmer-stripes opacity-15 z-0 pointer-events-none"></div>
      {/if}
    {/snippet}
    {#snippet left()}
    <!-- Left side: Progress/Result status info -->
    <div class="flex items-center gap-4 select-none z-10 min-w-0 flex-1">
      {#if isTranscribing}
        <!-- Loading spinner and status message -->
        <div class="flex items-center gap-3">
          <div class="w-5 h-5 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin"></div>
          <div class="flex flex-col">
            <span class="text-[10px] text-cyan-400 font-bold uppercase tracking-wider">{t("transcribe.transcribing")}</span>
            <span class="text-xs text-white font-medium truncate max-w-lg">
              {progressMessage || t("transcribe.running")} ({progress}%)
            </span>
          </div>
        </div>
      {:else if result}
        <!-- Success/Result state -->
        <div class="flex items-center gap-3">
          {#if result.success}
            <div class="flex items-center justify-center w-8 h-8 rounded-full bg-emerald-500/10 border border-emerald-500/30 text-emerald-400">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
              </svg>
            </div>
            <div class="flex flex-col">
              <span class="text-[10px] text-emerald-400 font-bold uppercase tracking-wider">{t("transcribe.finished")}</span>
              {#if result.output_path}
                <CopyPathChip
                  path={result.output_path}
                  copiedMessage={t("common.pathCopied")}
                  tooltip={t("common.clickToCopyPath")}
                />
              {:else}
                <span class="text-xs text-white font-medium">{result.message}</span>
              {/if}
            </div>
          {:else}
            <!-- Failure state -->
            <div class="flex items-center justify-center w-8 h-8 rounded-full bg-red-500/10 border border-red-500/30 text-red-400">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </div>
            <div class="flex flex-col">
              <span class="text-[10px] text-red-400 font-bold uppercase tracking-wider">{t("transcribe.error")}</span>
              <span class="text-xs text-red-300 font-medium truncate max-w-lg">{result.message}</span>
            </div>
          {/if}
        </div>
      {:else if error}
        <WorkflowErrorStatus title={t("transcribe.error")} message={error} warning />
      {/if}
    </div>
    {/snippet}
    {#snippet right()}
    <!-- Right side: Action Buttons -->
    <div class="flex items-center gap-4 z-10 select-none shrink-0">
      {#if isTranscribing}
        <button
          disabled={localWhisperOperation.cancelling || localWhisperOperation.cancelRequested}
          onclick={cancelTranscription}
          class="px-5 py-2.5 bg-red-600/80 hover:bg-red-500/80 border border-red-500/30 text-red-100 rounded-xl font-bold text-sm transition-all shadow-lg shadow-red-950/20 flex items-center gap-2 hover:scale-[1.02] active:scale-[0.98] cursor-pointer"
        >
          <svg
            class="w-4 h-4 text-red-100"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
          {t("transcribe.cancel")}
        </button>
      {:else if result || error}
        <div class="relative group">
          <button
            onclick={resetTranscription}
            class="px-5 py-2.5 bg-amber-500/10 hover:bg-amber-500/20 text-amber-300 rounded-xl font-bold text-sm transition-all border border-amber-500/30 flex items-center gap-2 hover:scale-[1.02] active:scale-[0.98] cursor-pointer"
          >
            <svg
              class="w-4 h-4 text-amber-300"
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
            {t("transcribe.newTranscription")}
          </button>
          <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-amber-500/30 bg-gray-950/95 p-3 text-center text-xs text-amber-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-nowrap">
            {t("transcribe.newTranscriptionDesc")}
          </div>
        </div>
      {:else}
        <div class="relative group">
          <button
            onclick={startTranscription}
            disabled={!canStart}
            class="px-5 py-2.5 bg-emerald-600/80 hover:bg-emerald-500/80 border border-emerald-500/30 disabled:bg-emerald-600/40 text-emerald-100 rounded-xl font-bold text-sm transition-all shadow-lg shadow-emerald-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-55 cursor-pointer"
          >
            <svg
              class="w-4 h-4 text-emerald-100"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
              />
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            {t("transcribe.startTranscription")}
          </button>
          <div class="pointer-events-none absolute bottom-full right-0 z-50 mb-3 rounded-xl border border-teal-500/30 bg-gray-950/95 p-3 text-center text-xs text-teal-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-pre-line w-80">
            {transcribeBlockedReason || t("transcribe.startTranscription")}
          </div>
        </div>
      {/if}
    </div>
    {/snippet}
  </FooterActions>

  <PathPreviewModal
    isOpen={!!expandedPathField}
    title={expandedPathField === "input"
      ? t("transcribe.inputFile")
      : t("transcribe.outputFile")}
    value={expandedPathField === "input" ? inputPath : outputPath}
    onclose={() => (expandedPathField = null)}
    editable={true}
    secondaryText={t("transcribe.editPath")}
    desc={expandedPathField === "input"
      ? t("transcribe.inputPathDesc")
      : t("transcribe.outputPathDesc")}
    onsave={expandedPathField === "input" ? saveInputPath : saveOutputPath}
  />

  <ConfirmDialog
    show={showOverwriteConfirm}
    title={t("common.replaceFilesTitle")}
    message={t("common.replaceFilesMessage")}
    confirmText={t("sync.confirm")}
    cancelText={t("common.cancel")}
    variant="warning"
    on:cancel={() => {
      showOverwriteConfirm = false;
      pendingDroppedPaths = [];
    }}
    on:confirm={confirmOverwrite}
  />
</div>

<style>
  @keyframes progress-stripes {
    0% { background-position: 0 0; }
    100% { background-position: 40px 0; }
  }
  .bg-shimmer-stripes {
    background-image: linear-gradient(
      45deg,
      rgba(6, 182, 212, 0.15) 25%,
      transparent 25%,
      transparent 50%,
      rgba(6, 182, 212, 0.15) 50%,
      rgba(6, 182, 212, 0.15) 75%,
      transparent 75%,
      transparent
    );
    background-size: 40px 40px;
    animation: progress-stripes 1.2s linear infinite;
  }
</style>

{#if modelMenu && contextModel}
  <WhisperModelActionsMenu model={contextModel} busy={modelStore.busy} x={modelMenu.x} y={modelMenu.y} onclose={() => modelMenu = null}
    oninstall={async () => {
      const model = contextModel;
      modelMenu = null;
      if (model && !model.downloaded && !modelStore.busy) await modelStore.downloadModel(model.id);
    }}
    onuninstall={async () => {
      const model = contextModel;
      modelMenu = null;
      if (!model?.downloaded || modelStore.busy) return;
      await modelStore.uninstallModel(model.id);
      if (!modelStore.whisperModels.find(item => item.id === model.id)?.downloaded && localModelOverride === model.id) localModelOverride = null;
      await refreshModels();
    }} />
{/if}
