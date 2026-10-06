import type { FlashcardGenerateResult } from "./flashcardGeneration";
import { formatElapsedTime } from "$lib/utils/elapsedTime";
import { buildFlashcardConfig } from "$lib/utils/flashcardConfig";
import { deriveDeckNameFromFile, type EpisodeEntry } from "$lib/utils/seriesFileMatching";
import type { EpisodeMediaOverrides } from "$lib/types/flashcardMediaTypes";
import type { CardFilterSettings } from "$lib/types/flashcardFilterTypes";
import type { NoteTypeDef } from "$lib/types/noteTypes";
import type { generationStore as store } from "$lib/stores/generationStore.svelte";

/** UI-independent orchestration with injected IPC, cancellation and progress callbacks. */
export interface FlashcardSeriesOptions {
  generationStore: Pick<typeof store, "deckName" | "deckNameAuto" | "error" | "result" | "progress" | "progressMessage" | "progressStage" | "isProcessing" | "seriesOutputMode" | "effectiveExportFormat" | "effectiveCpuCores" | "exportFormat" | "addLog">;
  episodes: EpisodeEntry[];
  easyMode: boolean;
  needsDeckName: boolean;
  t: (key: string, params?: Record<string, string>) => string;
  outputDir: string;
  cardFilters: CardFilterSettings;
  videoHwAccel: string;
  activeNoteType: NoteTypeDef;
  automaticNoteType?: boolean;
  ankiStore: { autoCardFont: boolean; embedCardFont: boolean };
  previewStore: { applyOverrides: (config: ReturnType<typeof buildFlashcardConfig>) => Promise<unknown> };
  getEpisodeMediaSettings: (episode: EpisodeEntry) => Required<EpisodeMediaOverrides>;
  pickAudioTrackIndexForEpisode: (episode: EpisodeEntry) => Promise<number | null>;
  getStudiedLanguagePreference: () => string;
  maybeAutoImportToAnki: (path: string) => Promise<void>;
  invoke: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
  isCancelled: () => boolean;
  setEpisodeProgress: (current: number, total: number) => void;
}

