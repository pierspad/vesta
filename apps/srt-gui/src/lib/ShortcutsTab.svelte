<script lang="ts">
  import { onMount } from "svelte";
  import { locale } from "./i18n";
  import {
    defaultShortcuts,
    getShortcuts,
    resetShortcuts,
    saveShortcutOverride,
    type ShortcutDefinition,
  } from "./models";

  let shortcuts = $state<ShortcutDefinition[]>([]);
  let editingShortcut = $state<string | null>(null);
  let recordingKey = $state(false);
  let success = $state<string | null>(null);
  let error = $state<string | null>(null);
  let showResetAllConfirm = $state(false);
  let filter = $state<"all" | "global" | "translate" | "sync" | "flashcards" | "alignment" | "transcribe">(
    "all",
  );

  let t = $derived($locale);

  let filteredShortcuts = $derived(
    filter === "all"
      ? shortcuts
      : shortcuts.filter((s) => s.category === filter),
  );

  let groupedShortcuts = $derived.by(() => {
    const groups: Record<string, ShortcutDefinition[]> = {
      global: [],
      flashcards: [],
      translate: [],
      sync: [],
      alignment: [],
      transcribe: [],
    };
    filteredShortcuts.forEach((s) => {
      if (groups[s.category]) {
        groups[s.category].push(s);
      }
    });
    return groups;
  });

  let syncShortcutsLeft = $derived.by(() => {
    const syncShorts = groupedShortcuts.sync;
    return syncShorts.slice(0, Math.ceil(syncShorts.length / 2));
  });

  let syncShortcutsRight = $derived.by(() => {
    const syncShorts = groupedShortcuts.sync;
    return syncShorts.slice(Math.ceil(syncShorts.length / 2));
  });

  const categoryLabels: Record<string, string> = {
    global: "shortcuts.category.global",
    flashcards: "shortcuts.category.flashcards",
    translate: "shortcuts.category.translate",
    sync: "shortcuts.category.sync",
    alignment: "shortcuts.category.alignment",
    transcribe: "shortcuts.category.transcribe",
  };

  const categoryIcons: Record<string, string> = {
    alignment: "🛠️",
    flashcards: "🃏",
    global: "🌐",
    sync: "⏱️",
    transcribe: "🎙️",
    translate: "🌍",
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

  const categoryDescriptions: Record<string, string> = {
    global: "shortcuts.category.global.desc",
    flashcards: "shortcuts.category.flashcards.desc",
    translate: "shortcuts.category.translate.desc",
    sync: "shortcuts.category.sync.desc",
    alignment: "shortcuts.category.alignment.desc",
    transcribe: "shortcuts.category.transcribe.desc",
  };

  onMount(() => {
    shortcuts = getShortcuts();
  });

  let currentKeyHandler: ((e: KeyboardEvent) => void) | null = null;

  function startEditing(shortcutId: string) {
    if (currentKeyHandler) {
      window.removeEventListener("keydown", currentKeyHandler);
      currentKeyHandler = null;
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
          error = t("shortcuts.conflict", {
            key: newKey,
            action: t(conflict.description),
          });
          setTimeout(() => (error = null), 3000);
        } else {
          saveShortcutOverride(editingShortcut!, newKey);
          shortcuts = getShortcuts();
          success = t("shortcuts.updated", { key: newKey });
          setTimeout(() => (success = null), 3000);
        }

        editingShortcut = null;
        recordingKey = false;
        window.removeEventListener("keydown", handler);
        currentKeyHandler = null;
      }
    };

    currentKeyHandler = handler;
    window.addEventListener("keydown", handler);
  }

  function cancelEditing() {
    if (currentKeyHandler) {
      window.removeEventListener("keydown", currentKeyHandler);
      currentKeyHandler = null;
    }
    editingShortcut = null;
    recordingKey = false;
  }

  function confirmResetToDefaults() {
    showResetAllConfirm = false;
    resetShortcuts();
    shortcuts = getShortcuts();
    success = t("shortcuts.reset");
    setTimeout(() => (success = null), 3000);
  }

  function getDefaultKey(shortcutId: string): string {
    return defaultShortcuts.find((s) => s.id === shortcutId)?.defaultKey || "";
  }

  function isModified(shortcut: ShortcutDefinition): boolean {
    const defaultKey = getDefaultKey(shortcut.id);
    return shortcut.defaultKey !== defaultKey;
  }

  function resetSingle(shortcutId: string) {
    const defaultKey = getDefaultKey(shortcutId);
    saveShortcutOverride(shortcutId, defaultKey);
    shortcuts = getShortcuts();
    success = t("shortcuts.resetSingle");
    setTimeout(() => (success = null), 3000);
  }
