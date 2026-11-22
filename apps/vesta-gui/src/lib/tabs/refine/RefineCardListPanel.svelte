<script lang="ts">
  import { locale } from "$lib/i18n";
  import HistoryButton from "$lib/components/HistoryButton.svelte";
  import SearchInput from "$lib/components/SearchInput.svelte";

  export interface RefineCard {
    id: string;
    expression: string;
    meaning: string;
    notes: string;
    initialNotes?: string;
  }

  interface Props {
    cards: RefineCard[];
    filteredCount: number;
    virtualVisibleCards: RefineCard[];
    totalVirtualHeight: number;
    virtualTopOffset: number;
    selectedCardIndex: number | null;
    searchQuery: string;
    canUndo: boolean;
    canRedo: boolean;
    operationBusy: boolean;
    cardIndices: Map<string, number>;
    cardPlainTexts: Map<string, { expression: string; meaning: string }>;
    isCardRefining: (cardId: string) => boolean;
    cardsContainerEl?: HTMLElement | null;
    cardListViewportHeight?: number;
    onScroll: (e: Event) => void;
    onHistory: (dir: "undo" | "redo") => void;
    onSelectCard: (index: number) => void;
  }

  let {
    cards,
    filteredCount,
    virtualVisibleCards,
    totalVirtualHeight,
    virtualTopOffset,
    selectedCardIndex,
    searchQuery = $bindable(),
    canUndo,
    canRedo,
    operationBusy,
    cardIndices,
    cardPlainTexts,
    isCardRefining,
    cardsContainerEl = $bindable(null),
    cardListViewportHeight = $bindable(600),
    onScroll,
    onHistory,
    onSelectCard,
  }: Props = $props();

  let t = $derived($locale);
</script>

<div class="md:col-span-5 glass-card perf-scroll-card p-5 flex flex-col min-h-0 overflow-hidden">
  <div class="mb-3 flex shrink-0 items-center gap-2">
    <SearchInput bind:value={searchQuery} placeholder={t("refine.searchPlaceholder")} class="w-full" />
    <HistoryButton direction="undo" disabled={!canUndo || operationBusy} onclick={() => onHistory("undo")} />
    <HistoryButton direction="redo" disabled={!canRedo || operationBusy} onclick={() => onHistory("redo")} />
  </div>

  <div
    bind:this={cardsContainerEl}
    onscroll={onScroll}
    bind:clientHeight={cardListViewportHeight}
    class="flex-1 min-h-0 overflow-y-auto [scrollbar-gutter:stable] p-2 bg-white/[0.03] border border-white/10 rounded-xl scrollbar-thin"
  >
    {#if cards.length === 0}
      <div class="relative min-h-full">
        <div aria-hidden="true" class="space-y-1.5">
          {#each Array(6) as _}
            <div class="h-[88px] rounded-xl border border-white/5 bg-white/[0.02] p-3 space-y-2">
              <div class="h-2 w-8 rounded bg-white/5"></div>
              <div class="h-3 w-2/3 rounded bg-white/5"></div>
              <div class="h-2 w-1/2 rounded bg-white/[0.03]"></div>
            </div>
          {/each}
        </div>
        <div class="absolute inset-0 flex items-center justify-center text-xs text-gray-400 italic">
          <span class="rounded-lg bg-gray-900/90 px-4 py-2">{t("refine.noFlashcardsLoaded")}</span>
        </div>
      </div>
    {:else if filteredCount === 0}
      <div class="flex min-h-full items-center justify-center text-xs text-gray-400">{t("common.noResults")}</div>
    {:else}
      <div style="height: {totalVirtualHeight}px; position: relative; width: 100%;">
        <div style="position: absolute; top: {virtualTopOffset}px; left: 0; right: 0; display: flex; flex-direction: column; gap: 6px;">
          {#each virtualVisibleCards as card (card.id)}
            {@const isSelected = selectedCardIndex !== null && cards[selectedCardIndex]?.id === card.id}
            {@const globalIndex = cardIndices.get(card.id) ?? -1}
            <button
              onclick={() => onSelectCard(globalIndex)}
              class="h-[88px] w-full text-left p-3 rounded-xl transition-all border flex flex-col gap-1 cursor-pointer
                {isSelected
                  ? 'bg-rose-500/15 border-rose-500/40 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-gray-200'}"
            >
              <div class="flex justify-between items-center text-[10px] text-gray-500">
                <span class="font-mono">#{globalIndex + 1}</span>
                <div class="flex items-center gap-1.5">
                  {#if isCardRefining(card.id)}
                    <span class="flex items-center gap-1 text-[9px] text-indigo-400 font-bold uppercase tracking-wider">
                      <svg class="animate-spin h-2.5 w-2.5 text-indigo-400" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                      </svg>
                      {t("refine.btn.generating")}
                    </span>
                  {/if}
                  {#if card.notes.trim() !== ""}
                    <span class="bg-rose-500/20 text-rose-300 px-1.5 py-0.5 rounded-full border border-rose-500/20 text-[9px] font-bold uppercase tracking-wider">{t("refine.badge.annotated")}</span>
                  {/if}
                  {#if card.notes !== (card.initialNotes || "")}
                    <span class="bg-amber-500/20 text-amber-300 px-1.5 py-0.5 rounded-full border border-amber-500/20 text-[9px] font-bold uppercase tracking-wider">{t("refine.badge.modified")}</span>
                  {/if}
                </div>
              </div>
              <p class="text-xs font-semibold line-clamp-1 break-all text-gray-100 select-none">
                {cardPlainTexts.get(card.id)?.expression || "—"}
              </p>
              <p class="text-[10px] line-clamp-1 break-all opacity-70 select-none">
                {cardPlainTexts.get(card.id)?.meaning || "—"}
              </p>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>
