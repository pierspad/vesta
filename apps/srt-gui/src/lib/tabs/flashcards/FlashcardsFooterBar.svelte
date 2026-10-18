<script lang="ts">
  import FooterActions from "$lib/components/FooterActions.svelte";
  import GenerationStatusDisplay from "$lib/panels/GenerationStatusDisplay.svelte";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import { generationStore } from "$lib/stores/generationStore.svelte";
  import { ankiStore } from "$lib/stores/ankiStore.svelte";
  import { locale } from "$lib/i18n";
  import type { NoteTypeDef } from "$lib/types/noteTypes";
  import type { EpisodeEntry } from "$lib/utils/seriesFileMatching";

  export type SettingsSection = "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts";

  export interface GenerationRequirement {
    panel: "files";
    label: string;
  }

  interface Props {
    easyMode: boolean;
    activeNoteType: NoteTypeDef;
    noteTypeOptions: any[];
    automaticNoteType: boolean;
    selectedNoteTypeId: string;
    selectNoteType: (id: string) => void;
    episodes: EpisodeEntry[];
    isApkgSwitchEnabled: boolean;
    canRunFlashcards: boolean;
    generationPromptOpen: boolean;
    generationRequirements: GenerationRequirement[];
    generationRequirementsText: string;
    onOpenBottomContextMenu: (e: MouseEvent, section: "overview" | "anki") => void;
    onGoToSettings?: (section?: SettingsSection) => void;
    onPreview: () => void;
    onStartGeneration: () => void;
    onCancelGeneration: () => void;
    onResetGeneration: () => void;
    onPromptMissingRequirements: () => void;
    onCloseGenerationPrompt: () => void;
  }

  let {
    easyMode,
    activeNoteType,
    noteTypeOptions,
    automaticNoteType,
    selectedNoteTypeId,
    selectNoteType,
    episodes,
    isApkgSwitchEnabled,
    canRunFlashcards,
    generationPromptOpen,
    generationRequirements,
    generationRequirementsText,
    onOpenBottomContextMenu,
    onGoToSettings,
    onPreview,
    onStartGeneration,
    onCancelGeneration,
    onResetGeneration,
    onPromptMissingRequirements,
    onCloseGenerationPrompt,
  }: Props = $props();

  let t = $derived($locale);
</script>

