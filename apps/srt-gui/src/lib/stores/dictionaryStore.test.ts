import { describe, it, expect, beforeEach } from "vitest";
import { dictionaryStore } from "./dictionaryStore.svelte";

describe("dictionaryStore", () => {
  beforeEach(() => {
    // Clear custom dictionaries
    dictionaryStore.customDictionaries = [];
  });

  it("adds and retrieves a custom dictionary", () => {
    const res = dictionaryStore.addCustomDictionary({
      name: "Custom Treccani",
      targetLanguage: "it",
      wordUrlTemplate: "https://treccani.it/vocabolario/{word}",
      sentenceUrlTemplate: "https://context.reverso.net/{sentence}",
      description: "Dizionario Treccani personalizzato",
    });

    expect(res.success).toBe(true);
    expect(res.dictionary).toBeDefined();
    expect(dictionaryStore.customDictionaries.length).toBe(1);

    const provider = dictionaryStore.getDictionaryById(`custom_${res.dictionary!.id}`);
    expect(provider).toBeDefined();
    expect(provider?.name).toContain("Custom Treccani");
    expect(provider?.wordUrlTemplate).toBe("https://treccani.it/vocabolario/{word}");
  });

  it("fails to add custom dictionary without {word} placeholder", () => {
    const res = dictionaryStore.addCustomDictionary({
      name: "Invalid Dict",
      targetLanguage: "en",
      wordUrlTemplate: "https://invalid.com/search",
    });

    expect(res.success).toBe(false);
    expect(res.error).toContain("{word}");
  });

  it("removes custom dictionary", () => {
    const res = dictionaryStore.addCustomDictionary({
      name: "To Delete",
      targetLanguage: "es",
      wordUrlTemplate: "https://rae.es/{word}",
    });

    expect(res.success).toBe(true);
    const id = res.dictionary!.id;
    const removed = dictionaryStore.removeCustomDictionary(id);
    expect(removed).toBe(true);
    expect(dictionaryStore.customDictionaries.length).toBe(0);
  });

  it("resolves built-in provider by default", () => {
    const provider = dictionaryStore.resolveProvider("ja", "it");
    expect(provider.id).toBe("ja_jisho");
  });
});
