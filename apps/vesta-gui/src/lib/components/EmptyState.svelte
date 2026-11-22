<script lang="ts">
  import type { Snippet } from "svelte";

  /**
   * Stato vuoto standard: icona centrata + titolo + descrizione opzionale.
   * Oggi ogni tab lo reinventa (icona diversa, testo diverso, a volte solo
   * una chiave i18n grezza). Un solo componente per "qui non c'è ancora
   * niente da vedere" tiene coerente look & tono in tutta l'app.
   */
  interface Props {
    title: string;
    description?: string;
    /** Path SVG (24x24, stroke). Default: icona "documento vuoto" generica. */
    iconPath?: string;
    /** Azione opzionale sotto al testo (es. bottone "Carica file"). */
    action?: Snippet;
  }

  let {
    title,
    description = "",
    iconPath = "M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V18a2 2 0 01-2 2z",
    action,
  }: Props = $props();
</script>

<div class="flex-1 flex flex-col items-center justify-center gap-3 text-center px-6 py-12">
  <div class="w-12 h-12 rounded-2xl bg-white/5 border border-white/5 flex items-center justify-center">
    <svg class="w-6 h-6 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={iconPath} />
    </svg>
  </div>
  <div class="flex flex-col gap-1 max-w-sm">
    <span class="text-sm font-semibold text-gray-300">{title}</span>
    {#if description}
      <span class="text-xs text-gray-500 leading-relaxed">{description}</span>
    {/if}
  </div>
  {#if action}
    {@render action()}
  {/if}
</div>
