<script lang="ts">
  import { locale } from "$lib/i18n";
  import PathPickerField from "$lib/components/PathPickerField.svelte";
  import EpisodeTable from "$lib/components/EpisodeTable.svelte";
  import type { EpisodeMediaOverrides } from "$lib/types/flashcardMediaTypes";
  import type { NoteTypeDef } from "$lib/types/noteTypes";

  /** Structural subset of FlashcardsTab.svelte's EpisodeEntry, forwarded
   * as-is to EpisodeTable — this panel never owns the `episodes` array
   * itself (see [[vesta-flashcards-refactor]] for why mutations stay in
   * the parent, same pattern as EpisodeTable.svelte). */
  interface EpisodeRow {
    id: number;
    targetSubsPath: string;
    nativeSubsPath: string;
    mediaPath: string;
    mediaOverrides?: EpisodeMediaOverrides;
  }

  type ExpandableField = "targetSubs" | "nativeSubs" | "media" | "output";
  type ClearableField = "target" | "native" | "media" | "output";

  interface Props {
    highlightClass: string;
    seriesMode?: boolean;
    onToggleSeriesMode?: () => void;
    episodes: EpisodeRow[];
    onAddFiles: () => void;
    onClearAll: () => void;
    showSnackbar: (message: string, variant?: "success" | "info" | "warning" | "error") => void;
    onSwapAll: () => void;
    onSwap: (idx: number) => void;
    onEdit: (idx: number) => void;
    onMediaSettings: (idx: number) => void;
    onRemove: (idx: number) => void;
    onContextMenu: (event: MouseEvent, idx: number) => void;
    targetSubsPath?: string;
    nativeSubsPath?: string;
    mediaPath?: string;
    outputDir: string;
    activeNoteType?: NoteTypeDef;
    onExpand: (field: ExpandableField) => void;
    onSelectTarget?: () => void;
    onSelectNative?: () => void;
    onSelectMedia?: () => void;
    onSelectOutput: () => void;
    onClearField: (field: ClearableField) => void;
  }

  let {
    highlightClass,
    episodes,
    onAddFiles,
    onClearAll,
    showSnackbar,
    onSwapAll,
    onSwap,
    onEdit,
    onMediaSettings,
    onRemove,
    onContextMenu,
    outputDir,
    onExpand,
    onSelectOutput,
    onClearField,
  }: Props = $props();

  let t = $derived($locale);
</script>

<div class="glass-card p-5 {highlightClass}">
  <div class="mb-3 flex items-center justify-between gap-3">
    <h3
      class="flex min-w-0 items-center gap-2 text-lg font-semibold text-violet-400"
    >
      <svg
        class="w-5 h-5 shrink-0"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
        />
      </svg>
      {t("common.filesAndOutput")}
    </h3>
    <div class="flex items-center gap-2">
      <button
        onclick={onAddFiles}
        class="bg-violet-700 hover:bg-violet-600 text-white font-semibold py-1 px-3 text-xs flex items-center gap-1.5 h-8 rounded-lg shrink-0 transition-colors cursor-pointer"
      >
        <svg
          class="w-3.5 h-3.5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          ><path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M12 4v16m8-8H4"
          /></svg
        >
        {t("flashcards.addFiles")}
      </button>
      {#if episodes.length > 0}
        <button
          onclick={onClearAll}
          class="border border-red-500/30 bg-red-500/10 hover:border-red-400/60 hover:bg-red-500/20 text-red-300 font-semibold py-1 px-3 text-xs flex items-center gap-1.5 h-8 rounded-lg shrink-0 transition-colors cursor-pointer"
        >
          <svg
            class="w-3.5 h-3.5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            ><path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
            /></svg
          >
          {t("flashcards.clearAll")}
        </button>
      {/if}
    </div>
  </div>

  <div class="space-y-3">
    <!-- Episode table -->
    <EpisodeTable
      {episodes}
      {showSnackbar}
      {onSwapAll}
      {onSwap}
      {onEdit}
      {onMediaSettings}
      {onRemove}
      {onContextMenu}
    />

    <!-- Output dir (always visible) -->
    <div>
      <span class="flex items-center gap-1.5 text-xs text-gray-400 mb-1 font-medium">
        <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
        </svg>
        <span>{t("flashcards.outputDir")}</span>
        <span class="text-red-400">*</span>
      </span>
      <PathPickerField
        value={outputDir}
        placeholder={t("flashcards.selectDir")}
        browseTitle={t("flashcards.selectDir")}
        onexpand={() => {
          if (outputDir) onExpand("output");
        }}
        onbrowse={onSelectOutput}
        onclear={() => onClearField("output")}
      />
    </div>
  </div>
</div>
