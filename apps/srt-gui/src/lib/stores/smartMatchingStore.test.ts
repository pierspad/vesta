import { describe, it, expect, beforeEach, vi } from "vitest";

const { storage } = vi.hoisted(() => {
  return { storage: new Map<string, string>() };
});

vi.mock("$lib/config/vestaConfig", () => ({
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, val: string) => {
    storage.set(key, val);
  },
  removeItem: (key: string) => {
    storage.delete(key);
  },
}));

import {
  smartMatchingStore,
  DEFAULT_SMART_MATCHING_RULES,
} from "./smartMatchingStore.svelte";

describe("smartMatchingStore", () => {
  beforeEach(() => {
    storage.clear();
    smartMatchingStore.resetRules();
    smartMatchingStore.setEnabled(true);
  });

  it("initializes enabled by default with default rules", () => {
    expect(smartMatchingStore.enabled).toBe(true);
    expect(smartMatchingStore.rules.episodeRegexes.length).toBeGreaterThan(0);
    expect(smartMatchingStore.rules.originalSubtitleHints).toContain("native");
    expect(smartMatchingStore.rules.referenceSubtitleHints).toContain("translation");
  });

  it("persists enabled / disabled state", () => {
    smartMatchingStore.setEnabled(false);
    expect(smartMatchingStore.enabled).toBe(false);
    expect(storage.get("vesta-flashcards-smart-file-matching-enabled")).toBe("false");

    smartMatchingStore.setEnabled(true);
    expect(smartMatchingStore.enabled).toBe(true);
    expect(storage.get("vesta-flashcards-smart-file-matching-enabled")).toBe("true");
  });

  it("saves and reloads custom matching rules", () => {
    const customRules = {
      ...DEFAULT_SMART_MATCHING_RULES,
      originalSubtitleHints: ["jap", "orig"],
      removableNameTokens: ["1080p", "hevc"],
    };

    smartMatchingStore.saveRules(customRules);
    expect(smartMatchingStore.rules.originalSubtitleHints).toEqual(["jap", "orig"]);
    expect(smartMatchingStore.rules.removableNameTokens).toEqual(["1080p", "hevc"]);

    smartMatchingStore.resetRules();
    expect(smartMatchingStore.rules).toEqual(DEFAULT_SMART_MATCHING_RULES);
  });

  it("handles malformed JSON in storage by falling back to defaults", () => {
    storage.set("vesta-flashcards-smart-matching-rules", "{ broken json !!! }");
    smartMatchingStore.load();
    expect(smartMatchingStore.rules).toEqual(DEFAULT_SMART_MATCHING_RULES);
  });
});

it("normalizes partial and malformed rule fields before they reach live matching", () => {
  smartMatchingStore.saveRules({ episodeRegexes: ["S(\\d+)", 7], originalSubtitleHints: false } as any);
  expect(smartMatchingStore.rules.episodeRegexes).toEqual(["S(\\d+)"]);
  expect(smartMatchingStore.rules.originalSubtitleHints).toEqual(DEFAULT_SMART_MATCHING_RULES.originalSubtitleHints);
  expect(smartMatchingStore.rules.referenceSubtitleHints).toEqual(DEFAULT_SMART_MATCHING_RULES.referenceSubtitleHints);
  expect(JSON.parse(storage.get("vesta-flashcards-smart-matching-rules")!)).toEqual(smartMatchingStore.rules);
});
