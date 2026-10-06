<script lang="ts">
  import type { Snippet } from "svelte";
  import MediaIcon from "$lib/components/MediaIcon.svelte";
  import OutputFileActions from "$lib/components/OutputFileActions.svelte";
  import { locale } from "$lib/i18n";
  import { generationStore } from "$lib/stores/generationStore.svelte";

  /** Generation status with the parent's note-type picker available in both UI modes. */
  interface Props {
    easyMode: boolean;
    noteTypeControl: Snippet;
    noteTypeName: string;
  }
  let { noteTypeControl, easyMode, noteTypeName }: Props = $props();

  let t = $derived($locale);
</script>

<!-- Left side: Note type template AND progress text/result messages -->
<div class="flex items-center gap-4 select-none z-10 min-w-0 flex-1">
  {#if !generationStore.result && !generationStore.isProcessing}
    {@render noteTypeControl()}
  {:else if generationStore.isProcessing}
    <!-- Loading status message overlay -->
    <div class="flex items-center gap-4">
      {#if !easyMode}
        <!-- Disabled Template Cycle button during loading just for aesthetic presence -->
        <button
          disabled
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-xs font-semibold select-none border-gray-700 bg-gray-800/40 text-gray-500 opacity-60 pointer-events-none"
        >
          <svg class="w-3.5 h-3.5 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M4 5a1 1 0 011-1h14a1 1 0 011 1v4H4V5zm0 8h8v7H5a1 1 0 01-1-1v-6zm12 0h4v6a1 1 0 01-1 1h-3v-7z"
            />
          </svg>
          {t("settings.noteType")}: {noteTypeName}
        </button>
      {/if}
      <div class="flex flex-col justify-center">
        <span class="text-xs font-semibold text-emerald-300 flex items-center gap-2">
          <svg class="w-3.5 h-3.5 animate-spin text-emerald-400" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          {generationStore.progressMessage || t("refine.btn.generating")}
        </span>
        {#if generationStore.phaseProgress !== null}
          <progress class="mt-1 h-1.5 w-52 max-w-full accent-emerald-400" value={generationStore.phaseProgress} max="100" aria-label={generationStore.progressMessage}></progress>
        {/if}
        <span class="text-[10px] text-emerald-400/80 font-bold mt-0.5">{generationStore.progress}%</span>
      </div>
    </div>
  {:else if generationStore.result}
    <!-- Result Display -->
    <div class="flex items-center gap-4 min-w-0">
      {#if generationStore.result.success}
        <!-- Success icon -->
        <div class="flex items-center justify-center w-8 h-8 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 shrink-0">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
          </svg>
        </div>
        <!-- Success Info -->
        <div class="flex flex-col min-w-0">
          <div class="flex items-baseline gap-2">
            <span class="text-sm font-bold text-emerald-400 whitespace-nowrap">
              {generationStore.result.cardsGenerated} {t("flashcards.cardsGenerated")}
            </span>
            <span class="text-[11px] text-gray-400 flex gap-2 font-medium shrink-0">
              {#if generationStore.result.audioClips > 0}
                <span><MediaIcon kind="audio" /> {generationStore.result.audioClips}</span>
              {/if}
              {#if generationStore.result.snapshots > 0}
                <span><MediaIcon kind="snapshot" /> {generationStore.result.snapshots}</span>
              {/if}
              {#if generationStore.result.videoClips > 0}
                <span><MediaIcon kind="video" /> {generationStore.result.videoClips}</span>
              {/if}
            </span>
          </div>
          {#if generationStore.result.apkgPath}
            <OutputFileActions path={generationStore.result.apkgPath} packageFile />
          {:else if generationStore.result.tsvPath}
            <OutputFileActions path={generationStore.result.tsvPath} />
          {/if}
        </div>
      {:else}
        <!-- Error icon -->
        <div class="flex items-center justify-center w-8 h-8 rounded-full bg-red-500/10 border border-red-500/20 text-red-400 shrink-0">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        </div>
        <!-- Error details -->
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-bold text-red-400">{t("flashcards.generationFailed") || 'Generation Failed'}</span>
          <span class="text-[11px] text-gray-400 truncate max-w-[320px]" title={generationStore.result.message}>
            {generationStore.result.message ? (generationStore.result.message.includes("No active") ? t("flashcards.noActiveLines") : generationStore.result.message) : t("flashcards.errorGenerating")}
          </span>
        </div>
      {/if}
    </div>
  {/if}
</div>
