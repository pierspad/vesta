<script lang="ts">
  import { locale } from "./i18n";
  import PathPickerField from "./PathPickerField.svelte";
  import EpisodeTable from "./EpisodeTable.svelte";
  import type { EpisodeMediaOverrides } from "./flashcardMediaTypes";
  import type { NoteTypeDef } from "./noteTypes";

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
    seriesMode: boolean;
    onToggleSeriesMode: () => void;
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
    targetSubsPath: string;
    nativeSubsPath: string;
    mediaPath: string;
    outputDir: string;
    activeNoteType: NoteTypeDef;
    onExpand: (field: ExpandableField) => void;
    onSelectTarget: () => void;
    onSelectNative: () => void;
    onSelectMedia: () => void;
    onSelectOutput: () => void;
    onClearField: (field: ClearableField) => void;
  }

  let {
    highlightClass,
    seriesMode,
    onToggleSeriesMode,
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
    targetSubsPath,
    nativeSubsPath,
    mediaPath,
    outputDir,
    activeNoteType,
    onExpand,
    onSelectTarget,
    onSelectNative,
    onSelectMedia,
    onSelectOutput,
    onClearField,
  }: Props = $props();

  let t = $derived($locale);
</script>

<div class="glass-card p-5 {highlightClass}">
  <div class="mb-3 flex items-center gap-3">
    <h3
      class="flex min-w-0 items-center gap-2 text-lg font-semibold {seriesMode ? 'text-violet-400' : 'panel-title-files-output'}"
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
    <span class="flex shrink-0 items-center gap-1.5 rounded-full border border-gray-700/60 bg-gray-900/60 px-2 py-1">
      <button
        type="button"
        onclick={onToggleSeriesMode}
        class="flex items-center gap-1 text-xs font-semibold transition-colors {!seriesMode
          ? 'text-emerald-300'
          : 'text-gray-500 hover:text-gray-300'}"
        title={t("flashcards.modeMovie")}
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
          <rect x="2" y="4" width="20" height="16" rx="2" />
          <path d="M2 8h20M7 4v4M17 4v4" stroke-linecap="round" />
        </svg>
        {t("flashcards.modeMovieShort")}
      </button>
      <button
        type="button"
        class="relative h-5 w-9 shrink-0 rounded-full transition-colors {seriesMode ? 'bg-violet-500/60' : 'bg-emerald-500/50'}"
        onclick={onToggleSeriesMode}
        role="switch"
        aria-checked={seriesMode}
        title={seriesMode ? t("flashcards.modeSeries") : t("flashcards.modeMovie")}
      >
        <span
          class="absolute left-0.5 top-0.5 h-4 w-4 rounded-full bg-white shadow-sm transition-transform {seriesMode
            ? 'translate-x-4'
            : 'translate-x-0'}"
        ></span>
      </button>
      <button
        type="button"
        onclick={onToggleSeriesMode}
        class="flex items-center gap-1 text-xs font-semibold transition-colors {seriesMode
          ? 'text-violet-300'
          : 'text-gray-500 hover:text-gray-300'}"
        title={t("flashcards.modeSeries")}
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
          <rect x="2" y="3" width="20" height="6" rx="1" />
          <rect x="2" y="11" width="20" height="6" rx="1" />
          <line x1="6" y1="3" x2="6" y2="9" />
          <line x1="6" y1="11" x2="6" y2="17" />
        </svg>
        {t("flashcards.modeSeriesShort")}
      </button>
    </span>
    {#if seriesMode}
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
    {/if}
  </div>

  {#if !seriesMode}
    <div class="space-y-2.5">
      <div>
        <span class="block text-xs text-gray-400 mb-1">
          {t("flashcards.targetLangSubs")}
          <span class="text-red-400">*</span>
        </span>
        <PathPickerField
          value={targetSubsPath}
          placeholder={t("flashcards.selectFile")}
          browseTitle={t("flashcards.selectFile")}
          onexpand={() => {
            if (targetSubsPath) onExpand("targetSubs");
          }}
          onbrowse={onSelectTarget}
          onclear={() => onClearField("target")}
        />
      </div>

       <div>
        <span class="block text-xs mb-1 transition-colors text-gray-400">
          <span class={!activeNoteType.included.meaning ? 'text-gray-500 line-through opacity-60' : ''}>
            {t("flashcards.nativeLangSubs")}
          </span>
          {#if !activeNoteType.included.meaning}
            <span class="text-[10px] text-amber-500/80 ml-1.5 font-normal normal-case italic no-underline">({t("flashcards.inactiveNoteTypeField")})</span>
          {/if}
        </span>
        <PathPickerField
          value={nativeSubsPath}
          placeholder={t("flashcards.optional")}
          browseTitle={t("flashcards.optional")}
          disabled={!activeNoteType.included.meaning}
          onexpand={() => {
            if (nativeSubsPath) onExpand("nativeSubs");
          }}
          onbrowse={onSelectNative}
          onclear={() => onClearField("native")}
        />
      </div>

      <div>
        <span class="block text-xs mb-1 transition-colors text-gray-400">
          <span class={(!activeNoteType.included.audio && !activeNoteType.included.snapshot && !activeNoteType.included.video) ? 'text-gray-500 line-through opacity-60' : ''}>
            {t("flashcards.mediaFile")}
          </span>
          {#if !activeNoteType.included.audio && !activeNoteType.included.snapshot && !activeNoteType.included.video}
            <span class="text-[10px] text-amber-500/80 ml-1.5 font-normal normal-case italic no-underline">({t("flashcards.inactiveNoteTypeField")})</span>
          {/if}
        </span>
        <PathPickerField
          value={mediaPath}
          placeholder={t("flashcards.mediaPlaceholder")}
          browseTitle={t("flashcards.mediaPlaceholder")}
          disabled={!activeNoteType.included.audio && !activeNoteType.included.snapshot && !activeNoteType.included.video}
          onexpand={() => {
            if (mediaPath) onExpand("media");
          }}
          onbrowse={onSelectMedia}
          onclear={() => onClearField("media")}
        />
      </div>

      <div>
        <span class="block text-xs text-gray-400 mb-1">
          {t("flashcards.outputDir")} <span class="text-red-400">*</span>
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
  {:else}
    <!-- Series mode: batch file management -->
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

      {#if episodes.length > 0}
        <!-- Output dir (shared with movie mode) -->
        <div>
          <span class="block text-xs text-gray-400 mb-1">
            {t("flashcards.outputDir")}
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
      {/if}
    </div>
  {/if}
</div>
