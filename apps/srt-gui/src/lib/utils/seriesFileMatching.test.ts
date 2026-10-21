import { describe, expect, it } from "vitest";
import { DEFAULT_SMART_MATCHING_RULES, type SmartMatchingRules } from "$lib/stores/smartMatchingStore.svelte";
import type { AudioTrackInfo } from "$lib/types/flashcardMediaTypes";
import {
  autoMatchFiles,
  classifySubtitleCandidates,
  classifySubtitles,
  detectMediaType,
  deriveDeckNameFromFile,
  escapeRegExp,
  extractEpisodeNumber,
  getFileExtension,
  isMediaFile,
  isSubtitleFile,
  mergeSeriesDroppedFiles,
  mergeSeriesMediaFiles,
  mergeSeriesSubtitleFiles,
  normalizeSeriesBaseKey,
  parseSeriesMedia,
  parseSeriesSubtitle,
  pickBestAudioTrackIndex,
  scoreAudioTrackForLanguage,
} from "./seriesFileMatching";

const rules: SmartMatchingRules = DEFAULT_SMART_MATCHING_RULES;

describe("escapeRegExp", () => {
  it("escapes regex metacharacters", () => {
    expect(escapeRegExp("a.b*c?")).toBe("a\\.b\\*c\\?");
  });
});

describe("getFileExtension / isSubtitleFile / isMediaFile / detectMediaType", () => {
  it("extracts lowercase extension", () => {
    expect(getFileExtension("/path/to/File.SRT")).toBe("srt");
    expect(getFileExtension("noext")).toBe("noext"); // no dot -> whole string, matches current impl
  });

  it("classifies subtitle vs media files", () => {
    expect(isSubtitleFile("show.srt")).toBe(true);
    expect(isSubtitleFile("show.ass")).toBe(true);
    expect(isSubtitleFile("show.mp4")).toBe(false);
    expect(isMediaFile("show.mp4")).toBe(true);
    expect(isMediaFile("show.mp3")).toBe(true);
    expect(isMediaFile("show.srt")).toBe(false);
  });

  it("detects media type from extension", () => {
    expect(detectMediaType("clip.mkv")).toBe("video");
    expect(detectMediaType("clip.flac")).toBe("audio");
    expect(detectMediaType("clip.txt")).toBe("none");
  });
});

describe("extractEpisodeNumber", () => {
  it("finds SxxExx patterns", () => {
    expect(extractEpisodeNumber("Show.S01E07.mkv", rules.episodeRegexes)).toBe(7);
  });

  it("finds standalone episode markers", () => {
    expect(extractEpisodeNumber("Show - Ep. 12.srt", rules.episodeRegexes)).toBe(12);
    expect(extractEpisodeNumber("Show.Episode.03.srt", rules.episodeRegexes)).toBe(3);
    expect(extractEpisodeNumber("Show 5x09.mkv", rules.episodeRegexes)).toBe(9);
  });

  it("finds multi-lingual and international episode formats", () => {
    expect(extractEpisodeNumber("Anime_Show_第03话.mkv", rules.episodeRegexes)).toBe(3);
    expect(extractEpisodeNumber("Chinese_Drama_第12集.mp4", rules.episodeRegexes)).toBe(12);
    expect(extractEpisodeNumber("K-Drama.07화.mkv", rules.episodeRegexes)).toBe(7);
    expect(extractEpisodeNumber("Serie_Capitulo_15.srt", rules.episodeRegexes)).toBe(15);
    expect(extractEpisodeNumber("Serie_Capítulo_05.srt", rules.episodeRegexes)).toBe(5);
    expect(extractEpisodeNumber("Serie.Épisode.08.srt", rules.episodeRegexes)).toBe(8);
    expect(extractEpisodeNumber("Deutsche.Serie.Folge.06.mkv", rules.episodeRegexes)).toBe(6);
    expect(extractEpisodeNumber("Russian.Show.Серия.09.mkv", rules.episodeRegexes)).toBe(9);
    expect(extractEpisodeNumber("Anime.OVA.02.mkv", rules.episodeRegexes)).toBe(2);
    expect(extractEpisodeNumber("[SubGroup] Show Name - 04 [1080p].mkv", rules.episodeRegexes)).toBe(4);
    expect(extractEpisodeNumber("[Erai-raws] Jujutsu Kaisen - 23v2 [720p].mkv", rules.episodeRegexes)).toBe(23);
    expect(extractEpisodeNumber("[Fansub][12][720p].mkv", rules.episodeRegexes)).toBe(12);
    expect(extractEpisodeNumber("Show.Name.#05.mkv", rules.episodeRegexes)).toBe(5);
    expect(extractEpisodeNumber("Show.S01E01-E02.mkv", rules.episodeRegexes)).toBe(1);
  });

  it("finds a delimited bare number as last resort", () => {
    // Bare numbers only match with a delimiter (space/./-/_) on the
    // matching side — an undelimited "04" alone is deliberately too
    // ambiguous (could be a year fragment, a resolution, etc.) and is
    // correctly left unmatched.
    expect(extractEpisodeNumber("04.mkv", rules.episodeRegexes)).toBeNull();
    expect(extractEpisodeNumber("Show - 04.mkv", rules.episodeRegexes)).toBe(4);
    expect(extractEpisodeNumber("04 - Show.mkv", rules.episodeRegexes)).toBe(4);
  });

  it("returns null when no pattern matches", () => {
    expect(extractEpisodeNumber("Movie.mkv", rules.episodeRegexes)).toBeNull();
  });

  it("ignores invalid custom regexes instead of throwing", () => {
    expect(extractEpisodeNumber("Show.E01.mkv", ["(unterminated"])).toBeNull();
  });
});

