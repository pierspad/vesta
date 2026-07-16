<script lang="ts">
  import { locale } from "./i18n";
  import SectionHeader from "./components/SectionHeader.svelte";
  import WizardEmptyState from "./WizardEmptyState.svelte";

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
    srtLoaded: boolean;
    totalSubtitles: number;
    showSaveSuggestion: boolean;
    wizardSubtitle: SubtitleInfo | null;
    audioSrc: string | null;
    audioError: string | null;
    isPreparingMedia: boolean;
    currentVideoTime: number;
    audioDuration: number;
    audioElement: HTMLMediaElement | null;
    isPlaying: boolean;
    hasAudio: boolean;
    offsetAdjustment: number;
    formatTime: (ms: number) => string;
    formatOffset: (ms: number) => string;
    onSaveFile: () => void;
    onContinueChecking: () => void;
    onRetryAudio: () => void;
    onTogglePlay: () => void;
    onReplay: () => void;
    onAdjustOffset: (delta: number) => void;
    onSkip: () => void;
    onConfirm: () => void;
  }

  let {
    srtLoaded,
    totalSubtitles,
    showSaveSuggestion,
    wizardSubtitle,
    audioSrc,
    audioError,
    isPreparingMedia,
    currentVideoTime = $bindable(),
    audioDuration,
    audioElement,
    isPlaying,
    hasAudio,
    offsetAdjustment,
    formatTime,
    formatOffset,
    onSaveFile,
    onContinueChecking,
    onRetryAudio,
    onTogglePlay,
    onReplay,
    onAdjustOffset,
    onSkip,
    onConfirm,
  }: Props = $props();

  let t = $derived($locale);
</script>