</script>

{#snippet shortcutRow(shortcut: ShortcutDefinition)}
  <div
    class="flex items-center justify-between p-3 rounded-lg transition-colors
      {editingShortcut === shortcut.id
      ? 'bg-indigo-500/20 ring-1 ring-indigo-500'
      : 'bg-white/5 hover:bg-white/10'}"
    style="content-visibility: auto; contain-intrinsic-size: auto 52px;"
  >
    <div class="flex-1">
      <p class="text-sm text-white">{t(shortcut.description)}</p>
      {#if isModified(shortcut)}
        <p class="text-xs text-amber-400 mt-1">
          {t("shortcuts.modified", { key: getDefaultKey(shortcut.id) })}
        </p>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      {#if editingShortcut === shortcut.id}
        <div class="flex items-center gap-2 text-indigo-300 animate-pulse">
          <span class="text-sm">{t("shortcuts.pressKeys")}</span>
          <button
            onclick={cancelEditing}
            class="text-gray-400 hover:text-white p-1"
            aria-label="Cancel"
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
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>
      {:else}
        <div class="flex items-center gap-1">
          {#each shortcut.defaultKey.split("+") as keyPart, i}
            {#if i > 0}
              <span class="text-gray-500 text-xs font-bold">+</span>
            {/if}
            <kbd
              class="px-2 py-1 bg-gray-800 border border-gray-600 rounded text-xs text-gray-300 font-mono shadow-md"
            >
              {keyPart}
            </kbd>
          {/each}
        </div>

        <button
          onclick={() => startEditing(shortcut.id)}
          class="p-1.5 text-gray-500 hover:text-indigo-400 hover:bg-white/5 rounded transition-colors"
          title={t("shortcuts.editShortcut")}
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
              d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
            />
          </svg>
        </button>

        {#if isModified(shortcut)}
          <button
            onclick={() => resetSingle(shortcut.id)}
            class="p-1.5 text-gray-500 hover:text-amber-400 hover:bg-white/5 rounded transition-colors"
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
  class="h-full flex flex-col p-6 overflow-y-auto bg-gradient-to-br from-gray-900 via-gray-900 to-gray-950"
  style="contain: layout style; will-change: transform;"
>
  {#if error}
    <div
      class="mb-4 p-4 bg-red-500/10 border border-red-500/30 rounded-xl flex items-center gap-3 animate-fade-in"
    >
      <svg
        class="w-5 h-5 text-red-400"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
        />
      </svg>
      <span class="text-red-300 flex-1">{error}</span>
      <button
        onclick={() => (error = null)}
        class="text-red-400 hover:text-red-300">✕</button
      >
    </div>
  {/if}

  {#if success}
    <div
      class="mb-4 p-4 bg-green-500/10 border border-green-500/30 rounded-xl flex items-center gap-3 animate-fade-in"
    >
      <svg
        class="w-5 h-5 text-green-400"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M5 13l4 4L19 7"
        />
      </svg>
      <span class="text-green-300">{success}</span>
    </div>
  {/if}

  <div class="flex items-center gap-4 mb-6">
    <div class="flex items-center gap-2">
      <button
        onclick={() => (filter = "all")}
        class="px-4 py-2 rounded-lg text-sm font-medium transition-colors shadow-sm
          {filter === 'all'
          ? 'bg-indigo-500 text-white ring-1 ring-indigo-300/30'
          : 'bg-white/5 text-gray-400 hover:bg-white/10'}"
      >
        {t("shortcuts.filter.all")}
      </button>
      <span class="h-7 w-px bg-white/10" aria-hidden="true"></span>
      {#each categoryFilterItems as cat}
        <button
          onclick={() => (filter = cat as any)}
          class="px-4 py-2 rounded-lg text-sm font-medium transition-colors
            {filter === cat
            ? 'bg-indigo-500 text-white'
            : 'bg-white/5 text-gray-400 hover:bg-white/10'}"
        >
          {categoryText(cat)}
        </button>
      {/each}
    </div>

    <div class="flex-1"></div>

    <button onclick={() => (showResetAllConfirm = true)} class="btn-secondary py-2 px-4 text-sm">
      <svg
        class="w-4 h-4 inline mr-2"
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
      {t("shortcuts.resetAll")}
    </button>
  </div>

  <div class="grid grid-cols-1 xl:grid-cols-2 gap-6">
    {#each Object.entries(groupedShortcuts).filter(([cat]) => cat !== "sync") as [category, categoryShortcuts]}
      {#if categoryShortcuts.length > 0}
        <div class="glass-card p-5" style="content-visibility: auto; contain-intrinsic-size: auto 300px;">
          <div class="mb-4">
            <h3 class="text-lg font-semibold text-white flex items-center gap-2">
              <span aria-hidden="true">{categoryIcons[category]}</span>
              {categoryText(category)}
            </h3>
            <p class="text-xs text-gray-500 mt-1">
              {t(categoryDescriptions[category])}
            </p>
          </div>

          <div class="space-y-2">
            {#each categoryShortcuts as shortcut}
              {@render shortcutRow(shortcut)}
            {/each}
          </div>
        </div>
      {/if}
    {/each}
  </div>

  {#if groupedShortcuts.sync.length > 0}
    <div class="mt-6">
      <div class="glass-card p-5" style="content-visibility: auto; contain-intrinsic-size: auto 400px;">
        <div class="mb-4">
          <h3 class="text-lg font-semibold text-white flex items-center gap-2">
            <span aria-hidden="true">{categoryIcons.sync}</span>
            {categoryText("sync")}
          </h3>
          <p class="text-xs text-gray-500 mt-1">
            {t("shortcuts.category.sync.desc")}
          </p>
        </div>

        <div class="grid grid-cols-1 xl:grid-cols-2 gap-4">
          <div class="space-y-2">
            {#each syncShortcutsLeft as shortcut}
              {@render shortcutRow(shortcut)}
            {/each}
          </div>

          <div class="space-y-2">
            {#each syncShortcutsRight as shortcut}
              {@render shortcutRow(shortcut)}
            {/each}
          </div>
        </div>
      </div>
    </div>
  {/if}

  {#if showResetAllConfirm}
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={() => (showResetAllConfirm = false)}
      onkeydown={(e) => {
        if (e.key === "Escape") showResetAllConfirm = false;
      }}
    >
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="p-6 max-w-sm w-full mx-4 shadow-2xl border border-white/10 rounded-2xl"
        style="background: #1e1e2e;"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.stopPropagation()}
      >
        <h3 class="text-lg font-semibold text-white mb-2">
          {t("shortcuts.resetAll")}
        </h3>
        <p class="text-gray-400 text-sm mb-6">{t("shortcuts.confirmReset")}</p>
        <div class="flex gap-3 justify-end">
          <button
            onclick={() => (showResetAllConfirm = false)}
            class="btn-secondary py-2 px-5 text-sm"
          >
            {t("sync.cancelReset")}
          </button>
          <button onclick={confirmResetToDefaults} class="btn-danger py-2 px-5 text-sm">
            OK
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>
