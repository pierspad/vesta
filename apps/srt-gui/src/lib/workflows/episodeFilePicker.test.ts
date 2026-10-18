import { beforeEach, expect, it, vi } from "vitest";
vi.mock("$lib/utils/dialogGuard", () => ({ guardedOpen: vi.fn() }));
import { guardedOpen } from "$lib/utils/dialogGuard";
import { pickEpisodeFile } from "./episodeFilePicker";
import type { EpisodeEntry } from "$lib/utils/seriesFileMatching";
const original: EpisodeEntry = { id: 1, targetSubsPath: "/original.srt", nativeSubsPath: "/old.it.srt", mediaPath: "/clip.mp4", mediaType: "video" };
beforeEach(() => vi.mocked(guardedOpen).mockReset());
it("opens only the subtitle chooser and replaces only the requested reference", async () => {
  vi.mocked(guardedOpen).mockResolvedValue("/new.it.srt");
  const replacement = await pickEpisodeFile(original, "nativeSubsPath");
  expect(guardedOpen).toHaveBeenCalledOnce();
  expect(guardedOpen).toHaveBeenCalledWith(expect.objectContaining({
    multiple: false, defaultPath: "/old.it.srt",
    filters: [expect.objectContaining({ extensions: ["srt", "ass", "ssa", "vtt"] })],
  }));
  expect(replacement).toEqual({ ...original, nativeSubsPath: "/new.it.srt" });
  expect(original.nativeSubsPath).toBe("/old.it.srt");
});
it("leaves the episode unchanged on cancellation", async () => {
  vi.mocked(guardedOpen).mockResolvedValue(null);
  expect(await pickEpisodeFile(original, "mediaPath")).toBeNull();
  expect(original.mediaPath).toBe("/clip.mp4");
});
it("updates media type when the media is replaced with audio", async () => {
  vi.mocked(guardedOpen).mockResolvedValue("/sound.opus");
  expect(await pickEpisodeFile(original, "mediaPath")).toEqual({ ...original, mediaPath: "/sound.opus", mediaType: "audio" });
});
