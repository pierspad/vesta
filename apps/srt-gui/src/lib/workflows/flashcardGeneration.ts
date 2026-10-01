import type { GenerationResult, generationStore as store } from "$lib/stores/generationStore.svelte";
import type { buildFlashcardConfig } from "$lib/utils/flashcardConfig";
import { formatElapsedTime } from "$lib/utils/elapsedTime";

export interface FlashcardGenerateResult {
  success: boolean;
  message: string;
  cards_generated: number;
  audio_clips: number;
  snapshots: number;
  video_clips: number;
  tsv_path: string | null;
  apkg_path: string | null;
  output_size_bytes?: number;
}

export function generationResultFromResponse(response: FlashcardGenerateResult): GenerationResult {
  return {
    success: response.success, message: response.message || null,
    cardsGenerated: response.cards_generated, audioClips: response.audio_clips,
    snapshots: response.snapshots, videoClips: response.video_clips,
    tsvPath: response.tsv_path, apkgPath: response.apkg_path,
    outputSizeBytes: response.output_size_bytes ?? 0,
  };
}

export interface FlashcardGenerationOptions {
  generationStore: Pick<typeof store, "error" | "result" | "progress" | "progressMessage" | "progressStage" | "isProcessing" | "deckName" | "exportFormat" | "addLog">;
  config: ReturnType<typeof buildFlashcardConfig>;
  t: (key: string, params?: Record<string, string>) => string;
  invoke: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
  applyOverrides: (config: ReturnType<typeof buildFlashcardConfig>) => Promise<unknown>;
  isCancelled: () => boolean;
  importToAnki: (path: string) => Promise<void>;
}

export async function runFlashcardGeneration(options: FlashcardGenerationOptions): Promise<void> {
  const { generationStore: state, config, t, invoke, applyOverrides, isCancelled, importToAnki } = options;
  const importEnabled = state.exportFormat === "anki";
  state.error = null;
  state.result = null;
  state.progress = 0;
  state.isProcessing = true;
  state.addLog(`${t("flashcards.starting")}...`, "info");
  state.addLog(`${t("flashcards.deckName")}: ${state.deckName}`, "info");
  const started = Date.now();
  try {
    await applyOverrides(config);
    if (isCancelled()) return;
    const response = await invoke<FlashcardGenerateResult>("flashcard_generate", { config });
    if (isCancelled()) return;
    state.result = generationResultFromResponse(response);
    if (response.success) {
      state.addLog(`${response.cards_generated} ${t("flashcards.cardsGenerated")}`, "success");
      if (response.tsv_path) state.addLog(`TSV: ${response.tsv_path}`, "success");
      if (response.apkg_path) {
        state.addLog(`APKG: ${response.apkg_path}`, "success");
        if (importEnabled && !isCancelled()) await importToAnki(response.apkg_path);
      }
    } else state.addLog(response.message, "warning");
  } catch (error) {
    if (!isCancelled()) {
      state.error = `${t("flashcards.errorGenerating")}: ${error}`;
      state.addLog(state.error, "error");
      state.result = {
        success: false, message: String(error), cardsGenerated: 0, audioClips: 0,
        snapshots: 0, videoClips: 0, tsvPath: null, apkgPath: null, outputSizeBytes: 0,
      };
    }
  } finally {
    state.isProcessing = false;
    state.progress = 0;
    state.progressMessage = "";
    state.progressStage = "";
    state.addLog(`⏱ ${formatElapsedTime(Date.now() - started)}`, "info");
  }
}
