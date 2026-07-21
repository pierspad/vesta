<script lang="ts">
  import { locale } from "$lib/i18n";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import { previewStore, type PreviewLine } from "$lib/stores/previewStore.svelte";
  import EmptyState from "$lib/components/EmptyState.svelte";

  interface Props {
    /** Media backing the previewed subtitles, if any (movie mode: the
     * selected file; series mode: the first episode's). */
    mediaPath: string | null;
    mediaType: "video" | "audio" | "none";
    /** Reference-language subtitle path — only gates the subs2 column. */
    nativeSubsPath: string;
  }
  let { mediaPath, mediaType, nativeSubsPath }: Props = $props();

  let t = $derived($locale);

  // Only the "active" filter survives outside Expert mode, and any filter
  // or search change snaps back to page 1.
  $effect(() => {
    void previewStore.filter;
    void previewStore.search;
    previewStore.resetPaging(uiMode.expertMode);
  });

  $effect(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
        e.preventDefault();
        previewStore.undo();
      }
    };
    window.addEventListener("keydown", handleKeydown);
    return () => window.removeEventListener("keydown", handleKeydown);
  });

  $effect(() => {
    if (!previewStore.contextMenuVisible) return;
    const handleWindowClick = () => previewStore.closeContextMenu();
    window.addEventListener("click", handleWindowClick);
    return () => window.removeEventListener("click", handleWindowClick);
  });

  function close() {
    previewStore.close();
  }

  function togglePlayerPlayback() {
    const player = previewStore.playerElement;
    if (!player) return;
    if (player.paused) player.play().catch(() => {});
    else player.pause();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50 bg-black/70 flex items-center justify-start pl-6 md:pl-16 lg:pl-[8vw] p-6"
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  onclick={close}
  onkeydown={(e) => {
    if (e.key === "Escape") close();
  }}
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="bg-gray-900 border border-gray-800 rounded-xl w-full max-w-6xl max-h-[85vh] flex flex-col shadow-2xl"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <!-- Modal Header -->
    <div class="flex items-center justify-between p-4 border-b border-gray-800/80 bg-gray-900">
      <div class="flex items-center gap-3">
        <h2 class="text-lg font-bold text-emerald-400 flex items-center gap-2">
          <svg class="w-5 h-5 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
          </svg>
          {t("flashcards.preview")}
        </h2>
      </div>

      <div class="flex items-center gap-4">
        <div class="relative flex items-center">
          <span class="absolute left-3 text-gray-400">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
          </span>
          <input
            type="text"
            bind:value={previewStore.search}
            class="bg-gray-850 hover:bg-gray-800 focus:bg-gray-950 border border-gray-750 focus:border-emerald-500/50 text-xs text-gray-100 placeholder-gray-500 rounded-lg pl-9 pr-3 py-1.5 w-60 outline-none focus:ring-1 focus:ring-emerald-500/20 transition-all"
            placeholder={t("flashcards.previewSearch")}
          />
        </div>
        <button onclick={close} class="dialog-close-button text-gray-400 hover:text-white text-xl leading-none p-1 transition-colors">
          ✕
        </button>
      </div>
    </div>

    <!-- Dialog Body: Columns layout -->
    <div class="flex-1 flex overflow-hidden min-h-0">
      <!-- Left Column: Table of cards, filters, pagination -->
      <div class="flex-1 flex flex-col min-w-0 min-h-0">
        <!-- Sub-header (Filters, count, pagination) -->
        <div class="px-4 py-2 border-b border-gray-800/80 bg-gray-900/50 flex items-center justify-between">
          <div class="flex items-center gap-2">
            <div class="flex rounded-lg overflow-hidden border border-gray-800">
              {#if uiMode.expertMode}
                {#each [["all", t("flashcards.previewAll"), t("flashcards.previewAllTooltip")], ["active", t("flashcards.previewActive"), t("flashcards.previewActiveTooltip")], ["inactive", t("flashcards.previewInactive"), t("flashcards.previewInactiveTooltip")]] as [val, label, tooltip]}
                  <button
                    class="px-3 py-1 text-xs font-medium transition-colors {previewStore.filter === val
                      ? 'bg-emerald-500/20 text-emerald-300'
                      : 'text-gray-400 hover:bg-gray-800'}"
                    onclick={() => (previewStore.filter = val as any)}
                    title={tooltip}
                  >
                    {label}
                  </button>
                {/each}
              {:else}
                <div class="px-3 py-1 text-xs font-medium bg-emerald-500/20 text-emerald-300 select-none">
                  {t("flashcards.previewAll")}
                </div>
              {/if}
            </div>
            <span class="text-xs text-gray-500">
              {previewStore.filtered.length} {t("flashcards.linesShown")}
            </span>
          </div>

          {#if previewStore.totalPages > 1}
            <div class="flex items-center gap-1">
              <button
                disabled={previewStore.page <= 1}
                onclick={() => (previewStore.page = 1)}
                class="px-2 py-1 text-xs rounded text-gray-400 hover:text-white hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                >«</button
              >
              <button
                disabled={previewStore.page <= 1}
                onclick={() => previewStore.page--}
                class="px-2 py-1 text-xs rounded text-gray-400 hover:text-white hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                >‹</button
              >
              <span class="text-xs text-gray-400 px-2 font-mono">
                {previewStore.page} / {previewStore.totalPages}
              </span>
              <button
                disabled={previewStore.page >= previewStore.totalPages}
                onclick={() => previewStore.page++}
                class="px-2 py-1 text-xs rounded text-gray-400 hover:text-white hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                >›</button
              >
              <button
                disabled={previewStore.page >= previewStore.totalPages}
                onclick={() => (previewStore.page = previewStore.totalPages)}
                class="px-2 py-1 text-xs rounded text-gray-400 hover:text-white hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                >»</button
              >
            </div>
          {/if}
        </div>

        <!-- Table content -->
        <div class="flex-1 overflow-y-auto p-2">
          {#if previewStore.loading}
            <div class="flex items-center justify-center h-32">
              <div class="animate-spin w-8 h-8 border-2 border-emerald-500 border-t-transparent rounded-full"></div>
            </div>
          {:else}
            <table class="w-full text-xs">
              <thead class="sticky top-0 z-10">
                <tr class="text-gray-400 bg-gray-800 shadow-sm">
                  <th class="p-2 text-left w-12">#</th>
                  {#if mediaPath}
                    <th class="p-2 text-center w-12">{t("flashcards.previewPlay")}</th>
                  {/if}
                  <th class="p-2 text-left w-20">{t("flashcards.previewTime")}</th>
                  <th class="p-2 text-left">{t("flashcards.subs1")}</th>
                  {#if nativeSubsPath}
                    <th class="p-2 text-left">{t("flashcards.subs2")}</th>
                  {/if}
                  <th class="p-2 text-center w-16">{t("flashcards.previewStatus")}</th>
                </tr>
              </thead>
              <tbody>
                {#each previewStore.paged as line, i}
                  <tr
                    class="border-t border-gray-800/60 {line.active
                      ? 'bg-emerald-500/5 hover:bg-emerald-500/10'
                      : 'bg-red-500/5 opacity-60 hover:bg-red-500/10'} transition-colors"
                    onauxclick={(e) => {
                      if (uiMode.expertMode && e.button === 1) {
                        // middle click
                        e.preventDefault();
                        previewStore.toggleLineActive(line);
                      }
                    }}
                    oncontextmenu={(e) => {
                      if (uiMode.expertMode) {
                        e.preventDefault();
                        previewStore.openContextMenu(e, line);
                      }
                    }}
                  >
                    <td class="p-2 text-gray-500 font-mono">
                      {#if line.active}
                        {previewStore.activeCardNumbers.get(line.index)}
                      {:else}
                        <span class="text-red-500/70 font-bold">—</span>
                      {/if}
                    </td>
                    {#if mediaPath}
                      <td class="p-2 text-center">
                        <button
                          type="button"
                          onclick={() => previewStore.playLine(line, mediaPath)}
                          class="text-gray-400 hover:text-emerald-400 transition-colors p-1"
                          title="Riproduci questa riga"
                        >
                          {#if previewStore.playingLine && previewStore.playingLine.index === line.index && previewStore.isPlaying}
                            <svg class="w-4 h-4 text-emerald-400" fill="currentColor" viewBox="0 0 24 24">
                              <path fill-rule="evenodd" d="M6.75 5.25a.75.75 0 0 1 .75-.75H9a.75.75 0 0 1 .75.75v13.5a.75.75 0 0 1-.75.75H7.5a.75.75 0 0 1-.75-.75V5.25Zm7.5 0A.75.75 0 0 1 15 4.5h1.5a.75.75 0 0 1 .75.75v13.5a.75.75 0 0 1-.75.75H15a.75.75 0 0 1-.75-.75V5.25Z" clip-rule="evenodd" />
                            </svg>
                          {:else}
                            <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                              <path fill-rule="evenodd" d="M4.5 5.653c0-1.427 1.529-2.33 2.779-1.643l11.54 6.347c1.295.712 1.295 2.573 0 3.286L7.28 19.99c-1.25.687-2.779-.217-2.779-1.643V5.653Z" clip-rule="evenodd" />
                            </svg>
                          {/if}
                        </button>
                      </td>
                    {/if}
                    <td class="p-2 text-gray-400 font-mono">
                      {Math.floor(line.start_ms / 60000)}:{String(Math.floor((line.start_ms % 60000) / 1000)).padStart(2, "0")}
                    </td>
                    <td class="p-2 text-gray-200">
                      <span>{line.subs1_text}</span>
                    </td>
                    {#if nativeSubsPath}
                      <td class="p-2 text-gray-300">
                        <span>{line.subs2_text || "—"}</span>
                      </td>
                    {/if}
                    <td class="p-2 text-center select-none">
                      {#if line.active}
                        <span
                          class="inline-block w-2.5 h-2.5 bg-emerald-400 rounded-full"
                          title={uiMode.expertMode ? "Tasto destro / clic rotellina per disattivare" : undefined}
                        ></span>
                      {:else}
                        <span
                          class="inline-block w-2.5 h-2.5 bg-red-400 rounded-full"
                          title={uiMode.expertMode ? "Tasto destro / clic rotellina per attivare" : undefined}
                        ></span>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </div>
      </div>

      <!-- Right Column: Dedicated Player Sidebar -->
      {#if mediaPath}
        <div class="w-[360px] shrink-0 border-l border-gray-800 bg-gray-950/35 p-4 flex flex-col justify-between min-h-0 select-none">
          <!-- Player Container (Video/Audio/Placeholder) -->
          <div class="flex flex-col gap-4 flex-1 overflow-y-auto">
            <div class="flex items-center justify-between border-b border-gray-800 pb-2">
              <span class="text-xs font-semibold text-emerald-400 flex items-center gap-1.5">
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
                </svg>
                {t("flashcards.previewPlayback")}
              </span>
              {#if previewStore.playingLine}
                <button onclick={() => previewStore.stopPlayback()} class="text-gray-500 hover:text-white text-xs transition-colors">
                  {t("common.cancel")}
                </button>
              {/if}
            </div>

            {#if previewStore.playingLine}
              <!-- Player Media Panel -->
              {#if mediaType === "video"}
                <div class="w-full aspect-video rounded-lg bg-black overflow-hidden border border-gray-800/80 shadow-md">
                  <video
                    bind:this={previewStore.playerElement}
                    class="w-full h-full object-contain"
                    onplay={() => (previewStore.isPlaying = true)}
                    onpause={() => (previewStore.isPlaying = false)}
                    onended={() => {
                      previewStore.isPlaying = false;
                      previewStore.playingLine = null;
                    }}
                    controls={false}
                    autoplay
                  ></video>
                </div>
              {:else}
                <!-- Audio Player View -->
                <div class="w-full h-24 rounded-lg bg-emerald-950/10 border border-emerald-900/35 flex flex-col items-center justify-center gap-2 p-3 text-emerald-400">
                  <svg class="w-8 h-8 animate-pulse" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
                  </svg>
                  <span class="text-xs font-mono">{t("flashcards.previewAudioPlaying")}</span>
                  <audio
                    bind:this={previewStore.playerElement}
                    onplay={() => (previewStore.isPlaying = true)}
                    onpause={() => (previewStore.isPlaying = false)}
                    onended={() => {
                      previewStore.isPlaying = false;
                      previewStore.playingLine = null;
                    }}
                    autoplay
                    class="hidden"
                  ></audio>
                </div>
              {/if}

              <!-- Subtitle details -->
              <div class="flex flex-col gap-2 mt-2 bg-gray-900/40 p-3 rounded-lg border border-gray-850">
                <div class="flex justify-between items-center text-[11px] font-mono text-gray-500">
                  <span class="bg-gray-850 px-1.5 py-0.5 rounded text-emerald-400 font-semibold">
                    {t("flashcards.rowNum", { count: previewStore.playingLine.index + 1 })}
                  </span>
                  <span>
                    {Math.floor(previewStore.playingLine.start_ms / 60000)}:{String(Math.floor((previewStore.playingLine.start_ms % 60000) / 1000)).padStart(2, "0")} -
                    {Math.floor(previewStore.playingLine.end_ms / 60000)}:{String(Math.floor((previewStore.playingLine.end_ms % 60000) / 1000)).padStart(2, "0")}
                  </span>
                </div>
                <div class="text-xs text-gray-250 italic font-sans break-words bg-black/10 p-2 rounded border border-gray-900/20 leading-relaxed max-h-40 overflow-y-auto">
                  "{previewStore.playingLine.subs1_text}"
                </div>
                {#if previewStore.playingLine.subs2_text}
                  <div class="text-xs text-gray-400 italic font-sans break-words bg-black/10 p-2 rounded border border-gray-900/20 leading-relaxed max-h-40 overflow-y-auto">
                    "{previewStore.playingLine.subs2_text}"
                  </div>
                {/if}
              </div>
            {:else}
              <!-- Placeholder state when no line is selected -->
              <div class="flex-1 flex flex-col border-2 border-dashed border-gray-800/40 rounded-xl bg-gray-900/10 min-h-[220px]">
                <EmptyState
                  title={t("flashcards.previewNoActive")}
                  description={t("flashcards.previewNoActiveHint")}
                  iconPath="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664zM21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </div>
            {/if}
          </div>

          <!-- Action button at bottom of sidebar (Play/Pause control if a line is active) -->
          {#if previewStore.playingLine}
            <div class="flex items-center justify-center gap-4 mt-4 border-t border-gray-800/60 pt-3">
              <button
                onclick={togglePlayerPlayback}
                class="flex items-center justify-center gap-2 px-5 py-2 w-full rounded-lg bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-400 border border-emerald-500/20 hover:border-emerald-500/35 transition-all text-xs font-medium cursor-pointer"
              >
                {#if previewStore.isPlaying}
                  <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                    <path fill-rule="evenodd" d="M6.75 5.25a.75.75 0 0 1 .75-.75H9a.75.75 0 0 1 .75.75v13.5a.75.75 0 0 1-.75.75H7.5a.75.75 0 0 1-.75-.75V5.25Zm7.5 0A.75.75 0 0 1 15 4.5h1.5a.75.75 0 0 1 .75.75v13.5a.75.75 0 0 1-.75.75H15a.75.75 0 0 1-.75-.75V5.25Z" clip-rule="evenodd" />
                  </svg>
                  {t("common.pause")}
                {:else}
                  <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                    <path fill-rule="evenodd" d="M4.5 5.653c0-1.427 1.529-2.33 2.779-1.643l11.54 6.347c1.295.712 1.295 2.573 0 3.286L7.28 19.99c-1.25.687-2.779-.217-2.779-1.643V5.653Z" clip-rule="evenodd" />
                  </svg>
                  {t("common.play")}
                {/if}
              </button>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  {#if previewStore.contextMenuVisible && previewStore.contextMenuLine}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed z-50 bg-gray-900 border border-gray-800 rounded-lg shadow-xl py-1.5 min-w-[200px]"
      style="left: {previewStore.contextMenuX}px; top: {previewStore.contextMenuY}px;"
      onclick={(e) => e.stopPropagation()}
    >
      <button
        type="button"
        onclick={() => {
          navigator.clipboard.writeText(previewStore.contextMenuLine!.subs1_text);
          previewStore.closeContextMenu();
        }}
        class="w-full text-left px-4 py-2 text-xs text-gray-300 hover:bg-gray-800 hover:text-white transition-colors cursor-pointer"
      >
        {t("flashcards.previewCopyOriginal")}
      </button>

      {#if nativeSubsPath && previewStore.contextMenuLine.subs2_text}
        <button
          type="button"
          onclick={() => {
            navigator.clipboard.writeText(previewStore.contextMenuLine!.subs2_text || "");
            previewStore.closeContextMenu();
          }}
          class="w-full text-left px-4 py-2 text-xs text-gray-300 hover:bg-gray-800 hover:text-white transition-colors cursor-pointer"
        >
          {t("flashcards.previewCopyReference")}
        </button>
      {/if}

      <div class="border-t border-gray-800 my-1"></div>

      <button
        type="button"
        onclick={() => {
          previewStore.toggleLineActive(previewStore.contextMenuLine as PreviewLine);
          previewStore.closeContextMenu();
        }}
        class="w-full text-left px-4 py-2 text-xs font-semibold transition-colors cursor-pointer
          {previewStore.contextMenuLine.active
            ? 'text-red-400 hover:bg-red-500/10'
            : 'text-emerald-400 hover:bg-emerald-500/10'}"
      >
        {#if previewStore.contextMenuLine.active}
          {t("flashcards.previewDisableSub")}
        {:else}
          {t("flashcards.previewEnableSub")}
        {/if}
      </button>
    </div>
  {/if}
</div>
