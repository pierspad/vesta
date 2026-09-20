<script lang="ts">
  import { locale } from "$lib/i18n";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import {
    QUALITY_STEPS,
    matchQualityStep,
    type EpisodeMediaOverrides,
    type SnapshotFormat,
  } from "$lib/types/flashcardMediaTypes";

  interface Props {
    settings: Required<EpisodeMediaOverrides>;
    hasVideo: boolean;
    effectiveExportFormat: "tsv" | "apkg" | "anki";
    hintLoadVideoFirst: string;
  }
  let { settings = $bindable(), hasVideo, effectiveExportFormat, hintLoadVideoFirst }: Props = $props();

  let t = $derived($locale);
  let easyMode = $derived(!uiMode.expertMode);

  const CUSTOM = "__custom__";

  let activeQuality = $derived(
    matchQualityStep(settings.snapshotQuality, settings.snapshotWidth, settings.snapshotHeight)
  );

  function applyQualityStep(id: string) {
    const step = QUALITY_STEPS.find((s) => s.id === id);
    if (!step) return;
    settings.snapshotQuality = step.snapshotQuality;
    settings.snapshotWidth = step.snapshotWidth;
    settings.snapshotHeight = step.snapshotHeight;
    if (settings.audioFormat === "opus") settings.audioBitrate = step.opusBitrate;
  }
</script>

