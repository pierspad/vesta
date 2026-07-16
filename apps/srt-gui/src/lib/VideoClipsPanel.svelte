<script lang="ts">
  import { locale, currentLanguage } from "./i18n";
  import { uiMode } from "./uiModeStore.svelte";
  import SearchableSelect from "./SearchableSelect.svelte";
  import type { EpisodeMediaOverrides } from "./flashcardMediaTypes";

  interface Props {
    settings: Required<EpisodeMediaOverrides>;
    /** GPU-accel preference: not per-episode overridable, so it isn't part of `settings`. */
    videoHwAccel: string;
    hasVideo: boolean;
    effectiveExportFormat: "tsv" | "apkg" | "anki";
    hintLoadVideoFirst: string;
  }
  let { settings = $bindable(), videoHwAccel = $bindable(), hasVideo, effectiveExportFormat, hintLoadVideoFirst }: Props = $props();

  let t = $derived($locale);
  let easyMode = $derived(!uiMode.expertMode);
</script>

<div
  inert={!hasVideo}
  title={!hasVideo ? hintLoadVideoFirst : undefined}
  class="glass-card p-5 relative z-5 overflow-visible {!hasVideo ? 'opacity-40' : ''}"
>
  <div class="flex items-center justify-between mb-3">
    <h3 class="text-lg font-semibold flex items-center gap-2 text-rose-400">
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
          <span class="text-[10px] text-rose-300/60 font-normal normal-case mt-0.5">
            {$currentLanguage === "it" ? "Mutualmente esclusivo con gli snapshot in APKG" : "Mutually exclusive with snapshots in APKG"}
          </span>
        {/if}
      </span>
    </h3>
    <button
      onclick={() => {
        if (hasVideo) settings.generateVideoClips = !settings.generateVideoClips;
      }}
      class="w-10 h-5 rounded-full transition-all duration-200 relative
        {settings.generateVideoClips ? 'bg-rose-500' : 'bg-gray-600'}"
      aria-label="Toggle video clips"
      disabled={!hasVideo}
    >
      <div
        class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200
        {settings.generateVideoClips ? 'left-5' : 'left-0.5'}"
      ></div>
    </button>
  </div>

  {#if !easyMode}
    <div class="space-y-2 transition-all duration-200 {!settings.generateVideoClips ? 'opacity-40 pointer-events-none' : ''}">
      <div class="grid grid-cols-2 gap-2">
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
      </div>
      <div class="grid grid-cols-2 gap-2">
        <div>
          <span class="block text-xs text-gray-500 mb-1">{t("flashcards.videoCodec")}</span>
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
          <span class="block text-xs text-gray-500 mb-1">{t("flashcards.h264Preset")}</span>
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
      {#if uiMode.expertMode && settings.videoCodec === "h264"}
        <div>
          <span class="block text-xs text-gray-500 mb-1">{t("flashcards.videoEncoder")}</span>
          <SearchableSelect
            className="compact-select"
            noResultsText={t("common.noResults")}
            options={[
              { value: "auto", label: t("flashcards.videoEncoderAuto") },
              { value: "off", label: t("flashcards.videoEncoderX264") },
            ]}
            value={videoHwAccel}
            onchange={(v) => (videoHwAccel = v)}
            placeholder="Encoder"
          />
        </div>
      {/if}
      <div class="grid grid-cols-2 gap-2">
        <div>
          <span class="block text-xs text-gray-500 mb-1">{t("flashcards.videoBitrate")}</span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.videoBitrate} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">kb/s</span>
          </div>
        </div>
        <div>
          <span class="block text-xs text-gray-500 mb-1">{t("flashcards.audioBitrate")}</span>
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
          <span class="block text-xs text-gray-500 mb-1">{t("flashcards.padStart")}</span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.videoPadStart} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">ms</span>
          </div>
        </div>
        <div>
          <span class="block text-xs text-gray-500 mb-1">{t("flashcards.padEnd")}</span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.videoPadEnd} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">ms</span>
          </div>
        </div>
      </div>
    </div>
  {/if}

  {#if settings.generateVideoClips && !settings.generateAudio}
    <div class="mt-3 p-3 bg-amber-500/10 border border-amber-500/20 text-amber-200 rounded-xl text-xs flex items-start gap-2">
      <svg class="w-4.5 h-4.5 text-amber-400 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
      </svg>
      <div>
        <p class="font-bold text-amber-300">{$currentLanguage === "it" ? "Audio Disattivato" : "Audio Disabled"}</p>
        <p class="opacity-90">{$currentLanguage === "it" ? "Le clip video verranno generate senza audio (mute)." : "Video clips will be generated without audio (silent)."}</p>
      </div>
    </div>
  {/if}
</div>
