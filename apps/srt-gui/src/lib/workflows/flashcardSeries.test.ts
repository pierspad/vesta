import { expect, it, vi } from "vitest";
import { runFlashcardSeries, type FlashcardSeriesOptions } from "./flashcardSeries";
import { runFlashcardGeneration, type FlashcardGenerationOptions } from "./flashcardGeneration";
import { defaultMediaSettings } from "$lib/utils/mediaSettings";
import { inferLanguageFromPath } from "$lib/utils/models";
import { predefinedNoteTypeForLanguage } from "$lib/types/noteTypes";
const response = { success: true, message: "ok", cards_generated: 3, audio_clips: 3, snapshots: 3, video_clips: 0, apkg_path: "episode.apkg", tsv_path: null, output_size_bytes: 100 };
function options(): FlashcardSeriesOptions {
 return {
 generationStore: { deckName: "Deck", error: null, result: null, progress: 0, progressMessage: "", progressStage: "", isProcessing: false, seriesOutputMode: "single", effectiveExportFormat: "apkg", effectiveCpuCores: 2, exportFormat: "anki", addLog: vi.fn() },
 episodes: [1, 2].map(index => ({ id: index, targetSubsPath: `episode${index}.srt`, nativeSubsPath: "", mediaPath: `episode${index}.mkv`, mediaType: "video" as const })),
 t: key => key, outputDir: "/out",
 cardFilters: { enabled: false, minChars: 3, minCharsEnabled: false, maxChars: 100, maxCharsEnabled: false, minDurationMs: 500, minDurationEnabled: false, maxDurationMs: 15000, maxDurationEnabled: false, combineSentences: false, continuationChars: "..." },
 videoHwAccel: "auto", activeNoteType: predefinedNoteTypeForLanguage(""),
 ankiStore: { autoCardFont: false, embedCardFont: false }, previewStore: { applyOverrides: vi.fn().mockResolvedValue(undefined) },
 getEpisodeMediaSettings: () => defaultMediaSettings, getAudioTrackLanguageForEpisode: () => "en", pickAudioTrackIndexForEpisode: vi.fn().mockResolvedValue(0), getStudiedLanguagePreference: () => "en",
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
 expect(run.invoke).toHaveBeenNthCalledWith(2, "flashcard_generate", { config: expect.objectContaining({ output_dir: "/out/episodes/episode-2", deck_name: "episode1" }) });
 expect(run.invoke).toHaveBeenLastCalledWith("flashcard_merge_apkg", expect.objectContaining({ outputPath: "/out/episode1.apkg" }));
});
it("freezes note type and preferred audio language across a multi-episode run", async () => {
 const run = options();
 run.activeNoteType = {
  ...predefinedNoteTypeForLanguage(""),
  id: "custom:original", name: "Original note type", language: "it",
  fields: { ...predefinedNoteTypeForLanguage("").fields, expression: "Original expression" },
 };
 run.automaticNoteType = false;
 let preferredLanguage = "it";
 run.getAudioTrackLanguageForEpisode = () => preferredLanguage;
 run.invoke = vi.fn().mockImplementation(async (command, args) => {
  if (command === "flashcard_generate" && (args?.config as { episode_number: number }).episode_number === 1) {
   preferredLanguage = "ja";
   run.activeNoteType.name = "Changed note type";
   run.activeNoteType.fields.expression = "Changed expression";
  }
  return command === "flashcard_merge_apkg" ? "merged.apkg" : response;
 });

 await runFlashcardSeries(run);

 expect(run.pickAudioTrackIndexForEpisode).toHaveBeenNthCalledWith(1, expect.objectContaining({ id: 1 }), "it");
 expect(run.pickAudioTrackIndexForEpisode).toHaveBeenNthCalledWith(2, expect.objectContaining({ id: 2 }), "it");
 expect(run.invoke).toHaveBeenNthCalledWith(2, "flashcard_generate", {
  config: expect.objectContaining({ note_type_name: "Original note type", field_names: expect.objectContaining({ expression: "Original expression" }) }),
 });
});
it("does not call IPC when cancelled during preparation", async () => {
 const run = options(); let cancelled = false; run.isCancelled = () => cancelled;
 run.previewStore.applyOverrides = async () => { cancelled = true; }; await runFlashcardSeries(run);
 expect(run.invoke).not.toHaveBeenCalled(); expect(run.maybeAutoImportToAnki).not.toHaveBeenCalled();
 expect(run.generationStore.result?.success).toBe(false); expect(run.generationStore.isProcessing).toBe(false);
});
it("stops after cancellation during audio-track preparation without applying preview overrides", async () => {
 const run = options(); let cancelled = false; let resolveTrack!: (index: number | null) => void;
 run.isCancelled = () => cancelled;
 run.pickAudioTrackIndexForEpisode = vi.fn(() => new Promise<number | null>(resolve => { resolveTrack = resolve; }));
 const pending = runFlashcardSeries(run);
 await Promise.resolve(); cancelled = true; resolveTrack(1); await pending;
 expect(run.previewStore.applyOverrides).not.toHaveBeenCalled(); expect(run.invoke).not.toHaveBeenCalled();
 expect(run.generationStore.result).toMatchObject({ success: false, message: "flashcards.cancelled" });
});
it("does not merge or import after cancellation during native generation", async () => {
 const run = options(); let cancelled = false; run.isCancelled = () => cancelled;
 run.invoke = vi.fn().mockImplementation(async () => { cancelled = true; return response; }); await runFlashcardSeries(run);
 expect(run.invoke).toHaveBeenCalledOnce(); expect(run.maybeAutoImportToAnki).not.toHaveBeenCalled();
 expect(run.generationStore.result).toMatchObject({ success: false, cardsGenerated: 3, apkgPath: "episode.apkg", message: "flashcards.cancelled" });
});
it("exposes merge errors and preserves completed output", async () => {
 const run = options(); run.invoke = vi.fn().mockImplementation(async command => { if (command === "flashcard_merge_apkg") throw new Error("disk full"); return response; });
 await runFlashcardSeries(run);
 expect(run.generationStore.result).toMatchObject({ success: false, cardsGenerated: 6, apkgPath: "episode.apkg", message: expect.stringContaining("disk full") });
 expect(run.maybeAutoImportToAnki).not.toHaveBeenCalled();
});
it("keeps a successful series result when automatic Anki import fails", async () => {
 const run = options(); run.maybeAutoImportToAnki = vi.fn().mockRejectedValue(new Error("Anki unavailable"));
 await runFlashcardSeries(run);
 expect(run.generationStore.result).toMatchObject({ success: true, cardsGenerated: 6, apkgPath: "merged.apkg" });
 expect(run.generationStore.error).toBeNull();
 expect(run.generationStore.addLog).toHaveBeenCalledWith(expect.stringContaining("ankiAutoImportFailed"), "warning");
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
it("preserves the generated result when automatic Anki import fails", async () => {
 const run = singleOptions(); run.importToAnki = vi.fn().mockRejectedValue(new Error("Anki unavailable"));
 await runFlashcardGeneration(run);
 expect(run.generationStore.result).toMatchObject({ success: true, cardsGenerated: 3, apkgPath: "episode.apkg" });
 expect(run.generationStore.error).toBeNull();
 expect(run.generationStore.addLog).toHaveBeenCalledWith(expect.stringContaining("ankiAutoImportFailed"), "warning");
});
it("skips native work after preparation cancellation and preserves late successful output", async () => {
 for (const duringPreparation of [true, false]) {
 const run = singleOptions(); let cancelled = false; run.isCancelled = () => cancelled;
 if (duringPreparation) run.applyOverrides = async () => { cancelled = true; };
 else run.invoke = vi.fn().mockImplementation(async () => { cancelled = true; return response; });
 await runFlashcardGeneration(run);
 if (duringPreparation) expect(run.invoke).not.toHaveBeenCalled();
 if (duringPreparation) expect(run.generationStore.result).toBeNull();
 else expect(run.generationStore.result).toMatchObject({ success: false, cardsGenerated: 3, apkgPath: "episode.apkg", message: "flashcards.cancelled" });
 expect(run.importToAnki).not.toHaveBeenCalled(); expect(run.generationStore.isProcessing).toBe(false);
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
 run.getAudioTrackLanguageForEpisode = (episode) => inferLanguageFromPath(episode.targetSubsPath) || "en";
 await runFlashcardSeries(run);
 expect(run.invoke).toHaveBeenNthCalledWith(1, "flashcard_generate", { config: expect.objectContaining({ note_type_name: "Japanese_Vesta", target_language: "ja" }) });
 expect(run.invoke).toHaveBeenNthCalledWith(2, "flashcard_generate", { config: expect.objectContaining({ note_type_name: "ChineseTraditional_Vesta", target_language: "zh-tw" }) });
 expect(run.pickAudioTrackIndexForEpisode).toHaveBeenNthCalledWith(1, expect.objectContaining({ id: 1 }), "ja");
 expect(run.pickAudioTrackIndexForEpisode).toHaveBeenNthCalledWith(2, expect.objectContaining({ id: 2 }), "zh-tw");
});

it("ignores the saved single-APKG choice for TSV output", async () => {
 const run = options();
 run.generationStore = { ...run.generationStore, effectiveExportFormat: "tsv", exportFormat: "tsv" };
 run.invoke = vi.fn().mockResolvedValue({ ...response, apkg_path: null, tsv_path: "episode.tsv" });
 await runFlashcardSeries(run);
 expect(run.invoke).toHaveBeenCalledTimes(2);
 expect(run.invoke).toHaveBeenNthCalledWith(1, "flashcard_generate", { config: expect.objectContaining({ output_dir: "/out", export_format: "tsv" }) });
 expect(run.generationStore.result?.tsvPath).toBe("episode.tsv");
 expect(run.invoke).not.toHaveBeenCalledWith("flashcard_merge_apkg", expect.anything());
 expect(run.maybeAutoImportToAnki).not.toHaveBeenCalled();
});

it("derives a merged package name afresh instead of requiring or reusing a manual name", async () => {
 const run = options();
 run.generationStore.deckName = "Old custom name";
 run.episodes = [{ ...run.episodes[0], mediaPath: "/media/Detour.mp4" }];
 await runFlashcardSeries(run);
 expect(run.generationStore.deckName).toBe("Detour");
 expect(run.invoke).toHaveBeenCalledWith("flashcard_generate", { config: expect.objectContaining({ deck_name: "Detour" }) });
});
