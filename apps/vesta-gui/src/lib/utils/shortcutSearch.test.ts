import { describe, expect, it } from "vitest";
import type { ShortcutDefinition } from "$lib/utils/shortcuts";
import { filterAndSortShortcuts } from "./shortcutSearch";

const shortcuts: ShortcutDefinition[] = [
  { id: "z-sync", action: "", description: "sync.z", defaultKey: "Ctrl+Shift+Z", category: "sync" },
  { id: "b-global", action: "", description: "global.b", defaultKey: "Alt+2", category: "global" },
  { id: "a-global", action: "", description: "global.a", defaultKey: "Alt+1", category: "global" },
  { id: "a-flash", action: "", description: "flash.a", defaultKey: "Ctrl+Enter", category: "flashcards" },
];

const translated: Record<string, string> = {
  "sync.z": "Zoom subtitle",
  "global.b": "Beta action",
  "global.a": "Alpha action",
  "flash.a": "Generate flashcards",
};
const translate = (key: string) => translated[key] ?? key;
const defaults = { categories: [], searchMode: "text" as const, searchQuery: "", searchKeys: [], translate };

describe("filterAndSortShortcuts", () => {
  it("preserves category ranking and localized alphabetical order", () => {
    expect(filterAndSortShortcuts(shortcuts, defaults).map(item => item.id)).toEqual([
      "a-global", "b-global", "a-flash", "z-sync",
    ]);
  });

  it("filters by localized substring or fuzzy text while honoring categories", () => {
    expect(filterAndSortShortcuts(shortcuts, { ...defaults, categories: ["flashcards", "sync"], searchQuery: "gen flash" }).map(item => item.id)).toEqual(["a-flash"]);
    expect(filterAndSortShortcuts(shortcuts, { ...defaults, searchQuery: "zms" }).map(item => item.id)).toEqual(["z-sync"]);
  });

  it("requires every captured key in a shortcut part and matches case-insensitively", () => {
    expect(filterAndSortShortcuts(shortcuts, {
      ...defaults, searchMode: "keys", searchKeys: ["ctrl", "shift"],
    }).map(item => item.id)).toEqual(["z-sync"]);
  });
});
