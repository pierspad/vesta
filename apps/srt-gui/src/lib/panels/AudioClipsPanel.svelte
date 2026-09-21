<script lang="ts">
  import { locale } from "$lib/i18n";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { getFileName } from "$lib/utils/models";
  import {
    bitratesFor,
    equivalentBitrate,
    formatAudioTrackLabel,
    type AudioFormat,
    type AudioTrackInfo,
    type EpisodeMediaOverrides,
  } from "$lib/types/flashcardMediaTypes";

  interface EpisodeItem {
    id: number;
    targetSubsPath: string;
    nativeSubsPath: string;
    mediaPath: string;
    mediaOverrides?: EpisodeMediaOverrides;
  }

  interface SubtitleEntry {
    id: number;
    start_ms: number;
    end_ms: number;
    text: string;
  }

  interface Props {
    settings: Required<EpisodeMediaOverrides>;
    hasAudio: boolean;
    mediaType: "none" | "video" | "audio";
    audioTracks: AudioTrackInfo[];
    audioTracksLoading: boolean;
    hintLoadMediaFirst: string;
    firstEpisodeMediaPath?: string | null;
    firstEpisodeSubsPath?: string | null;
    episodes?: EpisodeItem[];
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
    episodes = [],
    onTrackPicked,
  }: Props = $props();

  let t = $derived($locale);
  let easyMode = $derived(!uiMode.expertMode);

  $effect(() => {
    if (!settings.normalizeAudio) settings.normalizeAudio = true;
  });

  let selectedEpisodeIdx = $state(0);
  let activeEpisode = $derived.by(() => {
    if (episodes && episodes.length > 0) {
      return episodes[selectedEpisodeIdx] ?? episodes[0];
    }
    return null;
  });

  let currentMediaPath = $derived(
    activeEpisode?.mediaPath || firstEpisodeMediaPath || ""
  );
  let currentSubsPath = $derived(
    activeEpisode?.targetSubsPath || firstEpisodeSubsPath || ""
  );

  let isPreviewPlaying = $state(false);
  let previewExpanded = $state(false);
  let isPreviewLoading = $state(false);
  let audioPlayer: HTMLAudioElement | null = null;

  let subEntries = $state<SubtitleEntry[]>([]);
  let currentSubIndex = $state(0);
  let previewTimeSec = $state(120);
  let previewSnapshotUrl = $state<string | null>(null);
  let isSnapshotLoading = $state(false);

  let effectiveNormalize = $derived(
    activeEpisode?.mediaOverrides?.normalizeAudio !== undefined
      ? activeEpisode.mediaOverrides.normalizeAudio
      : settings.normalizeAudio
  );

  let effectiveBoost = $derived(
    activeEpisode?.mediaOverrides?.audioBoost !== undefined
      ? activeEpisode.mediaOverrides.audioBoost
      : settings.audioBoost
  );
  let effectiveGainDb = $derived(activeEpisode?.mediaOverrides?.audioGainDb ?? settings.audioGainDb);

  function adjustGain(delta: number) {
    settings.audioGainDb = Math.max(-12, Math.min(12, settings.audioGainDb + delta));
    settings.audioBoost = false;
  }

  let effectiveTrackIndex = $derived(
    activeEpisode?.mediaOverrides?.audioTrackIndex !== undefined
      ? activeEpisode.mediaOverrides.audioTrackIndex
      : settings.audioTrackIndex
  );

  let maxDurationSec = $derived.by(() => {
    if (subEntries.length > 0) {
      return Math.max(120, Math.ceil(subEntries[subEntries.length - 1].end_ms / 1000));
    }
    return 7200;
  });

  let currentLineStartMs = $derived(
    subEntries.length > 0 && subEntries[currentSubIndex]
      ? subEntries[currentSubIndex].start_ms
      : previewTimeSec * 1000
  );

  let currentLineEndMs = $derived(
    subEntries.length > 0 && subEntries[currentSubIndex]
      ? subEntries[currentSubIndex].end_ms
      : (previewTimeSec + 4) * 1000
  );

  let currentLineText = $derived(
    subEntries.length > 0 && subEntries[currentSubIndex]
      ? subEntries[currentSubIndex].text
      : (t("flashcards.previewAudioNoSubs") || "Nessun sottotitolo: usa il cursore per navigare")
  );

  function formatTime(sec: number): string {
    const s = Math.max(0, Math.floor(sec));
    const m = Math.floor(s / 60);
    const rem = s % 60;
    return `${m}:${rem.toString().padStart(2, "0")}`;
  }

  $effect(() => {
    const sub = currentSubsPath;
    const media = currentMediaPath;
    if (sub) {
      loadSubtitles(sub);
    } else {
      subEntries = [];
      currentSubIndex = 0;
      if (media) {
        updateSnapshot(previewTimeSec * 1000);
      } else {
        previewSnapshotUrl = null;
      }
    }
  });

  async function loadSubtitles(path: string) {
    try {
      const entries = await invoke<SubtitleEntry[]>("flashcard_parse_subtitles", { path });
      subEntries = entries;
      if (entries.length > 0) {
        const goodIdx = entries.findIndex((e) => {
          const dur = e.end_ms - e.start_ms;
          const txt = e.text.trim();
          return (
            dur >= 1200 &&
            dur <= 8000 &&
            txt.length >= 10 &&
            !txt.startsWith("[") &&
            !txt.startsWith("(") &&
            !txt.startsWith("♪") &&
            !txt.startsWith("♫")
          );
        });
        currentSubIndex = goodIdx >= 0 ? goodIdx : 0;
        previewTimeSec = Math.floor(entries[currentSubIndex].start_ms / 1000);
        updateSnapshot(entries[currentSubIndex].start_ms);
      }
    } catch (e) {
      console.error("Failed to parse subtitles for preview:", e);
      subEntries = [];
      currentSubIndex = 0;
    }
  }

  async function updateSnapshot(timeMs: number) {
    if (!currentMediaPath) return;
    const ext = currentMediaPath.split(".").pop()?.toLowerCase() || "";
    if (["mp3", "wav", "flac", "aac", "ogg", "opus", "m4a", "wma"].includes(ext)) {
      previewSnapshotUrl = null;
      return;
    }
    isSnapshotLoading = true;
    try {
      const snapPath = await invoke<string>("flashcard_preview_snapshot", {
        mediaPath: currentMediaPath,
        timeMs: Math.max(0, timeMs),
      });
      const [port, token] = await invoke<[number, string]>("get_media_server_info");
      previewSnapshotUrl = `http://127.0.0.1:${port}/media?path=${encodeURIComponent(snapPath)}&token=${token}&_t=${Date.now()}`;
    } catch {
      // Snapshot is best-effort visual enhancement
    } finally {
      isSnapshotLoading = false;
    }
  }

  function goToPrevDialogue() {
    if (subEntries.length === 0) {
      previewTimeSec = Math.max(0, previewTimeSec - 5);
      updateSnapshot(previewTimeSec * 1000);
      if (isPreviewPlaying) playPreview();
      return;
    }
    if (currentSubIndex > 0) {
      currentSubIndex--;
      previewTimeSec = Math.floor(subEntries[currentSubIndex].start_ms / 1000);
      updateSnapshot(subEntries[currentSubIndex].start_ms);
      if (isPreviewPlaying) playPreview();
    }
  }

  function goToNextDialogue() {
    if (subEntries.length === 0) {
      previewTimeSec += 5;
      updateSnapshot(previewTimeSec * 1000);
      if (isPreviewPlaying) playPreview();
      return;
    }
    if (currentSubIndex < subEntries.length - 1) {
      currentSubIndex++;
      previewTimeSec = Math.floor(subEntries[currentSubIndex].start_ms / 1000);
      updateSnapshot(subEntries[currentSubIndex].start_ms);
      if (isPreviewPlaying) playPreview();
    }
  }

  function onScrub(newTimeSec: number) {
    previewTimeSec = newTimeSec;
    if (subEntries.length > 0) {
      const timeMs = newTimeSec * 1000;
      const closestIdx = subEntries.findIndex((e) => e.start_ms >= timeMs);
      if (closestIdx >= 0) {
        currentSubIndex = closestIdx;
      }
    }
    updateSnapshot(previewTimeSec * 1000);
    if (isPreviewPlaying) playPreview();
  }

  async function playPreview() {
    const media = currentMediaPath;
    if (!media) {
      snackbar.show(hintLoadMediaFirst, "warning", 2000);
      return;
    }

    isPreviewLoading = true;
    try {
      let startMs: number | null = null;
      let endMs: number | null = null;

      if (subEntries.length > 0 && subEntries[currentSubIndex]) {
        startMs = subEntries[currentSubIndex].start_ms;
        endMs = subEntries[currentSubIndex].end_ms;
      } else {
        startMs = previewTimeSec * 1000;
        endMs = (previewTimeSec + 4) * 1000;
      }

      const [port, token] = await invoke<[number, string]>("get_media_server_info");
      const previewFilePath = await invoke<string>("flashcard_preview_audio", {
        mediaPath: media,
        subPath: currentSubsPath || null,
        startMs,
        endMs,
        audioTrackIndex: effectiveTrackIndex,
        padStartMs: settings.audioPadStart,
        padEndMs: settings.audioPadEnd,
        format: settings.audioFormat,
        normalize: effectiveNormalize,
        boost: effectiveBoost,
        gainDb: effectiveGainDb,
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
      isPreviewPlaying = false;
    } finally {
      isPreviewLoading = false;
    }
  }

  function toggleAudioPreview() {
    if (isPreviewPlaying && audioPlayer) {
      audioPlayer.pause();
      isPreviewPlaying = false;
    } else {
      playPreview();
    }
  }

  let bitrateOptions = $derived(
    bitratesFor(settings.audioFormat).map((b) => ({ value: String(b), label: `${b} kb/s` })),
  );

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

  <div class="space-y-3 transition-all duration-200 {!settings.generateAudio ? 'opacity-40 pointer-events-none' : ''}">
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

    <!-- Compact Audio (Opus) Toggle Button -->
    <div>
      <button
        type="button"
        onclick={() => setAudioFormat(settings.audioFormat === "opus" ? "mp3" : "opus")}
        class="w-full h-8.5 px-3 rounded-lg border transition-all duration-200 flex items-center justify-between text-xs font-semibold cursor-pointer select-none
          {settings.audioFormat === 'opus'
            ? 'bg-cyan-500/25 border-cyan-400 text-cyan-200 shadow-md shadow-cyan-950/40 ring-1 ring-cyan-400/40'
            : 'bg-white/5 border-white/10 text-gray-400 hover:bg-white/10 hover:text-gray-200 hover:border-white/20'}"
        aria-pressed={settings.audioFormat === "opus"}
      >
        <span class="flex items-center gap-2">
          <svg class="w-3.5 h-3.5 text-white shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 100-6 3 3 0 000 6z" />
          </svg>
          <span>{t("flashcards.compactAudio")}</span>
        </span>
        <span class="text-[10px] font-mono uppercase tracking-wider px-1.5 py-0.5 rounded {settings.audioFormat === 'opus' ? 'bg-cyan-400/20 text-cyan-200' : 'bg-white/10 text-gray-400'}">
          {settings.audioFormat.toUpperCase()}
        </span>
      </button>
    </div>
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
          <button type="button" onclick={() => (previewExpanded = !previewExpanded)} class="w-full h-8.5 px-2 rounded-lg border border-cyan-400/40 bg-cyan-500/15 text-cyan-200 flex items-center justify-center gap-1.5 text-xs font-semibold">
            <span class="truncate">{t("flashcards.previewAudio")}</span><span>{previewExpanded ? "▴" : "▾"}</span>
          </button>
        </div>
      </div>
    {/if}

    <!-- Interactive Audio Preview Section -->
    {#if previewExpanded}
    <div class="mt-4 p-3.5 rounded-xl border border-cyan-500/25 bg-cyan-950/20 space-y-3">
      <!-- Episode Selector if multiple episodes -->
      {#if episodes && episodes.length > 1}
        <div class="pb-1 border-b border-white/5">
          <span class="flex items-center gap-1.5 text-[11px] text-gray-400 mb-1 font-medium">
            <svg class="w-3 h-3 text-cyan-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z" />
            </svg>
            <span>{t("flashcards.previewAudioEpisode")}</span>
          </span>
          <SearchableSelect
            noResultsText={t("common.noResults")}
            options={episodes.map((ep, idx) => ({
              value: String(idx),
              label: `#${ep.id} - ${getFileName(ep.mediaPath || ep.targetSubsPath || `Episodio ${ep.id}`)}`,
            }))}
            value={String(selectedEpisodeIdx)}
            onchange={(v) => {
              selectedEpisodeIdx = parseInt(v);
              if (isPreviewPlaying && audioPlayer) {
                audioPlayer.pause();
                isPreviewPlaying = false;
              }
            }}
          />
        </div>
      {/if}

      <div class="grid grid-cols-[1fr_auto_1fr] gap-2 items-center">
        <button type="button" onclick={() => adjustGain(-2)} class="h-8 rounded-lg border border-white/10 bg-white/5 text-xs font-semibold text-gray-300 hover:bg-white/10">−2 dB</button>
        <span class="min-w-14 text-center text-xs font-mono text-cyan-300">Gain {settings.audioGainDb > 0 ? "+" : ""}{settings.audioGainDb} dB</span>
        <button type="button" onclick={() => adjustGain(2)} class="h-8 rounded-lg border border-white/10 bg-white/5 text-xs font-semibold text-gray-300 hover:bg-white/10">+2 dB</button>
      </div>

      <!-- Status Bar: Preview Title + Badges -->
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-1.5 text-xs font-semibold text-cyan-300">
          <svg class="w-4 h-4 text-cyan-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072M12 6v12m-4-8v4" />
          </svg>
          <span>{t("flashcards.previewAudio")}</span>
        </div>
        <div class="flex items-center gap-1.5">
          {#if effectiveNormalize}
            <span class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-cyan-500/20 text-cyan-300 border border-cyan-500/30" title="Normalizzazione LUFS (EBU R128 a -14 LUFS)">
              -14 LUFS
            </span>
          {/if}
          {#if effectiveGainDb !== 0}
            <span class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-amber-500/20 text-amber-300 border border-amber-500/30" title="Audio Boost (+6 dB con limiter)">
              {effectiveGainDb > 0 ? "+" : ""}{effectiveGainDb} dB
            </span>
          {/if}
        </div>
      </div>

      <!-- Dialogue line and Snapshot display -->
      <div class="flex gap-3 items-start bg-black/35 p-2.5 rounded-lg border border-white/5">
        {#if previewSnapshotUrl}
          <div class="relative w-24 h-15 shrink-0 rounded overflow-hidden border border-white/10 bg-black shadow-inner">
            <img src={previewSnapshotUrl} alt="Preview frame" class="w-full h-full object-cover" />
            {#if isSnapshotLoading}
              <div class="absolute inset-0 bg-black/60 flex items-center justify-center">
                <div class="w-3.5 h-3.5 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin"></div>
              </div>
            {/if}
          </div>
        {/if}

        <div class="flex-1 min-w-0 space-y-1">
          <div class="flex items-center justify-between text-[11px] font-mono text-gray-400">
            {#if subEntries.length > 0}
              <span class="text-cyan-400 font-semibold">#{currentSubIndex + 1} / {subEntries.length}</span>
              <span>{formatTime(currentLineStartMs / 1000)} - {formatTime(currentLineEndMs / 1000)}</span>
            {:else}
              <span class="text-cyan-400 font-semibold">{formatTime(previewTimeSec)}</span>
              <span class="text-gray-500 text-[10px]">{t("flashcards.previewAudioNoSubs")}</span>
            {/if}
          </div>
          <p class="text-xs text-gray-200 italic leading-snug break-words max-h-12 overflow-y-auto">
            "{currentLineText}"
          </p>
        </div>
      </div>

      <!-- Navigation & Playback Controls -->
      <div class="flex items-center justify-between gap-2 pt-0.5">
        <button
          type="button"
          onclick={goToPrevDialogue}
          disabled={!hasAudio || (subEntries.length > 0 && currentSubIndex <= 0)}
          class="h-8 px-2.5 rounded-lg border border-white/10 bg-white/5 text-gray-300 hover:text-white hover:bg-white/10 transition-all text-xs flex items-center gap-1 cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
          title={t("flashcards.previewAudioPrevDialogue")}
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
          <span>{t("flashcards.previewAudioPrevDialogue")}</span>
        </button>

        <button
          type="button"
          disabled={!hasAudio || isPreviewLoading}
          onclick={toggleAudioPreview}
          class="flex-1 h-8 px-3 rounded-lg border transition-all duration-200 flex items-center justify-center gap-2 text-xs font-semibold cursor-pointer select-none disabled:opacity-40 disabled:cursor-not-allowed
            {isPreviewPlaying
              ? 'bg-emerald-500/25 border-emerald-400 text-emerald-200 shadow-md shadow-emerald-950/40 ring-1 ring-emerald-400/40'
              : 'bg-cyan-500/20 border-cyan-400/50 text-cyan-200 hover:bg-cyan-500/30 hover:border-cyan-400'}"
        >
          {#if isPreviewLoading}
            <div class="w-3.5 h-3.5 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin"></div>
          {:else if isPreviewPlaying}
            <svg class="w-3.5 h-3.5 shrink-0 text-emerald-300 animate-pulse" fill="currentColor" viewBox="0 0 24 24">
              <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>
            </svg>
          {:else}
            <svg class="w-3.5 h-3.5 shrink-0 text-cyan-300" fill="currentColor" viewBox="0 0 24 24">
              <path d="M8 5v14l11-7z"/>
            </svg>
          {/if}
          <span class="truncate">{isPreviewPlaying ? t("flashcards.previewAudioPlaying") : "Play audio"}</span>
        </button>

        <button
          type="button"
          onclick={goToNextDialogue}
          disabled={!hasAudio || (subEntries.length > 0 && currentSubIndex >= subEntries.length - 1)}
          class="h-8 px-2.5 rounded-lg border border-white/10 bg-white/5 text-gray-300 hover:text-white hover:bg-white/10 transition-all text-xs flex items-center gap-1 cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
          title={t("flashcards.previewAudioNextDialogue")}
        >
          <span>{t("flashcards.previewAudioNextDialogue")}</span>
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
          </svg>
        </button>
      </div>

      <!-- Scrubber Slider -->
      <div class="flex items-center gap-2 pt-0.5">
        <span class="text-[10px] font-mono text-gray-400 w-8 text-right shrink-0">{formatTime(previewTimeSec)}</span>
        <input
          type="range"
          min="0"
          max={maxDurationSec}
          step="1"
          value={previewTimeSec}
          oninput={(e) => onScrub(Number((e.currentTarget as HTMLInputElement).value))}
          class="slider-minimal w-full"
          disabled={!hasAudio}
        />
        <span class="text-[10px] font-mono text-gray-500 w-8 shrink-0">{formatTime(maxDurationSec)}</span>
      </div>
    </div>
    {/if}
  </div>
</div>
