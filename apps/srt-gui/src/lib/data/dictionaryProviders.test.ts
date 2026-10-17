import { describe, it, expect } from "vitest";
import {
  builtInDictionaries,
  inferDictionaryForLanguages,
  getDictionariesForLanguage,
  getAllDictionaries,
  getDictionarySearchTerms,
} from "./dictionaryProviders";

describe("dictionaryProviders", () => {
  it("has built-in dictionaries for major languages", () => {
    expect(builtInDictionaries.length).toBeGreaterThan(15);
    const targetLangs = builtInDictionaries.map((d) => d.targetLanguage);
    expect(targetLangs).toContain("ja");
    expect(targetLangs).toContain("zh");
    expect(targetLangs).toContain("ko");
    expect(targetLangs).toContain("ar");
    expect(targetLangs).toContain("ru");
    expect(targetLangs).toContain("en");
    expect(targetLangs).toContain("de");
    expect(targetLangs).toContain("es");
    expect(targetLangs).toContain("fr");
    expect(targetLangs).toContain("it");
    expect(targetLangs).toContain("pt");
  });

  it("infers Jisho for Japanese", () => {
    const dict = inferDictionaryForLanguages("ja", "it");
    expect(dict.id).toBe("ja_jisho");
    expect(dict.wordUrlTemplate).toContain("jisho.org");
    expect(dict.sentenceUrlTemplate).toContain("jisho.org");
  });

  it("infers WordReference EN-IT for English with Italian native", () => {
    const dict = inferDictionaryForLanguages("en", "it");
    expect(dict.id).toBe("en_wordref_it");
    expect(dict.wordUrlTemplate).toContain("wordreference.com/enit");
    expect(dict.sentenceUrlTemplate).toContain("reverso.net");
  });

  it("infers WordReference DE-IT for German with Italian native", () => {
    const dict = inferDictionaryForLanguages("de", "it");
    expect(dict.id).toBe("de_wordref_it");
    expect(dict.wordUrlTemplate).toContain("wordreference.com/deit");
  });

  it("infers MDBG for Chinese", () => {
    const dict = inferDictionaryForLanguages("zh", "it");
    expect(dict.id).toBe("zh_mdbg");
    expect(dict.wordUrlTemplate).toContain("mdbg.net");
  });

  it("infers Naver for Korean", () => {
    const dict = inferDictionaryForLanguages("ko", "it");
    expect(dict.id).toBe("ko_naver");
    expect(dict.wordUrlTemplate).toContain("naver.com");
  });

  it("infers Almaany for Arabic", () => {
    const dict = inferDictionaryForLanguages("ar", "it");
    expect(dict.id).toBe("ar_almaany");
    expect(dict.wordUrlTemplate).toContain("almaany.com");
  });

  it("infers OpenRussian for Russian", () => {
    const dict = inferDictionaryForLanguages("ru", "it");
    expect(dict.id).toBe("ru_openrussian");
    expect(dict.wordUrlTemplate).toContain("openrussian.org");
  });

  it("returns all dictionaries with target language prioritized first in getDictionariesForLanguage", () => {
    const customList = [
      {
        id: "custom_1",
        name: "My Custom Latin Dict",
        targetLanguage: "la",
        wordUrlTemplate: "https://latin.example.com/{word}",
        createdAt: Date.now(),
      },
    ];

    const dicts = getDictionariesForLanguage("la", customList);
    // Matching target language is first
    expect(dicts[0].targetLanguage).toBe("la");
    expect(dicts.some((d) => d.id === "ja_jisho")).toBe(true);
    expect(dicts.some((d) => d.id === "zh_mdbg")).toBe(true);
    expect(dicts.some((d) => d.id === "de_wordref_it")).toBe(true);
  });

  it("generates comprehensive multi-lingual search terms", () => {
    const jisho = builtInDictionaries.find((d) => d.id === "ja_jisho")!;
    const jishoTerms = getDictionarySearchTerms(jisho);
    expect(jishoTerms).toContain("giapponese");
    expect(jishoTerms).toContain("japanese");
    expect(jishoTerms).toContain("日本語");
    expect(jishoTerms).toContain("jisho");
    expect(jishoTerms).toContain("japanisch");

    const mdbg = builtInDictionaries.find((d) => d.id === "zh_mdbg")!;
    const mdbgTerms = getDictionarySearchTerms(mdbg);
    expect(mdbgTerms).toContain("cinese");
    expect(mdbgTerms).toContain("chinese");
    expect(mdbgTerms).toContain("中文");
    expect(mdbgTerms).toContain("mandarin");
    expect(mdbgTerms).toContain("mdbg");

    const duden = builtInDictionaries.find((d) => d.id === "de_duden")!;
    const dudenTerms = getDictionarySearchTerms(duden);
    expect(dudenTerms).toContain("tedesco");
    expect(dudenTerms).toContain("german");
    expect(dudenTerms).toContain("deutsch");
  });
});
