<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import LogPanel, { type LogEntry } from "./LogPanel.svelte";
  import { listen } from "@tauri-apps/api/event";
  import { guardedOpen, guardedSave } from "./dialogGuard";
  import { onDestroy, onMount } from "svelte";
  import { locale } from "./i18n";
  import { languages as allLanguages } from "./models";
  import PathPickerField from "./PathPickerField.svelte";
  import PathPreviewModal from "./PathPreviewModal.svelte";
  import SearchableSelect from "./SearchableSelect.svelte";
  import Snackbar from "./Snackbar.svelte";
  import InfoModal from "./InfoModal.svelte";
  import InfoButton from "./InfoButton.svelte";
  import { transcribeSections } from "./info";

  let { onGoToSettings } = $props<{ onGoToSettings?: () => void }>();

  let t = $derived($locale);

  let inputPath = $state("");
  let outputPath = $state("");
  let selectedModel = $state("base");
  let selectedLanguage = $state("auto");
  let previousLanguageForOutput = "auto";
  let translateToEnglish = $state(false);
  let wordTimestamps = $state(true);
  let maxSegmentLength = $state(30);

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

  let snackbarMessage = $state<string | null>(null);
  let snackbarTimeout = $state<ReturnType<typeof setTimeout> | null>(null);

  function showSnackbar(message: string) {
    if (snackbarTimeout) clearTimeout(snackbarTimeout);
    snackbarMessage = message;
    snackbarTimeout = setTimeout(() => {
      snackbarMessage = null;
    }, 3500);
  }

  let logIdCounter = 0;
  let logs = $state<LogEntry[]>([]);

  let helpSection = $state<string | null>(null);

  let showInputPathDialog = $state(false);
  let showOutputPathDialog = $state(false);
  let editInputPath = $state("");
  let editOutputPath = $state("");
  let inputPathError = $state<string | null>(null);
  let outputPathError = $state<string | null>(null);
  let expandedPathField = $state<string | null>(null);

  let backends = $state<{
    ffmpeg: boolean;
    whisper_cpp: boolean;
    python_whisper: boolean;
    any_whisper: boolean;
    whisper_binary: string | null;
  } | null>(null);

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
        `Language set to ${selectedLanguageLabel(currentLang)}; output updated to ${outputPath.split("/").pop()}`,
        "info",
      );
      previousLanguageForOutput = currentLang;
      if (effectivePrevLang !== effectiveCurrentLang) {
        result = null;
      }
    }
  });

  let unlistenProgress: (() => void) | null = null;

  onMount(async () => {
    selectedModel = localStorage.getItem("srt-default-whisper-model") || "base";

    window.addEventListener("whisper-model-updated", (e: any) => {
      if (e.detail) {
        selectedModel = e.detail;
      }
      refreshModels();
    });

    try {
      backends = await invoke<typeof backends>("transcribe_check_backends");
    } catch (e) {
      console.error("Could not check backends:", e);
    }

    await refreshModels();

    unlistenProgress = await listen<{
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
    });
  });

  onDestroy(() => {
    unlistenProgress?.();
  });

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
              "mp3",
              "wav",
              "m4a",
              "flac",
              "ogg",
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
          `${t("transcribe.fileSelected")}: ${inputPath.split("/").pop()}`,
          "file",
        );
        addLog(`Output file: ${outputPath.split("/").pop()}`, "info");
      }
    } catch (e) {
      error = `${t("transcribe.errorSelectingFile")}: ${e}`;
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
        addLog(`Output file set manually: ${outputPath.split("/").pop()}`, "file");
      }
    } catch (e) {
      error = `${t("transcribe.errorSelectingFile")}: ${e}`;
    }
  }

  function openInputPathDialog() {
    editInputPath = inputPath;
    inputPathError = null;
    showInputPathDialog = true;
  }

  function openOutputPathDialog() {
    editOutputPath = outputPath;
    outputPathError = null;
    showOutputPathDialog = true;
  }

  async function confirmInputPath() {
    let cleaned = editInputPath.trim().replace(/\/+$/, "");
    if (!cleaned) {
      inputPathError = "Path cannot be empty";
      return;
    }
    try {
      const exists = await invoke<boolean>("transcribe_check_file_exists", {
        path: cleaned,
      });
      if (!exists) {
        inputPathError = `File not found: ${cleaned}`;
        return;
      }
      inputPath = cleaned;
      if (!outputPath) {
        const outputLang = effectiveLanguageCodeForOutput(selectedLanguage);
        outputPath = generateOutputPathFromInput(inputPath, outputLang);
      }
      showInputPathDialog = false;
      addLog(
        `${t("transcribe.fileSelected")}: ${inputPath.split("/").pop()}`,
        "file",
      );
      addLog(`Output file: ${outputPath.split("/").pop()}`, "info");
    } catch (e) {
      inputPathError = `Error: ${e}`;
    }
  }

  async function confirmOutputPath() {
    let cleaned = editOutputPath.trim().replace(/\/+$/, "");
    if (!cleaned) {
      outputPathError = "Path cannot be empty";
      return;
    }
    const parentDir = cleaned.substring(0, cleaned.lastIndexOf("/"));
    if (parentDir) {
      try {
        const exists = await invoke<boolean>("transcribe_check_file_exists", {
          path: parentDir,
        });
        if (!exists) {
          outputPathError = `Directory not found: ${parentDir}`;
          return;
        }
      } catch (e) {
        outputPathError = `Error: ${e}`;
        return;
      }
    }
    outputPath = cleaned;
    showOutputPathDialog = false;
  }

  async function startTranscription() {
    if (!inputPath || !outputPath) {
      error = t("transcribe.selectFilesFirst");
      return;
    }

    if (!isModelDownloaded) {
      error = "The selected Whisper model is not downloaded. Please download it in Settings.";
      return;
    }

    error = null;
    result = null;
    progress = 0;
    isTranscribing = true;
    addLog(`${t("transcribe.starting")} (model: ${selectedModel})`, "info");
    addLog(`Source language: ${selectedLanguageLabel(selectedLanguage)}`, "info");
    addLog(`Word timestamps: ${wordTimestamps ? "enabled" : "disabled"}; max segment: ${maxSegmentLength}s`, "info");
    addLog(`Input: ${inputPath.split("/").pop()} → Output: ${outputPath.split("/").pop()}`, "file");

    const startTime = Date.now();

    try {
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
          model: selectedModel,
          language: selectedLanguage,
          translate_to_english: translateToEnglish,
          word_timestamps: wordTimestamps,
          max_segment_length: maxSegmentLength,
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
        addLog(`Saved: ${res.output_path.split("/").pop()}`, "success");
      }
      addLog(res.message, "success");
      await refreshModels();
    } catch (e: any) {
      error = `${e}`;
      addLog(`Error: ${e}`, "error");
    } finally {
      isTranscribing = false;
      const elapsed = Math.floor((Date.now() - startTime) / 1000);
      const hh = String(Math.floor(elapsed / 3600)).padStart(2, "0");
      const mm = String(Math.floor((elapsed % 3600) / 60)).padStart(2, "0");
      const ss = String(elapsed % 60).padStart(2, "0");
      addLog(`⏱ ${hh}:${mm}:${ss}`, "info");
    }
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
    "actions",
    "progress",
    "logs",
  ] as const;

  type TranscribePanelId = (typeof TRANSCRIBE_PANEL_IDS)[number];

  interface TranscribeColumnLayout {
    col1: TranscribePanelId[];
    col2: TranscribePanelId[];
  }

  const TRANSCRIBE_DEFAULT_LAYOUT: TranscribeColumnLayout = {
    col1: ["files"],
    col2: ["options", "actions", "progress", "logs"],
  };

  function loadTranscribeLayout(): TranscribeColumnLayout {
    try {
      const saved = localStorage.getItem("srt-transcribe-layout-v1");
      if (saved) {
        const parsed = JSON.parse(saved) as TranscribeColumnLayout;
        const all = [...parsed.col1, ...parsed.col2];
        const valid =
          TRANSCRIBE_PANEL_IDS.every((id) => all.includes(id)) &&
          all.length === TRANSCRIBE_PANEL_IDS.length;
        if (valid) return parsed;
      }
    } catch {}
    return { ...TRANSCRIBE_DEFAULT_LAYOUT };
  }

  function saveTranscribeLayout(layout: TranscribeColumnLayout) {
    localStorage.setItem("srt-transcribe-layout-v1", JSON.stringify(layout));
  }

  let transcribePanelLayout = $state<TranscribeColumnLayout>(
    loadTranscribeLayout(),
  );

  let tDraggedPanel = $state<TranscribePanelId | null>(null);
  let tDragOverCol = $state<"col1" | "col2" | null>(null);
  let tDragOverIdx = $state<number | null>(null);

  function tOnDragStart(e: DragEvent, panelId: TranscribePanelId) {
    const target = e.target as HTMLElement;
    if (
      target?.tagName === "INPUT" &&
      (target as HTMLInputElement).type === "range"
    ) {
      e.preventDefault();
      return;
    }
    tDraggedPanel = panelId;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", panelId);
    }
  }

  function tOnDragOver(e: DragEvent, col: "col1" | "col2", idx: number) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    tDragOverCol = col;
    tDragOverIdx = idx;
  }

  function tOnDragOverColumn(e: DragEvent, col: "col1" | "col2") {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    tDragOverCol = col;
    if (tDragOverIdx === null) {
      tDragOverIdx = transcribePanelLayout[col].length;
    }
  }

  function tOnDrop(col: "col1" | "col2", idx: number) {
    if (!tDraggedPanel) return;
    const newLayout = { ...transcribePanelLayout };
    for (const c of ["col1", "col2"] as const) {
      const i = newLayout[c].indexOf(tDraggedPanel);
      if (i !== -1) {
        newLayout[c] = [...newLayout[c]];
        newLayout[c].splice(i, 1);
        if (c === col && i < idx) idx--;
        break;
      }
    }
    newLayout[col] = [...newLayout[col]];
    newLayout[col].splice(idx, 0, tDraggedPanel);
    transcribePanelLayout = newLayout;
    saveTranscribeLayout(transcribePanelLayout);
    tDraggedPanel = null;
    tDragOverCol = null;
    tDragOverIdx = null;
  }

  function tOnDropColumn(col: "col1" | "col2") {
    tOnDrop(col, transcribePanelLayout[col].length);
  }

  function tOnDragEnd() {
    tDraggedPanel = null;
    tDragOverCol = null;
    tDragOverIdx = null;
  }

  function resetTranscribeLayout() {
    transcribePanelLayout = {
      col1: [...TRANSCRIBE_DEFAULT_LAYOUT.col1],
      col2: [...TRANSCRIBE_DEFAULT_LAYOUT.col2],
    };
    saveTranscribeLayout(transcribePanelLayout);
  }
