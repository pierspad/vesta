<script lang="ts">
  import type { Snippet } from "svelte";

  /**
   * Contenitore standard per una sezione di un tab: lo sfondo/bordo/radius
   * ripetuti in FlashcardsTab, ExperimentalTab, ecc. (`bg-white/[0.02]
   * border border-white/5 rounded-2xl p-6`). Usarlo al posto di ricopiare
   * quelle classi in ogni tab tiene la grafica coerente "gratis".
   */
  interface Props {
    /** Riduce il padding interno per card più dense (default: normale, p-6). */
    compact?: boolean;
    class?: string;
    children: Snippet;
  }

  let { compact = false, class: className = "", children }: Props = $props();
</script>

<div
  class="bg-white/[0.02] border border-white/5 rounded-2xl flex flex-col gap-4 {compact ? 'p-4' : 'p-6'} {className}"
>
  {@render children()}
</div>
