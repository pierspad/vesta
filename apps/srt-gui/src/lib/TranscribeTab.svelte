<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import LogPanel, { type LogEntry } from "./LogPanel.svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { guardedOpen, guardedSave } from "./utils/dialogGuard";
  import { onDestroy, onMount } from "svelte";
  import { locale, currentLanguage } from "./i18n";
  import { getFileName } from "./models";
  import {
    transcribeProviders,
    transcribeProviderOrder,
    loadTranscribeCloud,
    saveTranscribeCloud,
    type TranscribeCloudSettings,
  } from "./transcribeProviders";
  import {
    loadTranscribeTiers,
    transcribeTiersHaveUsableEntries,
    TRANSCRIBE_TIERS_UPDATED_EVENT,
    type TranscribeTier,
    type TranscribeTierEntry,
  } from "./transcribeTiers";
  import { loadAndValidateApiKeys, type ApiKeyConfig } from "./apiKeys";
  import { loadVadSelection, type VadSelection } from "./vadSelection";
  import { getLanguageSearchTerms, languages as allLanguages } from "./languages";
  import PathPickerField from "./PathPickerField.svelte";
  import PathPreviewModal from "./PathPreviewModal.svelte";
  import SearchableSelect from "./SearchableSelect.svelte";
  import { snackbar } from "./snackbarStore.svelte";
  import { uiMode } from "./uiModeStore.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import { aiStore } from "./aiStore.svelte";
  import FooterActions from "./components/FooterActions.svelte";
  import * as vestaConfig from "./vestaConfig";

  let { onGoToSettings, active = false } = $props<{
    onGoToSettings?: (section?: "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts", highlightItemId?: string) => void;
    active?: boolean;
  }>();

  let t = $derived($locale);

  let inputPath = $state("");
  let outputPath = $state("");
  let selectedModel = $state("base");
  const LAST_TRANSCRIBE_LANGUAGE_KEY = "vesta-transcribe-source-language";
  const DEFAULT_TRANSCRIBE_LANGUAGE_KEY = "vesta-default-transcribe-language";
  const initialTranscribeLanguage =
    vestaConfig.getItem(LAST_TRANSCRIBE_LANGUAGE_KEY) ||
    vestaConfig.getItem(DEFAULT_TRANSCRIBE_LANGUAGE_KEY) ||
    "auto";
  let selectedLanguage = $state(initialTranscribeLanguage);
  let previousLanguageForOutput = initialTranscribeLanguage;
  let translateToEnglish = $state(false);
  let wordTimestamps = $state(true);
  let maxSegmentLength = $state(30);

  // ─── Local-whisper add-ons: quality (beam search), VAD, GPU ────────────────
  const TRANSCRIBE_QUALITY_KEY = "vesta-transcribe-quality";
  const TRANSCRIBE_VAD_KEY = "vesta-transcribe-vad";
  const TRANSCRIBE_GPU_KEY = "vesta-transcribe-gpu";
  let qualityMode = $state(vestaConfig.getItem(TRANSCRIBE_QUALITY_KEY) === "true");
  // Defaults to ON: once a VAD model is installed it's a strict improvement
  // for local transcription (skips silence, fewer hallucinations). The toggle
  // itself stays disabled until `vadInstalled` is true, so this default is
  // inert for users without the model.
  const storedVad = vestaConfig.getItem(TRANSCRIBE_VAD_KEY);
  let vadEnabled = $state(storedVad === null ? true : storedVad === "true");
  // GPU defaults to ON in GPU-capable builds: whisper.cpp falls back to CPU
  // by itself when no usable device exists.
  let useGpu = $state(vestaConfig.getItem(TRANSCRIBE_GPU_KEY) !== "false");
  let vadInstalled = $state(false);
  let gpuSupported = $state(false);
  let vadModels = $state<{ id: string; size: string; downloaded: boolean }[]>([]);
  let vadSelection = $state<VadSelection>(loadVadSelection());

  function toggleQualityMode() {
    qualityMode = !qualityMode;
    vestaConfig.setItem(TRANSCRIBE_QUALITY_KEY, String(qualityMode));
  }
  function toggleVad() {
    vadEnabled = !vadEnabled;
    vestaConfig.setItem(TRANSCRIBE_VAD_KEY, String(vadEnabled));
  }
  function toggleGpu() {
    useGpu = !useGpu;
    vestaConfig.setItem(TRANSCRIBE_GPU_KEY, String(useGpu));
  }

  /** Re-derive `vadInstalled` for whichever variant (built-in or custom) is
   * currently selected in Settings → Whisper. */
  async function refreshVadReady() {
    vadSelection = loadVadSelection();
    if (vadSelection.customPath) {
      try {
        vadInstalled = await invoke<boolean>("transcribe_path_exists", {
          path: vadSelection.customPath,
        });
      } catch {
        vadInstalled = false;
      }
    } else {
      vadInstalled = vadModels.some(
        (m) => m.id === vadSelection.modelId && m.downloaded,
      );
    }
  }

  async function refreshAddons() {
    try {
      const s = await invoke<{
        vad_models: { id: string; size: string; downloaded: boolean }[];
        gpu_supported: boolean;
      }>("transcribe_addons_status");
      vadModels = s.vad_models;
      gpuSupported = s.gpu_supported;
      await refreshVadReady();
    } catch (e) {
      console.error("Could not read transcription add-ons status:", e);
    }
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

  let isTranscribing = $state(false);
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

  function showSnackbar(message: string, variant: "success" | "info" | "warning" | "error" = "info", duration = 3500) {
    snackbar.show(message, variant, duration);
  }

  let logIdCounter = 0;
  let logs = $state<LogEntry[]>([]);

  let expandedPathField = $state<string | null>(null);

  let isDraggingOver = $state(false);
  let showOverwriteConfirm = $state(false);
  let pendingDroppedPaths = $state<string[]>([]);

  let backends = $state<{
    ffmpeg: boolean;
    whisper_cpp: boolean;
    python_whisper: boolean;
    any_whisper: boolean;
    whisper_binary: string | null;
  } | null>(null);
  let isDownloadingFFmpeg = $state(false);

  let whisperModels = $state<
    {
      id: string;
      name: string;
      size: string;
      speed: string;
      downloaded: boolean;
    }[]
  >([]);

  let isModelDownloaded = $derived(
    whisperModels.find((m) => m.id === selectedModel)?.downloaded ?? false
  );
  let hasAnyWhisperModel = $derived(whisperModels.some((m) => m.downloaded));

  // ─── Transcription engine (Local Whisper vs cloud providers) ────────────────
  let transcribeTiers = $state<TranscribeTier[]>([]);
  let apiKeys = $state<ApiKeyConfig[]>([]);
  let transcribedSegments = $state<{ start_ms: number; end_ms: number; text: string }[]>([]);
  let scrollContainer = $state<HTMLDivElement | null>(null);

  function formatTime(ms: number): string {
    const totalSeconds = Math.floor(ms / 1000);
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;
    const millis = Math.floor(ms % 1000);
    if (hours > 0) {
      return `${hours}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}.${millis.toString().padStart(3, "0")}`;
    }
    return `${minutes}:${seconds.toString().padStart(2, "0")}.${millis.toString().padStart(3, "0")}`;
  }

  $effect(() => {
    if (transcribedSegments.length > 0 && scrollContainer) {
      scrollContainer.scrollTop = scrollContainer.scrollHeight;
    }
  });

  function refreshTranscribeTiers() {
    transcribeTiers = loadTranscribeTiers();
  }
  function refreshApiKeys() {
    apiKeys = loadAndValidateApiKeys();
  }

  function isTranscribeEntryReady(e: TranscribeTierEntry): boolean {
    if (e.provider === "local" || e.provider === "local_whisper") {
      return whisperModels.find((m) => m.id === e.model)?.downloaded ?? false;
    }
    const key = apiKeys.find((k) => k.id === e.apiKeyId);
    if (e.provider === "custom") {
      return !!key?.apiUrl?.trim();
    }
    return !!key?.apiKey?.trim();
  }

  let usableEntries = $derived(
    transcribeTiers.flatMap((t) => t.entries).filter(isTranscribeEntryReady)
  );

  let canStart = $derived(
    !!inputPath && !!outputPath && usableEntries.length > 0
  );

  let transcribeBlockedReason = $derived.by(() => {
    if (!inputPath || !outputPath) {
      return t("transcribe.blocked.selectFiles");
    }
    if (transcribeTiers.length === 0 || transcribeTiers.flatMap((t) => t.entries).length === 0) {
      return t("transcribe.blocked.noTiers");
    }
    if (usableEntries.length === 0) {
      const hasLocalConfigured = transcribeTiers.flatMap((t) => t.entries).some(e => e.provider === "local" || e.provider === "local_whisper");
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

  const knownLangCodes = new Set(allLanguages.map((l) => l.code.toLowerCase()));

  function effectiveLanguageCodeForOutput(langCode: string): string {
    if (langCode !== "auto") return langCode;
    return result?.detected_language?.toLowerCase() || "auto";
  }

  function rewriteOutputPathLanguage(path: string, langCode: string): string {
    const match = path.match(/^(.*\/)?([^/]+)$/);
    if (!match) return path;

    const dir = match[1] || "";
    const fileName = match[2];

    if (!/\.srt$/i.test(fileName)) {
      return path;
    }

    const stem = fileName.replace(/\.srt$/i, "");
    const tokenMatch = stem.match(/^(.*)([\-._])([^\-._]+)$/);
    if (tokenMatch) {
      const [, prefix, sep, token] = tokenMatch;
      const tokenLower = token.toLowerCase();
      const looksLikeLang = knownLangCodes.has(tokenLower) || tokenLower === "auto" || /^[a-z]{2,3}$/i.test(tokenLower);
      if (looksLikeLang) {
        return `${dir}${prefix}${sep}${langCode}.srt`;
      }
    }

    return `${dir}${stem}.${langCode}.srt`;
  }

  function generateOutputPathFromInput(input: string, langCode: string): string {
    const basePath = input.replace(/\.[^/.]+$/, "");
    return `${basePath}.${langCode}.srt`;
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

    const refreshBackends = () => {
      invoke<typeof backends>("transcribe_check_backends")
        .then((res) => { backends = res; })
        .catch((e) => console.error("Could not check backends:", e));
    };
    refreshBackends();
    window.addEventListener("vesta-ffmpeg-updated", refreshBackends);

    refreshModels().catch((e) => console.error("Could not list models:", e));
    void refreshAddons();
    window.addEventListener("vesta-vad-updated", refreshAddons);

    let activeListener = true;
    let unlisten: (() => void) | null = null;
    let unlistenDD: (() => void) | null = null;
    let unlistenSegment: (() => void) | null = null;

    getCurrentWebview().onDragDropEvent((event) => {
      if (!active) return;
      if (event.payload.type === "over") {
        isDraggingOver = true;
      } else if (event.payload.type === "drop") {
        isDraggingOver = false;
        if (event.payload.paths && event.payload.paths.length > 0) {
          handleDroppedFiles(event.payload.paths);
        }
      } else if (event.payload.type === "leave") {
        isDraggingOver = false;
      }
    }).then((fn) => {
      if (!activeListener) fn();
      else unlistenDD = fn;
    }).catch(console.error);

    listen<{
      stage: string;
      message: string;
      percentage: number;
    }>("transcribe-progress", (event) => {
      const p = event.payload;
      if (p.stage === "download") return; // Ignored here, handled in SettingsTab
      
      progress = Math.round(p.percentage);
      progressMessage = p.message;
      progressStage = p.stage;
      
      if (p.stage !== "done") {
        addLog(p.message, "progress");
      }
    }).then((fn) => {
      if (!activeListener) fn();
      else unlisten = fn;
    }).catch(console.error);

    listen<{
      start_ms: number;
      end_ms: number;
      text: string;
    }>("transcribe-segment", (event) => {
      const p = event.payload;
      transcribedSegments = [...transcribedSegments, {
        start_ms: p.start_ms,
        end_ms: p.end_ms,
        text: p.text,
      }];
    }).then((fn) => {
      if (!activeListener) fn();
      else unlistenSegment = fn;
    }).catch(console.error);

    return () => {
      activeListener = false;
      window.removeEventListener(TRANSCRIBE_TIERS_UPDATED_EVENT, refreshTranscribeTiers);
      window.removeEventListener("apikeys-updated", refreshApiKeys);
      window.removeEventListener("whisper-model-updated", handleWhisperModelUpdated);
      window.removeEventListener("vesta-language-defaults-updated", handleLanguageDefaultsUpdated);
      window.removeEventListener("vesta:transcribe-cloud-updated", handleTranscribeCloudUpdated);
      window.removeEventListener("vesta-ffmpeg-updated", refreshBackends);
      window.removeEventListener("vesta-vad-updated", refreshAddons);
      if (unlisten) unlisten();
      if (unlistenDD) unlistenDD();
      if (unlistenSegment) unlistenSegment();
    };
  });

  function handleLanguageDefaultsUpdated() {
    if (!vestaConfig.getItem(LAST_TRANSCRIBE_LANGUAGE_KEY)) {
      selectedLanguage = vestaConfig.getItem(DEFAULT_TRANSCRIBE_LANGUAGE_KEY) || "auto";
      previousLanguageForOutput = selectedLanguage;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
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

  async function refreshModels() {
    try {
      const models = await invoke<typeof whisperModels>(
        "transcribe_list_models",
      );
      whisperModels = models;
    } catch (e) {
      console.error("Could not list models:", e);
    }
  }

  function addLog(message: string, type: LogEntry["type"] = "info") {
    const timestamp = new Date().toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
    logs = [...logs, { id: ++logIdCounter, timestamp, message, type }];
  }

  function clearLogs() {
    logs = [];
    logIdCounter = 0;
  }


  async function selectInputFile() {
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
        inputPath = selected as string;
        if (!outputPath) {
          const outputLang = effectiveLanguageCodeForOutput(selectedLanguage);
          outputPath = generateOutputPathFromInput(inputPath, outputLang);
        }
        addLog(
          `${t("transcribe.fileSelected")}: ${getFileName(inputPath)}`,
          "file",
        );
        addLog(`Output file: ${getFileName(outputPath)}`, "info");
      }
    } catch (e) {
      error = `${t("transcribe.errorSelectingFile")}: ${e}`;
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
      const newMedia = paths[0];
      inputPath = newMedia;
      const outputLang = effectiveLanguageCodeForOutput(selectedLanguage);
      outputPath = generateOutputPathFromInput(inputPath, outputLang);
      addLog(
        `${t("transcribe.fileSelected")}: ${getFileName(inputPath)}`,
        "file",
      );
      addLog(`Output file: ${getFileName(outputPath)}`, "info");
    }
  }

  function handleDroppedFiles(paths: string[]) {
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
      error = `${t("transcribe.errorSelectingFile")}: ${e}`;
    }
  }

  async function saveInputPath(newPath: string): Promise<boolean> {
    let cleaned = newPath.trim().replace(/\/+$/, "");
    if (!cleaned) {
      showSnackbar("Path cannot be empty");
      return false;
    }
    try {
      const exists = await invoke<boolean>("transcribe_check_file_exists", {
        path: cleaned,
      });
      if (!exists) {
        showSnackbar(`File not found: ${cleaned}`);
        return false;
      }
      inputPath = cleaned;
      if (!outputPath) {
        const outputLang = effectiveLanguageCodeForOutput(selectedLanguage);
        outputPath = generateOutputPathFromInput(inputPath, outputLang);
      }
      addLog(
        `${t("transcribe.fileSelected")}: ${getFileName(inputPath)}`,
        "file",
      );
      addLog(`Output file: ${getFileName(outputPath)}`, "info");
      return true;
    } catch (e) {
      showSnackbar(`Error: ${e}`);
      return false;
    }
  }

  async function saveOutputPath(newPath: string): Promise<boolean> {
    let cleaned = newPath.trim().replace(/\/+$/, "");
    if (!cleaned) {
      showSnackbar("Path cannot be empty");
      return false;
    }
    const parentDir = cleaned.substring(0, cleaned.lastIndexOf("/"));
    if (parentDir) {
      try {
        const exists = await invoke<boolean>("transcribe_check_file_exists", {
          path: parentDir,
        });
        if (!exists) {
          showSnackbar(`Directory not found: ${parentDir}`);
          return false;
        }
      } catch (e) {
        showSnackbar(`Error: ${e}`);
        return false;
      }
    }
    outputPath = cleaned;
    return true;
  }

  async function startTranscription() {
    if (!inputPath || !outputPath) {
      error = t("transcribe.selectFilesFirst");
      showSnackbar(t("transcribe.selectFilesFirst"));
      return;
    }

    if (!canStart) {
      error = transcribeBlockedReason;
      showSnackbar(transcribeBlockedReason);
      return;
    }

    error = null;
    result = null;
    progress = 0;
    transcribedSegments = [];
    isTranscribing = true;

    const startTime = Date.now();
    let success = false;
    let lastErrorMsg = "";

    for (let tIdx = 0; tIdx < transcribeTiers.length; tIdx++) {
      const tier = transcribeTiers[tIdx];
      const readyEntries = tier.entries.filter(isTranscribeEntryReady);
      if (readyEntries.length === 0) continue;

      addLog(`🪜 Avvio Tier ${tIdx + 1} (${readyEntries.length} endpoint pronti)...`, "info");

      for (const entry of readyEntries) {
        const isCloudEntry = entry.provider !== "local" && entry.provider !== "local_whisper";
        const engineLabel = isCloudEntry
          ? `${transcribeProviders[entry.provider]?.name || entry.provider} · ${entry.model || "auto"}`
          : entry.model;

        addLog(`🎙️ Provando endpoint: ${engineLabel}...`, "info");
        addLog(`Source language: ${selectedLanguageLabel(selectedLanguage)}`, "info");
        addLog(`Word timestamps: ${wordTimestamps ? "enabled" : "disabled"}; max segment: ${maxSegmentLength}s`, "info");
        addLog(`Input: ${getFileName(inputPath)} → Output: ${getFileName(outputPath)}`, "file");

        try {
          const key = isCloudEntry ? apiKeys.find((k) => k.id === entry.apiKeyId) : null;
          const apiKeyVal = key?.apiKey?.trim() || null;
          const apiUrl = key?.apiUrl?.trim() || transcribeProviders[entry.provider]?.defaultUrl || null;

          const res = await invoke<{
            success: boolean;
            message: string;
            output_path?: string;
            subtitle_count?: number;
            detected_language?: string;
          }>("transcribe_start", {
            config: {
              input_path: inputPath,
              output_path: outputPath,
              model: entry.model,
              language: selectedLanguage,
              translate_to_english: translateToEnglish,
              word_timestamps: wordTimestamps,
              max_segment_length: maxSegmentLength,
              provider: entry.provider,
              api_key: apiKeyVal,
              api_url: apiUrl,
              quality: !isCloudEntry && qualityMode,
              vad: !isCloudEntry && vadEnabled && vadInstalled,
              vad_model_id: vadSelection.customPath ? null : vadSelection.modelId,
              vad_custom_path: vadSelection.customPath,
              use_gpu: !isCloudEntry && useGpu && gpuSupported,
            },
          });

          result = res;
          if (res.output_path) {
            outputPath = res.output_path;
          }
          if (res.detected_language) {
            addLog(`Detected language: ${res.detected_language}`, "success");
          }
          if (res.output_path) {
            addLog(`Saved: ${getFileName(res.output_path)}`, "success");
          }
          addLog(res.message, "success");
          await refreshModels();
          success = true;
          break; // Break inner loop on success
        } catch (e: any) {
          lastErrorMsg = e ? e.toString() : "Unknown error";
          addLog(`⚠️ Errore su ${engineLabel}: ${lastErrorMsg}`, "warning");
        }
      }

      if (success) {
        break; // Break outer loop on success
      }
    }

    if (!success) {
      error = `Tutti i tier di trascrizione sono falliti. Ultimo errore: ${lastErrorMsg}`;
      addLog(`❌ Errore finale: ${error}`, "error");
    }

    isTranscribing = false;
    const elapsedSeconds = Math.floor((Date.now() - startTime) / 1000);
    const hrs = String(Math.floor(elapsedSeconds / 3600)).padStart(2, "0");
    const mins = String(
      Math.floor((elapsedSeconds % 3600) / 60),
    ).padStart(2, "0");
    const secs = String(elapsedSeconds % 60).padStart(2, "0");
    addLog(`⏱ ${hrs}:${mins}:${secs}`, "info");
  }

  async function cancelTranscription() {
    try {
      await invoke("transcribe_cancel");
      isTranscribing = false;
      progress = 0;
      progressMessage = "";
      addLog(t("transcribe.cancelled"), "warning");
    } catch (e) {
      error = `Error: ${e}`;
    }
  }

  function resetTranscription() {
    inputPath = "";
    outputPath = "";
    isTranscribing = false;
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
  aria-label="Transcribe content"
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
      class="mx-6 mt-4 mb-0 p-3 bg-amber-500/10 border border-amber-500/30 rounded-xl flex items-center gap-3 shrink-0"
    >
      <svg
        class="w-5 h-5 text-amber-400 flex-shrink-0"
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
        <p class="text-amber-300 text-sm font-medium">
          {t("transcribe.ffmpegMissing")}
        </p>
        <p class="text-amber-200/60 text-xs mt-1">
          {t("transcribe.ffmpegMissingDesc")}
        </p>
      </div>
      <button
        type="button"
        disabled={isDownloadingFFmpeg}
        onclick={async () => {
          isDownloadingFFmpeg = true;
          try {
            await invoke("flashcard_download_ffmpeg");
            backends = await invoke<typeof backends>("transcribe_check_backends");
            window.dispatchEvent(new CustomEvent("vesta-ffmpeg-updated"));
          } catch (e) {
            error = `${t("flashcards.ffmpegDownloadFailed")}: ${e}`;
          } finally {
            isDownloadingFFmpeg = false;
          }
        }}
        class="flex-shrink-0 px-3 py-1.5 rounded-lg bg-amber-500/20 border border-amber-500/40 text-amber-300 text-xs font-semibold hover:bg-amber-500/30 transition-colors flex items-center gap-1.5 disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {#if isDownloadingFFmpeg}
          <svg class="animate-spin w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
          {t("flashcards.downloading") || "Downloading..."}
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
        class="glass-card p-5 {!inputPath
          ? 'opacity-50 pointer-events-none'
          : ''}"
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
            <span class="block text-sm text-gray-400 mb-1"
              >{t("transcribe.sourceLanguage")}</span
            >
            <SearchableSelect
              noResultsText={t("common.noResults")}
              options={transcriptionLanguages.map((lang) => ({
                value: lang.code,
                label:
                  lang.nameEn === lang.name
                    ? lang.name
                    : `${lang.nameEn} — ${lang.name}`,
                searchTerms: getLanguageSearchTerms(lang.code),
                icon: lang.flag,
              }))}
              value={selectedLanguage}
              onchange={(v) => (selectedLanguage = v)}
              placeholder={t("transcribe.sourceLanguage")}
            />
          </div>
          {#if uiMode.expertMode}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="flex items-center justify-between p-3 bg-white/5 rounded-lg"
          >
            <div>
              <span class="text-gray-200 text-sm"
                >{t("transcribe.wordTimestamps")}</span
              >
            </div>
            <button
              onclick={() => (wordTimestamps = !wordTimestamps)}
              class="w-12 h-6 rounded-full transition-all duration-200 relative {wordTimestamps
                ? 'bg-cyan-500'
                : 'bg-gray-600'}"
              aria-label="Toggle word timestamps"
            >
              <div
                class="absolute w-5 h-5 bg-white rounded-full top-0.5 transition-all duration-200 {wordTimestamps
                  ? 'left-6'
                  : 'left-0.5'}"
              ></div>
            </button>
          </div>
          <!-- Silero VAD (local whisper only; requires the model) -->
          <div class="flex items-center justify-between p-3 bg-white/5 rounded-lg {vadInstalled ? '' : 'opacity-60'}">
            <div class="min-w-0 pr-3">
              <span class="text-gray-200 text-sm">{t("transcribe.vad")}</span>
              {#if !vadInstalled}
                <p class="text-[11px] text-gray-500 mt-0.5">{t("transcribe.vadNotInstalled")}</p>
              {/if}
            </div>
            <button
              onclick={toggleVad}
              disabled={!vadInstalled}
              class="shrink-0 w-12 h-6 rounded-full transition-all duration-200 relative {vadEnabled && vadInstalled
                ? 'bg-cyan-500'
                : 'bg-gray-600'} {vadInstalled ? '' : 'cursor-not-allowed'}"
              aria-label="Toggle voice activity detection"
            >
              <div
                class="absolute w-5 h-5 bg-white rounded-full top-0.5 transition-all duration-200 {vadEnabled && vadInstalled
                  ? 'left-6'
                  : 'left-0.5'}"
              ></div>
            </button>
          </div>
          <!-- Quality mode (beam search, local whisper only) -->
          <div class="flex items-center justify-between p-3 bg-white/5 rounded-lg">
            <div class="min-w-0 pr-3">
              <span class="text-gray-200 text-sm">{t("transcribe.quality")}</span>
            </div>
            <button
              onclick={toggleQualityMode}
              class="shrink-0 w-12 h-6 rounded-full transition-all duration-200 relative {qualityMode
                ? 'bg-cyan-500'
                : 'bg-gray-600'}"
              aria-label="Toggle quality mode"
            >
              <div
                class="absolute w-5 h-5 bg-white rounded-full top-0.5 transition-all duration-200 {qualityMode
                  ? 'left-6'
                  : 'left-0.5'}"
              ></div>
            </button>
          </div>
          <!-- GPU offload (only shown in GPU-capable builds) -->
          {#if gpuSupported}
          <div class="flex items-center justify-between p-3 bg-white/5 rounded-lg">
            <div class="min-w-0 pr-3">
              <span class="text-gray-200 text-sm">{t("transcribe.useGpu")}</span>
              <p class="text-[11px] text-gray-500 mt-0.5">{t("transcribe.useGpuHint")}</p>
            </div>
            <button
              onclick={toggleGpu}
              class="shrink-0 w-12 h-6 rounded-full transition-all duration-200 relative {useGpu
                ? 'bg-cyan-500'
                : 'bg-gray-600'}"
              aria-label="Toggle GPU"
            >
              <div
                class="absolute w-5 h-5 bg-white rounded-full top-0.5 transition-all duration-200 {useGpu
                  ? 'left-6'
                  : 'left-0.5'}"
              ></div>
            </button>
          </div>
          {/if}
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
      <div class="glass-card p-5">
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
        <div class="space-y-3">
          <PathPickerField
            label={t("transcribe.inputMediaFile")}
            value={inputPath}
            placeholder={t("transcribe.noInputMediaSelected")}
            browseTitle={t("transcribe.selectFile")}
            onexpand={() => (expandedPathField = "input")}
            onbrowse={selectInputFile}
            required={true}
          />

          <PathPickerField
            label={t("transcribe.outputSrtFile")}
            value={outputPath}
            placeholder={t("transcribe.noOutputFileSelected")}
            browseTitle={t("transcribe.selectDestination")}
            onexpand={() => (expandedPathField = "output")}
            onbrowse={selectOutputFile}
            required={true}
          />
          {#if inputPath}
            <div
              class="p-3 bg-cyan-500/10 border border-cyan-500/30 rounded-lg"
            >
              <div class="flex items-center gap-3">
                <div
                  class="w-10 h-10 rounded-lg bg-cyan-500/20 flex items-center justify-center"
                >
                  <span class="text-xl">🎬</span>
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
                  📁 {result.output_path}
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
        {logs}
        onclear={clearLogs}
        minHeight="170px"
        maxHeightContent="100%"
      />
    {/if}
  {/snippet}

  {#snippet transcribedSentencesCard()}
    <div class="glass-card p-5 space-y-4">
      <div class="flex items-center justify-between">
        <span class="text-[10px] font-bold text-gray-500 uppercase tracking-wide">{t("transcribe.livePhrases")}</span>
        <span class="text-[10px] text-indigo-400 font-semibold">{transcribedSegments.length} {t("transcribe.segments") || "segments"}</span>
      </div>
      
      {#if transcribedSegments.length === 0}
        <div class="rounded-lg border border-indigo-500/20 bg-indigo-500/5 px-3 py-8 text-center text-xs text-indigo-300">
          {t("transcribe.livePhrasesHint")}
        </div>
      {:else}
        <div 
          bind:this={scrollContainer} 
          class="space-y-2 max-h-[220px] overflow-y-auto pr-1 transcribe-scroll"
        >
          {#each transcribedSegments as segment}
            <div class="p-2.5 rounded-lg bg-white/[0.02] border border-white/5 flex gap-3 text-xs hover:bg-white/5 transition-colors">
              <span class="font-mono text-indigo-300 shrink-0 select-none">
                {formatTime(segment.start_ms)} → {formatTime(segment.end_ms)}
              </span>
              <span class="text-gray-200 break-words">{segment.text}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/snippet}

  <div class="flex-1 overflow-y-auto p-6 min-h-0 {isTranscribing ? 'pointer-events-none opacity-60 select-none' : ''}">
    <div class="grid grid-cols-1 xl:grid-cols-2 gap-6">
      <div class="space-y-3 min-h-[100px]">
        {@render panelContent("files")}
        {@render transcribedSentencesCard()}
      </div>

      <div class="space-y-3 min-h-[100px]">
        {@render panelContent("options")}
        <!-- {@render panelContent("logs")} -->
      </div>
    </div>
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
            <span class="text-[10px] text-cyan-400 font-bold uppercase tracking-wider">{t("transcribe.transcribing") || "Trascrizione..."}</span>
            <span class="text-xs text-white font-medium truncate max-w-lg">
              {progressMessage || t("transcribe.running") || "Elaborazione in corso"} ({progress}%)
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
              <span class="text-[10px] text-emerald-400 font-bold uppercase tracking-wider">{t("transcribe.finished") || "Completato"}</span>
              {#if result.output_path}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div 
                  onclick={() => {
                    if (result?.output_path) {
                      navigator.clipboard.writeText(result.output_path);
                      showSnackbar($currentLanguage === 'it' ? 'Percorso copiato negli appunti!' : 'Path copied to clipboard!', 'success');
                    }
                  }}
                  class="flex items-center gap-1.5 text-xs text-gray-400 mt-0.5 cursor-pointer hover:text-white transition-colors bg-white/5 hover:bg-white/10 px-2 py-0.5 rounded border border-white/5 select-all"
                  title={$currentLanguage === 'it' ? 'Clicca per copiare il percorso' : 'Click to copy path'}
                >
                  <span class="truncate max-w-sm">📁 {getFileName(result.output_path)}</span>
                  <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
                  </svg>
                </div>
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
              <span class="text-[10px] text-red-400 font-bold uppercase tracking-wider">{t("transcribe.error") || "Errore"}</span>
              <span class="text-xs text-red-300 font-medium truncate max-w-lg">{result.message}</span>
            </div>
          {/if}
        </div>
      {:else if error}
        <!-- Error State -->
        <div class="flex items-center gap-3">
          <div class="flex items-center justify-center w-8 h-8 rounded-full bg-red-500/10 border border-red-500/30 text-red-400">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
          </div>
          <div class="flex flex-col">
            <span class="text-[10px] text-red-400 font-bold uppercase tracking-wider">{t("transcribe.error") || "Errore"}</span>
            <span class="text-xs text-red-300 font-medium truncate max-w-lg">{error}</span>
          </div>
        </div>
      {/if}
    </div>
    {/snippet}
    {#snippet right()}
    <!-- Right side: Action Buttons -->
    <div class="flex items-center gap-4 z-10 select-none shrink-0">
      {#if isTranscribing}
        <button
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
    secondaryText={`✏️ ${t("transcribe.editPath")}`}
    desc={expandedPathField === "input"
      ? t("transcribe.inputPathDesc")
      : t("transcribe.outputPathDesc")}
    onsave={expandedPathField === "input" ? saveInputPath : saveOutputPath}
  />

  <ConfirmDialog
    show={showOverwriteConfirm}
    title="Sovrascrivere il file esistente?"
    message="Hai già un file caricato per la trascrizione. Se procedi, il file attuale verrà sostituito con quello nuovo."
    confirmText="Sovrascrivi"
    cancelText="Annulla"
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
