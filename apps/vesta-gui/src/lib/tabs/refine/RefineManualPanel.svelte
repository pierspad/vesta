<script lang="ts">
  import { locale } from "$lib/i18n";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import CodeEditor from "$lib/components/CodeEditor.svelte";
  import type { RefineCard } from "./RefineCardListPanel.svelte";

  interface Props {
    selectedCard: RefineCard | null;
    selectedCardIndex: number | null;
    selectedFilteredIndex: number;
    filteredCount: number;
    plainText?: { expression: string; meaning: string };
    notesValue: string;
    readonly: boolean;
    isGeneratingSelected: boolean;
    isSingleRefining: boolean;
    autoRefining: boolean;
    isStopping: boolean;
    generateDisabled: boolean;
    onStop: () => void;
    onGenerateSingle: () => void;
    onNavigate: (direction: -1 | 1) => void;
  }

  let {
    selectedCard,
    selectedCardIndex,
    selectedFilteredIndex,
    filteredCount,
    plainText,
    notesValue = $bindable(),
    readonly,
    isGeneratingSelected,
    isSingleRefining,
    autoRefining,
    isStopping,
    generateDisabled,
    onStop,
    onGenerateSingle,
    onNavigate,
  }: Props = $props();

  let t = $derived($locale);
</script>

<div class="flex flex-col gap-4 flex-1 min-h-0 pt-3">
  <div class="grid grid-cols-2 gap-3 shrink-0">
    <div class="bg-white/5 border border-white/10 rounded-xl p-3 relative h-[68px] overflow-hidden">
      <span class="absolute top-2 right-3 text-[9px] font-bold text-gray-500 uppercase tracking-wider">{t("refine.card.front")}</span>
      <div class="text-xs font-semibold text-gray-200 mt-1 line-clamp-2">
        {selectedCard ? (plainText?.expression || "—") : "—"}
      </div>
    </div>

    <div class="bg-white/5 border border-white/10 rounded-xl p-3 relative h-[68px] overflow-hidden">
      <span class="absolute top-2 right-3 text-[9px] font-bold text-gray-500 uppercase tracking-wider">{t("refine.card.back")}</span>
      <div class="text-xs font-semibold text-gray-200 mt-1 line-clamp-2">
        {selectedCard ? (plainText?.meaning || "—") : "—"}
      </div>
    </div>
  </div>

  <div class="flex-1 flex flex-col min-h-0">
    <div class="flex justify-between items-center mb-1.5 shrink-0">
      <label for="card-notes" class="flex items-center gap-1.5 text-xs font-semibold text-gray-400">
        <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
        </svg>
        <span>{t("refine.notesLabel")}</span>
      </label>
      {#if isSingleRefining || autoRefining}
        <button type="button" onclick={onStop} disabled={isStopping} class="text-xs text-red-300 px-3 py-1">
          {isStopping ? t("refine.btn.stopping") : t("refine.btn.stop")}
        </button>
      {/if}
      {#if !aiStore.killSwitchActive}
        <button
          type="button"
          onclick={onGenerateSingle}
          disabled={generateDisabled}
          class="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-indigo-500/15 hover:bg-indigo-500/25 border border-indigo-500/30 hover:border-indigo-500/50 text-indigo-300 text-xs font-bold transition-all duration-200 cursor-pointer disabled:opacity-50"
        >
          {#if isGeneratingSelected}
            <svg class="animate-spin h-3.5 w-3.5 text-indigo-300" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            {t("refine.btn.generating")}
          {:else}
            <svg class="w-3.5 h-3.5 text-indigo-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
            </svg>
            {t("refine.btn.generateAI")}
          {/if}
        </button>
      {/if}
    </div>

    <CodeEditor
      id="card-notes"
      bind:value={notesValue}
      {readonly}
      placeholder={selectedCardIndex === null ? t("refine.notesPlaceholderEmpty") : (isGeneratingSelected ? t("refine.notesPlaceholderGenerating") : t("refine.notesPlaceholder"))}
      language="html"
      heightClass="flex-1 min-h-[160px]"
      textareaClass={selectedCardIndex === null || isGeneratingSelected ? "opacity-70" : ""}
    />
  </div>

  <div class="flex justify-between items-center text-xs text-gray-400 border-t border-white/5 pt-3 shrink-0">
    <span class="flex items-center gap-1.5">
      <kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">↑</kbd>/<kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">k</kbd> {t("refine.btn.prev")}
      <span class="opacity-30 mx-1">•</span>
      <kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">↓</kbd>/<kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">j</kbd> {t("refine.btn.next")}
    </span>

    <div class="flex items-center gap-2">
      <button
        onclick={() => onNavigate(-1)}
        disabled={selectedFilteredIndex <= 0}
        class="bg-white/5 hover:bg-white/10 disabled:opacity-30 border border-white/10 rounded-lg px-3 py-1.5 font-semibold transition-colors cursor-pointer"
      >
        {t("refine.btn.prev")}
      </button>
      <button
        onclick={() => onNavigate(1)}
        disabled={filteredCount === 0 || selectedFilteredIndex >= filteredCount - 1}
        class="bg-white/5 hover:bg-white/10 disabled:opacity-30 border border-white/10 rounded-lg px-3 py-1.5 font-semibold transition-colors cursor-pointer"
      >
        {t("refine.btn.next")}
      </button>
    </div>
  </div>
</div>
