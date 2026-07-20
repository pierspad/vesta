<script lang="ts">
  import { locale, currentLanguage } from "$lib/i18n";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import type { EpisodeMediaOverrides } from "$lib/types/flashcardMediaTypes";

  interface Props {
    settings: Required<EpisodeMediaOverrides>;
    hasVideo: boolean;
    effectiveExportFormat: "tsv" | "apkg" | "anki";
    hintLoadVideoFirst: string;
  }
  let { settings = $bindable(), hasVideo, effectiveExportFormat, hintLoadVideoFirst }: Props = $props();

  let t = $derived($locale);
  let easyMode = $derived(!uiMode.expertMode);
</script>

<div
  inert={!hasVideo}
  title={!hasVideo ? hintLoadVideoFirst : undefined}
  class="glass-card p-5 {!hasVideo ? 'opacity-40' : ''}"
>
  <div class="flex items-center justify-between mb-3">
    <h3 class="text-lg font-semibold flex items-center gap-2 text-purple-400">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
        />
      </svg>
      <span class="flex flex-col">
        <span>{t("flashcards.generateSnapshots")}</span>
        {#if effectiveExportFormat === "apkg"}
          <span class="text-[10px] text-purple-300/60 font-normal normal-case mt-0.5">
            {$currentLanguage === "it" ? "Mutualmente esclusivo con i video in APKG" : "Mutually exclusive with video clips in APKG"}
          </span>
        {/if}
      </span>
    </h3>
    <button
      onclick={() => {
        if (hasVideo) settings.generateSnapshots = !settings.generateSnapshots;
      }}
      class="w-10 h-5 rounded-full transition-all duration-200 relative
        {settings.generateSnapshots ? 'bg-purple-500' : 'bg-gray-600'}"
      aria-label="Toggle snapshots"
      disabled={!hasVideo}
    >
      <div
        class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200
        {settings.generateSnapshots ? 'left-5' : 'left-0.5'}"
      ></div>
    </button>
  </div>

  {#if !easyMode}
    <div class="space-y-2 transition-all duration-200 {!settings.generateSnapshots ? 'opacity-40 pointer-events-none' : ''}">
      <div class="grid grid-cols-3 gap-2">
        <div>
          <span class="block text-xs text-gray-500 mb-1">{t("flashcards.width")}</span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.snapshotWidth} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">px</span>
          </div>
        </div>
        <div>
          <span class="block text-xs text-gray-500 mb-1">{t("flashcards.height")}</span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.snapshotHeight} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">px</span>
          </div>
        </div>
        <div>
          <span class="block text-xs text-gray-500 mb-1">{t("flashcards.cropBottom")}</span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.cropBottom} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">px</span>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
