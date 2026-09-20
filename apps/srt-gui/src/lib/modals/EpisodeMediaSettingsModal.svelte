<script lang="ts">
  import { locale } from "$lib/i18n";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import { formatAudioTrackLabel, type EpisodeMediaOverrideKey } from "$lib/types/flashcardMediaTypes";
  import { episodeMediaEditorStore as editor } from "$lib/stores/episodeMediaEditorStore.svelte";
  import { getFileName } from "$lib/utils/models";

  interface Props {
    /** Whether a given override key differs from the generic (movie-mode) setting — drives the "changed" glow. */
    mediaOverrideClass: (key: EpisodeMediaOverrideKey) => string;
    onSave: () => void;
    onReset: () => void;
  }
  let { mediaOverrideClass, onSave, onReset }: Props = $props();

  let t = $derived($locale);

  function close() {
    editor.close();
  }
</script>

{#if editor.episode && editor.overrides}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/65 p-6"
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
      class="flex max-h-[92vh] w-[96vw] flex-col rounded-xl border border-gray-700 bg-gray-900 shadow-2xl"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="flex items-center justify-between gap-3 border-b border-gray-700 px-5 py-4">
        <div class="min-w-0">
          <p class="text-xs uppercase tracking-wide text-violet-300">
            {t("flashcards.perMovieSettings")}
          </p>
          <h3 class="truncate text-lg font-bold text-white" title={editor.episode.mediaPath}>
            {getFileName(editor.episode.mediaPath)}
          </h3>
        </div>
        <button type="button" onclick={close} class="dialog-close-button p-1 text-xl leading-none text-gray-400 hover:text-white" aria-label={t("common.close")}>×</button>
      </div>

      <div class="flex-1 overflow-y-auto p-5">
        <div class="media-settings-panels">
          <!-- AUDIO PANEL -->
          <div class="relative z-30 space-y-4 rounded-xl border border-gray-800 bg-gray-800/30 p-5 shadow-inner">
            <div class="flex items-center justify-between rounded-lg border border-cyan-500/20 bg-cyan-500/10 p-3">
              <span class="text-sm font-semibold text-cyan-200">
                {t("flashcards.generateAudioClips")}
              </span>
              <button
                type="button"
                aria-label={t("flashcards.generateAudioClips")}
                class="relative h-5 w-10 rounded-full transition-colors {editor.overrides.generateAudio ? 'bg-cyan-500' : 'bg-gray-600'} {mediaOverrideClass('generateAudio')}"
                onclick={() => editor.update("generateAudio", !editor.overrides?.generateAudio)}
              >
                <span class="absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all {editor.overrides.generateAudio ? 'left-5' : 'left-0.5'}"></span>
              </button>
            </div>

            {#if editor.overrides.generateAudio}
              <div class="space-y-4 animate-fade-in">
                {#if editor.episode.mediaType === "video"}
                  <div>
                    <span class="mb-1 block text-xs text-gray-500">{t("flashcards.audioTrack")}</span>
                    {#if editor.audioTracksLoading}
                      <div class="input-modern text-xs text-gray-500">{t("flashcards.audioTracksLoading")}</div>
                    {:else if editor.audioTracks.length > 1}
                      <SearchableSelect
                        className={mediaOverrideClass("audioTrackIndex")}
                        noResultsText={t("common.noResults")}
                        options={editor.audioTracks.map((track) => ({
                          value: String(track.index),
                          label: formatAudioTrackLabel(track),
                        }))}
                        value={editor.overrides.audioTrackIndex === null ? "" : String(editor.overrides.audioTrackIndex)}
                        onchange={(value) => editor.update("audioTrackIndex", value === "" ? null : Number(value))}
                        placeholder={t("flashcards.audioTrack")}
                      />
                    {:else if editor.audioTracks.length === 1}
                      <div class="input-modern text-xs text-gray-500 opacity-60 cursor-not-allowed {mediaOverrideClass('audioTrackIndex')}">
                        {formatAudioTrackLabel(editor.audioTracks[0])}
                      </div>
                    {:else}
                      <div class="input-modern text-xs text-gray-500 {mediaOverrideClass('audioTrackIndex')}">
                        {t("flashcards.audioTrackAuto")}
                      </div>
                    {/if}
                  </div>
                {/if}

                <div class="grid grid-cols-2 gap-3">
                  <div>
                    <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                      <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
                      </svg>
                      <span>{t("flashcards.bitrate")}</span>
                    </span>
                    <SearchableSelect
                      className={mediaOverrideClass("audioBitrate")}
                      noResultsText={t("common.noResults")}
                      options={[
                        { value: "64", label: "64 kb/s" },
                        { value: "128", label: "128 kb/s" },
                        { value: "192", label: "192 kb/s" },
                        { value: "256", label: "256 kb/s" },
                        { value: "320", label: "320 kb/s" },
                      ]}
                      value={String(editor.overrides.audioBitrate)}
                      onchange={(v) => editor.update("audioBitrate", parseInt(v))}
                      placeholder="Bitrate"
                    />
                  </div>
                  <label class="vesta-check-row mt-5">
                    <input
                      type="checkbox"
                      checked={!!editor.overrides.normalizeAudio}
                      onchange={(event) => editor.update("normalizeAudio", (event.currentTarget as HTMLInputElement).checked)}
                      class="vesta-check-input shrink-0 {mediaOverrideClass('normalizeAudio')}"
                    />
                    <span class="text-xs font-medium text-gray-300">{t("flashcards.normalizeAudio")}</span>
                  </label>
                </div>

                <div class="grid grid-cols-2 gap-3">
                  <div>
                    <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                      <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 4v16m4-8h11m-3-3l3 3-3 3" />
                      </svg>
                      <span>{t("flashcards.padStart")}</span>
                    </span>
                    <div class="flex items-center gap-1">
                      <input
                        type="number"
                        value={editor.overrides.audioPadStart}
                        oninput={(event) => editor.update("audioPadStart", Number((event.currentTarget as HTMLInputElement).value))}
                        class="input-modern w-full text-xs {mediaOverrideClass('audioPadStart')}"
                      />
                      <span class="text-xs text-gray-500">ms</span>
                    </div>
                  </div>
                  <div>
                    <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                      <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 4v16m-4-8H4m3-3l-3 3 3 3" />
                      </svg>
                      <span>{t("flashcards.padEnd")}</span>
                    </span>
                    <div class="flex items-center gap-1">
                      <input
                        type="number"
                        value={editor.overrides.audioPadEnd}
                        oninput={(event) => editor.update("audioPadEnd", Number((event.currentTarget as HTMLInputElement).value))}
                        class="input-modern w-full text-xs {mediaOverrideClass('audioPadEnd')}"
                      />
                      <span class="text-xs text-gray-500">ms</span>
                    </div>
                  </div>
                </div>
              </div>
            {/if}
          </div>
          <!-- SNAPSHOT PANEL -->
          <div class="relative z-20 space-y-4 rounded-xl border border-gray-800 bg-gray-800/30 p-5 shadow-inner {editor.episode.mediaType !== 'video' ? 'opacity-45' : ''}">
            <div class="flex items-center justify-between rounded-lg border border-purple-500/20 bg-purple-500/10 p-3">
              <span class="text-sm font-semibold text-purple-200">
                {t("flashcards.generateSnapshots")}
              </span>
              <button
                type="button"
                aria-label={t("flashcards.generateSnapshots")}
                disabled={editor.episode.mediaType !== "video"}
                class="relative h-5 w-10 rounded-full transition-colors {editor.overrides.generateSnapshots && editor.episode.mediaType === 'video' ? 'bg-purple-500' : 'bg-gray-600'} {mediaOverrideClass('generateSnapshots')}"
                onclick={() => editor.update("generateSnapshots", !editor.overrides?.generateSnapshots)}
              >
                <span class="absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all {editor.overrides.generateSnapshots && editor.episode.mediaType === 'video' ? 'left-5' : 'left-0.5'}"></span>
              </button>
            </div>

            {#if editor.overrides.generateSnapshots && editor.episode.mediaType === "video"}
              <div class="grid grid-cols-3 gap-3 animate-fade-in">
                <div>
                  <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                    <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h8m0 0l-2.5-2.5M16 7l-2.5 2.5M8 7l2.5-2.5M8 7l2.5 2.5M4 4v16m16-16v16" />
                    </svg>
                    <span>{t("flashcards.width")}</span>
                  </span>
                  <div class="flex items-center gap-1">
                    <input type="number" value={editor.overrides.snapshotWidth} oninput={(event) => editor.update("snapshotWidth", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('snapshotWidth')}" />
                    <span class="text-xs text-gray-500">px</span>
                  </div>
                </div>
                <div>
                  <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                    <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 8v8m0 0l-2.5-2.5M7 16l2.5-2.5M7 8l-2.5 2.5M7 8l2.5 2.5M4 4h16M4 20h16" />
                    </svg>
                    <span>{t("flashcards.height")}</span>
                  </span>
                  <div class="flex items-center gap-1">
                    <input type="number" value={editor.overrides.snapshotHeight} oninput={(event) => editor.update("snapshotHeight", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('snapshotHeight')}" />
                    <span class="text-xs text-gray-500">px</span>
                  </div>
                </div>
                <div>
                  <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                    <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 3v14a2 2 0 002 2h14M3 7h14a2 2 0 012 2v14" />
                    </svg>
                    <span>{t("flashcards.cropBottom")}</span>
                  </span>
                  <div class="flex items-center gap-1">
                    <input type="number" value={editor.overrides.cropBottom} oninput={(event) => editor.update("cropBottom", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('cropBottom')}" />
                    <span class="text-xs text-gray-500">px</span>
                  </div>
                </div>
                <div>
                  <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                    <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4M7.835 4.697a3.42 3.42 0 001.946-.806 3.42 3.42 0 014.438 0 3.42 3.42 0 001.946.806 3.42 3.42 0 013.138 3.138 3.42 3.42 0 00.806 1.946 3.42 3.42 0 010 4.438 3.42 3.42 0 00-.806 1.946 3.42 3.42 0 01-3.138 3.138 3.42 3.42 0 00-1.946.806 3.42 3.42 0 01-4.438 0 3.42 3.42 0 00-1.946-.806 3.42 3.42 0 01-3.138-3.138 3.42 3.42 0 00-.806-1.946 3.42 3.42 0 010-4.438 3.42 3.42 0 00.806-1.946 3.42 3.42 0 013.138-3.138z" />
                    </svg>
                    <span>{t("flashcards.qualityValue")}</span>
                  </span>
                  <div class="flex items-center gap-1">
                    <input type="number" min="0" max="100" value={editor.overrides.snapshotQuality} oninput={(event) => editor.update("snapshotQuality", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('snapshotQuality')}" />
                    <span class="text-xs text-gray-500">/100</span>
                  </div>
                </div>
              </div>
            {/if}
          </div>
          <!-- VIDEO PANEL -->
          <div class="relative z-10 space-y-4 rounded-xl border border-gray-800 bg-gray-800/30 p-5 shadow-inner {editor.episode.mediaType !== 'video' ? 'opacity-45' : ''}">
            <div class="flex items-center justify-between rounded-lg border border-rose-500/20 bg-rose-500/10 p-3">
              <span class="text-sm font-semibold text-rose-200">
                {t("flashcards.generateVideoClips")}
              </span>
              <button
                type="button"
                aria-label={t("flashcards.generateVideoClips")}
                disabled={editor.episode.mediaType !== "video"}
                class="relative h-5 w-10 rounded-full transition-colors {editor.overrides.generateVideoClips && editor.episode.mediaType === 'video' ? 'bg-rose-500' : 'bg-gray-600'} {mediaOverrideClass('generateVideoClips')}"
                onclick={() => editor.update("generateVideoClips", !editor.overrides?.generateVideoClips)}
              >
                <span class="absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all {editor.overrides.generateVideoClips && editor.episode.mediaType === 'video' ? 'left-5' : 'left-0.5'}"></span>
              </button>
            </div>

            {#if editor.overrides.generateVideoClips && editor.episode.mediaType === "video"}
              <div class="space-y-4 animate-fade-in">
                <div class="grid grid-cols-2 gap-3">
                  <div>
                    <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                      <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h8m0 0l-2.5-2.5M16 7l-2.5 2.5M8 7l2.5-2.5M8 7l2.5 2.5M4 4v16m16-16v16" />
                      </svg>
                      <span>{t("flashcards.width")}</span>
                    </span>
                    <div class="flex items-center gap-1">
                      <input type="number" value={editor.overrides.videoWidth} oninput={(event) => editor.update("videoWidth", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('videoWidth')}" />
                      <span class="text-xs text-gray-500">px</span>
                    </div>
                  </div>
                  <div>
                    <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                      <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 8v8m0 0l-2.5-2.5M7 16l2.5-2.5M7 8l-2.5 2.5M7 8l2.5 2.5M4 4h16M4 20h16" />
                      </svg>
                      <span>{t("flashcards.height")}</span>
                    </span>
                    <div class="flex items-center gap-1">
                      <input type="number" value={editor.overrides.videoHeight} oninput={(event) => editor.update("videoHeight", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('videoHeight')}" />
                      <span class="text-xs text-gray-500">px</span>
                    </div>
                  </div>
                </div>
                <div class="grid grid-cols-2 gap-3">
                  <div>
                    <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                      <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
                      </svg>
                      <span>{t("flashcards.videoCodec")}</span>
                    </span>
                    <SearchableSelect
                      className="compact-select {mediaOverrideClass('videoCodec')}"
                      noResultsText={t("common.noResults")}
                      options={[
                        { value: "h264", label: "H.264 (MP4)" },
                        { value: "mpeg4", label: "MPEG-4 (AVI)" },
                      ]}
                      value={editor.overrides.videoCodec}
                      onchange={(v) => editor.update("videoCodec", v)}
                      placeholder="Codec"
                    />
                  </div>
                  <div>
                    <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                      <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                      </svg>
                      <span>{t("flashcards.h264Preset")}</span>
                    </span>
                    <SearchableSelect
                      className="compact-select {mediaOverrideClass('h264Preset')}"
                      noResultsText={t("common.noResults")}
                      options={[
                        { value: "ultrafast", label: "Ultrafast" },
                        { value: "fast", label: "Fast" },
                        { value: "medium", label: "Medium" },
                        { value: "slow", label: "Slow" },
                        { value: "veryslow", label: "Very slow" },
                      ]}
                      value={editor.overrides.h264Preset}
                      onchange={(v) => editor.update("h264Preset", v)}
                      placeholder="Preset"
                    />
                  </div>
                </div>
                <div class="grid grid-cols-2 gap-3">
                  <div>
                    <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                      <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
                      </svg>
                      <span>{t("flashcards.videoBitrate")}</span>
                    </span>
                    <div class="flex items-center gap-1">
                      <input type="number" value={editor.overrides.videoBitrate} oninput={(event) => editor.update("videoBitrate", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('videoBitrate')}" />
                      <span class="text-xs text-gray-500">kb/s</span>
                    </div>
                  </div>
                  <div>
                    <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                      <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
                      </svg>
                      <span>{t("flashcards.audioBitrate")}</span>
                    </span>
                    <SearchableSelect
                      className="compact-select {mediaOverrideClass('videoAudioBitrate')}"
                      noResultsText={t("common.noResults")}
                      options={[
                        { value: "64", label: "64 kb/s" },
                        { value: "128", label: "128 kb/s" },
                        { value: "192", label: "192 kb/s" },
                        { value: "256", label: "256 kb/s" },
                      ]}
                      value={String(editor.overrides.videoAudioBitrate)}
                      onchange={(v) => editor.update("videoAudioBitrate", parseInt(v))}
                      placeholder="Bitrate"
                    />
                  </div>
                </div>
                <div class="grid grid-cols-2 gap-3">
                  <div>
                    <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                      <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 4v16m4-8h11m-3-3l3 3-3 3" />
                      </svg>
                      <span>{t("flashcards.padStart")}</span>
                    </span>
                    <div class="flex items-center gap-1">
                      <input type="number" value={editor.overrides.videoPadStart} oninput={(event) => editor.update("videoPadStart", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('videoPadStart')}" />
                      <span class="text-xs text-gray-500">ms</span>
                    </div>
                  </div>
                  <div>
                    <span class="mb-1 flex items-center gap-1.5 text-xs text-gray-400 font-medium">
                      <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 4v16m-4-8H4m3-3l-3 3 3 3" />
                      </svg>
                      <span>{t("flashcards.padEnd")}</span>
                    </span>
                    <div class="flex items-center gap-1">
                      <input type="number" value={editor.overrides.videoPadEnd} oninput={(event) => editor.update("videoPadEnd", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('videoPadEnd')}" />
                      <span class="text-xs text-gray-500">ms</span>
                    </div>
                  </div>
                </div>
              </div>
            {/if}
          </div>
        </div>
      </div>

      <div class="flex items-center justify-between gap-3 border-t border-gray-700 px-5 py-4">
        <button type="button" onclick={onReset} class="btn-secondary px-4 py-2 text-sm">
          {t("flashcards.useGenericSettings")}
        </button>
        <div class="flex gap-2">
          <button type="button" onclick={close} class="btn-secondary px-4 py-2 text-sm">
            {t("settings.modal.cancel")}
          </button>
          <button
            type="button"
            disabled={!editor.isDirty}
            onclick={onSave}
            class="rounded-lg border border-violet-400/40 bg-violet-500/20 px-4 py-2 text-sm font-semibold text-violet-100 shadow-lg shadow-violet-500/10 transition-all hover:border-violet-300/60 hover:bg-violet-500/30 disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:border-violet-400/40 disabled:hover:bg-violet-500/20"
          >
            {t("settings.modal.save")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
