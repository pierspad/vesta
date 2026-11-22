<script lang="ts">
  import { isInputActive } from "$lib/utils/keyboardFocus";
  import { onDestroy, onMount } from "svelte";
  import { locale } from "$lib/i18n";
  import { formatKeyPart, getShortcuts, getSortedKeys, type ShortcutDefinition } from "$lib/utils/shortcuts";

  interface Props {
    activeTab: string;
  }

  let { activeTab }: Props = $props();

  let t = $derived($locale);
  let visible = $state(false);
  let shortcuts = $state<ShortcutDefinition[]>([]);

  const tabToCategory: Record<string, string> = {
    flashcards: "flashcards",
    translate: "translate",
    sync: "sync",
    align: "alignment",
    settings: "settings",
    refine: "refine",
    shortcuts: "global",
    transcribe: "transcribe",
  };

  let contextShortcuts = $derived.by(() => {
    const category = tabToCategory[activeTab] || "global";
    const global = shortcuts.filter((s) => s.category === "global");
    const contextual =
      category !== "global"
        ? shortcuts.filter((s) => s.category === category)
        : [];
    return { global, contextual, category };
  });


  function handleKeyDown(e: KeyboardEvent) {
    if (e.defaultPrevented || e.isComposing || isInputActive()) {
      return;
    }

    // Toggle help overlay with Shift+F1 (or Shift+? alias)
    if (e.shiftKey && (e.key === "F1" || e.key === "?")) {
      e.preventDefault();
      visible = !visible;
      return;
    }

    if (e.key === "Escape" && visible) {
      e.preventDefault();
      visible = false;
      return;
    }
  }

  $effect(() => {
    if (visible) {
      shortcuts = getShortcuts();
    }
  });

  onMount(() => {
    shortcuts = getShortcuts();
    window.addEventListener("keydown", handleKeyDown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeyDown);
  });
</script>

{#if visible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 bg-black/40 z-[9998] backdrop-blur-sm"
    onclick={() => (visible = false)}
    onkeydown={(e) => e.key === "Escape" && (visible = false)}
  ></div>

  <div
    class="fixed bottom-4 right-4 z-[9999] w-[700px] max-w-[90vw] max-h-[85vh] overflow-y-auto bg-gray-900 border border-white/10 rounded-2xl shadow-2xl animate-fade-in"
  >
    <div
      class="sticky top-0 bg-gray-900/95 backdrop-blur-sm border-b border-white/10 p-4 flex items-center justify-between gap-3 rounded-t-2xl"
    >
      <div class="flex items-center gap-3 min-w-0">
        <div class="w-8 h-8 rounded-lg bg-indigo-500/15 border border-indigo-400/25 flex items-center justify-center text-indigo-300 shrink-0" aria-hidden="true">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
            <rect x="3" y="5" width="18" height="14" rx="2.5" />
            <path stroke-linecap="round" d="M7 9h2m3 0h2m3 0h.01M7 13h2m3 0h2m3 0h.01M8 16h8" />
          </svg>
        </div>
        <div class="min-w-0">
          <div class="flex items-center gap-2 flex-wrap">
            <h3 class="text-sm font-semibold text-white">
              {t("shortcuts.overlay.title")}
            </h3>
            <span class="inline-flex items-center gap-0.5 rounded-md border border-indigo-400/30 bg-indigo-500/15 px-1.5 py-0.5 font-mono text-[10px] text-indigo-200">
              <kbd>{formatKeyPart("Shift", t)}</kbd>
              <span class="text-indigo-400/70">+</span>
              <kbd>{formatKeyPart("F1", t)}</kbd>
            </span>
          </div>
          <p class="text-[11px] text-gray-400 mt-0.5">
            {t("shortcuts.overlay.hint")}
          </p>
        </div>
      </div>
      <button
        onclick={() => (visible = false)}
        class="p-1.5 text-gray-400 hover:text-white hover:bg-white/10 rounded-lg transition-colors"
        aria-label={t("common.close")}
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

    <div class="p-4 space-y-4">
      {#if contextShortcuts.contextual.length > 0}
        <div>
          <h4
            class="text-xs font-semibold text-emerald-400 uppercase tracking-wider mb-2"
          >
            {t(contextShortcuts.category === "settings" || contextShortcuts.category === "refine" ? `nav.${contextShortcuts.category}` : `shortcuts.category.${contextShortcuts.category}`)}
          </h4>
          <div class="grid grid-cols-2 gap-2">
            {#each contextShortcuts.contextual as shortcut}
              <div
                class="flex items-center justify-between py-1.5 px-3 rounded-lg bg-white/5"
              >
                <span class="text-xs text-gray-300"
                  >{t(shortcut.description)}</span
                >
                <div class="flex items-center gap-0.5">
                  {#each getSortedKeys(shortcut.defaultKey) as keyPart, i}
                    {#if i > 0}
                      <span class="text-gray-600 text-[9px]">+</span>
                    {/if}
                    <kbd
                      class="px-1.5 py-0.5 bg-gray-800 border border-gray-700 rounded text-[10px] text-gray-400 font-mono"
                    >
                      {formatKeyPart(keyPart, t)}
                    </kbd>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      {#if contextShortcuts.global.length > 0}
        <div>
          <h4
            class="text-xs font-semibold text-indigo-400 uppercase tracking-wider mb-2"
          >
            {t("shortcuts.overlay.global")}
          </h4>
          <div class="grid grid-cols-2 gap-2">
            {#each contextShortcuts.global as shortcut}
              <div
                class="flex items-center justify-between py-1.5 px-3 rounded-lg bg-white/5"
              >
                <span class="text-xs text-gray-300"
                  >{t(shortcut.description)}</span
                >
                <div class="flex items-center gap-0.5">
                  {#each getSortedKeys(shortcut.defaultKey) as keyPart, i}
                    {#if i > 0}
                      <span class="text-gray-600 text-[9px]">+</span>
                    {/if}
                    <kbd
                      class="px-1.5 py-0.5 bg-gray-800 border border-gray-700 rounded text-[10px] text-gray-400 font-mono"
                    >
                      {formatKeyPart(keyPart, t)}
                    </kbd>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}
