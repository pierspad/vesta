import { describe, expect, it } from "vitest";
import { generateTranslationOutputPath } from "./translationPaths";

describe("translation destinations", () => {
  it("replaces a filename language suffix while preserving directory and separator", () => {
    expect(generateTranslationOutputPath("/films/Movie.eng.srt", "it")).toBe("/films/Movie.it.srt");
    expect(generateTranslationOutputPath("C:\\films\\Movie-en.SRT", "it")).toBe("C:\\films\\Movie-it.srt");
  });
  it("never returns the source path when its language already matches", () => {
    expect(generateTranslationOutputPath("/films/Movie.it.srt", "it")).toBe("/films/Movie.it.translated.it.srt");
    expect(generateTranslationOutputPath("C:\\films\\Movie-IT.SRT", "it")).toBe("C:\\films\\Movie-IT.translated.it.srt");
  });
  it("keeps intermediate language tags and language-like directory names", () => {
    expect(generateTranslationOutputPath("/series.en.srt/sub_fr_720p.srt", "it")).toBe("/series.en.srt/sub_fr_720p.it.srt");
    expect(generateTranslationOutputPath("/en/Movie.srt", "ja")).toBe("/en/Movie.ja.srt");
  });
});
