import { invokeCommand as invoke } from "$lib/services/tauriClient";
import { listen } from "@tauri-apps/api/event";
import * as vestaConfig from "$lib/config/vestaConfig";
import { languages } from "$lib/config/languages";
import { CARD_TEMPLATES_UPDATED_EVENT, NOTE_TYPES_UPDATED_EVENT, type NoteTypeDef } from "$lib/types/noteTypes";
import type { CardFilterSettings } from "$lib/types/flashcardFilterTypes";
import type { MediaSettings } from "$lib/utils/mediaSettings";
import { buildFlashcardConfig } from "$lib/utils/flashcardConfig";
import { deriveDeckNameFromFile, type EpisodeEntry } from "$lib/utils/seriesFileMatching";
import { defaultOutputDirectory } from "$lib/utils/defaultOutputDirectory";
import { ankiStore } from "$lib/stores/ankiStore.svelte";
import { generationStore } from "$lib/stores/generationStore.svelte";
import { snackbar } from "$lib/stores/snackbarStore.svelte";
import { errorMessage } from "$lib/utils/errorMessage";

export const OUTPUT_DIR_KEY = "vesta-last-output-dir";

export async function loadInitialFlashcardsOutputDir(): Promise<string> {
  let outputDir = "";
  try {
    const savedDir = vestaConfig.getItem(OUTPUT_DIR_KEY);
    if (savedDir) {
      const exists = await invoke<boolean>("flashcard_check_dir_exists", { path: savedDir });
      if (exists) outputDir = savedDir;
      else vestaConfig.removeItem(OUTPUT_DIR_KEY);
    }
  } catch {
    /* storage unavailable */
  }
  if (!outputDir) {
    try {
      outputDir = await defaultOutputDirectory();
    } catch {
      /* no default directory */
    }
  }
  return outputDir;
}

export async function loadInitialFlashcardsCpuCores(): Promise<void> {
  try {
    generationStore.systemCpuCount = await invoke<number>("flashcard_get_cpu_count");
    const startupMaxCores = Math.max(2, generationStore.systemCpuCount - 1);
    const savedCores = vestaConfig.getItem("vesta_cpu_cores");
    generationStore.cpuCores = savedCores ? parseInt(savedCores) : startupMaxCores;
  } catch {
    generationStore.systemCpuCount = 4;
    const savedCores = vestaConfig.getItem("vesta_cpu_cores");
    generationStore.cpuCores = savedCores
      ? parseInt(savedCores)
      : Math.max(2, generationStore.systemCpuCount - 1);
  }
}

export async function maybeAutoImportApkgToAnki(
  apkgPath: string,
  t: (key: string, params?: Record<string, string | number>) => string,
): Promise<void> {
  const url = ankiStore.url;
  if (!(await ankiStore.checkConnection(true))) {
    if (generationStore.exportFormat === "anki") {
      const fallbackUpper = generationStore.fallbackFormat.toUpperCase();
      const msg = t("flashcards.ankiNotRunningFallback", { format: fallbackUpper });
      generationStore.addLog(`ℹ ${msg}`, "info");
      snackbar.show(msg, "info");
    }
    return;
  }

  try {
    await invoke("ankiconnect_import_package", { path: apkgPath, url });
    generationStore.addLog(t("flashcards.ankiAutoImportSuccess"), "success");
    snackbar.show(t("flashcards.ankiAutoImportSuccess"), "success");
  } catch (e) {
    generationStore.addLog(`${t("flashcards.ankiAutoImportFailed")}: ${errorMessage(e)}`, "warning");
  }
}

export function subscribeFlashcardProgressEvents({
  isActive,
  isCancelled,
  getSeriesProgress,
  translate,
}: {
  isActive: () => boolean;
  isCancelled: () => boolean;
  getSeriesProgress: () => { current: number; total: number };
  translate: (key: string, params?: Record<string, string | number>) => string;
}): Promise<() => void> {
  return listen<{
    stage: string;
    message: string;
    current: number;
    total: number;
    percentage: number;
    params: Record<string, string>;
  }>("flashcard-progress", (event) => {
    if (!generationStore.isBusy || isCancelled()) return;
    const p = event.payload;
    const translated = translate(p.message, p.params || {});
    const { current, total } = getSeriesProgress();

    if (total > 0) {
      generationStore.progress = Math.round(((current - 1) * 100) / total + p.percentage / total);
      generationStore.progressMessage = `[Ep. ${current}/${total}] ${translated}`;
    } else {
      generationStore.progress = Math.round(p.percentage);
      generationStore.progressMessage = translated;
    }

    generationStore.progressStage = p.stage;
    generationStore.phaseProgress =
      p.stage === "optimizing" && p.params?.percent ? Number(p.params.percent) : null;
    if (p.stage !== "done") {
      generationStore.addLog(generationStore.progressMessage, "progress", undefined, p.message);
    }
  }).then((fn) => {
    if (!isActive()) {
      fn();
      return () => {};
    }
    return fn;
  });
}

