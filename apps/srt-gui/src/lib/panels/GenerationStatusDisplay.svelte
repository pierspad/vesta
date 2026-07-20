<script lang="ts">
  import { locale, currentLanguage } from "$lib/i18n";
  import { generationStore } from "$lib/stores/generationStore.svelte";

  /** Left side of the bottom action bar: note-type cycle button when idle,
   * a spinner while generating, or the result/error summary once done.
   * Reads generationStore directly (isProcessing/progress/result are pure
   * generation run-state, no parent-domain dependency — see
   * generationStore.svelte.ts) and only takes props for the handful of
   * things it can't own itself: easyMode/noteTypeName are parent-derived,
   * and the click handlers call back into parent-owned functions
   * (cycleTemplates touches note-type selection state, showSnackbar is
   * shared UI feedback). */
  interface Props {
    easyMode: boolean;
    noteTypeName: string;
    onCycleTemplates: () => void;
    onNoteTypeContextMenu: (event: MouseEvent) => void;
    onNoteTypeMiddleClick: () => void;
    showSnackbar: (message: string, variant?: "success" | "info" | "warning" | "error") => void;
  }
  let { easyMode, noteTypeName, onCycleTemplates, onNoteTypeContextMenu, onNoteTypeMiddleClick, showSnackbar }: Props = $props();

  let t = $derived($locale);
</script>

<!-- Left side: Note type template AND progress text/result messages -->
<div class="flex items-center gap-4 select-none z-10 min-w-0 flex-1">
  {#if !generationStore.result && !generationStore.isProcessing}
    {#if !easyMode}
      <!-- Template cycle button -->
      <div class="relative group/tmpl">
        <button
          type="button"
          onclick={onCycleTemplates}
          oncontextmenu={onNoteTypeContextMenu}
          onmousedown={(e) => {
            if (e.button === 1) {
              e.preventDefault();
              onNoteTypeMiddleClick();
            }
          }}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-xs font-semibold cursor-pointer transition-all duration-200 border-violet-500/40 bg-violet-500/10 text-violet-300 hover:bg-violet-500/20 hover:border-violet-500/50 hover:scale-[1.02] active:scale-[0.98] select-none"
        >
          <svg class="w-3.5 h-3.5 text-violet-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M4 5a1 1 0 011-1h14a1 1 0 011 1v4H4V5zm0 8h8v7H5a1 1 0 01-1-1v-6zm12 0h4v6a1 1 0 01-1 1h-3v-7z"
            />
          </svg>
          {t("settings.noteType")}: {noteTypeName}
        </button>
        <div class="pointer-events-none absolute bottom-full left-1/2 -translate-x-1/2 mb-3 z-50
          rounded-xl border border-violet-500/30 bg-gray-950/95 p-3 text-xs text-violet-300 shadow-2xl shadow-black/40 ring-1 ring-white/10
          opacity-0 group-hover/tmpl:opacity-100 transition-all duration-150 whitespace-nowrap text-center">
          {t("flashcards.clickToCycleTemplates")}
        </div>
      </div>
    {/if}
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
                <span>🔊 {generationStore.result.audioClips}</span>
              {/if}
              {#if generationStore.result.snapshots > 0}
                <span>📸 {generationStore.result.snapshots}</span>
              {/if}
              {#if generationStore.result.videoClips > 0}
                <span>🎬 {generationStore.result.videoClips}</span>
              {/if}
            </span>
          </div>
          {#if generationStore.result.apkgPath}
            <button
              onclick={() => {
                if (generationStore.result) {
                  navigator.clipboard.writeText(generationStore.result.apkgPath || '');
                  showSnackbar($currentLanguage === 'it' ? 'Percorso copiato negli appunti!' : 'Path copied to clipboard!', 'success');
                }
              }}
              class="text-[11px] text-gray-500 hover:text-gray-300 transition-colors text-left truncate cursor-pointer font-medium hover:underline flex items-center gap-1 mt-0.5"
              title={generationStore.result.apkgPath}
            >
              📦 {generationStore.result.apkgPath.split('/').pop()}
            </button>
          {:else if generationStore.result.tsvPath}
            <button
              onclick={() => {
                if (generationStore.result) {
                  navigator.clipboard.writeText(generationStore.result.tsvPath || '');
                  showSnackbar($currentLanguage === 'it' ? 'Percorso copiato negli appunti!' : 'Path copied to clipboard!', 'success');
                }
              }}
              class="text-[11px] text-gray-500 hover:text-gray-300 transition-colors text-left truncate cursor-pointer font-medium hover:underline flex items-center gap-1 mt-0.5"
              title={generationStore.result.tsvPath}
            >
              📄 {generationStore.result.tsvPath.split('/').pop()}
            </button>
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
