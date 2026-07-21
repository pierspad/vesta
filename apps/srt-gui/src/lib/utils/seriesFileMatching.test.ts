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
  generateDefaultDeckName,
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

  it("falls back to matching by episode number when the baseKey text differs", () => {
    const withSubs = mergeSeriesSubtitleFiles([], ["/x/My Show - Episode 01.srt"], "auto", rules, "", "");
    const withMedia = mergeSeriesMediaFiles(withSubs, ["/x/Completely Different Name E01.mkv"], rules);
    expect(withMedia).toHaveLength(1);
    expect(withMedia[0].mediaPath).toContain("Completely Different Name");
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

  it("falls back to lexicographic index pairing when targets lack episode numbers", () => {
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

describe("generateDefaultDeckName", () => {
  it("strips role hints, language codes and episode markers", () => {
    expect(generateDefaultDeckName("Detour.1945.original.en.srt")).not.toMatch(/original|\ben\b/i);
  });

  it("falls back to a default label when nothing meaningful remains", () => {
    expect(generateDefaultDeckName("S01E01.srt")).toBe("Default Deck");
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
