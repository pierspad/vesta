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
  defaultShortcuts,
  getShortcuts,
  localShortcutAction,
  saveShortcutOverride,
  resetShortcuts,
  resetSingleShortcut,
  getSortedKeys,
  formatKeyPart,
} from "./shortcuts";

describe("shortcuts", () => {
  beforeEach(() => {
    storage.clear();
  });

  describe("defaultShortcuts integrity", () => {
    it("has unique IDs for every shortcut definition", () => {
      const ids = defaultShortcuts.map((s) => s.id);
      const uniqueIds = new Set(ids);
      expect(uniqueIds.size).toBe(ids.length);
    });

    it("has valid categories and default keys", () => {
      const validCategories = new Set(["global", "translate", "sync", "flashcards", "alignment", "transcribe", "refine", "settings"]);
      for (const s of defaultShortcuts) {
        expect(validCategories.has(s.category)).toBe(true);
        expect(s.defaultKey.length).toBeGreaterThan(0);
        expect(s.action.length).toBeGreaterThan(0);
      }
    });
  });

  describe("persistence & overrides", () => {
    it("returns default shortcuts when no overrides exist", () => {
      const shortcuts = getShortcuts();
      expect(shortcuts).toEqual(defaultShortcuts);
    });

    it("applies user overrides correctly", () => {
      saveShortcutOverride("flashcards-generate", "Ctrl+G");

      const shortcuts = getShortcuts();
      const generateSc = shortcuts.find((s) => s.id === "flashcards-generate");
      expect(generateSc?.defaultKey).toBe("Ctrl+G");

      // Other shortcuts remain untouched
      const cancelSc = shortcuts.find((s) => s.id === "flashcards-cancel");
      expect(cancelSc?.defaultKey).toBe("Escape");
    });

    it("allows the same custom key in different tabs and reserves global keys", () => {
      saveShortcutOverride("sync-new", "Ctrl+K");
      saveShortcutOverride("flashcards-preview", "Ctrl+K");
      saveShortcutOverride("sync-new", "Alt+1");
      saveShortcutOverride("tab-flashcards", "Ctrl+K");
      const key = (id: string) => getShortcuts().find(shortcut => shortcut.id === id)?.defaultKey;
      expect(key("sync-new")).toBe("Ctrl+K");
      expect(key("flashcards-preview")).toBe("Ctrl+K");
      expect(key("tab-flashcards")).toBe("Alt+1");
      const event = { key: "k", ctrlKey: true, metaKey: false, altKey: false, shiftKey: false } as KeyboardEvent;
      expect(localShortcutAction(event, "sync")).toBe("newSync");
      expect(localShortcutAction(event, "flashcards")).toBe("previewCards");
      expect(localShortcutAction(event, "translate")).toBeUndefined();
    });

    it("resets all shortcuts back to defaults", () => {
      saveShortcutOverride("flashcards-generate", "Ctrl+G");
      saveShortcutOverride("flashcards-cancel", "Ctrl+Q");
      expect(storage.get("srt-tools-shortcut-overrides")).toBeDefined();

      resetShortcuts();
      expect(storage.get("srt-tools-shortcut-overrides")).toBeUndefined();
      expect(getShortcuts()).toEqual(defaultShortcuts);
    });

    it("resets a single shortcut and resolves collision cascades", () => {
      // User maps shortcut A to "Ctrl+P" (which was default of B)
      saveShortcutOverride("flashcards-generate", "Ctrl+P");
      saveShortcutOverride("flashcards-preview", "Ctrl+Alt+P");

      resetSingleShortcut("flashcards-generate");

      const shortcuts = getShortcuts();
      const generateSc = shortcuts.find((s) => s.id === "flashcards-generate");
      expect(generateSc?.defaultKey).toBe("Ctrl+Enter");
    });
  });

  describe("getSortedKeys", () => {
    it("orders modifiers Ctrl -> Alt -> Shift -> Key consistently", () => {
      expect(getSortedKeys("Shift+Ctrl+S")).toEqual(["Ctrl", "Shift", "S"]);
      expect(getSortedKeys("Alt+Ctrl+Enter")).toEqual(["Ctrl", "Alt", "Enter"]);
      expect(getSortedKeys("Shift+Alt+Ctrl+K")).toEqual(["Ctrl", "Alt", "Shift", "K"]);
      expect(getSortedKeys("Escape")).toEqual(["Escape"]);
      expect(getSortedKeys("")).toEqual([]);
    });
  });

  describe("formatKeyPart", () => {
    it("translates known keys using dictionary and falls back for unknown", () => {
      const mockTranslate = (key: string) => {
        const dict: Record<string, string> = {
          "keys.ctrl": "Strg",
          "keys.shift": "Umschalt",
          "keys.enter": "Eingabe",
        };
        return dict[key] || key;
      };

      expect(formatKeyPart("Ctrl", mockTranslate)).toBe("Strg");
      expect(formatKeyPart("Shift", mockTranslate)).toBe("Umschalt");
      expect(formatKeyPart("Enter", mockTranslate)).toBe("Eingabe");
      expect(formatKeyPart("K", mockTranslate)).toBe("K");
    });
  });
});

it("dispatches undo/redo aliases and respects custom bindings immediately", async () => {
  const { historyShortcutAction } = await import("./shortcuts");
  const key = (key: string, options = {}) => ({ key, ctrlKey: true, metaKey: false, altKey: false, shiftKey: false, defaultPrevented: false, isComposing: false, ...options }) as KeyboardEvent;
  resetShortcuts();
  expect(historyShortcutAction(key("z"), "sync")).toBe("undo");
  expect(historyShortcutAction(key("Z", { shiftKey: true }), "sync")).toBe("redo");
  expect(historyShortcutAction(key("y"), "sync")).toBe("redo");
  expect(historyShortcutAction(key("z", { ctrlKey: false, metaKey: true }), "align")).toBe("undo");
  saveShortcutOverride("sync-redo", "Ctrl+R");
  expect(historyShortcutAction(key("y"), "sync")).toBeNull();
  expect(historyShortcutAction(key("r"), "sync")).toBe("redo");
  expect(historyShortcutAction(key("r", { isComposing: true }), "sync")).toBeNull();
  resetShortcuts();
});
