import { describe, expect, it } from "vitest";
import type { NoteTypeDef } from "$lib/types/noteTypes";
import { DEFAULT_SMART_MATCHING_RULES } from "$lib/stores/smartMatchingStore.svelte";
import {
  buildSeriesDraftMap,
  seriesDraftEntriesToEpisodes,
} from "./seriesFileMatching";
import {
  detectSubtitleLanguage,
  resolveEpisodeNoteType,
} from "./episodeNoteType";

const noteType = (
  id: string,
  language: string,
  predefined = true,
): NoteTypeDef => ({
  id,
  name: id,
  predefined,
  language,
  fields: {
    expression: "Expression",
    meaning: "Meaning",
    reading: "Reading",
    audio: "Audio",
    snapshot: "Snapshot",
    video: "Video",
    tags: "Tags",
    sequenceMarker: "Sequence",
    notes: "Notes",
  },
  included: {
    expression: true,
    meaning: true,
    reading: true,
    audio: true,
    snapshot: true,
    video: true,
    tags: true,
    sequenceMarker: true,
    notes: true,
  },
});

const types = [
  noteType("default", ""),
  noteType("predef:en", "en"),
  noteType("predef:it", "it"),
  noteType("predef:ja", "ja"),
  noteType("custom:movie", "", false),
];

describe("resolveEpisodeNoteType", () => {
  it("uses each subtitle filename language for mixed-language episodes", () => {
    const english = resolveEpisodeNoteType(
      { targetSubsPath: "/Show/S01E01.en.srt" },
      types,
      "predef:it",
    );
    const italian = resolveEpisodeNoteType(
      { targetSubsPath: "/Show/S01E02.it.srt" },
      types,
      "predef:en",
    );
    expect([english.id, italian.id]).toEqual(["predef:en", "predef:it"]);
  });

  it("honors an explicit per-episode override before filename or detected language", () => {
    const result = resolveEpisodeNoteType(
      {
        targetSubsPath: "/Show/E01.en.srt",
        noteTypeId: "custom:movie",
        detectedLanguage: "ja",
      },
      types,
      "default",
    );
    expect(result.id).toBe("custom:movie");
  });

  it("uses detected content language after filename inference and before the fallback", () => {
    const detected = resolveEpisodeNoteType(
      { targetSubsPath: "/Show/E01.srt", detectedLanguage: "ja-JP" },
      types,
      "predef:it",
    );
    const pathWins = resolveEpisodeNoteType(
      { targetSubsPath: "/Show/E01.en.srt", detectedLanguage: "ja" },
      types,
      "predef:it",
    );
    expect(detected.id).toBe("predef:ja");
    expect(pathWins.id).toBe("predef:en");
  });

  it("does not map a language to a custom note type and recovers from deleted IDs", () => {
    const result = resolveEpisodeNoteType(
      {
        targetSubsPath: "/Show/E01.srt",
        detectedLanguage: "ja",
        noteTypeId: "deleted",
      },
      types,
      "deleted-fallback",
    );
    expect(result.id).toBe("predef:ja");
  });

  it("uses a still-valid fallback, then the default if that fallback was removed", () => {
    expect(
      resolveEpisodeNoteType(
        { targetSubsPath: "unknown.srt" },
        types,
        "predef:it",
      ).id,
    ).toBe("predef:it");
    expect(
      resolveEpisodeNoteType(
        { targetSubsPath: "unknown.srt" },
        [types[1]],
        "gone",
      ).id,
    ).toBe("default");
  });
});

describe("detectSubtitleLanguage", () => {
  it("detects strong per-film subtitle text across distinct scripts and lexical evidence", () => {
    expect(
      detectSubtitleLanguage(
        "これは日本語の字幕です。今日は映画を見ています。",
      ),
    ).toBe("ja");
    expect(
      detectSubtitleLanguage(
        "The night is cold and the road is long. We have to go now.",
      ),
    ).toBe("en");
    expect(
      detectSubtitleLanguage(
        "Non voglio andare, ma questa notte dobbiamo restare qui, perché abbiamo una scelta.",
      ),
    ).toBe("it");
  });

  it("returns null for timestamps, short cues, and ambiguous shared-script text", () => {
    expect(detectSubtitleLanguage("00:00:01,000 --> 00:00:02,000")).toBeNull();
    expect(detectSubtitleLanguage("Hello!")).toBeNull();
    expect(
      detectSubtitleLanguage("Привет, как дела? До встречи завтра вечером."),
    ).toBeNull();
  });
});

it("preserves per-episode type overrides and detected language through series draft editing", () => {
  const episodes = [
    {
      id: 1,
      targetSubsPath: "/Show/E01.en.srt",
      nativeSubsPath: "",
      mediaPath: "",
      mediaType: "none" as const,
      noteTypeId: "custom:movie",
      detectedLanguage: "ja",
    },
  ];
  const drafts = buildSeriesDraftMap(episodes, DEFAULT_SMART_MATCHING_RULES);
  const restored = seriesDraftEntriesToEpisodes(drafts);
  expect(restored[0]).toMatchObject({
    noteTypeId: "custom:movie",
    detectedLanguage: "ja",
  });
});

it("treats legacy automatic IDs as automatic rather than overriding the film language", () => {
  for (const noteTypeId of ["default", "auto"]) {
    expect(resolveEpisodeNoteType({targetSubsPath: "movie.en.srt", noteTypeId}, types, "predef:it").id).toBe("predef:en");
  }
});
