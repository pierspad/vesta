<script lang="ts">
  import EmptyStatusLabel from "$lib/components/EmptyStatusLabel.svelte";
  import { locale } from "$lib/i18n";

  interface SubtitleInfo {
    id: number;
    start_ms: number;
    end_ms: number;
    text: string;
    synced_start_ms: number;
    synced_end_ms: number;
    offset_ms: number;
    is_anchor: boolean;
  }

  interface Props {
    subtitles: SubtitleInfo[];
    isLoaded: boolean;
    currentPage: number;
    totalPages: number;
    pageSize: number;
    activeSubtitleId: number | null;
    formatTime: (ms: number) => string;
    formatOffset: (ms: number) => string;
    listElement?: HTMLDivElement | null;
    onPageChange: (page: number) => void;
    onClickSub: (sub: SubtitleInfo) => void;
    onDblClickSub: (sub: SubtitleInfo) => void;
    onContextMenu: (e: MouseEvent, sub: SubtitleInfo) => void;
    onRemoveAnchor: (id: number) => void;
  }

  let {
    subtitles,
    isLoaded,
    currentPage,
    totalPages,
    pageSize,
    activeSubtitleId,
    formatTime,
    formatOffset,
    listElement = $bindable(null),
    onPageChange,
    onClickSub,
    onDblClickSub,
    onContextMenu,
    onRemoveAnchor,
  }: Props = $props();

  let t = $derived($locale);
</script>

<div class="glass-card flex flex-col flex-1 min-h-0 min-w-0 overflow-hidden">
  <!-- Header -->
  <div class="px-5 pt-5 pb-2 flex-shrink-0">
    <h3 class="text-lg font-semibold flex items-center gap-2 text-purple-400">
      <svg
        class="w-5 h-5"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        ><path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M4 6h16M4 10h16M4 14h16M4 18h16"
        /></svg
      >
      <span>
        {t("sync.subtitles")}
        <span class="text-gray-500 font-normal"
          >{t("sync.pageOf", { current: isLoaded ? currentPage : 0, total: totalPages })}</span
        >
      </span>
    </h3>
  </div>

  <!-- Pagination controls — TOP -->
    <div class="px-3 pb-2 flex items-center justify-between flex-shrink-0">
      <button
        onclick={() => onPageChange(currentPage - 1)}
        disabled={!isLoaded || currentPage <= 1}
        class="btn-secondary py-1 px-2.5 text-xs flex items-center gap-1 disabled:opacity-30"
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/></svg>
        {t("sync.pagination.prev")}
      </button>
      <span class="text-xs text-gray-400">{t("sync.pageSlash", { current: isLoaded ? currentPage : 0, total: totalPages })}</span>
      <button
        onclick={() => onPageChange(currentPage + 1)}
        disabled={!isLoaded || currentPage >= totalPages}
        class="btn-secondary py-1 px-2.5 text-xs flex items-center gap-1 disabled:opacity-30"
      >
        {t("sync.pagination.next")}
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/></svg>
      </button>
    </div>

  <!-- Keep the empty label outside the scrolling placeholder grid. -->
  <div class="relative flex min-h-0 flex-1 flex-col">
    {#if !isLoaded}<EmptyStatusLabel message={t("media.noSubtitlesLoaded")} />{/if}
  <!-- Subtitle list -->
  <div
    class="grid auto-rows-min grid-cols-1 md:grid-cols-2 2xl:grid-cols-3 gap-2 flex-1 p-2 min-h-0 min-w-0 overflow-y-auto overflow-x-hidden scrollbar-thin"
    bind:this={listElement}
  >
    {#each subtitles as sub (sub.id)}
      <button
        onclick={() => onClickSub(sub)}
        ondblclick={() => onDblClickSub(sub)}
        oncontextmenu={(e) => onContextMenu(e, sub)}
        data-subtitle-id={sub.id}
        class="w-full min-w-0 text-left p-3 border-b border-white/5 hover:bg-white/5
          {activeSubtitleId === sub.id
          ? 'bg-indigo-500/20 border-l-4 border-l-indigo-500'
          : ''}
          {sub.is_anchor ? 'bg-green-500/5' : ''}"
      >
        <div class="flex items-start gap-2">
          <span class="text-xs text-gray-500 w-8 flex-shrink-0"
            >#{sub.id}</span
          >
          <div class="flex-1 min-w-0">
            <p class="text-sm truncate text-gray-200">{sub.text}</p>
            <div class="flex flex-wrap gap-x-2 gap-y-1 text-xs text-gray-500 mt-1">
              <span class="font-mono"
                >{formatTime(sub.synced_start_ms)}</span
              >
              <span class="text-gray-700">→</span>
              <span class="font-mono"
                >{formatTime(sub.synced_end_ms)}</span
              >
              {#if sub.offset_ms !== 0}<span
                  class={sub.offset_ms > 0
                    ? "text-green-400"
                    : "text-red-400"}>{formatOffset(sub.offset_ms)}</span
                >{/if}
            </div>
          </div>
          {#if sub.is_anchor}
            <div
              role="button"
              tabindex="0"
              onclick={(e) => {
                e.stopPropagation();
                onRemoveAnchor(sub.id);
              }}
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.stopPropagation();
                  e.preventDefault();
                  onRemoveAnchor(sub.id);
                }
              }}
              class="text-green-400 hover:text-red-400 transition-colors flex-shrink-0 p-1 rounded hover:bg-white/5 cursor-pointer"
              title={t("sync.tooltipRemoveAnchor")}
            >
              <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24"><path d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"/></svg>
            </div>
          {/if}
        </div>
      </button>
    {:else}
      {#if !isLoaded}
        {#each Array(pageSize) as _, index (index)}
          <div aria-hidden="true" class="w-full min-w-0 p-3 border-b border-white/5">
            <div class="flex items-start gap-2">
              <div class="w-8 flex-shrink-0 pt-1"><div class="h-2 w-5 rounded bg-white/5"></div></div>
              <div class="flex-1 min-w-0">
                <div class="h-5 flex items-center"><div class="h-3 w-4/5 rounded bg-white/5"></div></div>
                <div class="flex flex-wrap items-center gap-x-2 gap-y-1 mt-1 min-h-4">
                  <div class="h-2 w-16 rounded bg-white/[0.03]"></div>
                  <span class="text-xs text-gray-700">→</span>
                  <div class="h-2 w-16 rounded bg-white/[0.03]"></div>
                </div>
              </div>
            </div>
          </div>
        {/each}
      {/if}
    {/each}
  </div>
  </div>
</div>
