import * as vestaConfig from "$lib/config/vestaConfig";

// Shortcut predefinite
export interface ShortcutDefinition {
  id: string;
  action: string;
  description: string; // i18n key, resolved via t() in components
  defaultKey: string;
  category: "global" | "translate" | "sync" | "flashcards" | "alignment" | "transcribe" | "refine" | "settings";
}

export const defaultShortcuts: ShortcutDefinition[] = [
  // Global — tab navigation follows sidebar order (Flashcards first)
  { id: "tab-flashcards", action: "switchToFlashcards", description: "shortcuts.action.goToFlashcards", defaultKey: "Alt+1", category: "global" },
  { id: "tab-refine", action: "switchToRefine", description: "shortcuts.action.goToRefine", defaultKey: "Alt+2", category: "global" },
  { id: "tab-translate", action: "switchToTranslate", description: "shortcuts.action.goToTranslation", defaultKey: "Alt+3", category: "global" },
  { id: "tab-sync", action: "switchToSync", description: "shortcuts.action.goToSync", defaultKey: "Alt+4", category: "global" },
  { id: "tab-align", action: "switchToAlign", description: "shortcuts.action.goToAlign", defaultKey: "Alt+5", category: "global" },
  { id: "tab-transcribe", action: "switchToTranscribe", description: "shortcuts.action.goToTranscribe", defaultKey: "Alt+6", category: "global" },
  { id: "tab-extract", action: "switchToExtract", description: "shortcuts.action.goToExtract", defaultKey: "Alt+Shift+E", category: "global" },
  { id: "tab-plugins", action: "switchToPlugins", description: "shortcuts.action.goToPlugins", defaultKey: "Alt+Shift+P", category: "global" },
  { id: "tab-settings", action: "switchToSettings", description: "shortcuts.action.goToSettings", defaultKey: "Alt+7", category: "global" },
  { id: "tab-shortcuts", action: "switchToShortcuts", description: "shortcuts.action.goToShortcuts", defaultKey: "Alt+8", category: "global" },
  { id: "tab-diagnostics", action: "switchToDiagnostics", description: "settings.section.diagnostics", defaultKey: "Alt+9", category: "global" },
  { id: "settings-add-key", action: "addApiKey", description: "shortcuts.action.addApiKey", defaultKey: "Ctrl+N", category: "settings" },
  { id: "show-help", action: "showShortcutHelp", description: "shortcuts.action.showHelp", defaultKey: "Shift+?", category: "global" },

  // Flashcards
  { id: "flashcards-generate", action: "generateFlashcards", description: "shortcuts.action.generateFlashcards", defaultKey: "Ctrl+Enter", category: "flashcards" },
  { id: "flashcards-cancel", action: "cancelGeneration", description: "shortcuts.action.cancelGeneration", defaultKey: "Escape", category: "flashcards" },
  { id: "flashcards-preview", action: "previewCards", description: "shortcuts.action.previewCards", defaultKey: "Ctrl+P", category: "flashcards" },

  { id: "flashcards-undo", action: "flashcardsUndo", description: "common.undo", defaultKey: "Ctrl+Z", category: "flashcards" },
  { id: "flashcards-redo", action: "flashcardsRedo", description: "common.redo", defaultKey: "Ctrl+Shift+Z", category: "flashcards" },
  { id: "refine-undo", action: "refineUndo", description: "common.undo", defaultKey: "Ctrl+Z", category: "refine" },
  { id: "refine-redo", action: "refineRedo", description: "common.redo", defaultKey: "Ctrl+Shift+Z", category: "refine" },

  // Translation
  { id: "translate-open-file", action: "openInputFile", description: "shortcuts.action.openSrt", defaultKey: "Ctrl+O", category: "translate" },
  { id: "translate-start", action: "startTranslation", description: "shortcuts.action.startTranslation", defaultKey: "Ctrl+Enter", category: "translate" },
  { id: "translate-cancel", action: "cancelTranslation", description: "shortcuts.action.cancelTranslation", defaultKey: "Escape", category: "translate" },
  { id: "translate-clear-logs", action: "clearLogs", description: "shortcuts.action.clearLogs", defaultKey: "Ctrl+L", category: "translate" },

  // Synchronization
  { id: "sync-open-file", action: "openSrt", description: "shortcuts.action.openSrt", defaultKey: "Ctrl+O", category: "sync" },
  { id: "sync-auto", action: "startAutoSync", description: "shortcuts.action.autoSync", defaultKey: "Ctrl+A", category: "sync" },
  { id: "sync-new", action: "newSync", description: "shortcuts.action.newSync", defaultKey: "Ctrl+N", category: "sync" },
  { id: "sync-load-session", action: "loadSession", description: "shortcuts.action.loadSession", defaultKey: "Ctrl+L", category: "sync" },
  { id: "sync-save-session", action: "saveSession", description: "shortcuts.action.saveSession", defaultKey: "Ctrl+Shift+S", category: "sync" },
  { id: "sync-play-pause", action: "playPause", description: "shortcuts.action.playPause", defaultKey: "Space", category: "sync" },
  { id: "sync-replay", action: "replaySubtitle", description: "sync.wizard.replay", defaultKey: "R", category: "sync" },
  { id: "sync-seek-back", action: "seekBack", description: "shortcuts.action.back01s", defaultKey: "ArrowLeft", category: "sync" },
  { id: "sync-seek-forward", action: "seekForward", description: "shortcuts.action.forward01s", defaultKey: "ArrowRight", category: "sync" },
  { id: "sync-seek-back-fast", action: "seekBackFast", description: "shortcuts.action.back1s", defaultKey: "Shift+ArrowLeft", category: "sync" },
  { id: "sync-seek-forward-fast", action: "seekForwardFast", description: "shortcuts.action.forward1s", defaultKey: "Shift+ArrowRight", category: "sync" },
  { id: "sync-offset-up", action: "offsetUp", description: "shortcuts.action.offsetUp", defaultKey: "ArrowUp", category: "sync" },
  { id: "sync-offset-down", action: "offsetDown", description: "shortcuts.action.offsetDown", defaultKey: "ArrowDown", category: "sync" },
  { id: "sync-offset-up-fast", action: "offsetUpFast", description: "shortcuts.action.offsetUpFast", defaultKey: "Shift+ArrowUp", category: "sync" },
  { id: "sync-offset-down-fast", action: "offsetDownFast", description: "shortcuts.action.offsetDownFast", defaultKey: "Shift+ArrowDown", category: "sync" },
  { id: "sync-undo", action: "syncUndo", description: "shortcuts.action.syncUndo", defaultKey: "Ctrl+Z", category: "sync" },
  { id: "sync-redo", action: "syncRedo", description: "common.redo", defaultKey: "Ctrl+Shift+Z", category: "sync" },
  { id: "sync-confirm", action: "confirmAnchor", description: "shortcuts.action.confirmAnchor", defaultKey: "Enter", category: "sync" },
  { id: "sync-next-sub", action: "nextSubtitle", description: "shortcuts.action.nextSubtitle", defaultKey: "PageDown", category: "sync" },
  { id: "sync-prev-sub", action: "prevSubtitle", description: "shortcuts.action.prevSubtitle", defaultKey: "PageUp", category: "sync" },
  { id: "sync-prev-anchor", action: "prevAnchor", description: "shortcuts.action.prevAnchor", defaultKey: "Ctrl+ArrowUp", category: "sync" },
  { id: "sync-next-anchor", action: "nextAnchor", description: "shortcuts.action.nextAnchor", defaultKey: "Ctrl+ArrowDown", category: "sync" },
  { id: "sync-go-suggested", action: "goToSuggested", description: "shortcuts.action.goToSuggested", defaultKey: "Ctrl+G", category: "sync" },
  { id: "sync-save", action: "saveFile", description: "shortcuts.action.saveFile", defaultKey: "Ctrl+S", category: "sync" },

  // Alignment
  { id: "align-open-file", action: "openInputFile", description: "shortcuts.action.openSrt", defaultKey: "Ctrl+O", category: "alignment" },
  { id: "align-next-page", action: "alignNextPage", description: "shortcuts.action.alignNextPage", defaultKey: "PageDown", category: "alignment" },
  { id: "align-prev-page", action: "alignPrevPage", description: "shortcuts.action.alignPrevPage", defaultKey: "PageUp", category: "alignment" },
  { id: "align-swap-files", action: "alignSwapFiles", description: "shortcuts.action.alignSwapFiles", defaultKey: "Ctrl+Shift+S", category: "alignment" },
  { id: "align-undo", action: "alignUndo", description: "shortcuts.action.alignUndo", defaultKey: "Ctrl+Z", category: "alignment" },
  { id: "align-redo", action: "alignRedo", description: "common.redo", defaultKey: "Ctrl+Shift+Z", category: "alignment" },
  { id: "align-save", action: "alignSave", description: "shortcuts.action.alignSave", defaultKey: "Ctrl+S", category: "alignment" },
  { id: "align-cycle-per-page", action: "alignCyclePerPage", description: "shortcuts.action.alignCyclePerPage", defaultKey: "Ctrl+Shift+P", category: "alignment" },

  // Transcribe
  { id: "transcribe-open-file", action: "openInputFile", description: "shortcuts.action.openSrt", defaultKey: "Ctrl+O", category: "transcribe" },
  { id: "transcribe-start", action: "startTranscription", description: "shortcuts.action.startTranscription", defaultKey: "Ctrl+Enter", category: "transcribe" },
  { id: "transcribe-cancel", action: "cancelTranscription", description: "shortcuts.action.cancelTranscription", defaultKey: "Escape", category: "transcribe" },
];

