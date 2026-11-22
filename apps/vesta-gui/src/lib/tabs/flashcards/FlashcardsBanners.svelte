<script lang="ts">
  import { locale } from "$lib/i18n";
  import { invokeCommand as invoke } from "$lib/services/tauriClient";
  import { ankiStore } from "$lib/stores/ankiStore.svelte";
  import { fontStore } from "$lib/stores/fontStore.svelte";
  import { generationStore } from "$lib/stores/generationStore.svelte";
  import { errorMessage } from "$lib/utils/errorMessage";

  interface FontSuggestion {
    id: string;
    language_name: string;
    approx_size: string;
    downloaded: boolean;
  }

  interface Props {
    ffmpegAvailable: boolean | null;
    hasAnyFiles: boolean;
    proactiveLang: string;
    proactiveFont: FontSuggestion | null | undefined;
    isFontBannerDismissed: (lang: string) => boolean;
    onFfmpegInstalled: (available: boolean) => void;
    onDismissFontBanner: (lang: string, permanent: boolean) => void;
  }

  let {
    ffmpegAvailable,
    hasAnyFiles,
    proactiveLang,
    proactiveFont,
    isFontBannerDismissed,
    onFfmpegInstalled,
    onDismissFontBanner,
  }: Props = $props();

  let t = $derived($locale);
  let isDownloadingFFmpeg = $state(false);

  async function downloadFfmpeg() {
    isDownloadingFFmpeg = true;
    try {
      await invoke("flashcard_download_ffmpeg");
      const ok = await invoke<boolean>("flashcard_check_deps");
      onFfmpegInstalled(ok);
      window.dispatchEvent(new CustomEvent("vesta-ffmpeg-updated"));
    } catch (e) {
      generationStore.error = `${t("flashcards.ffmpegDownloadFailed")}: ${errorMessage(e)}`;
    } finally {
      isDownloadingFFmpeg = false;
    }
  }
</script>

{#if ffmpegAvailable === false}
  <div
    class="mb-4 p-3 bg-white/[0.03] border border-white/10 rounded-lg flex items-center gap-3"
  >
    <svg
      class="w-5 h-5 text-cyan-400 flex-shrink-0"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke-width="2"
        d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"
      />
    </svg>
    <p class="text-gray-200 text-sm flex-1">
      {t("flashcards.ffmpegMissing")}
    </p>
    <button
      type="button"
      disabled={isDownloadingFFmpeg}
      onclick={downloadFfmpeg}
      class="flex-shrink-0 px-3 py-1.5 rounded-lg bg-cyan-500/10 border border-cyan-500/30 text-gray-200 text-xs font-semibold hover:bg-cyan-500/20 transition-colors flex items-center gap-1.5 disabled:opacity-50 disabled:cursor-not-allowed"
    >
      {#if isDownloadingFFmpeg}
        <svg class="animate-spin w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
        {t("flashcards.downloading")}
      {:else}
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" /></svg>
        {t("flashcards.downloadAuto")}
      {/if}
    </button>
  </div>
{/if}

{#if hasAnyFiles && ankiStore.autoCardFont}
  {#if ankiStore.autoCardFont && ankiStore.embedCardFont && proactiveFont && !proactiveFont.downloaded && !isFontBannerDismissed(proactiveLang)}
    {@const isThisDownloading = fontStore.downloadingFontId === proactiveFont.id}
    <div class="mb-4 p-3.5 bg-cyan-500/10 border border-cyan-500/30 rounded-xl flex flex-col md:flex-row md:items-center justify-between gap-3 animate-fade-in shadow-lg shadow-black/20">
      <div class="flex items-start md:items-center gap-3 min-w-0">
        <div class="w-9 h-9 rounded-lg bg-cyan-500/20 text-cyan-300 flex items-center justify-center shrink-0 mt-0.5 md:mt-0">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10" />
          </svg>
        </div>
        <div class="min-w-0">
          <p class="text-xs font-bold text-cyan-200">
            {t("flashcards.fontSuggestionTitle")} {proactiveFont.language_name}
          </p>
          <p class="text-[11px] text-gray-300 mt-0.5 leading-relaxed">
            {t("flashcards.fontSuggestionDesc")} ({proactiveFont.approx_size}) {t("flashcards.fontSuggestionDescEnd")} {t("flashcards.fontSuggestionDescLight")}
          </p>
        </div>
      </div>
      <div class="flex items-center gap-2 shrink-0 self-end md:self-center">
        <button
          type="button"
          disabled={fontStore.isDownloading}
          onclick={() => fontStore.downloadFont(proactiveFont.id)}
          class="px-3 py-1.5 rounded-lg bg-cyan-500/20 border border-cyan-500/40 text-cyan-200 text-xs font-semibold hover:bg-cyan-500/30 transition-colors flex items-center gap-1.5 disabled:opacity-50 disabled:cursor-not-allowed shadow-sm"
        >
          {#if isThisDownloading}
            <svg class="animate-spin w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
            <span>{fontStore.downloadProgress}%</span>
          {:else}
            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
            </svg>
            <span>{t("settings.download")} ({proactiveFont.approx_size})</span>
          {/if}
        </button>
        <button
          type="button"
          onclick={() => onDismissFontBanner(proactiveLang, true)}
          class="px-2.5 py-1.5 rounded-lg border border-white/10 bg-white/5 text-gray-300 text-xs font-medium hover:bg-white/10 hover:text-white transition-colors"
          title={t("flashcards.dontShowAgain")}
        >
          <span>{t("flashcards.dontShowAgain")}</span>
        </button>
        <button
          type="button"
          onclick={() => onDismissFontBanner(proactiveLang, false)}
          class="p-1.5 rounded-lg text-gray-400 hover:text-white hover:bg-white/10 transition-colors"
          title={t("common.close")}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  {/if}
{/if}