export async function runFlashcardSeries(options: FlashcardSeriesOptions) {
  const { generationStore, episodes, easyMode, needsDeckName, t, outputDir, cardFilters, videoHwAccel, activeNoteType, ankiStore, previewStore, getEpisodeMediaSettings, pickAudioTrackIndexForEpisode, getStudiedLanguagePreference, maybeAutoImportToAnki, invoke, isCancelled, setEpisodeProgress } = options;
  if (easyMode && needsDeckName && !generationStore.deckName.trim() && episodes.length > 0) {
    generationStore.deckName = deriveDeckNameFromFile(episodes[0]);
    generationStore.deckNameAuto = true;
  }
  generationStore.error = null;
  generationStore.result = null;
  generationStore.progress = 0;
  generationStore.isProcessing = true;
  setEpisodeProgress(0, episodes.length);

  generationStore.addLog(
    `${t("flashcards.starting")}... (${t("flashcards.modeSeries")}: ${episodes.length} ${t("flashcards.seriesEpisodes")})`,
    "info",
  );
  generationStore.addLog(`${t("flashcards.deckName")}: ${generationStore.deckName}`, "info");

  const { effectiveExportFormat, effectiveCpuCores, exportFormat, deckName } = generationStore;
  const seriesOutputMode = effectiveExportFormat === "apkg" ? generationStore.seriesOutputMode : "separate";
  const targetLanguage = getStudiedLanguagePreference();
  const startTime = Date.now();
  let totalCards = 0;
  let totalAudio = 0;
  let totalSnapshots = 0;
  let totalVideoClips = 0;
  let totalOutputBytes = 0;
  const apkgPaths: string[] = [];
  const tsvPaths: string[] = [];
  let hadError = false;
  const errorMessages: string[] = [];

  try {
    for (let i = 0; i < episodes.length; i++) {
      if (isCancelled()) { hadError = true; errorMessages.push(t("flashcards.cancelled")); break; }
      setEpisodeProgress(i + 1, episodes.length);
      const ep = episodes[i];
      const epNum = i + 1;

      generationStore.addLog(
        `${t("flashcards.processingEpisode", { current: String(epNum), total: String(episodes.length) })}`,
        "info",
      );

      // Determine media availability for this episode
      const epMediaType = ep.mediaType;
      const epHasVideo = epMediaType === "video";
      const epHasMedia = epMediaType !== "none";
      const epMediaSettings = getEpisodeMediaSettings(ep);
      const epAudioTrackIndex =
        ep.mediaOverrides?.audioTrackIndex !== undefined
          ? ep.mediaOverrides.audioTrackIndex
          : await pickAudioTrackIndexForEpisode(ep);

      const epConfig = buildFlashcardConfig({
        targetSubsPath: ep.targetSubsPath,
        nativeSubsPath: ep.nativeSubsPath || null,
        videoPath: epHasVideo ? ep.mediaPath : null,
        audioPath: epHasMedia && !epHasVideo ? ep.mediaPath : null,
        outputDir: seriesOutputMode === "single"
          ? `${outputDir}/episodes/episode-${epNum}`
          : outputDir,
        cardFilters,
        media: epMediaSettings,
        generateAudio: ep.mediaPath ? epMediaSettings.generateAudio : false,
        generateSnapshots: epHasVideo ? epMediaSettings.generateSnapshots : false,
        generateVideoClips: epHasVideo ? epMediaSettings.generateVideoClips : false,
        audioTrackIndex: epAudioTrackIndex,
        videoHwAccel,
        deckName: seriesOutputMode === "separate" ? deriveDeckNameFromFile(ep) : deckName,
        episodeNumber: epNum,
        exportFormat: effectiveExportFormat,
        noteType: activeNoteType,
        automaticNoteType: options.automaticNoteType,
        cpuCores: effectiveCpuCores,
        targetLanguage: targetLanguage,
        autoCardFont: ankiStore.autoCardFont,
        embedCardFont: ankiStore.embedCardFont,
      });

      await previewStore.applyOverrides(epConfig);
      if (isCancelled()) { hadError = true; errorMessages.push(t("flashcards.cancelled")); break; }

      try {
        const res = await invoke<FlashcardGenerateResult>("flashcard_generate", {
          config: epConfig,
        });
        if (res.success) {
          totalCards += res.cards_generated;
          totalAudio += res.audio_clips;
          totalSnapshots += res.snapshots;
          totalVideoClips += res.video_clips;
          totalOutputBytes += res.output_size_bytes ?? 0;
          if (res.apkg_path) apkgPaths.push(res.apkg_path);
          if (res.tsv_path) tsvPaths.push(res.tsv_path);
          generationStore.addLog(
            `✓ Ep ${epNum}: ${res.cards_generated} ${t("flashcards.cardsGenerated")}`,
            "success",
          );
        } else {
          hadError = true;
          generationStore.addLog(`⚠ Ep ${epNum}: ${res.message}`, "warning");
          errorMessages.push(`Ep ${epNum}: ${res.message}`);
        }
      } catch (e: unknown) {
        const errMsg = e ? e.toString() : "Unknown error";
        generationStore.addLog(`✗ Ep ${epNum}: ${errMsg}`, "error");
        hadError = true;
        errorMessages.push(`Ep ${epNum}: ${errMsg}`);
      }
    }

    let finalApkgPath: string | null = null;
    if (apkgPaths.length > 0) {
      finalApkgPath = apkgPaths[apkgPaths.length - 1];
    }

    // Merge APKGs if single mode selected
    if (
      !isCancelled() && seriesOutputMode === "single" &&
      apkgPaths.length > 1 &&
      effectiveExportFormat === "apkg"
    ) {
      generationStore.addLog(t("flashcards.mergingApkg"), "info");
      try {
        const mergedPath = await invoke<string>("flashcard_merge_apkg", {
          apkgPaths,
          outputPath: `${outputDir}/${deckName.replace(/[^a-zA-Z0-9_\-\. ]/g, "_")}.apkg`,
        });
        finalApkgPath = mergedPath;
        generationStore.addLog(`APKG: ${mergedPath}`, "success");
      } catch (e) {
        generationStore.addLog(`${t("flashcards.mergeFailed")}: ${e}`, "error");
        hadError = true;
        errorMessages.push(`${t("flashcards.mergeFailed")}: ${e}`);
      }
    }

    if (finalApkgPath && exportFormat === "anki" && !hadError && !isCancelled()) {
      await maybeAutoImportToAnki(finalApkgPath);
    }

    if (isCancelled()) hadError = true;
    generationStore.result = {
      success: !hadError && totalCards > 0,
      message: errorMessages.length > 0 ? errorMessages.join(", ") : (totalCards === 0 ? "No active subtitle lines after filtering" : null),
      cardsGenerated: totalCards,
      audioClips: totalAudio,
      snapshots: totalSnapshots,
      videoClips: totalVideoClips,
      tsvPath: tsvPaths[0] ?? null,
      apkgPath: finalApkgPath,
      outputSizeBytes: totalOutputBytes,
    };

    generationStore.addLog(
      `${t("flashcards.seriesComplete", { total: String(episodes.length) })}`,
      hadError ? "warning" : "success",
    );

  } catch (e: unknown) {
    const errMsg = e ? e.toString() : "Unknown error";
    generationStore.error = `${t("flashcards.errorGenerating")}: ${errMsg}`;
    generationStore.addLog(`${generationStore.error}`, "error");
    generationStore.result = {
      success: false,
      message: errMsg,
      cardsGenerated: 0,
      audioClips: 0,
      snapshots: 0,
      videoClips: 0,
      tsvPath: tsvPaths[0] ?? null,
      apkgPath: null,
      outputSizeBytes: 0,
    };
  } finally {
    generationStore.isProcessing = false;
    setEpisodeProgress(0, 0);
    generationStore.progress = 0;
    generationStore.progressMessage = "";
    generationStore.progressStage = "";
    generationStore.addLog(`⏱ ${formatElapsedTime(Date.now() - startTime)}`, "info");
  }
}