export function shortcutsShareScope(a: ShortcutDefinition, b: ShortcutDefinition): boolean {
  return a.category === b.category || a.category === "global" || b.category === "global";
}

export function isReservedShortcut(key: string): boolean {
  return defaultShortcuts.some(shortcut => shortcut.category === "global" && shortcut.defaultKey === key)
    || /^Alt\+Shift\+[1-6]$|^Ctrl\+Page(?:Up|Down)$/.test(key);
}

// Funzione per ottenere le shortcut (con override utente)
export function getShortcuts(): ShortcutDefinition[] {
  const overridesJson = vestaConfig.getItem("srt-tools-shortcut-overrides");
  if (!overridesJson) return defaultShortcuts;

  try {
    const overrides: Record<string, string> = JSON.parse(overridesJson);
    return defaultShortcuts.map((shortcut) => ({
      ...shortcut,
      defaultKey: shortcut.category !== "global" && overrides[shortcut.id] && !isReservedShortcut(overrides[shortcut.id])
        ? overrides[shortcut.id] : shortcut.defaultKey,
    }));
  } catch {
    return defaultShortcuts;
  }
}

// Funzione per salvare override shortcut
export function saveShortcutOverride(shortcutId: string, newKey: string): void {
  if (!defaultShortcuts.some(shortcut => shortcut.id === shortcutId && shortcut.category !== "global")
    || isReservedShortcut(newKey)) return;
  const overridesJson = vestaConfig.getItem("srt-tools-shortcut-overrides");
  let overrides: Record<string, string> = {};

  if (overridesJson) {
    try {
      overrides = JSON.parse(overridesJson);
    } catch {
      overrides = {};
    }
  }

  overrides[shortcutId] = newKey;
  vestaConfig.setItem("srt-tools-shortcut-overrides", JSON.stringify(overrides));
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("shortcuts-updated"));
  }
}