describe("normalizeSeriesBaseKey", () => {
  it("strips year, resolution/codec tags and language codes into a stable key", () => {
    const a = normalizeSeriesBaseKey("The.Show.2019.1080p.BluRay.x264.it", rules);
    const b = normalizeSeriesBaseKey("The.Show.2019.720p.WEBRip.x265.en", rules);
    expect(a).toBe(b);
    expect(a).not.toContain("1080p");
    expect(a).not.toContain("bluray");
  });

  it("collapses original/reference role hints so both sides of a pair share a key", () => {
    const original = normalizeSeriesBaseKey("Show.S01E01.original", rules);
    const reference = normalizeSeriesBaseKey("Show.S01E01.translated", rules);
    expect(original).toBe(reference);
  });
});

describe("parseSeriesSubtitle / parseSeriesMedia", () => {
  it("detects role hint from filename keywords", () => {
    const original = parseSeriesSubtitle("/x/Show.S01E01.original.srt", rules);
    expect(original.roleHint).toBe("original");
    const reference = parseSeriesSubtitle("/x/Show.S01E01.translated.srt", rules);
    expect(reference.roleHint).toBe("reference");
    const unknown = parseSeriesSubtitle("/x/Show.S01E01.srt", rules);
    expect(unknown.roleHint).toBe("unknown");
  });

  it("extracts episode number and language from subtitle path", () => {
    const parsed = parseSeriesSubtitle("/x/Show.S02E05.it.srt", rules);
    expect(parsed.episodeNumber).toBe(5);
    expect(parsed.language).toBe("it");
  });

  it("gives matching subtitle/media pairs the same baseKey", () => {
    const sub = parseSeriesSubtitle("/x/Show.S01E03.srt", rules);
    const media = parseSeriesMedia("/x/Show.S01E03.mkv", rules);
    expect(sub.baseKey).toBe(media.baseKey);
  });
});

describe("classifySubtitleCandidates / classifySubtitles", () => {
  it("assigns the single file to target by default, to native when requested", () => {
    expect(classifySubtitleCandidates(["/x/a.srt"], "auto", rules, "", "")).toEqual({
      target: "/x/a.srt",
      native: "",
    });
    expect(classifySubtitleCandidates(["/x/a.srt"], "native", rules, "", "")).toEqual({
      target: "",
      native: "/x/a.srt",
    });
  });

  it("uses studied/native language codes to split a pair", () => {
    const result = classifySubtitles(["/x/Show.en.srt", "/x/Show.it.srt"], rules, "en", "it");
    expect(result.target).toBe("/x/Show.en.srt");
    expect(result.native).toBe("/x/Show.it.srt");
  });

  it("prefers the 'original' role hint over language guessing when ambiguous", () => {
    const result = classifySubtitles(["/x/Show.original.srt", "/x/Show.translated.srt"], rules, "", "");
    expect(result.target).toBe("/x/Show.original.srt");
    expect(result.native).toBe("/x/Show.translated.srt");
  });

  it("returns empty target/native for an empty input", () => {
    expect(classifySubtitleCandidates([], "auto", rules, "", "")).toEqual({ target: "", native: "" });
  });
});

