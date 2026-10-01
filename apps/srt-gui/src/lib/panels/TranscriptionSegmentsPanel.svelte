<script lang="ts">
  import { locale } from "$lib/i18n";
  import { formatSubtitleTime } from "$lib/utils/transcriptionPaths";
  let { segments } = $props<{ segments: { start_ms: number; end_ms: number; text: string }[] }>();
  let t = $derived($locale);
  let scrollContainer = $state<HTMLDivElement | null>(null);
  $effect(() => {
    if (segments.length && scrollContainer) scrollContainer.scrollTop = scrollContainer.scrollHeight;
  });
</script>

<div class="glass-card p-5 space-y-4">
  <div class="flex items-center justify-between">
    <span class="text-[10px] font-bold text-gray-500 uppercase tracking-wide">{t("transcribe.livePhrases")}</span>
    <span class="text-[10px] text-indigo-400 font-semibold">{segments.length} {t("transcribe.segments")}</span>
  </div>
  {#if segments.length === 0}
    <div class="rounded-lg border border-indigo-500/20 bg-indigo-500/5 px-3 py-8 text-center text-xs text-indigo-300">
      {t("transcribe.livePhrasesHint")}
    </div>
  {:else}
    <div bind:this={scrollContainer} class="space-y-2 max-h-[220px] overflow-y-auto pr-1 transcribe-scroll">
      {#each segments as segment}
        <div class="p-2.5 rounded-lg bg-white/[0.02] border border-white/5 flex gap-3 text-xs hover:bg-white/5 transition-colors">
          <span class="font-mono text-indigo-300 shrink-0 select-none">
            {formatSubtitleTime(segment.start_ms)} → {formatSubtitleTime(segment.end_ms)}
          </span>
          <span class="text-gray-200 break-words">{segment.text}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>