<div class="glass-card relative flex flex-col overflow-visible">
  <div class="p-5 pb-3 flex-shrink-0">
    <SectionHeader
      title={t("sync.wizard.title")}
      accent="indigo"
      iconPath="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
    />
  </div>

  <div class="flex flex-col items-center justify-center p-5 min-h-0">
    {#if !srtLoaded}
      <WizardEmptyState total={0} messageKey="sync.noSrtFileSelected" playButtonGradient={true} />
    {:else if showSaveSuggestion}
      <div class="text-center max-w-md">
        <div
          class="w-20 h-20 mx-auto mb-6 rounded-full bg-gradient-to-br from-green-500/20 to-emerald-500/20 flex items-center justify-center"
        >
          <svg
            class="w-10 h-10 text-green-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            ><path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M5 13l4 4L19 7"
            /></svg
          >
        </div>
        <h3 class="text-2xl font-bold text-white mb-3">
          {t("sync.wizard.allSynced")}
        </h3>
        <p class="text-gray-400 mb-6">{t("sync.wizard.suggestSave")}</p>
        <div class="flex gap-3 justify-center">
          <button
            onclick={onSaveFile}
            class="btn-success py-3 px-8 flex items-center gap-2 text-lg shadow-lg shadow-green-500/30"
          >
            <svg
              class="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              ><path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M5 13l4 4L19 7"
              /></svg
            >
            {t("sync.saveFile")}
          </button>
          <button
            onclick={onContinueChecking}
            class="btn-secondary py-3 px-6"
          >
            {t("sync.wizard.continueChecking")}
          </button>
        </div>
      </div>
    {:else if wizardSubtitle}
      <div class="w-full max-w-6xl flex flex-col gap-4">
        <div class="text-center flex-shrink-0">
          <span
            class="inline-flex items-center gap-2 px-4 py-1.5 rounded-full bg-indigo-500/20 text-indigo-300 text-sm font-medium"
          >
            <svg
              class="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              ><path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
              /></svg
            >
            {t("sync.wizard.checkpoint")} — #{wizardSubtitle.id} / {totalSubtitles}
          </span>
        </div>

        <div
          class="bg-white/5 rounded-xl p-5 text-center flex-shrink-0 flex flex-col items-center justify-center min-h-[100px]"
        >
          <p class="text-xl text-white font-medium leading-relaxed">
            {wizardSubtitle.text}
          </p>
          <p class="text-sm text-gray-500 mt-3 font-mono">
            {formatTime(wizardSubtitle.start_ms)} → {formatTime(
              wizardSubtitle.end_ms,
            )}
          </p>
        </div>

        <div class="flex-shrink-0">
          {#if audioSrc && !audioError}
            <div class="flex items-center gap-4">
              <span class="text-sm text-gray-400 font-mono w-24"
                >{formatTime(currentVideoTime * 1000)}</span
              >
              <input
                type="range"
                min="0"
                max={audioDuration || 100}
                step="0.01"
                bind:value={currentVideoTime}
                oninput={() => {
                  if (audioElement)
                    audioElement.currentTime = currentVideoTime;
                }}
                class="flex-1"
              />
              <span
                class="text-sm text-gray-400 font-mono w-24 text-right"
                >{audioDuration
                  ? formatTime(audioDuration * 1000)
                  : "--:--"}</span
              >
            </div>
          {:else if isPreparingMedia}
            <div class="text-center py-4">
              <div class="flex items-center justify-center gap-3">
                <svg class="animate-spin h-5 w-5 text-purple-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                <p class="text-purple-300">{t("sync.transcodingMedia")}</p>
              </div>
              <p class="text-xs text-gray-500 mt-2">
                {t("sync.transcodingHint")}
              </p>
            </div>
          {:else if !audioSrc}
            <div class="text-center py-4">
              <p class="text-gray-500">{t("sync.audioPlaceholder")}</p>
              <p class="text-xs text-gray-600 mt-1">
                {t("sync.audioFormats")}
              </p>
            </div>
          {:else if audioError}
            <div class="text-center py-4 max-w-xl mx-auto">
              <p
                class="text-red-400 text-sm whitespace-pre-wrap break-words"
              >
                {audioError}
              </p>
              <button
                onclick={onRetryAudio}
                class="btn-secondary text-sm mt-2"
                >{t("sync.tryAnotherFile")}</button
              >
            </div>
          {/if}
        </div>

        <div
          class="flex items-center justify-center gap-4 flex-wrap flex-shrink-0 min-h-[60px]"
        >
          <button
            onclick={onTogglePlay}
            disabled={!hasAudio}
            class="w-14 h-14 flex items-center justify-center rounded-full bg-indigo-600 hover:bg-indigo-700 shadow-lg shadow-indigo-500/30 transition-all disabled:opacity-40 disabled:cursor-not-allowed"
            title={isPlaying ? t("sync.tooltipPause") : t("sync.tooltipPlay")}
          >
            {#if isPlaying}
              <svg
                class="w-7 h-7 text-white"
                fill="currentColor"
                viewBox="0 0 24 24"
                ><path d="M6 4h4v16H6V4zm8 0h4v16h-4V4z" /></svg
              >
            {:else}
              <svg
                class="w-7 h-7 text-white ml-1"
                fill="currentColor"
                viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg
              >
            {/if}
          </button>

          <button
            onclick={onReplay}
            disabled={!hasAudio}
            class="w-12 h-12 flex items-center justify-center rounded-full bg-white/10 hover:bg-white/20 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
            title={t("sync.wizard.replay") + " (R)"}
          >
            <svg
              class="w-6 h-6 text-cyan-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              ><path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
              /></svg
            >
          </button>

          <div
            class="flex items-center gap-2 bg-white/5 rounded-xl px-4 py-2 {!hasAudio
              ? 'opacity-40 pointer-events-none'
              : ''}"
          >
            <button
              onclick={() => onAdjustOffset(-3000)}
              disabled={!hasAudio}
              class="w-12 h-8 flex items-center justify-center bg-white/10 hover:bg-white/20 rounded-lg text-[11px] font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              title="-3s"
            >
              −3s
            </button>
            <button
              onclick={() => onAdjustOffset(-500)}
              disabled={!hasAudio}
              class="w-12 h-8 flex items-center justify-center bg-white/10 hover:bg-white/20 rounded-lg text-[11px] font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              title="-0.5s"
            >
              −0.5s
            </button>
            <button
              onclick={() => onAdjustOffset(-100)}
              disabled={!hasAudio}
              class="w-8 h-8 flex items-center justify-center bg-white/10 hover:bg-white/20 rounded-lg text-lg font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              title="-100ms"
            >
              −
            </button>
            <div class="flex flex-col items-center min-w-[80px]">
              <span class="text-xs text-gray-500 uppercase tracking-wide"
                >{t("sync.offset")}</span
              >
              <span
                class="text-base font-mono font-medium {offsetAdjustment >
                0
                  ? 'text-green-400'
                  : offsetAdjustment < 0
                    ? 'text-red-400'
                    : 'text-white'}"
                >{formatOffset(offsetAdjustment)}</span
              >
            </div>
            <button
              onclick={() => onAdjustOffset(100)}
              disabled={!hasAudio}
              class="w-8 h-8 flex items-center justify-center bg-white/10 hover:bg-white/20 rounded-lg text-lg font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              title="+100ms"
            >
              +
            </button>
            <button
              onclick={() => onAdjustOffset(500)}
              disabled={!hasAudio}
              class="w-12 h-8 flex items-center justify-center bg-white/10 hover:bg-white/20 rounded-lg text-[11px] font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              title="+0.5s"
            >
              +0.5s
            </button>
            <button
              onclick={() => onAdjustOffset(3000)}
              disabled={!hasAudio}
              class="w-12 h-8 flex items-center justify-center bg-white/10 hover:bg-white/20 rounded-lg text-[11px] font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              title="+3s"
            >
              +3s
            </button>
          </div>

          <button
            onclick={onSkip}
            class="btn-secondary py-3 px-6 flex items-center gap-2 text-base font-medium"
            title={t("sync.wizard.skip")}
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7" /></svg>
            {t("sync.wizard.skip")}
          </button>

          <button
            onclick={onConfirm}
            disabled={!audioSrc || !!audioError}
            class="btn-success py-3 px-6 flex items-center gap-2 disabled:opacity-50 shadow-lg shadow-green-500/20 text-base font-medium"
          >
            <svg
              class="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              ><path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M5 13l4 4L19 7"
              /></svg
            >
            {t("sync.wizard.confirm")}
          </button>
        </div>

        <div
          class="flex flex-wrap gap-3 text-xs text-gray-500 justify-center flex-shrink-0"
        >
          <div class="flex items-center gap-1">
            <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
              >{t("keys.space") || "Space"}</kbd
            ><span>{t("sync.playPause")}</span>
          </div>
          <div class="flex items-center gap-1">
            <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
              >←/→</kbd
            ><span>{t("sync.seek")}</span>
          </div>
          <div class="flex items-center gap-1">
            <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
              >↑/↓</kbd
            ><span>{t("sync.offsetAdjust")}</span>
          </div>
          <div class="flex items-center gap-1">
            <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
              >R</kbd
            ><span>{t("sync.wizard.replay")}</span>
          </div>
          <div class="flex items-center gap-1">
            <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
              >{t("keys.enter") || "Enter"}</kbd
            ><span>{t("sync.confirm")}</span>
          </div>
        </div>
      </div>
    {:else}
      <WizardEmptyState total={totalSubtitles} messageKey="sync.wizard.selectCheckpoint" />
    {/if}
  </div>
</div>