</script>

<div
  class="h-full flex flex-col p-6 overflow-y-auto bg-gradient-to-br from-gray-900 via-gray-900 to-gray-950"
>
  <!-- FFmpeg Warning (whisper-rs is always available natively) -->
  {#if backends && !backends.ffmpeg}
    <div
      class="mb-4 p-3 bg-amber-500/10 border border-amber-500/30 rounded-lg flex items-center gap-3 shrink-0"
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
        onclick={async () => {
          const { open } = await import("@tauri-apps/plugin-shell");
          await open("https://www.ffmpeg.org/download.html");
        }}
        class="flex-shrink-0 px-3 py-1.5 rounded-lg bg-amber-500/20 border border-amber-500/40 text-amber-300 text-xs font-semibold hover:bg-amber-500/30 transition-colors flex items-center gap-1.5"
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" /></svg>
        {t("transcribe.ffmpegDownload")}
      </button>
    </div>
  {/if}

  {#if isTranscribing || result || error}
    <div class="flex items-center justify-end mb-4 shrink-0">
      <button
        onclick={resetTranscription}
        disabled={isTranscribing}
        class="py-1.5 px-4 rounded-lg border border-amber-500/30 bg-amber-500/10 text-amber-300 hover:bg-amber-500/20 transition-colors text-sm font-medium disabled:opacity-30 disabled:cursor-not-allowed flex items-center gap-2"
        title={t("transcribe.newTranscriptionDesc")}
      >
        <svg
          class="w-4 h-4"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          ><path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
          /></svg
        >
        {t("transcribe.newTranscription")}
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
        <h3 class="text-lg font-semibold mb-4 flex items-center gap-2">
          <svg
            class="w-5 h-5 text-blue-400"
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
          <InfoButton onclick={() => (helpSection = "options")} />
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
                searchTerms: `${lang.nameEn} ${lang.name}`,
                icon: lang.flag,
              }))}
              value={selectedLanguage}
              onchange={(v) => (selectedLanguage = v)}
              placeholder={t("transcribe.sourceLanguage")}
            />
          </div>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="flex items-center justify-between p-3 bg-white/5 rounded-lg"
          >
            <div>
              <span class="text-gray-200 text-sm"
                >{t("transcribe.wordTimestamps")}</span
              >
              <p class="text-xs text-gray-500">
                {t("transcribe.wordTimestampsDesc")}
              </p>
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
          <div>
            <div class="flex items-center justify-between mb-2">
              <span class="text-sm text-gray-400 flex items-center gap-2">
                {t("transcribe.maxSegmentLength")}
                <InfoButton onclick={() => (helpSection = "segmentLength")} />
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
            <div class="mt-2 flex items-center justify-between text-xs">
              <span class="text-gray-500">Segment length</span>
              <span
                class="text-white font-mono bg-white/10 px-2 py-0.5 rounded text-sm"
                >{maxSegmentLength}s</span
              >
            </div>
          </div>
        </div>
      </div>
    {:else if panelId === "files"}
      <div class="glass-card p-5">
        <h3 class="text-lg font-semibold mb-4 flex items-center gap-2">
          <svg
            class="w-5 h-5 text-indigo-400"
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
          {t("transcribe.files")}
          <InfoButton onclick={() => (helpSection = "files")} />
        </h3>
        <div class="space-y-3">
          <PathPickerField
            label={t("transcribe.inputFile")}
            value={inputPath}
            placeholder={t("transcribe.noInputMediaSelected")}
            browseTitle={t("transcribe.selectFile")}
            onexpand={() => (expandedPathField = "input")}
            onbrowse={selectInputFile}
            browseButtonClass="btn-primary py-2 px-3"
            browseIconPath="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
          />

          <PathPickerField
            label={t("transcribe.outputFile")}
            value={outputPath}
            placeholder={t("transcribe.noOutputFileSelected")}
            browseTitle={t("transcribe.selectDestination")}
            onexpand={() => (expandedPathField = "output")}
            onbrowse={selectOutputFile}
            browseButtonClass="btn-secondary py-2 px-3"
            browseIconPath="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"
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
                    {inputPath.split("/").pop()}
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
    {:else if panelId === "actions"}
      <div class="space-y-3">
        <div class="flex gap-3">
          {#if isTranscribing}
            <button
              onclick={cancelTranscription}
              class="btn-danger flex-1 py-4 text-lg"
            >
              <svg
                class="w-5 h-5 inline mr-2"
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
          {:else}
            <button
              onclick={startTranscription}
              disabled={!inputPath || !outputPath}
              class="btn-success flex-1 py-4 text-lg disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <svg
                class="w-5 h-5 inline mr-2"
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
          {/if}
        </div>
        {#if !isModelDownloaded}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="p-3 bg-amber-500/10 border border-amber-500/30 rounded-lg cursor-pointer hover:bg-amber-500/20 transition-colors"
            onclick={() => onGoToSettings?.()}
          >
            <div class="flex items-start gap-2">
              <span class="text-amber-400">⚠️</span>
              <div class="flex-1 flex flex-col items-center text-center">
                <p class="text-xs text-amber-200">
                  {t("transcribe.modelDownloadNote") || "It is necessary to download and set a Whisper model from Settings. Click here to go to Settings."}
                </p>
              </div>
            </div>
          </div>
        {/if}
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

  <div class="flex-1 grid grid-cols-1 xl:grid-cols-2 gap-6 min-h-0 overflow-y-auto">
    <div
      class="space-y-3 overflow-y-auto pr-1 min-h-[100px]"
      role="list"
    >
      {#each transcribePanelLayout.col1 as tPanelId, idx (tPanelId)}
        <div
          class="transition-all duration-150"
          role="listitem"
        >
          {@render panelContent(tPanelId)}
        </div>
      {/each}
    </div>

    <div
      class="space-y-3 overflow-y-auto pr-1 min-h-[100px]"
      role="list"
    >
      {#each transcribePanelLayout.col2 as tPanelId, idx (tPanelId)}
        <div
          class="transition-all duration-150"
          role="listitem"
        >
          {@render panelContent(tPanelId)}
        </div>
      {/each}
    </div>
  </div>

  {#if showInputPathDialog}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-6"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={() => (showInputPathDialog = false)}
      onkeydown={(e) => {
        if (e.key === "Escape") showInputPathDialog = false;
      }}
    >
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="bg-gray-900 border border-gray-700 rounded-xl w-full max-w-xl p-6"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => {
          if (e.key === "Enter") confirmInputPath();
          e.stopPropagation();
        }}
      >
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-lg font-bold text-white">
            {t("transcribe.inputFile")}
          </h2>
          <button
            onclick={() => (showInputPathDialog = false)}
            class="text-gray-400 hover:text-white text-xl">✕</button
          >
        </div>
        <p class="text-xs text-gray-400 mb-3">
          {t("transcribe.inputPathDesc")}
        </p>
        <input
          type="text"
          bind:value={editInputPath}
          class="input-modern w-full text-sm mb-2"
          placeholder="/path/to/audio-or-video.mp4"
        />
        {#if inputPathError}
          <p class="text-red-400 text-xs mb-2">{inputPathError}</p>
        {/if}
        <div class="flex justify-end gap-2 mt-4">
          <button
            onclick={() => (showInputPathDialog = false)}
            class="btn-secondary py-1.5 px-4 text-sm"
          >
            {t("settings.modal.cancel")}
          </button>
          <button
            onclick={confirmInputPath}
            class="btn-primary py-1.5 px-4 text-sm">OK</button
          >
        </div>
      </div>
    </div>
  {/if}

  {#if showOutputPathDialog}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-6"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={() => (showOutputPathDialog = false)}
      onkeydown={(e) => {
        if (e.key === "Escape") showOutputPathDialog = false;
      }}
    >
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="bg-gray-900 border border-gray-700 rounded-xl w-full max-w-xl p-6"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => {
          if (e.key === "Enter") confirmOutputPath();
          e.stopPropagation();
        }}
      >
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-lg font-bold text-white">
            {t("transcribe.outputFile")}
          </h2>
          <button
            onclick={() => (showOutputPathDialog = false)}
            class="text-gray-400 hover:text-white text-xl">✕</button
          >
        </div>
        <p class="text-xs text-gray-400 mb-3">
          {t("transcribe.outputPathDesc")}
        </p>
        <input
          type="text"
          bind:value={editOutputPath}
          class="input-modern w-full text-sm mb-2"
          placeholder="/path/to/output.srt"
        />
        {#if outputPathError}
          <p class="text-red-400 text-xs mb-2">{outputPathError}</p>
        {/if}
        <div class="flex justify-end gap-2 mt-4">
          <button
            onclick={() => (showOutputPathDialog = false)}
            class="btn-secondary py-1.5 px-4 text-sm"
          >
            {t("settings.modal.cancel")}
          </button>
          <button
            onclick={confirmOutputPath}
            class="btn-primary py-1.5 px-4 text-sm">OK</button
          >
        </div>
      </div>
    </div>
  {/if}

  <InfoModal 
    section={helpSection} 
    sections={transcribeSections} 
    onclose={() => (helpSection = null)} 
  />

  <PathPreviewModal
    isOpen={!!expandedPathField}
    title={expandedPathField === "input"
      ? t("transcribe.inputFile")
      : t("transcribe.outputFile")}
    value={expandedPathField === "input" ? inputPath : outputPath}
    onclose={() => (expandedPathField = null)}
    secondaryText={`✏️ ${t("transcribe.editPath")}`}
    onsecondary={() => {
      const field = expandedPathField;
      expandedPathField = null;
      if (field === "input") openInputPathDialog();
      else if (field === "output") openOutputPathDialog();
    }}
  />



  {#if snackbarMessage}
    <Snackbar
      message={snackbarMessage}
      variant="success"
      onclose={() => (snackbarMessage = null)}
    />
  {/if}
</div>
