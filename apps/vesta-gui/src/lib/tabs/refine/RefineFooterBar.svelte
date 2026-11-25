<script lang="ts">
  import { locale } from "$lib/i18n";
  import FooterActions from "$lib/components/FooterActions.svelte";

  interface Props {
    filePath: string;
    fileName: string;
    fileExtension: string;
    hasCards: boolean;
    operationBusy: boolean;
    isSaving: boolean;
    onOverwrite: () => void;
    onSaveAs: (ext: "apkg" | "tsv") => void;
    onBlockedSave?: (kind: "overwrite" | "apkg" | "tsv") => void;
  }

  let {
    filePath,
    fileName,
    fileExtension,
    hasCards,
    operationBusy,
    isSaving,
    onOverwrite,
    onSaveAs,
    onBlockedSave,
  }: Props = $props();

  let t = $derived($locale);
  let overwriteBlocked = $derived(!operationBusy && !hasCards);
  let apkgBlocked = $derived(!operationBusy && (!hasCards || fileExtension === "TSV"));
  let tsvBlocked = $derived(!operationBusy && !hasCards);
</script>

<FooterActions>
  {#snippet left()}
    <div class="flex flex-1 min-w-0 items-center gap-3 max-[960px]:basis-full">
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        class="relative group shrink-0 {overwriteBlocked ? 'cursor-help' : ''}"
        onclick={() => {
          if (overwriteBlocked) onBlockedSave?.("overwrite");
        }}
      >
        <button
          onclick={onOverwrite}
          disabled={operationBusy || !hasCards}
          class="min-w-[155px] justify-center px-3 py-2.5 bg-rose-600 hover:bg-rose-500 disabled:bg-rose-600/40 text-white disabled:opacity-50 rounded-xl font-bold text-sm transition-all shadow-lg shadow-rose-950/30 disabled:shadow-none flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] border border-rose-500/20 disabled:border-white/10 {overwriteBlocked ? 'pointer-events-none saturate-75' : 'cursor-pointer'}"
        >
          {#if isSaving}
            <svg class="animate-spin h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            {t("refine.action.saving")}
          {:else}
            <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
            </svg>
            {t("refine.action.overwrite", { extension: fileExtension })}
          {/if}
        </button>
        <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-rose-500/30 bg-gray-950/95 p-3 text-center text-xs text-rose-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
          {t("refine.action.tooltipOverwrite")}
        </div>
      </div>
      <div class="flex min-w-0 flex-1 max-w-[300px] items-center gap-1.5 text-xs text-gray-400 bg-white/5 border border-white/10 px-3 py-1.5 rounded-lg" title={filePath || undefined}>
        <span class="shrink-0 font-bold text-gray-300">{t("refine.action.fileLabel")}</span>
        <span class="min-w-0 truncate">{fileName || "—"}</span>
      </div>
    </div>
  {/snippet}
  {#snippet right()}
    <div class="flex max-w-full flex-wrap items-center gap-2">
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        class="relative group {apkgBlocked ? 'cursor-help' : ''}"
        onclick={() => {
          if (apkgBlocked) onBlockedSave?.("apkg");
        }}
      >
        <button
          onclick={() => onSaveAs("apkg")}
          disabled={operationBusy || !hasCards || fileExtension === "TSV"}
          class="min-w-[135px] justify-center px-3 py-2.5 bg-emerald-600 hover:bg-emerald-500 disabled:bg-emerald-600/40 disabled:opacity-50 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-emerald-900/30 disabled:shadow-none flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:cursor-not-allowed border border-emerald-500/20 disabled:border-white/10 {apkgBlocked ? 'pointer-events-none saturate-75' : 'cursor-pointer'}"
        >
          {#if isSaving}
            <svg class="animate-spin h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            {t("refine.action.saving")}
          {:else}
            <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6M5 19V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2H7a2 2 0 01-2-2z" />
            </svg>
            {t("refine.action.saveAsApkg")}
          {/if}
        </button>
        <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-emerald-500/30 bg-gray-950/95 p-3 text-center text-xs text-emerald-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
          {fileExtension === "TSV" ? t("refine.action.cannotSaveTsvAsApkg") : t("refine.action.tooltipSaveAsApkg")}
        </div>
      </div>

      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        class="relative group {tsvBlocked ? 'cursor-help' : ''}"
        onclick={() => {
          if (tsvBlocked) onBlockedSave?.("tsv");
        }}
      >
        <button
          onclick={() => onSaveAs("tsv")}
          disabled={operationBusy || !hasCards}
          class="min-w-[135px] justify-center px-3 py-2.5 bg-cyan-600 hover:bg-cyan-500 disabled:bg-cyan-600/40 disabled:opacity-50 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-cyan-950/20 disabled:shadow-none flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:cursor-not-allowed border border-cyan-500/20 disabled:border-white/10 {tsvBlocked ? 'pointer-events-none saturate-75' : 'cursor-pointer'}"
        >
          {#if isSaving}
            <svg class="animate-spin h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            {t("refine.action.saving")}
          {:else}
            <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6M5 19V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2H7a2 2 0 01-2-2z" />
            </svg>
            {t("refine.action.saveAsTsv")}
          {/if}
        </button>
        <div class="pointer-events-none absolute bottom-full right-0 z-50 mb-3 rounded-xl border border-cyan-500/30 bg-gray-950/95 p-3 text-center text-xs text-cyan-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
          {t("refine.action.tooltipSaveAsTsv")}
        </div>
      </div>
    </div>
  {/snippet}
</FooterActions>
