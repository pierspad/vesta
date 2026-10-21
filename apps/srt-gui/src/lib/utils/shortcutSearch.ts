import type { ShortcutDefinition } from "$lib/utils/shortcuts";

export type ShortcutSearchMode = "text" | "keys";

const CATEGORY_ORDER = ["global", "flashcards", "translate", "sync", "alignment", "transcribe"];

function fuzzyMatch(text: string, query: string): boolean {
  if (!query) return true;
  if (!text) return false;

  const normalizedQuery = query.toLowerCase().replace(/\s+/g, "");
  const normalizedText = text.toLowerCase();
  let queryIndex = 0;
  for (let index = 0; index < normalizedText.length; index += 1) {
    if (normalizedText[index] === normalizedQuery[queryIndex]) {
      queryIndex += 1;
      if (queryIndex === normalizedQuery.length) return true;
    }
  }
  return false;
}

export function filterAndSortShortcuts(
  shortcuts: ShortcutDefinition[],
  options: {
    categories: string[];
    searchMode: ShortcutSearchMode;
    searchQuery: string;
    searchKeys: string[];
    translate: (descriptionKey: string) => string;
  },
): ShortcutDefinition[] {
  let list = [...shortcuts];
  if (options.categories.length > 0) {
    list = list.filter((shortcut) => options.categories.includes(shortcut.category));
  }

  if (options.searchMode === "text") {
    if (options.searchQuery.trim()) {
      const query = options.searchQuery.toLowerCase().trim();
      list = list.filter((shortcut) => {
        const description = options.translate(shortcut.description);
        const key = shortcut.defaultKey;
        return description.toLowerCase().includes(query)
          || key.toLowerCase().includes(query)
          || fuzzyMatch(description, query)
          || fuzzyMatch(key, query);
      });
    }
  } else if (options.searchKeys.length > 0) {
    const searchParts = options.searchKeys.map((key) => key.toUpperCase());
    list = list.filter((shortcut) => {
      const shortcutParts = shortcut.defaultKey.toUpperCase().split("+").map((part) => part.trim());
      return searchParts.every((key) => shortcutParts.some((part) => part.includes(key)));
    });
  }

  list.sort((a, b) => {
    const categoryDifference = CATEGORY_ORDER.indexOf(a.category) - CATEGORY_ORDER.indexOf(b.category);
    if (categoryDifference !== 0) return categoryDifference;
    return options.translate(a.description).localeCompare(options.translate(b.description), undefined, {
      sensitivity: "base",
    });
  });
  return list;
}