export function buildFirstEpisodeFlashcardConfig({
  ep,
  outputDir,
  cardFilters,
  mediaSettings,
  videoHwAccel,
  activeNoteType,
  getEpisodeNoteType,
  targetLanguage,
}: {
  ep: EpisodeEntry | undefined;
  outputDir: string;
  cardFilters: CardFilterSettings;
  mediaSettings: MediaSettings;
  videoHwAccel: string;
  activeNoteType: NoteTypeDef;
  getEpisodeNoteType: (ep: Pick<EpisodeEntry, "targetSubsPath" | "noteTypeId" | "detectedLanguage">) => NoteTypeDef;
  targetLanguage: string;
}) {
  return buildFlashcardConfig({
    targetSubsPath: ep?.targetSubsPath || "",
    nativeSubsPath: ep?.nativeSubsPath || null,
    videoPath: ep?.mediaType === "video" ? ep.mediaPath : null,
    audioPath: ep?.mediaType === "audio" ? ep.mediaPath : null,
    outputDir,
    cardFilters,
    media: mediaSettings,
    generateAudio: mediaSettings.generateAudio,
    generateSnapshots: mediaSettings.generateSnapshots,
    generateVideoClips: mediaSettings.generateVideoClips,
    audioTrackIndex: mediaSettings.audioTrackIndex,
    videoHwAccel,
    deckName: ep ? deriveDeckNameFromFile(ep) : "",
    episodeNumber: 1,
    exportFormat: generationStore.effectiveExportFormat,
    noteType: ep ? getEpisodeNoteType(ep) : activeNoteType,
    automaticNoteType: false,
    cpuCores: generationStore.effectiveCpuCores,
    targetLanguage,
    dictionaryLookup: ankiStore.dictionaryLookup,
    autoCardFont: ankiStore.autoCardFont,
    embedCardFont: ankiStore.embedCardFont,
  });
}

export function setupFlashcardsWindowSubscriptions({
  defaultLanguageKey,
  getLayoutHost,
  onNoteTypesChanged,
  onLanguageDefaultChanged,
  onLayoutWidthChanged,
}: {
  defaultLanguageKey: string;
  getLayoutHost: () => HTMLElement | null;
  onNoteTypesChanged: () => void;
  onLanguageDefaultChanged: (code: string) => void;
  onLayoutWidthChanged: (width: number) => void;
}): () => void {
  const handleLanguageDefaultsUpdated = () => {
    try {
      const code = vestaConfig.getItem(defaultLanguageKey);
      if (code && languages.some((l) => l.code === code)) {
        onLanguageDefaultChanged(code);
      }
    } catch {
      /* storage unavailable */
    }
  };

  const updateLayoutWidth = () => {
    const hostWidth = getLayoutHost()?.getBoundingClientRect().width;
    onLayoutWidthChanged(hostWidth && hostWidth > 0 ? Math.round(hostWidth) : window.innerWidth);
  };

  const resizeObserver =
    typeof ResizeObserver !== "undefined" ? new ResizeObserver(updateLayoutWidth) : null;
  const host = getLayoutHost();
  if (resizeObserver && host) resizeObserver.observe(host);

  window.addEventListener(NOTE_TYPES_UPDATED_EVENT, onNoteTypesChanged);
  window.addEventListener(CARD_TEMPLATES_UPDATED_EVENT, onNoteTypesChanged);
  window.addEventListener("vesta-language-defaults-updated", handleLanguageDefaultsUpdated);
  window.addEventListener("resize", updateLayoutWidth);
  updateLayoutWidth();

  return () => {
    window.removeEventListener(NOTE_TYPES_UPDATED_EVENT, onNoteTypesChanged);
    window.removeEventListener(CARD_TEMPLATES_UPDATED_EVENT, onNoteTypesChanged);
    window.removeEventListener("vesta-language-defaults-updated", handleLanguageDefaultsUpdated);
    window.removeEventListener("resize", updateLayoutWidth);
    resizeObserver?.disconnect();
  };
}

