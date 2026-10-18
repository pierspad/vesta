import { expect, it, vi } from "vitest";
import { expandFlashcardFiles } from "./flashcardFileDiscovery";
it("limits concurrent discovery, deduplicates and preserves order despite different completion times", async () => {
 let active = 0, maximum = 0;
 const invoke = vi.fn().mockImplementation(async (command, args) => {
   active++; maximum = Math.max(maximum, active);
   await new Promise(resolve => setTimeout(resolve, String(args.mediaPath || args.srtPath).includes("0") ? 3 : 0));
   active--;
   if (command === "sync_suggest_subtitles_for_media") return { target: `${args.mediaPath}.srt`, native: "shared.srt" };
   if (command === "sync_suggest_companion_subtitle_for_srt") return "shared.srt";
   return null;
 });
 const media = Array.from({ length: 12 }, (_, i) => `movie${i}`);
 const result = await expandFlashcardFiles([], media, { enabled: true, targetLanguage: "en", nativeLanguage: "it", invoke });
 expect(maximum).toBe(8); expect(result.mediaFiles).toEqual(media);
 expect(result.subtitleFiles).toEqual(["movie0.srt", "shared.srt", ...media.slice(1).map(path => `${path}.srt`)]);
});
it("keeps the successful companion suggestion when the independent media lookup fails", async () => {
 const invoke = vi.fn(async <T>(command: string): Promise<T> => {
   if (command === "sync_suggest_companion_subtitle_for_srt") return "film.native.srt" as T;
   if (command === "sync_suggest_media_for_srt") throw new Error("media lookup unavailable");
   return null as T;
 }) as unknown as <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
 expect(await expandFlashcardFiles(["film.srt"], [], { enabled: true, targetLanguage: "en", nativeLanguage: "it", invoke }))
   .toEqual({ subtitleFiles: ["film.srt", "film.native.srt"], mediaFiles: [] });
});
it("keeps supplied files on discovery errors and makes no calls when disabled", async () => {
 const invoke = vi.fn().mockRejectedValue(new Error("offline"));
 const options = { enabled: true, targetLanguage: "", nativeLanguage: "", invoke };
 expect(await expandFlashcardFiles(["film.srt"], ["film.mkv"], options)).toEqual({ subtitleFiles: ["film.srt"], mediaFiles: ["film.mkv"] });
 invoke.mockClear(); await expandFlashcardFiles(["film.srt"], [], { ...options, enabled: false }); expect(invoke).not.toHaveBeenCalled();
});

it("filters backend suggestions with the actual custom matching rules", async () => {
 const { DEFAULT_SMART_MATCHING_RULES: rules } = await import("$lib/stores/smartMatchingStore.svelte");
 const invoke = vi.fn(async <T>(command: string): Promise<T> => {
   if (command === "sync_suggest_subtitles_for_media") return { target: "Show.S01E01.en.srt", native: "Show.S02E01.it.srt" } as T;
   if (command === "sync_suggest_companion_subtitle_for_srt") return "Unrelated.S01E01.it.srt" as T;
   return null as T;
 }) as unknown as <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
 const result = await expandFlashcardFiles([], ["Show.S01E01.mp4"], { enabled: true, rules, targetLanguage: "en", nativeLanguage: "it", invoke });
 expect(result.subtitleFiles).toEqual(["Show.S01E01.en.srt"]);
});