describe("mergeSeriesSubtitleFiles", () => {
  it("groups target+native subtitle pairs into one episode by baseKey", () => {
    const episodes = mergeSeriesSubtitleFiles(
      [],
      ["/x/Show.S01E01.en.srt", "/x/Show.S01E01.it.srt"],
      "auto",
      rules,
      "en",
      "it",
    );
    expect(episodes).toHaveLength(1);
    expect(episodes[0].targetSubsPath).toBe("/x/Show.S01E01.en.srt");
    expect(episodes[0].nativeSubsPath).toBe("/x/Show.S01E01.it.srt");
  });

  it("creates separate episodes for separate episode numbers", () => {
    const episodes = mergeSeriesSubtitleFiles(
      [],
      ["/x/Show.S01E01.srt", "/x/Show.S01E02.srt"],
      "auto",
      rules,
      "",
      "",
    );
    expect(episodes).toHaveLength(2);
    expect(episodes.map((e) => e.targetSubsPath).sort()).toEqual([
      "/x/Show.S01E01.srt",
      "/x/Show.S01E02.srt",
    ]);
  });

  it("is order-independent: dropping native-then-target yields the same pairing as target-then-native", () => {
    const a = mergeSeriesSubtitleFiles([], ["/x/Show.S01E01.en.srt", "/x/Show.S01E01.it.srt"], "auto", rules, "en", "it");
    const b = mergeSeriesSubtitleFiles([], ["/x/Show.S01E01.it.srt", "/x/Show.S01E01.en.srt"], "auto", rules, "en", "it");
    expect(a[0].targetSubsPath).toBe(b[0].targetSubsPath);
    expect(a[0].nativeSubsPath).toBe(b[0].nativeSubsPath);
  });
});

describe("mergeSeriesMediaFiles", () => {
  it("attaches media to an existing episode sharing its baseKey", () => {
    const withSubs = mergeSeriesSubtitleFiles([], ["/x/Show.S01E01.srt"], "auto", rules, "", "");
    const withMedia = mergeSeriesMediaFiles(withSubs, ["/x/Show.S01E01.mkv"], rules);
    expect(withMedia).toHaveLength(1);
    expect(withMedia[0].mediaPath).toBe("/x/Show.S01E01.mkv");
    expect(withMedia[0].mediaType).toBe("video");
    expect(withMedia[0].targetSubsPath).toBe("/x/Show.S01E01.srt");
  });

  it("does not pair unrelated titles merely because their episode numbers agree", () => {
    const withSubs = mergeSeriesSubtitleFiles([], ["/x/My Show - Episode 01.srt"], "auto", rules, "", "");
    const withMedia = mergeSeriesMediaFiles(withSubs, ["/x/Completely Different Name E01.mkv"], rules);
    expect(withMedia).toHaveLength(2);
    expect(withMedia.find(e => e.targetSubsPath)?.mediaPath).toBe("");
  });
});

describe("mergeSeriesDroppedFiles", () => {
  it("merges a mixed drop of subtitles and media into matched episodes", () => {
    const episodes = mergeSeriesDroppedFiles(
      [],
      ["/x/Show.S01E01.en.srt", "/x/Show.S01E01.it.srt", "/x/Show.S01E02.en.srt"],
      ["/x/Show.S01E01.mkv", "/x/Show.S01E02.mkv"],
      rules,
      "en",
      "it",
    );
    expect(episodes).toHaveLength(2);
    const ep1 = episodes.find((e) => e.mediaPath.includes("E01"));
    expect(ep1?.targetSubsPath).toContain("E01.en");
    expect(ep1?.nativeSubsPath).toContain("E01.it");
  });
});

