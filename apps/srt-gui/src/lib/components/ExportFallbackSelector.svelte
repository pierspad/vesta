<script lang="ts">
  import { locale } from "$lib/i18n";
  import type { ExportFallbackFormat } from "$lib/stores/exportFormatStore.svelte";

  let {
    value,
    onchange,
    className = "",
  }: {
    value: ExportFallbackFormat;
    onchange: (value: ExportFallbackFormat) => void;
    className?: string;
  } = $props();
  let t = $derived($locale);

  function toggleValue() {
    onchange(value === "apkg" ? "tsv" : "apkg");
  }
</script>

<div class="flex min-w-0 flex-wrap items-center gap-3 {className}">
  <div class="flex items-center gap-2 text-xs font-semibold text-gray-300">
    <svg class="h-4 w-4 shrink-0 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 17v-6m0-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
    <span>{t("flashcards.fallbackFormatLabel")}</span>
  </div>
  <div class="relative grid shrink-0 grid-cols-2 rounded-lg border border-white/10 bg-black/40 p-1">
    <span class="absolute bottom-1 left-1 top-1 w-[calc(50%-4px)] rounded-md border transition-transform duration-200 ease-out
      {value === 'apkg' ? 'translate-x-0 border-emerald-500/50 bg-emerald-500/25' : 'translate-x-[calc(100%+4px)] border-violet-500/50 bg-violet-500/25'}"></span>
    <button type="button" aria-pressed={value === "apkg"} onclick={toggleValue} class="relative z-10 flex items-center gap-1.5 whitespace-nowrap rounded-md px-3 py-1.5 text-xs font-bold {value === 'apkg' ? 'text-emerald-200' : 'text-gray-400'}">
      APKG <span class="rounded-full bg-emerald-500/20 px-1.5 py-0.5 text-[8px] uppercase text-emerald-300">{t("common.recommended")}</span>
    </button>
    <button type="button" aria-pressed={value === "tsv"} onclick={toggleValue} class="relative z-10 whitespace-nowrap rounded-md px-3 py-1.5 text-xs font-bold {value === 'tsv' ? 'text-violet-200' : 'text-gray-400'}">TSV</button>
  </div>
</div>
