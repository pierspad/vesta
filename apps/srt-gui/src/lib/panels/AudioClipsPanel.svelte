<script lang="ts">
  import { locale } from "$lib/i18n";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import {
    bitratesFor,
    equivalentBitrate,
    formatAudioTrackLabel,
    type AudioFormat,
    type AudioTrackInfo,
    type EpisodeMediaOverrides,
  } from "$lib/types/flashcardMediaTypes";

  interface Props {
    settings: Required<EpisodeMediaOverrides>;
    hasAudio: boolean;
    mediaType: "none" | "video" | "audio";
    audioTracks: AudioTrackInfo[];
    audioTracksLoading: boolean;
    hintLoadMediaFirst: string;
    firstEpisodeMediaPath?: string | null;
    firstEpisodeSubsPath?: string | null;
    /** Called (in addition to updating settings.audioTrackIndex) whenever the
     * user manually picks a track, so the caller can stop auto-selecting it. */
    onTrackPicked: () => void;
  }
  let {
    settings = $bindable(),
    hasAudio,
    mediaType,
    audioTracks,
    audioTracksLoading,
    hintLoadMediaFirst,
    firstEpisodeMediaPath = null,
    firstEpisodeSubsPath = null,
    onTrackPicked,
  }: Props = $props();

  let t = $derived($locale);
  let easyMode = $derived(!uiMode.expertMode);

  let isPreviewPlaying = $state(false);
  let isPreviewLoading = $state(false);
  let audioPlayer: HTMLAudioElement | null = null;

  async function toggleAudioPreview() {
    if (isPreviewPlaying && audioPlayer) {
      audioPlayer.pause();
      isPreviewPlaying = false;
      return;
    }

    const media = firstEpisodeMediaPath;
    if (!media) {
      snackbar.show(hintLoadMediaFirst, "warning", 2000);
      return;
    }

    isPreviewLoading = true;
    try {
      const [port, token] = await invoke<[number, string]>("get_media_server_info");
      const previewFilePath = await invoke<string>("flashcard_preview_audio", {
        mediaPath: media,
        subPath: firstEpisodeSubsPath || null,
        startMs: null,
        endMs: null,
        audioTrackIndex: settings.audioTrackIndex,
        padStartMs: settings.audioPadStart,
        padEndMs: settings.audioPadEnd,
        format: settings.audioFormat,
        normalize: settings.normalizeAudio,
        boost: settings.audioBoost,
        bitrate: settings.audioBitrate,
      });

      const audioUrl = `http://127.0.0.1:${port}/media?path=${encodeURIComponent(previewFilePath)}&token=${token}&_t=${Date.now()}`;
      if (!audioPlayer) {
        audioPlayer = new Audio();
        audioPlayer.onended = () => {
          isPreviewPlaying = false;
        };
        audioPlayer.onerror = () => {
          isPreviewPlaying = false;
          snackbar.show(t("flashcards.previewAudioError"), "error", 2000);
        };
      }
      audioPlayer.src = audioUrl;
      await audioPlayer.play();
      isPreviewPlaying = true;
    } catch (e: any) {
      console.error("Audio preview failed:", e);
      snackbar.show(String(e), "error", 2500);
    } finally {
      isPreviewLoading = false;
    }
  }

  let bitrateOptions = $derived(
    bitratesFor(settings.audioFormat).map((b) => ({ value: String(b), label: `${b} kb/s` })),
  );

  /** Switching codec carries the quality *intent*, not the number: 128k MP3 and
   *  128k Opus are not the same trade-off, and leaving 128k on Opus would throw
   *  away most of the saving the user just asked for. */
  function setAudioFormat(next: AudioFormat) {
    const previous = settings.audioFormat;
    if (previous === next) return;
    settings.audioBitrate = equivalentBitrate(previous, next, settings.audioBitrate);
    settings.audioFormat = next;
  }
</script>

<div
  inert={!hasAudio}
  title={!hasAudio ? hintLoadMediaFirst : undefined}
  class="glass-card p-5 relative z-30 overflow-visible {!hasAudio ? 'opacity-40' : ''}"
