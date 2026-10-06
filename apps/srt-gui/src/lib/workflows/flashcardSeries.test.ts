import { expect, it, vi } from "vitest";
import { runFlashcardSeries, type FlashcardSeriesOptions } from "./flashcardSeries";
import { runFlashcardGeneration, type FlashcardGenerationOptions } from "./flashcardGeneration";
import { defaultMediaSettings } from "$lib/utils/mediaSettings";
import { predefinedNoteTypeForLanguage } from "$lib/types/noteTypes";
const response = { success: true, message: "ok", cards_generated: 3, audio_clips: 3, snapshots: 3, video_clips: 0, apkg_path: "episode.apkg", tsv_path: null, output_size_bytes: 100 };
function options(): FlashcardSeriesOptions {
 return {
 generationStore: { deckName: "Deck", deckNameAuto: false, error: null, result: null, progress: 0, progressMessage: "", progressStage: "", isProcessing: false, seriesOutputMode: "single", effectiveExportFormat: "apkg", effectiveCpuCores: 2, exportFormat: "anki", addLog: vi.fn() },
 episodes: [1, 2].map(index => ({ id: index, targetSubsPath: `episode${index}.srt`, nativeSubsPath: "", mediaPath: `episode${index}.mkv`, mediaType: "video" as const })),
 easyMode: false, needsDeckName: true, t: key => key, outputDir: "/out",
 cardFilters: { enabled: false, minChars: 3, minCharsEnabled: false, maxChars: 100, maxCharsEnabled: false, minDurationMs: 500, minDurationEnabled: false, maxDurationMs: 15000, maxDurationEnabled: false, combineSentences: false, continuationChars: "..." },
 videoHwAccel: "auto", activeNoteType: predefinedNoteTypeForLanguage(""),
 ankiStore: { autoCardFont: false, embedCardFont: false }, previewStore: { applyOverrides: vi.fn().mockResolvedValue(undefined) },
 getEpisodeMediaSettings: () => defaultMediaSettings, pickAudioTrackIndexForEpisode: vi.fn().mockResolvedValue(0), getStudiedLanguagePreference: () => "en",
 maybeAutoImportToAnki: vi.fn().mockResolvedValue(undefined), invoke: vi.fn().mockImplementation(async command => command === "flashcard_merge_apkg" ? "merged.apkg" : response),
 isCancelled: () => false, setEpisodeProgress: vi.fn(),
 };
}
it("aggregates episodes, merges once and imports only the final deck", async () => {
 const run = options(); await runFlashcardSeries(run);
 expect(run.generationStore.result).toMatchObject({ success: true, cardsGenerated: 6, outputSizeBytes: 200, apkgPath: "merged.apkg" });
 expect(run.invoke).toHaveBeenCalledTimes(3);
 expect(run.invoke).toHaveBeenNthCalledWith(1, "flashcard_generate", { config: expect.objectContaining({ output_dir: "/out/episodes/episode-1", audio_track_index: 0 }) });
 expect(run.maybeAutoImportToAnki).toHaveBeenCalledExactlyOnceWith("merged.apkg");
 expect(run.generationStore.isProcessing).toBe(false); expect(run.setEpisodeProgress).toHaveBeenLastCalledWith(0, 0);
});
it("freezes output choices for later episodes", async () => {
 const run = options(); run.previewStore.applyOverrides = async () => { run.generationStore.seriesOutputMode = "separate"; run.generationStore.deckName = "Changed"; };
 await runFlashcardSeries(run);
 expect(run.invoke).toHaveBeenNthCalledWith(2, "flashcard_generate", { config: expect.objectContaining({ output_dir: "/out/episodes/episode-2", deck_name: "Deck" }) });
 expect(run.invoke).toHaveBeenLastCalledWith("flashcard_merge_apkg", expect.objectContaining({ outputPath: "/out/Deck.apkg" }));
});
it("does not call IPC when cancelled during preparation", async () => {
 const run = options(); let cancelled = false; run.isCancelled = () => cancelled;
 run.previewStore.applyOverrides = async () => { cancelled = true; }; await runFlashcardSeries(run);
 expect(run.invoke).not.toHaveBeenCalled(); expect(run.maybeAutoImportToAnki).not.toHaveBeenCalled();
 expect(run.generationStore.result?.success).toBe(false); expect(run.generationStore.isProcessing).toBe(false);
});
it("does not merge or import after cancellation during native generation", async () => {
 const run = options(); let cancelled = false; run.isCancelled = () => cancelled;
 run.invoke = vi.fn().mockImplementation(async () => { cancelled = true; return response; }); await runFlashcardSeries(run);
 expect(run.invoke).toHaveBeenCalledOnce(); expect(run.maybeAutoImportToAnki).not.toHaveBeenCalled(); expect(run.generationStore.result?.success).toBe(false);
});
it("exposes merge errors and preserves completed output", async () => {
 const run = options(); run.invoke = vi.fn().mockImplementation(async command => { if (command === "flashcard_merge_apkg") throw new Error("disk full"); return response; });
 await runFlashcardSeries(run);
 expect(run.generationStore.result).toMatchObject({ success: false, cardsGenerated: 6, apkgPath: "episode.apkg", message: expect.stringContaining("disk full") });
 expect(run.maybeAutoImportToAnki).not.toHaveBeenCalled();
});
it("continues after an episode failure and cleans up after a preparation error", async () => {
 const run = options(); run.invoke = vi.fn().mockRejectedValueOnce(new Error("bad subtitle")).mockResolvedValue(response);
 await runFlashcardSeries(run); expect(run.generationStore.result).toMatchObject({ success: false, cardsGenerated: 3 });
 const failed = options(); failed.pickAudioTrackIndexForEpisode = async () => { throw new Error("probe failed"); };
 await runFlashcardSeries(failed); expect(failed.generationStore.error).toContain("probe failed"); expect(failed.generationStore.isProcessing).toBe(false);
});

