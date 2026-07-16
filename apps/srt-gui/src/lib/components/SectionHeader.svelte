<script lang="ts">
  import type { Snippet } from "svelte";

  /**
   * Header standard di una card: icona + titolo colorato (+ badge/status
   * opzionale a destra) + sottotitolo opzionale. Ricalca esattamente il
   * pattern già in uso in ExperimentalTab/FlashcardsTab, solo estratto in
   * un componente così ogni tab non lo reimplementa a modo suo.
   *
   * `accent` sceglie il colore del titolo/icona fra le tinte già usate
   * nell'app per distinguere le sezioni (ambra per condensed audio, blu
   * cielo per AnkiConnect, ecc.).
   */
  type Accent = "amber" | "sky" | "emerald" | "violet" | "rose" | "indigo" | "gray";

  interface Props {
    title: string;
    subtitle?: string;
    accent?: Accent;
    /** Path SVG (24x24, stroke) per l'icona nel pallino a sinistra del titolo. */
    iconPath?: string;
    /** Contenuto libero renderizzato a destra del titolo (badge di stato, ecc.). */
    trailing?: Snippet;
  }

  let { title, subtitle = "", accent = "gray", iconPath = "", trailing }: Props = $props();

  const accentText: Record<Accent, string> = {
    amber: "text-amber-300",
    sky: "text-sky-300",
    emerald: "text-emerald-300",
    violet: "text-violet-300",
    rose: "text-rose-300",
    indigo: "text-indigo-400",
    gray: "text-gray-200",
  };
  const accentIcon: Record<Accent, string> = {
    amber: "text-amber-400",
    sky: "text-sky-400",
    emerald: "text-emerald-400",
    violet: "text-violet-400",
    rose: "text-rose-400",
    indigo: "text-indigo-400",
    gray: "text-gray-400",
  };
</script>

<div class="flex flex-col gap-1">
  <div class="flex items-center gap-2.5">
    {#if iconPath}
      <svg class="w-5 h-5 {accentIcon[accent]}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={iconPath} />
      </svg>
    {/if}
    <h2 class="text-base font-bold {accentText[accent]}">{title}</h2>
    {#if trailing}
      {@render trailing()}
    {/if}
  </div>
  {#if subtitle}
    <p class="text-xs text-gray-400 leading-relaxed">{subtitle}</p>
  {/if}
</div>
