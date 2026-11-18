<script lang="ts">
  import { locale } from "$lib/i18n";

  interface Props {
    downloading: boolean;
    onDownload: () => void;
  }

  let { downloading, onDownload }: Props = $props();
  let t = $derived($locale);
</script>

<div class="mx-6 mt-4 mb-0 p-3 bg-white/[0.03] border border-white/10 rounded-xl flex items-center gap-3 shrink-0">
  <svg class="w-5 h-5 text-cyan-400 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z" />
  </svg>
  <div class="flex-1">
    <p class="text-gray-200 text-sm font-medium">{t("transcribe.ffmpegMissing")}</p>
    <p class="text-gray-400 text-xs mt-1">{t("transcribe.ffmpegMissingDesc")}</p>
  </div>
  <button
    type="button"
    disabled={downloading}
    onclick={onDownload}
    class="flex-shrink-0 px-3 py-1.5 rounded-lg bg-cyan-500/10 border border-cyan-500/30 text-gray-200 text-xs font-semibold hover:bg-cyan-500/20 transition-colors flex items-center gap-1.5 disabled:opacity-50 disabled:cursor-not-allowed"
  >
    {#if downloading}
      <svg class="animate-spin w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
      {t("flashcards.downloading")}
    {:else}
      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" /></svg>
      {t("flashcards.downloadAuto")}
    {/if}
  </button>
</div>