// Funzione per resettare le shortcut
export function resetShortcuts(): void {
  vestaConfig.removeItem("srt-tools-shortcut-overrides");
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("shortcuts-updated"));
  }
}

// Funzione per resettare una singola shortcut con cascade dei conflitti
export function resetSingleShortcut(shortcutId: string): void {
  const overridesJson = vestaConfig.getItem("srt-tools-shortcut-overrides");
  let overrides: Record<string, string> = {};
  if (overridesJson) {
    try {
      overrides = JSON.parse(overridesJson);
    } catch {
      overrides = {};
    }
  }

  const resetQueue: string[] = [shortcutId];
  const processed = new Set<string>();

  while (resetQueue.length > 0) {
    const currentId = resetQueue.shift()!;
    if (processed.has(currentId)) continue;
    processed.add(currentId);

    // Rimuovi l'override per far tornare la scorciatoia al valore predefinito
    delete overrides[currentId];

    // Trova il tasto predefinito di questa scorciatoia
    const defaultDef = defaultShortcuts.find((s) => s.id === currentId);
    if (!defaultDef) continue;
    const revertedKey = defaultDef.defaultKey;

    // Coda per il reset tutte le ALTRE scorciatoie che attualmente risolvono a revertedKey
    for (const s of defaultShortcuts) {
      if (s.id === currentId || processed.has(s.id) || !shortcutsShareScope(s, defaultDef)) continue;

      const currentKey = overrides[s.id] || s.defaultKey;
      if (currentKey === revertedKey) {
        resetQueue.push(s.id);
      }
    }
  }

  vestaConfig.setItem("srt-tools-shortcut-overrides", JSON.stringify(overrides));
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("shortcuts-updated"));
  }
}

