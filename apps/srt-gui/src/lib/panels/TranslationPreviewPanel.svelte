<script lang="ts">
  import { locale } from "$lib/i18n";
  import EmptyStatusLabel from "$lib/components/EmptyStatusLabel.svelte";
  import type { SubtitlePair } from "$lib/services/translate";
  let { source = [], pairs, loaded, translating } = $props<{
    source?: { id: number; text: string }[];
    pairs: SubtitlePair[];
    loaded: boolean;
    translating: boolean;
  }>();
  let t = $derived($locale);
  let originals = $derived(pairs.length ? pairs.map((pair: SubtitlePair) => ({ id: pair.id, text: pair.original })) : source);
</script>

<div class="glass-card p-5 min-h-[400px]">
  <h3 class="mb-4 flex items-center gap-2 text-lg font-semibold text-purple-400">
    <svg aria-hidden="true" class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
    {t("translate.livePreview")}
    {#if pairs.length}<span class="text-xs font-normal text-gray-500">({pairs.length} {t("translate.subtitles")})</span>{/if}
  </h3>
  <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
    <section class="flex min-w-0 flex-col rounded-xl bg-white/5 p-4">
      <h4 class="mb-3 shrink-0 text-xs uppercase tracking-wide text-gray-500">{t("translate.original")}</h4>
      <div class="relative min-h-[280px] flex-1">
        <div class="max-h-[340px] space-y-2 overflow-y-auto scrollbar-thin">
          {#each originals as subtitle (subtitle.id)}
            <div class="rounded-lg border-l-2 border-gray-600 bg-black/20 p-2">
              <span class="font-mono text-[10px] text-gray-500">#{subtitle.id}</span>
              <p class="mt-0.5 whitespace-pre-line break-words text-sm text-gray-300">{subtitle.text}</p>
            </div>
          {:else}
            <div aria-hidden="true" class="space-y-2">
              {#each Array(5) as _, index (index)}
                <div class="rounded-lg border-l-2 border-white/5 bg-black/10 p-3"><div class="mb-3 h-2 w-8 rounded bg-white/5"></div><div class="h-3 w-4/5 rounded bg-white/5"></div></div>
              {/each}
            </div>
          {/each}
        </div>
        {#if !loaded || !originals.length}<EmptyStatusLabel message={t("media.noSubtitlesLoaded")} />{/if}
      </div>
    </section>
    <section class="flex min-w-0 flex-col rounded-xl bg-white/5 p-4">
      <h4 class="mb-3 shrink-0 text-xs uppercase tracking-wide text-gray-500">{t("translate.translated")}</h4>
      <div class="relative min-h-[280px] flex-1" aria-busy={translating}>
        <div class="max-h-[340px] space-y-2 overflow-y-auto scrollbar-thin">
          {#each pairs as pair (pair.id)}
            <div class="rounded-lg border-l-2 border-green-500/50 bg-green-500/5 p-2">
              <span class="font-mono text-[10px] text-green-500">#{pair.id}</span>
              <p class="mt-0.5 whitespace-pre-line break-words text-sm text-green-300">{pair.translated}</p>
            </div>
          {:else}
            <div aria-hidden="true" class="space-y-2">
              {#each Array(5) as _, index (index)}
                <div class="rounded-lg border-l-2 border-white/5 bg-black/10 p-3"><div class="mb-3 h-2 w-8 rounded bg-white/5"></div><div class="h-3 w-4/5 rounded bg-white/5"></div></div>
              {/each}
            </div>
          {/each}
        </div>
        {#if !pairs.length}<EmptyStatusLabel message={t(translating ? "translate.starting" : "translate.waitingForTranslation")} />{/if}
      </div>
    </section>
  </div>
</div>
