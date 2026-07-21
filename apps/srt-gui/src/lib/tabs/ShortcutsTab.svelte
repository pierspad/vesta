<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { locale, currentLanguage } from "$lib/i18n";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import FooterActions from "$lib/components/FooterActions.svelte";
  import {
    defaultShortcuts,
    formatKeyPart,
    getShortcuts,
    getSortedKeys,
    resetShortcuts,
    resetSingleShortcut,
    saveShortcutOverride,
    type ShortcutDefinition,
  } from "$lib/utils/shortcuts";

  let shortcuts = $state<ShortcutDefinition[]>([]);
  let editingShortcut = $state<string | null>(null);
  let recordingKey = $state(false);
  let showResetAllConfirm = $state(false);
  let selectedCategories = $state<string[]>([]);
  let searchQuery = $state<string>("");

  let searchMode = $state<"text" | "keys">("text");
  let searchKeys = $state<string[]>([]);
  let isListeningForSearchKeys = $state(false);



  const categoryOrder = ["global", "flashcards", "translate", "sync", "alignment", "transcribe"];

  let t = $derived($locale);

  let labels = $derived((() => {
    const lang = $currentLanguage || "en";
    const dict = {
      it: { text: "TESTO", keys: "TASTI", textPlaceholder: "Cerca per testo...", keysPlaceholder: "Cerca per tasti...", pressKeys: "Premi i tasti..." },
      es: { text: "TEXTO", keys: "TECLAS", textPlaceholder: "Buscar por texto...", keysPlaceholder: "Buscar por teclas...", pressKeys: "Presiona las teclas..." },
      fr: { text: "TEXTE", keys: "TOUCHES", textPlaceholder: "Rechercher par texte...", keysPlaceholder: "Rechercher par touches...", pressKeys: "Appuyez sur les touches..." },
      pt: { text: "TEXTO", keys: "TECLAS", textPlaceholder: "Pesquisar por texto...", keysPlaceholder: "Pesquisar por teclas...", pressKeys: "Pressione as teclas..." },
      en: { text: "TEXT", keys: "KEYS", textPlaceholder: "Search by text...", keysPlaceholder: "Search by keys...", pressKeys: "Press keys..." }
    };
    return (dict as Record<string, typeof dict.en>)[lang] || dict["en"];
  })());

  function fuzzyMatch(text: string, query: string): boolean {
    if (!query) return true;
    if (!text) return false;
    
    const q = query.toLowerCase().replace(/\s+/g, "");
    const t = text.toLowerCase();
    
    let queryIdx = 0;
    for (let i = 0; i < t.length; i++) {
      if (t[i] === q[queryIdx]) {
        queryIdx++;
        if (queryIdx === q.length) {
          return true;
        }
      }
    }
    return false;
  }

  function toggleSearchMode() {
    searchMode = searchMode === "text" ? "keys" : "text";
    if (searchMode === "text") {
      clearSearchKeys();
    }
  }

  function setSearchMode(mode: "text" | "keys") {
    searchMode = mode;
    if (mode === "text") {
      clearSearchKeys();
    }
  }

  let globalSearchKeysHandler: ((e: KeyboardEvent) => void) | null = null;

  function startSearchKeysListening() {
    isListeningForSearchKeys = true;
    if (globalSearchKeysHandler) {
      window.removeEventListener("keydown", globalSearchKeysHandler, true);
    }
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        stopSearchKeysListening();
        return;
      }

      e.preventDefault();
      e.stopPropagation();

      const parts: string[] = [];
      if (e.ctrlKey || e.metaKey) parts.push("Ctrl");
      if (e.altKey) parts.push("Alt");
      if (e.shiftKey) parts.push("Shift");

      let keyName = e.key;
      if (keyName === " ") keyName = "Space";
      else if (keyName.length === 1) keyName = keyName.toUpperCase();
      else if (keyName.startsWith("Arrow")) keyName = keyName;

      if (!["Control", "Alt", "Shift", "Meta"].includes(e.key)) {
        if (!parts.includes(keyName)) {
          parts.push(keyName);
        }
      }

      searchKeys = parts;
    };
    globalSearchKeysHandler = handler;
    window.addEventListener("keydown", handler, true);
  }

  function stopSearchKeysListening() {
    isListeningForSearchKeys = false;
    if (globalSearchKeysHandler) {
      window.removeEventListener("keydown", globalSearchKeysHandler, true);
      globalSearchKeysHandler = null;
    }
  }

  function clearSearchKeys() {
    searchKeys = [];
    stopSearchKeysListening();
  }

  let globalSearchClickHandler: ((e: MouseEvent) => void) | null = null;

  $effect(() => {
    if (isListeningForSearchKeys) {
      const clickHandler = () => {
        stopSearchKeysListening();
      };
      globalSearchClickHandler = clickHandler;
      window.addEventListener("click", clickHandler);
    } else {
      if (globalSearchClickHandler) {
        window.removeEventListener("click", globalSearchClickHandler);
        globalSearchClickHandler = null;
      }
    }
  });

  onDestroy(() => {
    if (globalSearchKeysHandler) {
      window.removeEventListener("keydown", globalSearchKeysHandler, true);
    }
    if (globalSearchClickHandler) {
      window.removeEventListener("click", globalSearchClickHandler);
    }
  });

  let filteredShortcuts = $derived.by(() => {
    let list = [...shortcuts];
    // 1. Filter by active category
    if (selectedCategories.length > 0) {
      list = list.filter((s) => selectedCategories.includes(s.category));
    }
    // 2. Filter by search query / keys
    if (searchMode === "text") {
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase().trim();
        list = list.filter((s) => {
          const desc = t(s.description);
          const key = s.defaultKey;
          if (desc.toLowerCase().includes(q) || key.toLowerCase().includes(q)) {
            return true;
          }
          return fuzzyMatch(desc, q) || fuzzyMatch(key, q);
        });
      }
    } else {
      if (searchKeys.length > 0) {
        list = list.filter((s) => {
          const shortcutValue = s.defaultKey.toUpperCase();
          const shortcutParts = shortcutValue.split("+").map((p) => p.trim());
          const searchUpper = searchKeys.map((k) => k.toUpperCase());
          return searchUpper.every((sk) => shortcutParts.some((sp) => sp.includes(sk)));
        });
      }
    }
    // 3. Sort by category and then alphabetically by localized description
    list.sort((a, b) => {
      const catA = categoryOrder.indexOf(a.category);
      const catB = categoryOrder.indexOf(b.category);
      if (catA !== catB) {
        return catA - catB;
      }
      return t(a.description).localeCompare(t(b.description), undefined, {
        sensitivity: "base",
      });
    });
    return list;
  });

  const categoryLabels: Record<string, string> = {
    global: "shortcuts.category.global",
    flashcards: "shortcuts.category.flashcards",
    translate: "shortcuts.category.translate",
    sync: "shortcuts.category.sync",
    alignment: "shortcuts.category.alignment",
    transcribe: "shortcuts.category.transcribe",
  };

  function categoryText(category: string): string {
    return t(categoryLabels[category]).replace(/^[^\p{L}\p{N}]+/u, "").trim();
  }

  let categoryFilterItems = $derived(
    Object.keys(categoryLabels).sort((a, b) =>
      categoryText(a).localeCompare(categoryText(b), undefined, {
        sensitivity: "base",
      }),
    ),
  );

  onMount(() => {
    shortcuts = getShortcuts();
  });

  let currentKeyHandler: ((e: KeyboardEvent) => void) | null = null;

  function startEditing(shortcutId: string) {
    if (editingShortcut === shortcutId) return;

    if (editingShortcut !== null) {
      cancelEditing();
    }

    editingShortcut = shortcutId;
    recordingKey = true;

    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      const parts: string[] = [];
      if (e.ctrlKey || e.metaKey) parts.push("Ctrl");
      if (e.altKey) parts.push("Alt");
      if (e.shiftKey) parts.push("Shift");

      let keyName = e.key;
      if (keyName === " ") keyName = "Space";
      else if (keyName.length === 1) keyName = keyName.toUpperCase();
      else if (keyName.startsWith("Arrow")) keyName = keyName;

      if (!["Control", "Alt", "Shift", "Meta"].includes(e.key)) {
        parts.push(keyName);
        const newKey = parts.join("+");

        const conflict = shortcuts.find(
          (s) => s.id !== editingShortcut && s.defaultKey === newKey,
        );

        if (conflict) {
          snackbar.show(
            t("shortcuts.conflict", {
              key: newKey,
              action: t(conflict.description),
            }),
            "error",
            3000,
          );
        } else {
          saveShortcutOverride(editingShortcut!, newKey);
          shortcuts = getShortcuts();
          snackbar.show(t("shortcuts.updated", { key: newKey }), "success", 2500);
        }

        editingShortcut = null;
        recordingKey = false;
        window.removeEventListener("keydown", handler, true);
        currentKeyHandler = null;
      }
    };

    currentKeyHandler = handler;
    window.addEventListener("keydown", handler, true);
  }

  function cancelEditing() {
    if (currentKeyHandler) {
      window.removeEventListener("keydown", currentKeyHandler, true);
      currentKeyHandler = null;
    }
    editingShortcut = null;
    recordingKey = false;
  }

  function confirmResetToDefaults() {
    showResetAllConfirm = false;
    resetShortcuts();
    shortcuts = getShortcuts();
    snackbar.show(t("shortcuts.reset"), "success", 2500);
  }

  function getDefaultKey(shortcutId: string): string {
    return defaultShortcuts.find((s) => s.id === shortcutId)?.defaultKey || "";
  }

  function isModified(shortcut: ShortcutDefinition): boolean {
    const defaultKey = getDefaultKey(shortcut.id);
    return shortcut.defaultKey !== defaultKey;
  }

  function resetSingle(shortcutId: string) {
    resetSingleShortcut(shortcutId);
    shortcuts = getShortcuts();
    snackbar.show(t("shortcuts.resetSingle"), "success", 2500);
  }

  function toggleCategory(cat: string, event: MouseEvent) {
    const isCtrl = event.ctrlKey || event.metaKey;
    if (isCtrl) {
      if (selectedCategories.includes(cat)) {
        selectedCategories = selectedCategories.filter((c) => c !== cat);
      } else {
        selectedCategories = [...selectedCategories, cat];
      }
    } else {
      if (selectedCategories.length === 1 && selectedCategories[0] === cat) {
        selectedCategories = [];
      } else {
        selectedCategories = [cat];
      }
    }
  }

  function getShortcutIcon(id: string): string {
    // Return custom inline SVG based on ID
    if (id.includes("open-file") || id.includes("load-session")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" /></svg>`;
    }
    if (id.includes("save-session") || id.includes("save")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4H8m-2 4h10m-5-8V3m0 0l-3 3m3-3l3 3" /></svg>`;
    }
    if (id.includes("start") || id.includes("generate")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" /><path stroke-linecap="round" stroke-linejoin="round" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>`;
    }
    if (id.includes("cancel")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" /></svg>`;
    }
    if (id.includes("preview")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /><path stroke-linecap="round" stroke-linejoin="round" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" /></svg>`;
    }
    if (id.includes("clear-logs")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>`;
    }
    if (id.includes("add-key") || id.includes("new")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4v16m8-8H4" /></svg>`;
    }
    if (id.includes("show-help")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>`;
    }
    if (id === "tab-flashcards") {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" /></svg>`;
    }
    if (id === "tab-refine") {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" /><path stroke-linecap="round" stroke-linejoin="round" d="M19 8l1.5-1.5M19 4l1.5 1.5M16 5l1.5-1.5" /></svg>`;
    }
    if (id === "tab-translate") {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129" /></svg>`;
    }
    if (id === "tab-sync") {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>`;
    }
    if (id === "tab-align") {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M4 6h16M4 12h16m-7 6h7"/></svg>`;
    }
    if (id === "tab-transcribe") {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" /></svg>`;
    }
    if (id === "tab-settings") {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" /><path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg>`;
    }
    if (id === "tab-shortcuts") {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M4 7a3 3 0 013-3h10a3 3 0 013 3v10a3 3 0 01-3 3H7a3 3 0 01-3-3V7zm4 2h2m2 0h2m2 0h2M7 13h2m2 0h2m2 0h2M7 17h6" /></svg>`;
    }
    if (id === "sync-auto") {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" /></svg>`;
    }
    if (id.includes("play-pause")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" /><path stroke-linecap="round" stroke-linejoin="round" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>`;
    }
    if (id === "sync-seek-back-fast") {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M11 19l-7-7 7-7m8 14l-7-7 7-7" /></svg>`;
    }
    if (id === "sync-seek-forward-fast") {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M13 5l7 7-7 7M5 5l7 7-7 7" /></svg>`;
    }
    if (id === "sync-seek-back" || id.includes("prev-sub")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M15 19l-7-7 7-7" /></svg>`;
    }
    if (id === "sync-seek-forward" || id.includes("next-sub")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" /></svg>`;
    }
    if (id.includes("offset-up-fast") || id === "sync-prev-anchor") {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M5 11l7-7 7 7M5 19l7-7 7 7" /></svg>`;
    }
    if (id.includes("offset-down-fast") || id === "sync-next-anchor") {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M19 13l-7 7-7-7m14-8l-7 7-7-7" /></svg>`;
    }
    if (id.includes("offset-up") || id.includes("prev-page")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M5 15l7-7 7 7" /></svg>`;
    }
    if (id.includes("offset-down") || id.includes("next-page")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" /></svg>`;
    }
    if (id.includes("undo")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" /></svg>`;
    }
    if (id.includes("confirm")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" /></svg>`;
    }
    if (id.includes("suggested")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" /><path stroke-linecap="round" stroke-linejoin="round" d="M15 11a3 3 0 11-6 0 3 3 0 016 0z" /></svg>`;
    }
    if (id.includes("swap")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4" /></svg>`;
    }
    if (id.includes("cycle")) {
      return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M4 6h16M4 12h16M4 18h16" /></svg>`;
    }
    // Fallback: keyboard icon
    return `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9" /></svg>`;
  }
