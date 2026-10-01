import { describe, expect, it } from "vitest";
import { filterSubtitleTracks, SUBTITLE_PAGE_SIZE } from "./subtitleTracks";
import { languageFlagUrl } from "$lib/config/languages";
const track = (index: number, language: string, title = "") => ({ index, language, title, codec: "subrip", text_based: true });
describe("subtitle language listing", () => {
  it("maps language codes to country flags", () => {
    expect(["ar", "ja", "hi", "ko"].map(languageFlagUrl)).toEqual(["/flags/sa.svg", "/flags/jp.svg", "/flags/in.svg", "/flags/kr.svg"]);
  });
  it("sorts ISO aliases by language and keeps tracks distinct", () => {
    expect(filterSubtitleTracks([track(8, "ita"), track(2, "eng"), track(3, "ita")], "").map(t => t.index)).toEqual([2, 3, 8]);
  });
  it("puts downloadable tracks first and keeps localized alphabetical order within both groups", () => {
    const tracks = [
      { ...track(1, "ara"), codec: "hdmv_pgs_subtitle", text_based: false },
      track(8, "ita"),
      { ...track(5, "eng"), codec: "dvd_subtitle", text_based: false },
      track(3, "ita"),
      track(2, "eng"),
    ];
    expect(filterSubtitleTracks(tracks, "", "it").map(t => t.index)).toEqual([2, 3, 8, 1, 5]);
    expect(filterSubtitleTracks(tracks, "inglese", "it").map(t => t.index)).toEqual([2, 5]);
  });
  it("searches native and Italian aliases and falls back to titles", () => {
    const tracks = [track(1, "jpn"), track(2, "und", "Italian commentary"), track(3, "eng", "Italian commentary")];
    expect(filterSubtitleTracks(tracks, "giapponese").map(t => t.index)).toEqual([1]);
    expect(filterSubtitleTracks(tracks, "日本語").map(t => t.index)).toEqual([1]);
    expect(filterSubtitleTracks(tracks, "").find(t => t.index === 3)?.code).toBe("it");
    expect(filterSubtitleTracks(tracks, "").find(t => t.index === 2)?.code).toBe("it");
    expect(SUBTITLE_PAGE_SIZE).toBe(10);
  });
});

it("reuses the prepared ordering and searches without rebuilding language metadata", async () => {
  const { prepareSubtitleTracks, filterPreparedSubtitleTracks } = await import("./subtitleTracks");
  const prepared = prepareSubtitleTracks([track(1, "fra", "Français SDH"), track(2, "ita"), track(3, "eng")]);
  expect(filterPreparedSubtitleTracks(prepared, "")).toBe(prepared);
  expect(filterPreparedSubtitleTracks(prepared, "francais sdh").map(t => t.index)).toEqual([1]);
  expect(filterPreparedSubtitleTracks(prepared, "italiano")[0]).toBe(prepared.find(t => t.index === 2));
});

it("suggests portable movie names with a two-letter language suffix", async () => {
  const { subtitleOutputName } = await import("./subtitleTracks");
  expect(subtitleOutputName("/films/La finestra sul cortile.mkv", track(24, "ita"))).toBe("La_finestra_sul_cortile_it.srt");
  expect(subtitleOutputName("C:\\Films\\Rear Window.mkv", track(8, "eng"))).toBe("Rear_Window_en.srt");
  expect(subtitleOutputName("movie.mp4", track(4, "pt-br"))).toBe("movie_pt.srt");
  expect(subtitleOutputName("movie.mkv", track(9, "und"))).toBe("movie_xx.srt");
});

it("exposes conflicting container metadata and uses explicit title language for grouping", () => {
  const result = filterSubtitleTracks([track(30, "eng", "Korean"), track(37, "eng", "Turkish"), track(24, "ita", "Italian")], "");
  expect(result.find(t => t.index === 30)).toMatchObject({ code: "ko", languageConflict: true });
  expect(result.find(t => t.index === 37)).toMatchObject({ code: "tr", languageConflict: true });
  expect(result.find(t => t.index === 24)?.languageConflict).toBe(false);
});

it("keeps exact ISO aliases and fuzzy matching consistent", async () => {
  const { detectLanguageCode } = await import("$lib/config/languages");
  expect(["eng", "ita", "jpn", "por", "chi", "Français", "日本語", "pt-br"].map(detectLanguageCode)).toEqual(["en", "it", "ja", "pt", "zh", "fr", "ja", "pt-br"]);
  expect(detectLanguageCode("Italian commentary")).toBe("it");
});

it("displays and searches language names in the UI language", () => {
  const prepared = filterSubtitleTracks([track(1, "eng"), track(2, "ita")], "inglese", "it");
  expect(prepared).toHaveLength(1);
  expect(prepared[0].languageName).toBe("inglese");
  expect(filterSubtitleTracks([track(1, "eng")], "anglais", "fr")[0].languageName).toBe("anglais");
});

it("groups variants by language, prefers ordinary text, and sorts actionable languages first", async () => {
  const { prepareSubtitleTracks, groupSubtitleTracks } = await import("./subtitleTracks");
  const tracks = [track(37, "eng", "Turkish"), track(9, "eng", "English SDH"), track(8, "eng", "English SRT"), track(11, "eng", "English Commentary"), { ...track(1, "ara"), text_based: false }, track(3, "fra")];
  const groups = groupSubtitleTracks(prepareSubtitleTracks(tracks, "it"), "it");
  expect(groups.map(group => group.code)).toEqual(["fr", "en", "tr", "ar"]);
  expect(groups.find(group => group.code === "en")?.tracks.map(track => track.index)).toEqual([8, 9, 11]);
  expect(groups.find(group => group.code === "tr")?.tracks[0].languageConflict).toBe(true);
});
