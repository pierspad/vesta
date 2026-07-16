<script lang="ts" generics="T extends { id: string }">
  import type { Snippet } from "svelte";

  /**
   * Shell condiviso da TranscribeTiers.svelte e TranslationTiers.svelte:
   * accordion dei tier (header con chevron/badge/priorità, sposta su/giù,
   * rimuovi), stato di collapse, e i due empty state ("nessuna key" /
   * "nessun tier"). Prima dell'estrazione i due file duplicavano ~90 righe
   * di questo markup pressoché byte-per-byte (vedi REFACTOR-PLAN.md §3.1).
   *
   * Cosa NON fa: non sa nulla delle singole entry (provider, modello, RPM,
   * download di modelli Whisper, discovery LLM, ecc.) — quella parte resta
   * specifica di ogni feature e viene passata come snippet `tierBody`,
   * perché è lì che le due implementazioni divergono davvero.
   */
  interface Props {
    tiers: T[];
    /** Messaggio mostrato quando `tiers` è vuoto. */
    emptyMessage: string;
    /** Banner opzionale sopra la lista (es. "nessuna API key configurata"). */
    warning?: Snippet;
    onRemoveTier: (index: number) => void;
    onMoveTier: (index: number, dir: -1 | 1) => void;
    /** Titolo di un tier: default "Tier {n}", personalizzabile se serve. */
    tierLabel?: (index: number) => string;
    /** Sotto-etichetta di priorità: default gestito dal chiamante via i18n. */
    priorityLabel: (index: number) => string;
    moveUpLabel: string;
    moveDownLabel: string;
    removeTierLabel: string;
    /** Corpo di un tier (lista entry + bottone "aggiungi endpoint"): specifico della feature. */
    tierBody: Snippet<[tier: T, tierIndex: number]>;
  }

  let {
    tiers,
    emptyMessage,
    warning,
    onRemoveTier,
    onMoveTier,
    tierLabel,
    priorityLabel,
    moveUpLabel,
    moveDownLabel,
    removeTierLabel,
    tierBody,
  }: Props = $props();

  let collapsedTiers = $state<Set<string>>(new Set());

  function toggleTier(tierId: string) {
    const next = new Set(collapsedTiers);
    if (next.has(tierId)) {
      next.delete(tierId);
    } else {
      next.add(tierId);
    }
    collapsedTiers = next;
  }
</script>

<div class="space-y-4">
  {#if warning}
    {@render warning()}
  {/if}

  {#if tiers.length === 0}
    <div class="rounded-lg border border-white/10 bg-white/5 px-4 py-6 text-center text-sm text-gray-400">
      {emptyMessage}
    </div>
  {:else}
    <div class="space-y-6">
      {#each tiers as tier, tierIndex (tier.id)}
        <div class="border-b border-white/5 pb-6 last:border-b-0 last:pb-0">
          <!-- Tier Accordion Header -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            role="button"
            tabindex="0"
            onclick={() => toggleTier(tier.id)}
            onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggleTier(tier.id); } }}
            class="flex items-center justify-between gap-2 py-3 cursor-pointer group select-none outline-none"
          >
            <div class="flex items-center gap-3">
              <!-- Chevron indicator -->
              <span class="text-gray-500 group-hover:text-white transition-colors duration-150 flex-shrink-0">
                {#if collapsedTiers.has(tier.id)}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M9 5l7 7-7 7" /></svg>
                {:else}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M19 9l-7 7-7-7" /></svg>
                {/if}
              </span>

              <!-- Badge with Order Number -->
              <span class="inline-flex items-center justify-center w-6 h-6 rounded-md bg-indigo-500/15 text-indigo-300 text-xs font-bold flex-shrink-0">
                {tierIndex + 1}
              </span>

              <!-- Title & Priority sub-label -->
              <div class="flex flex-col">
                <span class="text-sm font-bold text-white group-hover:text-indigo-200 transition-colors">
                  {tierLabel ? tierLabel(tierIndex) : `Tier ${tierIndex + 1}`}
                </span>
                <span class="text-[10px] text-gray-500 font-medium mt-0.5 uppercase tracking-wider">
                  {priorityLabel(tierIndex)}
                </span>
              </div>
            </div>

            <!-- Tier Actions (Control Buttons) -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div class="flex items-center gap-1" onclick={(e) => e.stopPropagation()}>
              <button
                type="button"
                onclick={() => onMoveTier(tierIndex, -1)}
                disabled={tierIndex === 0}
                title={moveUpLabel}
                class="p-1.5 rounded-lg text-indigo-400 bg-indigo-500/5 border border-indigo-500/10 hover:text-white hover:bg-indigo-500/20 hover:border-indigo-500/30 disabled:opacity-20 disabled:hover:bg-transparent disabled:border-transparent transition cursor-pointer"
                aria-label={moveUpLabel}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 15l7-7 7 7" /></svg>
              </button>
              <button
                type="button"
                onclick={() => onMoveTier(tierIndex, 1)}
                disabled={tierIndex === tiers.length - 1}
                title={moveDownLabel}
                class="p-1.5 rounded-lg text-indigo-400 bg-indigo-500/5 border border-indigo-500/10 hover:text-white hover:bg-indigo-500/20 hover:border-indigo-500/30 disabled:opacity-20 disabled:hover:bg-transparent disabled:border-transparent transition cursor-pointer"
                aria-label={moveDownLabel}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M19 9l-7 7-7-7" /></svg>
              </button>
              <button
                type="button"
                onclick={() => onRemoveTier(tierIndex)}
                title={removeTierLabel}
                class="p-1.5 rounded-lg text-red-400 bg-red-500/5 border border-red-500/10 hover:text-white hover:bg-red-500/20 hover:border-red-500/30 transition cursor-pointer"
                aria-label={removeTierLabel}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
              </button>
            </div>
          </div>

          <!-- Tier Content (feature-specific: entries list + add-entry button) -->
          {#if !collapsedTiers.has(tier.id)}
            <div class="pl-6 pr-2 py-2 space-y-4">
              {@render tierBody(tier, tierIndex)}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
