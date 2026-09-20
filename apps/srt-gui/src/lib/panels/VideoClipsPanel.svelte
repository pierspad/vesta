<script lang="ts">
  import { locale } from "$lib/i18n";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import {
    VIDEO_QUALITY_STEPS,
    matchVideoQualityStep,
    type EpisodeMediaOverrides,
    type VideoQualityStep,
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

  let activeVideoQuality = $derived(
    matchVideoQualityStep(settings.videoBitrate, settings.videoWidth, settings.videoHeight)
  );

  function applyVideoQualityStep(stepOrId: VideoQualityStep | string) {
    const step = typeof stepOrId === "string" ? VIDEO_QUALITY_STEPS.find((s) => s.id === stepOrId) : stepOrId;
    if (!step) return;
    settings.videoBitrate = step.videoBitrate;
    settings.videoAudioBitrate = step.videoAudioBitrate;
    settings.h264Preset = step.h264Preset;
    settings.videoWidth = step.width;
    settings.videoHeight = step.height;
  }
</script>

<div
  inert={!hasVideo}
  title={!hasVideo ? hintLoadVideoFirst : undefined}
  class="glass-card p-5 relative z-20 overflow-visible {!hasVideo ? 'opacity-40' : ''}"
>
  <div class="flex items-center justify-between mb-3">
    <h3 class="text-lg font-semibold flex items-center gap-2 text-orange-400">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"
        />
      </svg>
      <span class="flex flex-col">
        <span>{t("flashcards.generateVideoClips")}</span>
        {#if effectiveExportFormat === "apkg"}
          <span class="text-[10px] text-orange-300/60 font-normal normal-case mt-0.5">
            {t("flashcards.videoExclusiveHint")}
          </span>
        {/if}
      </span>
    </h3>
    <button
      onclick={() => {
        if (hasVideo) settings.generateVideoClips = !settings.generateVideoClips;
      }}
      class="w-10 h-5 rounded-full transition-all duration-200 relative
        {settings.generateVideoClips ? 'bg-orange-500' : 'bg-gray-600'}"
      aria-label="Toggle video clips"
      disabled={!hasVideo}
    >
      <div
        class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200
        {settings.generateVideoClips ? 'left-5' : 'left-0.5'}"
      ></div>
    </button>
  </div>

  <div class="space-y-2 transition-all duration-200 {!settings.generateVideoClips ? 'opacity-40 pointer-events-none' : ''}">
    {#if easyMode}
      <!-- Easy Mode: Simple Video Quality vs Weight Balance Selector -->
      <div class="space-y-3 bg-gray-950/30 p-3.5 rounded-xl border border-white/5">
        <div class="flex items-center justify-between text-xs">
          <span class="font-medium text-gray-300 flex items-center gap-1.5">
            <svg class="w-4 h-4 text-orange-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
            </svg>
            {t("flashcards.videoQualityVsSize")}
          </span>
          <span class="text-[11px] font-semibold px-2 py-0.5 rounded-full bg-orange-500/20 text-orange-300 border border-orange-500/30">
            {activeVideoQuality ? t(`flashcards.quality.${activeVideoQuality.id}`) : t("flashcards.custom")}
          </span>
        </div>

        <div class="grid grid-cols-3 gap-1.5 p-1 bg-black/40 rounded-xl border border-white/5">
          {#each VIDEO_QUALITY_STEPS as step}
            <button
              type="button"
              onclick={() => applyVideoQualityStep(step)}
              class="py-1.5 px-2 rounded-lg text-xs font-medium transition-all duration-150 flex flex-col items-center gap-0.5
                {activeVideoQuality?.id === step.id
                  ? 'bg-orange-500 text-white shadow-md shadow-orange-900/40 font-semibold scale-[1.02]'
                  : 'text-gray-400 hover:text-gray-200 hover:bg-white/5'}"
            >
              <span>{t(`flashcards.quality.${step.id}`)}</span>
              <span class="text-[9px] opacity-70">
                {step.id === 'light' ? '~0.5 MB' : step.id === 'balanced' ? '~1.2 MB' : '~2.5 MB'}
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
              <svg class="w-3 h-3 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 10l7-7m0 0l7 7m-7-7v18" />
              </svg>
            </span>
          </div>
          <div class="h-2 rounded-full bg-gray-800/80 p-0.5 relative overflow-hidden">
            <div
              class="h-full w-full rounded-full transition-[clip-path] duration-300 ease-out"
              style:background="linear-gradient(90deg, #059669 0%, #10b981 18%, #34d399 35%, #22c55e 38%, #84cc16 52%, #eab308 68%, #f97316 85%, #dc2626 100%)"
              style:clip-path="inset(0 {activeVideoQuality?.id === 'light' ? '68%' : activeVideoQuality?.id === 'balanced' ? '32%' : '0%'} 0 0)"
            ></div>
          </div>
        </div>
      </div>
    {:else}
      <!-- Expert Mode: Fine-grained video options -->
      <div class="grid grid-cols-2 gap-2">
        <div class="col-span-2">
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" />
            </svg>
            <span>{t("flashcards.quality")}</span>
          </span>
          <SearchableSelect
            noResultsText={t("common.noResults")}
            options={[
              ...VIDEO_QUALITY_STEPS.map((s) => ({
                value: s.id,
                label: `${t(`flashcards.quality.${s.id}`)} (${s.width}x${s.height} · ${s.videoBitrate} kb/s)`,
              })),
              ...(activeVideoQuality ? [] : [{ value: CUSTOM, label: `${t("flashcards.custom")} (${settings.videoWidth}x${settings.videoHeight} · ${settings.videoBitrate} kb/s)` }]),
            ]}
            value={activeVideoQuality?.id ?? CUSTOM}
            onchange={(val) => {
              if (val !== CUSTOM) applyVideoQualityStep(val);
            }}
            placeholder={t("flashcards.quality")}
          />
        </div>
      </div>
      <div class="grid grid-cols-2 gap-2">
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h8m0 0l-2.5-2.5M16 7l-2.5 2.5M8 7l2.5-2.5M8 7l2.5 2.5M4 4v16m16-16v16" />
            </svg>
            <span>{t("flashcards.width")}</span>
          </span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.videoWidth} class="input-modern w-full text-xs" />
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
            <input type="number" bind:value={settings.videoHeight} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">px</span>
          </div>
        </div>
      </div>
      <div class="grid grid-cols-2 gap-2">
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 01-2-2V8a2 2 0 012-2z" />
            </svg>
            <span>{t("flashcards.videoCodec")}</span>
          </span>
          <SearchableSelect
            className="compact-select"
            noResultsText={t("common.noResults")}
            options={[
              { value: "h264", label: "H.264 (MP4)" },
              { value: "mpeg4", label: "MPEG-4 (AVI)" },
            ]}
            value={settings.videoCodec}
            onchange={(v) => (settings.videoCodec = v)}
            placeholder="Codec"
          />
        </div>
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
            <span>{t("flashcards.h264Preset")}</span>
          </span>
          <SearchableSelect
            className="compact-select"
            noResultsText={t("common.noResults")}
            options={[
              { value: "ultrafast", label: "Ultrafast" },
              { value: "fast", label: "Fast" },
              { value: "medium", label: "Medium" },
              { value: "slow", label: "Slow" },
              { value: "veryslow", label: "Very slow" },
            ]}
            value={settings.h264Preset}
            onchange={(v) => (settings.h264Preset = v)}
            placeholder="Preset"
          />
        </div>
      </div>
      <div class="grid grid-cols-2 gap-2">
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
            </svg>
            <span>{t("flashcards.videoBitrate")}</span>
          </span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.videoBitrate} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">kb/s</span>
          </div>
        </div>
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
            </svg>
            <span>{t("flashcards.audioBitrate")}</span>
          </span>
          <SearchableSelect
            className="compact-select"
            noResultsText={t("common.noResults")}
            options={[
              { value: "64", label: "64 kb/s" },
              { value: "128", label: "128 kb/s" },
              { value: "192", label: "192 kb/s" },
              { value: "256", label: "256 kb/s" },
            ]}
            value={String(settings.videoAudioBitrate)}
            onchange={(v) => (settings.videoAudioBitrate = parseInt(v))}
            placeholder="Bitrate"
          />
        </div>
      </div>
      <div class="grid grid-cols-2 gap-2">
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 4v16m4-8h11m-3-3l3 3-3 3" />
            </svg>
            <span>{t("flashcards.padStart")}</span>
          </span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.videoPadStart} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">ms</span>
          </div>
        </div>
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 4v16m-4-8H4m3-3l-3 3 3 3" />
            </svg>
            <span>{t("flashcards.padEnd")}</span>
          </span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.videoPadEnd} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">ms</span>
          </div>
        </div>
      </div>
    {/if}
  </div>

  {#if settings.generateVideoClips && !settings.generateAudio}
    <div class="mt-3 p-3 bg-amber-500/10 border border-amber-500/20 text-amber-200 rounded-xl text-xs flex items-start gap-2">
      <svg class="w-4.5 h-4.5 text-amber-400 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
      </svg>
      <div>
        <p class="font-bold text-amber-300">{t("flashcards.videoAudioDisabledTitle")}</p>
        <p class="opacity-90">{t("flashcards.videoAudioDisabledDesc")}</p>
      </div>
    </div>
  {/if}
</div>