export function getSortedKeys(keyStr: string): string[] {
  if (!keyStr) return [];
  const keys = keyStr.split("+").map(k => k.trim());
  const order = ["Ctrl", "Alt", "Shift"];
  keys.sort((a, b) => {
    const idxA = order.indexOf(a);
    const idxB = order.indexOf(b);
    if (idxA !== -1 && idxB !== -1) {
      return idxA - idxB;
    }
    if (idxA !== -1) return -1;
    if (idxB !== -1) return 1;
    return a.localeCompare(b);
  });
  return keys;
}

/**
 * Localized label for a single shortcut key part.
 *
 * Modifier and special keys (Ctrl, Shift, Enter, Space, arrows, ...) are looked
 * up under the `keys.*` namespace so each language can show its own convention
 * (e.g. German `Strg`/`Umschalt`, French `Maj`/`Entrée`). Letters, digits and
 * any key without a `keys.*` entry fall back to the raw part. The translator is
 * injected so this module stays free of any i18n import.
 */
export function formatKeyPart(part: string, translate: (key: string) => string): string {
  const key = `keys.${part.toLowerCase()}`;
  const label = translate(key);
  return label === key ? part : label;
}

function matchesKey(event: KeyboardEvent, binding: string): boolean {
  const parts = binding.toLowerCase().split("+");
  const key = parts.at(-1);
  return (parts.includes("ctrl") === (event.ctrlKey || event.metaKey))
    && parts.includes("alt") === event.altKey
    && parts.includes("shift") === event.shiftKey
    && (key === "space" ? event.key === " " : key === event.key.toLowerCase());
}

/** Resolve editable shortcuts at dispatch time so rebinding takes effect immediately. */
export function matchesConfiguredShortcut(event: KeyboardEvent, id: string): boolean {
  const shortcut = getShortcuts().find(shortcut => shortcut.id === id);
  return !!shortcut && matchesKey(event, shortcut.defaultKey);
}

export function localShortcutAction(event: KeyboardEvent, category: Exclude<ShortcutDefinition["category"], "global">): string | undefined {
  if (event.defaultPrevented || event.isComposing) return;
  return getShortcuts().find(shortcut => shortcut.category === category && matchesKey(event, shortcut.defaultKey))?.action;
}

export function historyShortcutAction(event: KeyboardEvent, scope: "sync" | "align" | "flashcards" | "refine"): "undo" | "redo" | null {
  if (event.defaultPrevented || event.isComposing) return null;
  if (matchesConfiguredShortcut(event, `${scope}-undo`)) return "undo";
  if (matchesConfiguredShortcut(event, `${scope}-redo`)) return "redo";
  // Ctrl+Y is a conventional alias while redo retains its default binding.
  const redo = getShortcuts().find(shortcut => shortcut.id === `${scope}-redo`);
  const category = scope === "align" ? "alignment" : scope;
  if (!localShortcutAction(event, category) && redo?.defaultKey === "Ctrl+Shift+Z" && (event.ctrlKey || event.metaKey)
    && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "y") return "redo";
  return null;
}