>
  <div class="flex items-center justify-between mb-3">
    <h3 class="text-lg font-semibold flex items-center gap-2 text-cyan-400">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z"
        />
      </svg>
      {t("flashcards.generateAudioClips")}
    </h3>
    <button
      onclick={() => {
        if (hasAudio) settings.generateAudio = !settings.generateAudio;
      }}
      class="w-10 h-5 rounded-full transition-all duration-200 relative
        {settings.generateAudio ? 'bg-cyan-500' : 'bg-gray-600'}"
      aria-label="Toggle audio clips"
      disabled={!hasAudio}
    >
      <div
        class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200
        {settings.generateAudio ? 'left-5' : 'left-0.5'}"
      ></div>
    </button>
  </div>

  <div class="space-y-2 transition-all duration-200 {!settings.generateAudio ? 'opacity-40 pointer-events-none' : ''}">
    <div class="grid grid-cols-2 gap-2">
      {#if mediaType === "video" && (audioTracksLoading || audioTracks.length >= 1)}
        <div class={easyMode ? "col-span-2" : ""}>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2z" />
            </svg>
            <span>{t("flashcards.audioTrack")}</span>
          </span>
          {#if audioTracksLoading}
            <div class="input-modern text-xs text-gray-500">
              {t("flashcards.audioTracksLoading")}
            </div>
          {:else if audioTracks.length > 1}
            <SearchableSelect
              noResultsText={t("common.noResults")}
              options={audioTracks.map((track) => ({
                value: String(track.index),
                label: formatAudioTrackLabel(track),
              }))}
              value={settings.audioTrackIndex === null ? "" : String(settings.audioTrackIndex)}
              onchange={(value) => {
                settings.audioTrackIndex = value === "" ? null : Number(value);
                onTrackPicked();
              }}
              placeholder={t("flashcards.audioTrack")}
            />
          {:else}
            <div class="input-modern text-xs text-gray-500 opacity-60 cursor-not-allowed">
              {formatAudioTrackLabel(audioTracks[0])}
            </div>
          {/if}
        </div>
      {/if}

      {#if !easyMode}
        <div class={mediaType === "video" && (audioTracksLoading || audioTracks.length >= 1) ? "" : "col-span-2"}>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
            </svg>
            <span>{t("flashcards.bitrate")}</span>
          </span>
          <SearchableSelect
            noResultsText={t("common.noResults")}
            options={bitrateOptions}
            value={String(settings.audioBitrate)}
            onchange={(v) => (settings.audioBitrate = parseInt(v))}
            placeholder="Bitrate"
          />
        </div>
      {/if}
    </div>

    <!-- Shown in both modes: shrinking a shared deck is the whole reason Opus
         is here, and that is not an expert-only concern. -->
    <label class="vesta-check-row">
      <input
        type="checkbox"
        checked={settings.audioFormat === "opus"}
        onchange={(e) => setAudioFormat(e.currentTarget.checked ? "opus" : "mp3")}
        class="vesta-check-input shrink-0"
      />
      <span class="min-w-0 text-left text-xs font-medium text-gray-300">
        {t("flashcards.compactAudio")}
      </span>
    </label>
    {#if settings.audioFormat === "opus"}
      <p class="text-[10px] text-amber-500/80 leading-snug">{t("flashcards.opusWarning")}</p>
    {/if}
    {#if !easyMode}
      <div class="grid grid-cols-3 gap-2 items-end">
        <div>
          <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 4v16m4-8h11m-3-3l3 3-3 3" />
            </svg>
            <span>{t("flashcards.padStart")}</span>
          </span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={settings.audioPadStart} class="input-modern w-full text-xs" />
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
            <input type="number" bind:value={settings.audioPadEnd} class="input-modern w-full text-xs" />
            <span class="text-xs text-gray-500">ms</span>
          </div>
        </div>
        <div>
          <button
            type="button"
            onclick={() => (settings.normalizeAudio = !settings.normalizeAudio)}
            class="w-full h-8.5 px-2 rounded-lg border transition-all duration-200 flex items-center justify-center gap-1.5 text-xs font-semibold cursor-pointer select-none
              {settings.normalizeAudio
                ? 'bg-cyan-500/25 border-cyan-400 text-cyan-200 shadow-md shadow-cyan-950/40 ring-1 ring-cyan-400/40'
                : 'bg-white/5 border-white/10 text-gray-400 hover:bg-white/10 hover:text-gray-200 hover:border-white/20'}"
            aria-pressed={settings.normalizeAudio}
          >
            <svg class="w-3.5 h-3.5 shrink-0 {settings.normalizeAudio ? 'text-cyan-300' : 'text-gray-500'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              {#if settings.normalizeAudio}
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
              {:else}
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2z" />
              {/if}
            </svg>
            <span class="truncate">{t("flashcards.normalizeAudio")}</span>
          </button>
        </div>
      </div>

      <div class="grid grid-cols-2 gap-2 pt-1">
        <div>
          <button
            type="button"
            onclick={() => (settings.audioBoost = !settings.audioBoost)}
            class="w-full h-8.5 px-2 rounded-lg border transition-all duration-200 flex items-center justify-center gap-1.5 text-xs font-semibold cursor-pointer select-none
              {settings.audioBoost
                ? 'bg-cyan-500/25 border-cyan-400 text-cyan-200 shadow-md shadow-cyan-950/40 ring-1 ring-cyan-400/40'
                : 'bg-white/5 border-white/10 text-gray-400 hover:bg-white/10 hover:text-gray-200 hover:border-white/20'}"
            aria-pressed={settings.audioBoost}
          >
            <svg class="w-3.5 h-3.5 shrink-0 {settings.audioBoost ? 'text-cyan-300' : 'text-gray-500'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              {#if settings.audioBoost}
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M13 10V3L4 14h7v7l9-11h-7z" />
              {:else}
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072M12 6v12m-4-8v4" />
              {/if}
            </svg>
            <span class="truncate">{t("flashcards.audioBoost")}</span>
          </button>
        </div>

        <div>
          <button
            type="button"
            disabled={!hasAudio || isPreviewLoading}
            onclick={toggleAudioPreview}
            class="w-full h-8.5 px-2 rounded-lg border transition-all duration-200 flex items-center justify-center gap-1.5 text-xs font-semibold cursor-pointer select-none disabled:opacity-40 disabled:cursor-not-allowed
              {isPreviewPlaying
                ? 'bg-emerald-500/25 border-emerald-400 text-emerald-200 shadow-md shadow-emerald-950/40 ring-1 ring-emerald-400/40'
                : 'bg-white/5 border-white/10 text-gray-300 hover:bg-white/10 hover:text-white hover:border-white/20'}"
          >
            {#if isPreviewLoading}
              <div class="w-3.5 h-3.5 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin"></div>
            {:else if isPreviewPlaying}
              <svg class="w-3.5 h-3.5 shrink-0 text-emerald-300 animate-pulse" fill="currentColor" viewBox="0 0 24 24">
                <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>
              </svg>
            {:else}
              <svg class="w-3.5 h-3.5 shrink-0 text-gray-400" fill="currentColor" viewBox="0 0 24 24">
                <path d="M8 5v14l11-7z"/>
              </svg>
            {/if}
            <span class="truncate">{isPreviewPlaying ? t("flashcards.previewAudioPlaying") : t("flashcards.previewAudio")}</span>
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
