<script lang="ts">
  import EmptyStatusLabel from "$lib/components/EmptyStatusLabel.svelte";
  import { tick } from "svelte";
  import { locale } from "$lib/i18n";
  import { formatSubtitleTime } from "$lib/utils/transcriptionPaths";
  let { segments } = $props<{ segments: { start_ms: number; end_ms: number; text: string }[] }>();
  let t = $derived($locale);
  let scrollContainer = $state<HTMLDivElement | null>(null);
  let scrollRaf: number | null = null;
  $effect(() => {
    if (segments.length && scrollContainer) {
      const scroll = scrollContainer.closest<HTMLElement>("[data-workflow-scroll]");
      if (scroll && scroll.scrollHeight - scroll.scrollTop - scroll.clientHeight < 80) {
        void tick().then(() => {
          if (!scroll.isConnected || scrollRaf !== null) return;
          scrollRaf = requestAnimationFrame(() => {
            scrollRaf = null;
            if (scroll.isConnected) scroll.scrollTop = scroll.scrollHeight;
          });
        });
      }
    }
    return () => {
      if (scrollRaf !== null) {
        cancelAnimationFrame(scrollRaf);
        scrollRaf = null;
      }
    };
  });
</script>

<div class="glass-card perf-scroll-card p-5 flex flex-col flex-1 min-h-80 overflow-hidden gap-4">
  <div class="flex items-center justify-between shrink-0">
    <span class="text-[10px] font-bold text-gray-500 uppercase tracking-wide">{t("transcribe.livePhrases")}</span>
    <span class="text-[10px] text-indigo-400 font-semibold">{segments.length} {t("transcribe.segments")}</span>
  </div>
  <div class="relative flex min-h-0 flex-1 flex-col">
    {#if segments.length === 0}
      <EmptyStatusLabel message={t("transcribe.livePhrasesHint")} />
    {/if}
    <div bind:this={scrollContainer} class="min-h-0 flex-1 overflow-y-auto space-y-2 pr-1 scrollbar-thin">
      {#if segments.length === 0}
        <div class="space-y-3 opacity-30" aria-hidden="true">
          {#each [1, 2, 3] as row}
            <div class="rounded-lg border border-white/5 bg-white/[0.02] p-3">
              <div class="mb-3 h-2 w-24 rounded bg-indigo-400/20"></div>
              <div class="h-2 rounded bg-white/10" style:width={row === 2 ? "65%" : "85%"}></div>
            </div>
          {/each}
        </div>
      {:else}
        {#each segments as segment, idx (`${segment.start_ms}-${segment.end_ms}-${idx}`)}
          <div class="p-2.5 rounded-lg bg-white/[0.02] border border-white/5 flex gap-3 text-xs hover:bg-white/5 transition-colors">
            <span class="font-mono text-indigo-300 shrink-0 select-none">
              {formatSubtitleTime(segment.start_ms)} → {formatSubtitleTime(segment.end_ms)}
            </span>
            <span class="text-gray-200 break-words">{segment.text}</span>
          </div>
        {/each}
      {/if}
    </div>
  </div>
</div>