<FooterActions>
  {#snippet background()}
    {#if generationStore.isProcessing}
      <div
        class="absolute inset-y-0 left-0 bg-gradient-to-r from-emerald-500/15 to-teal-500/20 transition-all duration-300 ease-out z-0 pointer-events-none"
        style="width: {generationStore.progress}%"
      ></div>
      <div
        class="absolute inset-0 bg-shimmer-stripes opacity-15 z-0 pointer-events-none"
      ></div>
    {/if}
  {/snippet}
  {#snippet left()}
    <GenerationStatusDisplay
      {easyMode}
      noteTypeName={activeNoteType.name}
    >
      {#snippet noteTypeControl()}
        <div class="w-80 max-w-[40vw]">
          <SearchableSelect
            compact
            ariaLabel={t("settings.noteType")}
            options={noteTypeOptions}
            value={automaticNoteType ? "default" : selectedNoteTypeId}
            onchange={selectNoteType}
            placeholder={t("settings.noteType")}
            noResultsText={t("common.noResults")}
            placement="up"
          />
        </div>
      {/snippet}
    </GenerationStatusDisplay>
  {/snippet}
  {#snippet right()}
    <!-- Right side: Export format toggle button, series output mode selector, and action buttons -->
    <div class="flex items-center gap-4 z-10 select-none shrink-0">
      <div class="flex items-center gap-2">
        <!-- Format toggle button -->
        <div class="relative group/fmt">
          <button
            type="button"
            onclick={() => generationStore.cycleExportFormat()}
            oncontextmenu={(e) => onOpenBottomContextMenu(e, "overview")}
            onmousedown={(e) => {
              if (e.button === 1) {
                e.preventDefault();
                onGoToSettings?.("overview");
              }
            }}
            disabled={generationStore.isProcessing || !!generationStore.result}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-xs font-semibold cursor-pointer select-none transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 disabled:active:scale-100
              {generationStore.isProcessing || generationStore.result
                ? 'border-gray-700 bg-gray-800/40 text-gray-500 opacity-60 pointer-events-none'
                : generationStore.exportFormat === 'apkg'
                  ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300 hover:bg-emerald-500/20 hover:border-emerald-500/50 hover:scale-[1.02] active:scale-[0.98]'
                  : generationStore.exportFormat === 'tsv'
                    ? 'border-sky-500/40 bg-sky-500/10 text-sky-300 hover:bg-sky-500/20 hover:border-sky-500/50 hover:scale-[1.02] active:scale-[0.98]'
                    : 'border-violet-500/40 bg-violet-500/10 text-violet-300 hover:bg-violet-500/20 hover:border-violet-500/50 hover:scale-[1.02] active:scale-[0.98]'}"
          >
            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
              />
            </svg>
            {generationStore.exportFormat === 'apkg'
              ? 'APKG'
              : generationStore.exportFormat === 'tsv'
                ? 'TSV'
                : (ankiStore.status === 'online'
                  ? 'Anki Connect'
                  : `Anki Connect (${t("common.fallback")}: ${generationStore.fallbackFormat.toUpperCase()})`)}
          </button>
          {#if !generationStore.isProcessing && !generationStore.result}
            <div class="pointer-events-none absolute bottom-full left-1/2 -translate-x-1/2 mb-3 z-50
              rounded-xl bg-gray-950/95 p-3 text-xs shadow-2xl shadow-black/40 ring-1 ring-white/10
              opacity-0 group-hover/fmt:opacity-100 transition-all duration-150 whitespace-nowrap text-center border
              {generationStore.exportFormat === 'apkg'
                ? 'border-emerald-500/30 text-emerald-300'
                : generationStore.exportFormat === 'tsv'
                  ? 'border-sky-500/30 text-sky-300'
                  : 'border-violet-500/30 text-violet-300'}">
              {t("flashcards.clickToToggleFormat")}
            </div>
          {/if}
        </div>

        <!-- Package merging applies only to multi-episode APKG output. -->
        {#if episodes.length > 1 && generationStore.effectiveExportFormat === "apkg"}
          <div
            class="flex items-center bg-gray-800/60 border border-gray-700/60 rounded-lg p-0.5 select-none relative group/sw transition-opacity {!isApkgSwitchEnabled ? 'opacity-40 cursor-not-allowed pointer-events-none' : ''}"
          >
            <!-- Sliding indicator background -->
            <div 
              class="absolute top-0.5 bottom-0.5 left-0.5 rounded-md bg-violet-500/20 border border-violet-500/50 transition-all duration-200 ease-out"
              style="width: 160px; transform: translateX({generationStore.seriesOutputMode === 'separate' ? '0px' : '160px'});"
            ></div>

            <button
              onclick={() => { if(isApkgSwitchEnabled) generationStore.seriesOutputMode = 'separate'; }}
              disabled={!isApkgSwitchEnabled}
              class="w-[160px] py-1 rounded-md text-xs font-semibold transition-colors duration-200 flex items-center justify-center cursor-pointer select-none relative z-10 disabled:cursor-not-allowed
                {generationStore.seriesOutputMode === 'separate' ? 'text-violet-200' : 'text-gray-500 hover:text-gray-300'}"
            >
              {t("flashcards.outputPerEpisode")}
            </button>
            <button
              onclick={() => { if(isApkgSwitchEnabled) generationStore.seriesOutputMode = 'single'; }}
              disabled={!isApkgSwitchEnabled}
              class="w-[160px] py-1 rounded-md text-xs font-semibold transition-colors duration-200 flex items-center justify-center cursor-pointer select-none relative z-10 disabled:cursor-not-allowed
                {generationStore.seriesOutputMode === 'single' ? 'text-violet-200' : 'text-gray-500 hover:text-gray-300'}"
            >
              {t("flashcards.outputSingleApkg")}
            </button>

            <!-- Custom tooltip -->
            {#if isApkgSwitchEnabled}
              <div 
                class="pointer-events-none absolute bottom-full z-50 mb-3 -translate-x-1/2 rounded-xl border border-violet-500/30 bg-gray-950/95 p-3 text-center text-xs text-violet-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-[opacity,transform] duration-75 delay-0 opacity-0 group-hover/sw:opacity-100 group-hover/sw:translate-y-0 translate-y-1 whitespace-normal max-w-[280px] w-max"
                style="left: {generationStore.seriesOutputMode === 'separate' ? '82px' : '242px'};"
              >
                {generationStore.seriesOutputMode === 'separate' ? t("flashcards.outputPerEpisodeDesc") : t("flashcards.outputSingleApkgDesc")}
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Preview Button -->
      {#if !generationStore.result}
        <div class="relative group">
          <button
            class="px-5 py-2.5 bg-gray-800 hover:bg-gray-700 disabled:bg-gray-800/40 text-gray-300 disabled:text-gray-600 rounded-xl font-bold text-sm transition-all border border-white/10 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:cursor-not-allowed cursor-pointer disabled:border-white/5"
            disabled={!canRunFlashcards || generationStore.isProcessing}
            onclick={onPreview}
          >
            <svg
              class="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
              />
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
              />
            </svg>
            {t("flashcards.preview")}
          </button>
          {#if !generationStore.isProcessing}
            <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-amber-500/30 bg-gray-950/95 p-3 text-center text-xs text-amber-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
              {!canRunFlashcards ? t("flashcards.completePrefix", { steps: generationRequirementsText }) : t("flashcards.preview")}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Main action button (Generate / Cancel / New Generation) -->
      {#if generationStore.isProcessing}
        <button
          onclick={onCancelGeneration}
          class="px-5 py-2.5 bg-red-600 hover:bg-red-500 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-red-900/30 flex items-center gap-2 hover:scale-[1.02] active:scale-[0.98] cursor-pointer"
        >
          <svg
            class="w-4 h-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
          {t("flashcards.cancel")}
        </button>
      {:else}
        {#if !generationStore.result}
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <div
            class="group relative"
            role={!canRunFlashcards ? "button" : undefined}
            tabindex={!canRunFlashcards ? 0 : undefined}
            onclick={() => {
              if (!canRunFlashcards) onPromptMissingRequirements();
            }}
            onmouseleave={onCloseGenerationPrompt}
            onfocusout={onCloseGenerationPrompt}
            onkeydown={(event) => {
              if (!canRunFlashcards && (event.key === "Enter" || event.key === " ")) {
                event.preventDefault();
                onPromptMissingRequirements();
              }
            }}
          >
            <button
              onclick={onStartGeneration}
              disabled={!canRunFlashcards}
              aria-describedby={!canRunFlashcards ? "flashcards-generate-requirements" : undefined}
              class="px-5 py-2.5 bg-emerald-600 hover:bg-emerald-500 disabled:bg-emerald-600/40 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-emerald-900/30 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:cursor-help disabled:opacity-50 {!canRunFlashcards ? 'pointer-events-none saturate-75' : 'cursor-pointer'}"
            >
              <svg
                class="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M13 10V3L4 14h7v7l9-11h-7z"
                />
              </svg>
              {t("flashcards.generate")}
            </button>
            {#if !canRunFlashcards}
              <div
                id="flashcards-generate-requirements"
                class="pointer-events-none absolute bottom-full left-1/2 z-40 mb-3 w-[min(22rem,calc(100vw-3rem))] -translate-x-1/2 rounded-xl border border-amber-400/30 bg-gray-950/95 p-3 text-left text-xs text-gray-200 opacity-0 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 group-hover:opacity-100 group-hover:translate-y-0 {generationPromptOpen ? 'opacity-100 translate-y-0' : 'translate-y-1'}"
              >
                <p class="mb-2 font-semibold text-amber-200">
                  {t("flashcards.missingSteps")}
                </p>
                <ol class="list-decimal space-y-1 pl-4 text-gray-300">
                  {#each generationRequirements as requirement}
                    <li>{requirement.label}</li>
                  {/each}
                </ol>
              </div>
            {/if}
          </div>
        {:else}
          <div class="relative group">
            <button
              onclick={onResetGeneration}
              class="px-5 py-2.5 bg-amber-500/10 hover:bg-amber-500/20 text-amber-300 rounded-xl font-bold text-sm transition-all border border-amber-500/30 flex items-center gap-2 hover:scale-[1.02] active:scale-[0.98] cursor-pointer"
            >
              <svg
                class="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                />
              </svg>
              {t("flashcards.newGeneration")}
            </button>
            <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-amber-500/30 bg-gray-950/95 p-3 text-center text-xs text-amber-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
              {t("flashcards.newGenerationDesc")}
            </div>
          </div>
        {/if}
      {/if}
    </div>
  {/snippet}
</FooterActions>

<style>
  @keyframes progress-stripes {
    0% { background-position: 0 0; }
    100% { background-position: 40px 0; }
  }
  .bg-shimmer-stripes {
    background-image: linear-gradient(
      45deg,
      rgba(16, 185, 129, 0.15) 25%,
      transparent 25%,
      transparent 50%,
      rgba(16, 185, 129, 0.15) 50%,
      rgba(16, 185, 129, 0.15) 75%,
      transparent 75%,
      transparent
    );
    background-size: 40px 40px;
    animation: progress-stripes 1.2s linear infinite;
  }
</style>
