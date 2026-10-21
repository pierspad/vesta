import { describe, expect, it, vi } from "vitest";
import { DEFAULT_SMART_MATCHING_RULES, type SmartMatchingRules } from "$lib/stores/smartMatchingStore.svelte";
import { SmartMatchingRulesDraft, type SmartMatchingRulesStore } from "./smartMatchingRulesDraft.svelte";

function makeStore(overrides: Partial<SmartMatchingRulesStore> = {}) {
  const store: SmartMatchingRulesStore = {
    enabled: true,
    rules: { ...DEFAULT_SMART_MATCHING_RULES, episodeRegexes: ["original"] },
    setEnabled(enabled) { store.enabled = enabled; },
    saveRules(rules) { store.rules = rules; },
    ...overrides,
  };
  return store;
}

const messages = {
  invalidObject: () => "Expected object",
  invalidJson: () => "Invalid JSON",
  saveSuccess: () => "Saved",
  saveError: () => "Save failed",
};

describe("SmartMatchingRulesDraft", () => {
  it("preserves comment markers inside JSON strings while removing line and block comments", () => {
    const store = makeStore();
    const notify = vi.fn();
    const draft = new SmartMatchingRulesDraft(store, messages, notify);
    draft.value = `{
      // a line comment
      "episodeRegexes": ["https://example.test/a/*b*/", "// literal"],
      /* block comment */ "originalSubtitleHints": ["a/*b*/c"]
    }`;

    expect(draft.validate()).toBeNull();
    expect(draft.save()).toBe(true);
    expect(store.rules.episodeRegexes).toEqual(["https://example.test/a/*b*/", "// literal"]);
    expect(store.rules.originalSubtitleHints).toEqual(["a/*b*/c"]);
    expect(notify).toHaveBeenCalledWith("Saved");
  });

  it("rejects invalid JSON and non-object values without saving", () => {
    const saveRules = vi.fn();
    const draft = new SmartMatchingRulesDraft(makeStore({ saveRules }), messages, vi.fn());
    draft.value = `{"episodeRegexes": [}`;
    expect(draft.save()).toBe(false);
    expect(draft.error).toBeTruthy();
    draft.value = "null";
    expect(draft.save()).toBe(false);
    expect(draft.error).toBe("Expected object");
    expect(saveRules).not.toHaveBeenCalled();
  });

  it("surfaces persistence failures and reloads saved rules when re-enabled", () => {
    const persistedRules: SmartMatchingRules = { ...DEFAULT_SMART_MATCHING_RULES, episodeRegexes: ["latest"] };
    const store = makeStore({ saveRules() { throw new Error("disk unavailable"); } });
    const draft = new SmartMatchingRulesDraft(store, messages, vi.fn());
    draft.value = JSON.stringify(persistedRules);
    expect(draft.save()).toBe(false);
    expect(draft.error).toBe("disk unavailable");

    store.saveRules = (rules) => { store.rules = rules; };
    draft.toggle(); // disable
    draft.value = "unsaved edits";
    draft.error = "stale error";
    draft.toggle(); // enable and restore store state
    expect(draft.value).toBe(JSON.stringify(store.rules, null, 2));
    expect(draft.error).toBeNull();
  });
});
