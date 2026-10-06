import { expect, it } from "vitest";
import { normalizeLanguageCode, languageFlagPath, languages } from "./languages";
import { inferLanguageFromPath } from "$lib/utils/models";
import { predefinedNoteTypes, findNoteTypeById } from "$lib/types/noteTypes";
it("normalizes aliases while preserving script variants", () => {
  for (const [input, expected] of [["jpn", "ja"], ["deu", "de"], ["zh_Hant_TW", "zh-tw"], ["zh-HK", "zh-tw"], ["cht", "zh-tw"], ["zh-Hans-CN", "zh"], ["pt_BR", "pt-br"], ["unknown", ""]]) {
    expect(normalizeLanguageCode(input)).toBe(expected);
  }
  expect(languageFlagPath("jpn")).toBe("/flags/jp.svg");
});
it("recognizes filename suffixes without taking an earlier language from the title", () => {
  for (const [path, expected] of [["series.zh_Hant_TW.srt", "zh-tw"], ["series.zh_Hans.srt", "zh"], ["series.pt-br.srt", "pt-br"], ["series.jpn.srt", "ja"], ["movie.en.foo.ja.srt", "ja"], ["/de/movie.srt", null]] as const) {
    expect(inferLanguageFromPath(path)).toBe(expected);
  }
});
it("ships distinct stable note types for all supported languages", () => {
  const defaults = predefinedNoteTypes();
  expect(defaults).toHaveLength(languages.length + 1);
  expect(new Set(defaults.map((nt) => nt.name)).size).toBe(defaults.length);
  for (const nt of defaults) {
    expect(findNoteTypeById(nt.id)).toEqual(nt);
    expect(nt.name.length).toBeLessThanOrEqual(25);
    expect(nt.fields.expression).toBe("Expression");
    expect(Object.values(nt.included).every(Boolean)).toBe(true);
  }
});
