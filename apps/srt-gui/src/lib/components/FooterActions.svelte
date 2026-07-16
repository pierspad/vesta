<script lang="ts">
  import type { Snippet } from "svelte";

  /**
   * Banda fissa in fondo al tab. Il contratto visivo (h-[92px], bordo
   * superiore, sfondo gray-900) è già identico in FlashcardsTab, SyncTab,
   * RefineTab, TranscribeTab, TranslateTab e AlignTab — qui è solo estratto
   * per evitare che ogni tab lo riscriva a mano (e diverga, come successo
   * per il bottone "Reset to Defaults" di Settings).
   */
  interface Props {
    /** "between" (default, azione secondaria a sx + primaria a dx), "end" o "center". */
    justify?: "between" | "end" | "center";
    left?: Snippet;
    right?: Snippet;
    /** Terzo gruppo centrale (es. Autosync in SyncTab). Se presente, left/right/center diventano 3 colonne flex-1 (start/center/end) invece di condividere `justify`. */
    center?: Snippet;
    /** Layer assoluto dietro a left/right, es. l'overlay di progresso animato di Transcribe/Translate. */
    background?: Snippet;
  }

  let { justify = "between", left, right, center, background }: Props = $props();

  const justifyClass = {
    between: "justify-between",
    end: "justify-end",
    center: "justify-center",
  } as const;
</script>

<div class="h-[92px] border-t border-white/10 bg-gray-900 flex items-center {justifyClass[justify]} gap-4 px-6 shrink-0 z-40 relative">
  {#if background}
    {@render background()}
  {/if}
  {#if center}
    <div class="flex items-center gap-4 justify-start flex-1">
      {#if left}{@render left()}{/if}
    </div>
    <div class="flex items-center justify-center flex-1">
      {@render center()}
    </div>
    <div class="flex items-center gap-4 justify-end flex-1">
      {#if right}{@render right()}{/if}
    </div>
  {:else}
    {#if left}
      {@render left()}
    {/if}
    {#if right}
      {@render right()}
    {/if}
  {/if}
</div>