function singleOptions(): FlashcardGenerationOptions {
 const series = options();
 return { generationStore: series.generationStore, config: {} as FlashcardGenerationOptions["config"], t: series.t, invoke: series.invoke, applyOverrides: series.previewStore.applyOverrides, isCancelled: series.isCancelled, importToAnki: series.maybeAutoImportToAnki };
}
it("publishes a single-run result and imports its package", async () => {
 const run = singleOptions(); await runFlashcardGeneration(run);
 expect(run.generationStore.result).toMatchObject({ success: true, cardsGenerated: 3, outputSizeBytes: 100 });
 expect(run.invoke).toHaveBeenCalledExactlyOnceWith("flashcard_generate", { config: run.config });
 expect(run.importToAnki).toHaveBeenCalledExactlyOnceWith("episode.apkg"); expect(run.generationStore.isProcessing).toBe(false);
});
it("ignores late results and skips native work if single-run preparation was cancelled", async () => {
 for (const duringPreparation of [true, false]) {
 const run = singleOptions(); let cancelled = false; run.isCancelled = () => cancelled;
 if (duringPreparation) run.applyOverrides = async () => { cancelled = true; };
 else run.invoke = vi.fn().mockImplementation(async () => { cancelled = true; return response; });
 await runFlashcardGeneration(run);
 if (duringPreparation) expect(run.invoke).not.toHaveBeenCalled();
 expect(run.generationStore.result).toBeNull(); expect(run.importToAnki).not.toHaveBeenCalled(); expect(run.generationStore.isProcessing).toBe(false);
 }
});
it("reports single-run errors and clears transient progress in finally", async () => {
 const run = singleOptions(); run.applyOverrides = async () => { throw new Error("preview failed"); };
 await runFlashcardGeneration(run);
 expect(run.generationStore.error).toContain("preview failed"); expect(run.generationStore.result?.success).toBe(false);
 expect(run.generationStore.isProcessing).toBe(false); expect(run.generationStore.progress).toBe(0);
});

it("exports mixed-language episodes with their own note types and font language", async () => {
 const run = options(); run.automaticNoteType = true;
 run.episodes[0].targetSubsPath = "episode.ja.srt";
 run.episodes[1].targetSubsPath = "episode.zh-Hant.srt";
 await runFlashcardSeries(run);
 expect(run.invoke).toHaveBeenNthCalledWith(1, "flashcard_generate", { config: expect.objectContaining({ note_type_name: "Japanese_Vesta", target_language: "ja" }) });
 expect(run.invoke).toHaveBeenNthCalledWith(2, "flashcard_generate", { config: expect.objectContaining({ note_type_name: "ChineseTraditional_Vesta", target_language: "zh-tw" }) });
});