describe("autoMatchFiles", () => {
  it("pairs target/native/media by episode number when all targets carry one", () => {
    const episodes = autoMatchFiles(
      ["/x/Show.E02.srt", "/x/Show.E01.srt"],
      ["/x/Show.E01.it.srt", "/x/Show.E02.it.srt"],
      ["/x/Show.E01.mkv", "/x/Show.E02.mkv"],
      rules.episodeRegexes,
    );
    expect(episodes).toHaveLength(2);
    const ep1 = episodes.find((e) => e.targetSubsPath.includes("E01"));
    expect(ep1?.nativeSubsPath).toContain("E01");
    expect(ep1?.mediaPath).toContain("E01");
    expect(ep1?.mediaType).toBe("video");
  });

  it("matches normalized names when targets lack episode numbers", () => {
    const episodes = autoMatchFiles(["/x/Alpha.srt", "/x/Beta.srt"], ["/x/Alpha.native.srt", "/x/Beta.native.srt"], [], rules.episodeRegexes);
    expect(episodes).toHaveLength(2);
    expect(episodes[0].targetSubsPath).toBe("/x/Alpha.srt");
    expect(episodes[0].nativeSubsPath).toBe("/x/Alpha.native.srt");
  });
});

const audioTrack = (index: number, language: string, title: string): AudioTrackInfo => ({
  index,
  stream_index: index,
  codec: null,
  language,
  title,
  channels: null,
});

describe("scoreAudioTrackForLanguage / pickBestAudioTrackIndex", () => {
  it("scores 0 for an empty language code", () => {
    expect(scoreAudioTrackForLanguage(audioTrack(0, "eng", ""), "")).toBe(0);
  });

  it("prefers the track whose language matches the requested code", () => {
    const tracks = [audioTrack(0, "jpn", "Japanese"), audioTrack(1, "eng", "English")];
    expect(pickBestAudioTrackIndex(tracks, "en")).toBe(1);
  });

  it("returns null when there's only one track (nothing to pick)", () => {
    expect(pickBestAudioTrackIndex([audioTrack(0, "eng", "")], "en")).toBeNull();
  });
});

describe("deriveDeckNameFromFile", () => {
  it("prefers the media path over the subtitle path", () => {
    const name = deriveDeckNameFromFile({ mediaPath: "/x/Detour.1945.mkv", targetSubsPath: "/x/other.srt" });
    expect(name).toBe("Detour 1945");
  });

  it("strips a trailing language suffix", () => {
    const name = deriveDeckNameFromFile({ mediaPath: "", targetSubsPath: "/x/Detour.1945.en.srt" });
    expect(name).toBe("Detour 1945");
  });
});

it("matches extracted movie tracks with their media in either import order", () => {
 const subs = ["/x/La_finestra_sul_cortile_en_8.srt", "/x/La_finestra_sul_cortile_it_24.srt"];
 const media = ["/x/La finestra sul cortile.mkv"];
 for (const initial of [[], mergeSeriesMediaFiles([], media, rules)]) {
  const result = mergeSeriesDroppedFiles(initial, subs, initial.length ? [] : media, rules, "en", "it");
  expect(result).toHaveLength(1);
  expect(result[0]).toMatchObject({targetSubsPath: subs[0], nativeSubsPath: subs[1], mediaPath: media[0]});
 }
 expect(parseSeriesSubtitle(subs[0], rules).episodeNumber).toBeNull();
});
it("preserves title numbers and strips release tags separated by spaces", () => {
 expect(normalizeSeriesBaseKey("Apollo 13_en_8", rules)).toBe("apollo_13");
 expect(normalizeSeriesBaseKey("Movie 1080p BluRay en", rules)).toBe(normalizeSeriesBaseKey("Movie.it", rules));
 expect(normalizeSeriesBaseKey("Movie_unknown_24", rules)).toBe("movie_unknown_24");
});

it("rejects conflicting seasons and ambiguous episode fallback", () => {
 const episodes = mergeSeriesSubtitleFiles([], ["/x/Show.S01E01.en.srt", "/x/Show.S02E01.en.srt"], "auto", rules, "en", "it");
 expect(mergeSeriesMediaFiles(episodes, ["/x/Other.E01.mkv"], rules)).toHaveLength(3);
 expect(mergeSeriesMediaFiles(episodes.slice(0,1), ["/x/Other.S02E01.mkv"], rules)).toHaveLength(2);
});

