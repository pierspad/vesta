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
          <div class="space-y-4 rounded-xl border border-gray-800 bg-gray-800/30 p-5 shadow-inner">
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
                    <span class="mb-1 block text-xs text-gray-500">{t("flashcards.bitrate")}</span>
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
                    <span class="mb-1 block text-xs text-gray-500">{t("flashcards.padStart")}</span>
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
                    <span class="mb-1 block text-xs text-gray-500">{t("flashcards.padEnd")}</span>
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
          <div class="space-y-4 rounded-xl border border-gray-800 bg-gray-800/30 p-5 shadow-inner {editor.episode.mediaType !== 'video' ? 'opacity-45' : ''}">
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
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.width")}</span>
                  <div class="flex items-center gap-1">
                    <input type="number" value={editor.overrides.snapshotWidth} oninput={(event) => editor.update("snapshotWidth", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('snapshotWidth')}" />
                    <span class="text-xs text-gray-500">px</span>
                  </div>
                </div>
                <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.height")}</span>
                  <div class="flex items-center gap-1">
                    <input type="number" value={editor.overrides.snapshotHeight} oninput={(event) => editor.update("snapshotHeight", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('snapshotHeight')}" />
                    <span class="text-xs text-gray-500">px</span>
                  </div>
                </div>
                <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.cropBottom")}</span>
                  <div class="flex items-center gap-1">
                    <input type="number" value={editor.overrides.cropBottom} oninput={(event) => editor.update("cropBottom", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('cropBottom')}" />
                    <span class="text-xs text-gray-500">px</span>
                  </div>
                </div>
              </div>
            {/if}
          </div>
          <!-- VIDEO PANEL -->
          <div class="space-y-4 rounded-xl border border-gray-800 bg-gray-800/30 p-5 shadow-inner {editor.episode.mediaType !== 'video' ? 'opacity-45' : ''}">
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
                    <span class="mb-1 block text-xs text-gray-500">{t("flashcards.videoCodec")}</span>
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
                    <span class="mb-1 block text-xs text-gray-500">{t("flashcards.h264Preset")}</span>
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
                    <span class="mb-1 block text-xs text-gray-500">{t("flashcards.videoBitrate")}</span>
                    <div class="flex items-center gap-1">
                      <input type="number" value={editor.overrides.videoBitrate} oninput={(event) => editor.update("videoBitrate", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('videoBitrate')}" />
                      <span class="text-xs text-gray-500">kb/s</span>
                    </div>
                  </div>
                  <div>
                    <span class="mb-1 block text-xs text-gray-500">{t("flashcards.audioBitrate")}</span>
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
                    <span class="mb-1 block text-xs text-gray-500">{t("flashcards.padStart")}</span>
                    <div class="flex items-center gap-1">
                      <input type="number" value={editor.overrides.videoPadStart} oninput={(event) => editor.update("videoPadStart", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('videoPadStart')}" />
                      <span class="text-xs text-gray-500">ms</span>
                    </div>
                  </div>
                  <div>
                    <span class="mb-1 block text-xs text-gray-500">{t("flashcards.padEnd")}</span>
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