<div
  inert={!hasVideo}
  title={!hasVideo ? hintLoadVideoFirst : undefined}
  class="glass-card p-5 relative z-30 overflow-visible {!hasVideo ? 'opacity-40' : ''}"
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
            {t("flashcards.snapshotsExclusiveHint")}
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

  <div class="space-y-2 transition-all duration-200 {!settings.generateSnapshots ? 'opacity-40 pointer-events-none' : ''}">
    {#if easyMode}
      <!-- Easy Mode: Intuitive Quality vs Deck Size Balance Selector -->
      <div class="space-y-3 bg-gray-950/30 p-3.5 rounded-xl border border-white/5">
        <div class="flex items-center justify-between text-xs">
          <span class="font-medium text-gray-300 flex items-center gap-1.5">
            <svg class="w-4 h-4 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 6l3 18h12l3-18H3z" />
            </svg>
            {t("flashcards.qualityVsSize")}
          </span>
          <span class="text-[11px] font-semibold px-2 py-0.5 rounded-full bg-purple-500/20 text-purple-300 border border-purple-500/30">
            {activeQuality ? t(`flashcards.quality.${activeQuality.id}`) : t("flashcards.custom")}
          </span>
        </div>

        <div class="grid grid-cols-3 gap-1.5 p-1 bg-black/40 rounded-xl border border-white/5">
          {#each QUALITY_STEPS as step}
            <button
              type="button"
              onclick={() => applyQualityStep(step.id)}
              class="py-1.5 px-2 rounded-lg text-xs font-medium transition-all duration-150 flex flex-col items-center gap-0.5
                {activeQuality?.id === step.id
                  ? 'bg-purple-600/90 text-white shadow-md shadow-purple-900/40 font-semibold scale-[1.02]'
                  : 'text-gray-400 hover:text-gray-200 hover:bg-white/5'}"
            >
              <span>{t(`flashcards.quality.${step.id}`)}</span>
              <span class="text-[9px] opacity-70">
                {step.id === 'light' ? '~15 KB' : step.id === 'balanced' ? '~35 KB' : '~70 KB'}
              </span>
            </button>
          {/each}
        </div>

        <div class="space-y-1 pt-1">
          <div class="flex justify-between text-[10px] text-gray-400">
            <span class="flex items-center gap-1">
              <svg class="w-3 h-3 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 14l-7 7m0 0l-7-7m7 7V3" />
              </svg>
              {t("flashcards.weightLow")}
            </span>
            <span class="flex items-center gap-1">
              {t("flashcards.weightHigh")}
              <svg class="w-3 h-3 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 10l7-7m0 0l7 7m-7-7v18" />
              </svg>
            </span>
          </div>
          <div class="h-2 rounded-full bg-gray-800/80 p-0.5 relative overflow-hidden">
            <div
              class="h-full w-full rounded-full transition-[clip-path] duration-300 ease-out"
              style:background="linear-gradient(90deg, #059669 0%, #10b981 18%, #34d399 35%, #22c55e 38%, #84cc16 52%, #eab308 68%, #f97316 85%, #dc2626 100%)"
              style:clip-path="inset(0 {activeQuality?.id === 'light' ? '68%' : activeQuality?.id === 'balanced' ? '32%' : '0%'} 0 0)"
            ></div>
          </div>
        </div>
      </div>
    {:else}
      <!-- Expert Mode: Ordered rows (Quality Preset + Image Quality -> Width + Height -> Format + Crop Bottom) -->
      
      <!-- Row 1: Quality Preset (Left) + Image Quality (Right) -->
      <div class="grid grid-cols-2 gap-2">
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" />
            </svg>
            <span>{t("flashcards.qualityPreset")}</span>
          </span>
          <SearchableSelect
            noResultsText={t("common.noResults")}
            options={[
              ...QUALITY_STEPS.map((s) => ({
                value: s.id,
                label: t(`flashcards.quality.${s.id}`),
              })),
              ...(activeQuality ? [] : [{ value: CUSTOM, label: t("flashcards.custom") }]),
            ]}
            value={activeQuality?.id ?? CUSTOM}
            onchange={(val) => {
              if (val !== CUSTOM) applyQualityStep(val);
            }}
            placeholder={t("flashcards.qualityPreset")}
          />
        </div>
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4M7.835 4.697a3.42 3.42 0 001.946-.806 3.42 3.42 0 014.438 0 3.42 3.42 0 001.946.806 3.42 3.42 0 013.138 3.138 3.42 3.42 0 00.806 1.946 3.42 3.42 0 010 4.438 3.42 3.42 0 00-.806 1.946 3.42 3.42 0 01-3.138 3.138 3.42 3.42 0 00-1.946.806 3.42 3.42 0 01-4.438 0 3.42 3.42 0 00-1.946-.806 3.42 3.42 0 01-3.138-3.138 3.42 3.42 0 00-.806-1.946 3.42 3.42 0 010-4.438 3.42 3.42 0 00.806-1.946 3.42 3.42 0 013.138-3.138z" />
            </svg>
            <span>{t("flashcards.qualityValue")}</span>
          </span>
          <div class="flex items-center gap-1">
            <input
              type="number"
              min="0"
              max="100"
              bind:value={settings.snapshotQuality}
              class="input-modern w-full text-xs"
            />
            <span class="text-xs text-gray-500">/100</span>
          </div>
        </div>
      </div>

      <!-- Row 2: Width (Left) + Height (Right) -->
      <div class="grid grid-cols-2 gap-2">
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h8m0 0l-2.5-2.5M16 7l-2.5 2.5M8 7l2.5-2.5M8 7l2.5 2.5M4 4v16m16-16v16" />
            </svg>
            <span>{t("flashcards.width")}</span>
          </span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.snapshotWidth} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">px</span>
          </div>
        </div>
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 8v8m0 0l-2.5-2.5M7 16l2.5-2.5M7 8l-2.5 2.5M7 8l2.5 2.5M4 4h16M4 20h16" />
            </svg>
            <span>{t("flashcards.height")}</span>
          </span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.snapshotHeight} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">px</span>
          </div>
        </div>
      </div>

      <!-- Row 3: Format (Left) + Crop Bottom (Right) -->
      <div class="grid grid-cols-2 gap-2">
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
            </svg>
            <span>{t("flashcards.snapshotFormat")}</span>
          </span>
          <SearchableSelect
            noResultsText={t("common.noResults")}
            options={[
              { value: "webp", label: "WebP" },
              { value: "jpeg", label: "JPEG" },
              { value: "avif", label: "AVIF" },
            ]}
            value={settings.snapshotFormat}
            onchange={(v) => (settings.snapshotFormat = v as SnapshotFormat)}
            placeholder="WebP"
          />
        </div>
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 3v14a2 2 0 002 2h14M3 7h14a2 2 0 012 2v14" />
            </svg>
            <span>{t("flashcards.cropBottom")}</span>
          </span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.cropBottom} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">px</span>
          </div>
        </div>
      </div>

      {#if settings.snapshotFormat === "avif"}
        <p class="text-[10px] text-amber-500/80 leading-snug">{t("flashcards.avifWarning")}</p>
      {/if}
    {/if}
  </div>
</div>