describe("conservative matching and recorded evidence", () => {
  it("pairs equivalent episode syntax and release tags", () => {
    const subs = mergeSeriesSubtitleFiles([], ["/x/Show.S01E02.en.srt"], "auto", rules, "en", "it");
    const result = mergeSeriesMediaFiles(subs, ["/x/Show_1x02_1080p_WEB-DL_H.264.mkv"], rules);
    expect(result).toHaveLength(1);
    expect(result[0].mediaPath).toContain("1x02");
  });

  it("never pairs different seasons by list position or reuses a media file", () => {
    const result = autoMatchFiles(
      ["/x/Show.S01E01.srt", "/x/Show.S02E01.srt"], [],
      ["/x/Show.S02E01.mkv"], rules.episodeRegexes);
    expect(result[0].mediaPath).toBe("");
    expect(result[1].mediaPath).toBe("/x/Show.S02E01.mkv");
  });

  it("leaves ambiguous media and missing episodes unpaired", () => {
    const result = autoMatchFiles(["/x/Show.E01.srt", "/x/Show.E02.srt"], [],
      ["/x/Show.E02.mp4", "/x/Show.E02.mkv"], rules.episodeRegexes);
    expect(result.every(row => !row.mediaPath)).toBe(true);
  });

  it("does not invent a match for unrelated unnumbered files", () => {
    expect(autoMatchFiles(["/x/Alpha.srt"], [], ["/x/Beta.mp4"], rules.episodeRegexes)[0].mediaPath).toBe("");
  });

  it("removes technical tokens across underscores, dots and hyphens", () => {
    expect(normalizeSeriesBaseKey("Show_E01_WEB_DL_H.264_DDP5.1_en", rules)).toBe("show_e01");
    expect(normalizeSeriesBaseKey("Show.E01.it", rules)).toBe("show_e01");
  });

  it("ignores release years and resolutions as bare episode numbers", () => {
    expect(extractEpisodeNumber("Movie.[2024].[1080].mkv", rules.episodeRegexes)).toBeNull();
    expect(extractEpisodeNumber("Show.E2024.mkv", rules.episodeRegexes)).toBe(2024);
  });

  it("uses captures from global, named and capture-free custom regexes", () => {
    expect(extractEpisodeNumber("Show.E03.mkv", [/E(\d+)/g])).toBe(3);
    expect(extractEpisodeNumber("Show.E03.mkv", ["E(?<episode>\\d+)"])).toBe(3);
    expect(extractEpisodeNumber("Show.E03.mkv", ["E\\d+"])).toBe(3);
  });

  it("records actual regex captures and preserves them after rules change", () => {
    const subs = mergeSeriesSubtitleFiles([], ["/x/Show.S01E02.en.srt"], "auto", rules, "en", "it");
    const snapshot = subs[0].matching!.files[0];
    expect(snapshot.episodeRule?.episode).toBe(2);
    expect(snapshot.episodeRule?.pattern).toBe(rules.episodeRegexes[0]);
    const changed = { ...rules, episodeRegexes: ["never(\\d+)"] };
    const result = mergeSeriesMediaFiles(subs, ["/x/Show.S01E02.mkv"], changed);
    expect(result[0].matching!.files.find(file => file.path.endsWith(".srt"))).toEqual(snapshot);
  });
});

it("preserves ambiguous media alternatives without choosing a format arbitrarily", () => {
 const subs = mergeSeriesSubtitleFiles([], ["Show.E01.en.srt"], "auto", rules, "en", "it");
 const result = mergeSeriesMediaFiles(subs, ["Show.E01.mp4", "Show.E01.mkv"], rules);
 expect(result).toHaveLength(3);
 expect(result.find(row => row.targetSubsPath)?.mediaPath).toBe("");
 expect(result.filter(row => row.mediaPath)).toHaveLength(2);
 expect(mergeSeriesMediaFiles(result, [], rules)).toHaveLength(3);
});

it("does not discard additional subtitle languages in a grouped import", () => {
 const result = mergeSeriesSubtitleFiles([], ["Show.E01.en.srt", "Show.E01.it.srt", "Show.E01.fr.srt"], "auto", rules, "en", "it");
 expect(result).toHaveLength(2);
 expect(result.flatMap(row => [row.targetSubsPath, row.nativeSubsPath]).filter(Boolean).sort()).toEqual(
   ["Show.E01.en.srt", "Show.E01.fr.srt", "Show.E01.it.srt"]);
});