</script>

{#snippet shortcutRow(shortcut: ShortcutDefinition)}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="flex items-center justify-between p-4 rounded-xl border transition-all duration-200 cursor-pointer
      {editingShortcut === shortcut.id
      ? 'bg-red-500/10 border-red-500/50 shadow-[0_0_15px_rgba(239,68,68,0.25)] text-white font-semibold'
      : 'bg-white/5 border-white/5 hover:bg-white/10 hover:border-white/10 text-gray-300 hover:text-white'}"
    onclick={() => startEditing(shortcut.id)}
  >
    <div class="flex items-center gap-3 min-w-0 flex-1 pr-3">
      <!-- Custom shortcut icon -->
      <div class="shrink-0 text-gray-400 group-hover:text-white transition-colors flex items-center justify-center">
        {@html getShortcutIcon(shortcut.id)}
      </div>

      <div class="min-w-0 flex-1">
        <p class="text-sm font-semibold truncate">{t(shortcut.description)}</p>
        {#if isModified(shortcut)}
          <p class="text-[10px] text-amber-400/80 mt-0.5 font-medium">
            {t("shortcuts.modified", { key: getDefaultKey(shortcut.id) })}
          </p>
        {/if}
      </div>
    </div>

    <div class="flex items-center gap-2 shrink-0">
      {#if editingShortcut === shortcut.id}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="flex items-center gap-2 text-xs text-red-100 font-bold animate-pulse bg-red-600/20 border border-red-500/40 px-3 py-1.5 rounded-lg cursor-default"
          onclick={(e) => e.stopPropagation()}
        >
          <span>{t("shortcuts.recordingKeys")}</span>
          <button
            onclick={(e) => {
              e.stopPropagation();
              cancelEditing();
            }}
            class="text-red-300 hover:text-white p-0.5 transition-colors focus:outline-none cursor-pointer"
            aria-label="Cancel"
          >
            <svg
              class="w-3.5 h-3.5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2.5"
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>
      {:else}
        <div class="flex items-center gap-1 flex-wrap">
          {#each getSortedKeys(shortcut.defaultKey) as key, i}
            {#if i > 0}
              <span class="text-gray-500 text-xs font-semibold px-0.5">+</span>
            {/if}
            <kbd
              class="px-2.5 py-1 bg-gray-950/80 border border-white/10 rounded-lg text-xs text-gray-300 font-mono shadow-sm font-semibold tracking-wide whitespace-nowrap"
            >
              {formatKeyPart(key, t)}
            </kbd>
          {/each}
        </div>

        {#if isModified(shortcut)}
          <button
            onclick={(e) => {
              e.stopPropagation();
              resetSingle(shortcut.id);
            }}
            class="p-1.5 text-amber-400 hover:text-amber-300 hover:bg-amber-500/10 rounded-lg transition-all cursor-pointer"
            title={t("shortcuts.resetDefault")}
          >
            <svg
              class="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
              />
            </svg>
          </button>
        {/if}
      {/if}
    </div>
  </div>
{/snippet}

<div
  class="h-full flex flex-col bg-gray-900 overflow-hidden"
  style="contain: layout style; will-change: transform;"
>
  <!-- Top Bar with Category Filters & Search Input aligned exactly to Vesta top heights -->
  <div class="min-h-[89px] py-4 px-6 shrink-0 flex flex-wrap items-center justify-between gap-y-3 gap-x-4 border-b border-white/10 bg-gray-900">
    <!-- Left side: Category buttons -->
    <div class="flex items-center gap-4 min-w-0">
      <div class="flex flex-wrap gap-2 items-center">
        {#each categoryFilterItems as cat}
          <button
            onclick={(e) => toggleCategory(cat, e)}
            class="px-4 py-2 rounded-lg text-sm font-medium transition-all duration-200 border
              {selectedCategories.includes(cat)
              ? 'bg-gradient-to-r from-indigo-500 to-purple-600 text-white border-indigo-500/50 shadow-md shadow-indigo-500/20'
              : 'bg-white/5 text-gray-400 border-transparent hover:bg-white/10 hover:text-white'}"
          >
            {categoryText(cat)}
          </button>
        {/each}
      </div>
    </div>

    <!-- Right side: Unified Search Panel -->
    <div class="flex items-center gap-3 shrink-0">
      <!-- Unified Search Container -->
      <div class="flex items-center bg-white/5 border rounded-xl p-0.5 transition-all w-96 relative
        {isListeningForSearchKeys ? 'border-indigo-500/50 ring-1 ring-indigo-500/30 bg-indigo-500/5' : 'border-white/10 focus-within:border-indigo-500/50 focus-within:ring-1 focus-within:ring-indigo-500/30'}"
      >
        <!-- Search Icon (Left) -->
        <div class="pl-2.5 pr-2 flex items-center justify-center shrink-0">
          <svg class="w-4 h-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
        </div>

        <!-- Middle input area -->
        <div class="flex-1 min-w-0 pr-8 relative">
          {#if searchMode === "text"}
            <input
              type="text"
              bind:value={searchQuery}
              placeholder={labels.textPlaceholder}
              class="w-full h-8 py-0 bg-transparent border-0 text-sm text-white focus:outline-none placeholder-gray-500"
            />
            {#if searchQuery}
              <button
                onclick={() => (searchQuery = "")}
                class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-500 hover:text-white transition-colors"
                aria-label="Clear search"
              >
                ✕
              </button>
            {/if}
          {:else}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              onclick={(e) => {
                e.stopPropagation();
                if (isListeningForSearchKeys) {
                  stopSearchKeysListening();
                } else {
                  startSearchKeysListening();
                }
              }}
              class="w-full h-8 py-0 text-sm text-white flex items-center cursor-pointer select-none"
            >
              <div class="flex-1 flex flex-wrap items-center gap-1 min-w-0">
                {#if searchKeys.length > 0}
                  <div class="flex items-center gap-0.5 flex-wrap">
                    {#each searchKeys as key, i}
                      {#if i > 0}
                        <span class="text-gray-500 text-[10px]">+</span>
                      {/if}
                      <kbd class="px-1.5 py-0.5 bg-gray-950 border border-white/15 rounded text-[10px] text-gray-300 font-mono font-semibold shadow-sm tracking-wide whitespace-nowrap">
                        {formatKeyPart(key, t)}
                      </kbd>
                    {/each}
                    {#if isListeningForSearchKeys}
                      <span class="w-[1.5px] h-3.5 bg-indigo-400 animate-pulse ml-0.5 shrink-0"></span>
                    {/if}
                  </div>
                {:else}
                  <input
                    type="text"
                    readonly
                    placeholder={isListeningForSearchKeys ? labels.pressKeys : labels.keysPlaceholder}
                    class="w-full h-8 py-0 bg-transparent border-0 text-sm focus:outline-none cursor-pointer select-none pointer-events-none
                      {isListeningForSearchKeys ? 'placeholder-indigo-400 font-medium animate-pulse' : 'placeholder-gray-500'}"
                  />
                {/if}
              </div>
              {#if searchKeys.length > 0 || isListeningForSearchKeys}
                <button
                  onclick={(e) => {
                    e.stopPropagation();
                    clearSearchKeys();
                  }}
                  class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-500 hover:text-white transition-colors"
                  aria-label="Clear search keys"
                >
                  ✕
                </button>
              {/if}
            </div>
          {/if}
        </div>

        <!-- Switch Buttons (Right side inside the search input) -->
        <div class="flex items-center bg-white/5 border border-white/10 rounded-lg p-0.5 shrink-0 select-none mr-0.5">
          <button
            onclick={toggleSearchMode}
            class="px-2 py-1 rounded-md text-[9px] font-bold transition-all duration-200 flex items-center gap-1 cursor-pointer
              {searchMode === 'text'
                ? 'bg-indigo-600 text-white shadow shadow-indigo-600/20'
                : 'text-gray-400 hover:text-white hover:bg-white/5'}"
            title={labels.textPlaceholder}
          >
            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M4 6h16M4 12h16M4 18h7" />
            </svg>
            <span>{labels.text}</span>
          </button>
          <button
            onclick={toggleSearchMode}
            class="px-2 py-1 rounded-md text-[9px] font-bold transition-all duration-200 flex items-center gap-1 cursor-pointer
              {searchMode === 'keys'
                ? 'bg-indigo-600 text-white shadow shadow-indigo-600/20'
                : 'text-gray-400 hover:text-white hover:bg-white/5'}"
            title={labels.keysPlaceholder}
          >
            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M3 10h18M7 15h1m4 0h1-4m8 0h1m-9 0H3m18 0h-3M5 5h14a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2z" />
            </svg>
            <span>{labels.keys}</span>
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- Scrollable grid area for shortcut cards -->
  <div class="px-6 py-6 flex-1 overflow-y-auto min-h-0">
    <div class="grid gap-4 grid-cols-1 md:grid-cols-2 xl:grid-cols-3 content-start">
      {#each filteredShortcuts as shortcut}
        {@render shortcutRow(shortcut)}
      {/each}
    </div>
  </div>

  <!-- Fixed Bottom Band: reset è distruttivo e raro, quindi piccolo e in un
       angolo (con conferma) — stesso trattamento del bottone di Settings,
       non più un bottone rosso gigante centrato. -->
  <FooterActions justify="end">
    {#snippet right()}
      <button
        onclick={() => (showResetAllConfirm = true)}
        class="px-3.5 py-2 bg-red-500/10 hover:bg-red-500/20 border border-red-500/30 text-red-300 rounded-lg font-semibold text-xs transition-colors flex items-center gap-1.5 cursor-pointer"
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
          />
        </svg>
        {t("settings.resetDefaults") || "Ripristina predefiniti"}
      </button>
    {/snippet}
  </FooterActions>

  <ConfirmDialog
    show={showResetAllConfirm}
    title={t("settings.resetDefaults") || "Ripristinare i valori predefiniti?"}
    message={t("shortcuts.confirmReset") || "Tutte le scorciatoie personalizzate verranno ripristinate ai valori predefiniti."}
    confirmText="OK"
    cancelText={t("sync.cancelReset") || "Annulla"}
    variant="danger"
    on:cancel={() => (showResetAllConfirm = false)}
    on:confirm={confirmResetToDefaults}
  />
</div>
