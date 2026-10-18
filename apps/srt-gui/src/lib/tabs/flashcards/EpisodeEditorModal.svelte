<script lang="ts">
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import type { EpisodeEntry } from "$lib/utils/seriesFileMatching";
  import type { NoteTypeDef } from "$lib/types/noteTypes";

  export type EpisodeFileField = "targetSubsPath" | "nativeSubsPath" | "mediaPath";

  interface Props {
    editingEpisode: EpisodeEntry;
    onlyField?: EpisodeFileField | null;
    activeNoteType: NoteTypeDef;
    dropMediaHint: string;
    dropSubtitleHint: string;
    onClose: () => void;
    onRevert: () => void;
    onSelectFile: (field: EpisodeFileField) => void;
    onClearField: (field: EpisodeFileField) => void;
    onExpandField: (field: string) => void;
  }

  let {
    editingEpisode,
    onlyField = null,
    activeNoteType,
    dropMediaHint,
    dropSubtitleHint,
    onClose,
    onRevert,
    onSelectFile,
    onClearField,
    onExpandField,
  }: Props = $props();

  let t = $derived($locale);

  const episodeEditorFields: {
    field: EpisodeFileField;
    labelKey: "targetLangSubs" | "nativeLangSubs" | "mediaFile";
    placeholderKey: "selectFile" | "optional" | "mediaPlaceholder";
    required: boolean;
  }[] = [
    { field: "targetSubsPath", labelKey: "targetLangSubs", placeholderKey: "selectFile", required: true },
    { field: "nativeSubsPath", labelKey: "nativeLangSubs", placeholderKey: "optional", required: false },
    { field: "mediaPath", labelKey: "mediaFile", placeholderKey: "mediaPlaceholder", required: false },
  ];
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/65 p-6"
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  onclick={onClose}
  onkeydown={(e) => {
    if (e.key === "Escape") onClose();
  }}
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="w-full max-w-2xl rounded-xl border border-gray-700 bg-gray-900 p-5 shadow-2xl"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="mb-4 flex items-center justify-between gap-3">
      <div>
        <h3 class="text-lg font-bold text-white">
          {t("common.edit")} {getFileName((onlyField ? editingEpisode[onlyField] : "") || editingEpisode.targetSubsPath || editingEpisode.nativeSubsPath || editingEpisode.mediaPath) || `episodio ${editingEpisode.id}`}
        </h3>
      </div>
      <button
        type="button"
        onclick={onClose}
        class="dialog-close-button text-xl leading-none text-gray-400 hover:text-white"
      >×</button>
    </div>

    <div class="space-y-3">
      {#each episodeEditorFields.filter((item) => !onlyField || item.field === onlyField) as item}
        {@const field = item.field}
        {@const label = t(`flashcards.${item.labelKey}`)}
        {@const placeholder = t(`flashcards.${item.placeholderKey}`)}
        {@const isFieldDisabled = field === "nativeSubsPath" ? !activeNoteType.included.meaning : field === "mediaPath" ? (!activeNoteType.included.audio && !activeNoteType.included.snapshot && !activeNoteType.included.video) : false}
        <div
          data-episode-file-field={field}
          data-disabled={isFieldDisabled}
          class="rounded-lg border border-transparent p-2 transition-colors hover:border-indigo-400/30 hover:bg-indigo-500/5"
        >
          <div class="mb-1 flex items-center gap-3">
            <span class="text-xs font-medium transition-colors text-gray-400">
              <span class={isFieldDisabled ? 'text-gray-500 line-through opacity-60' : ''}>
                {label}
              </span>
              {#if isFieldDisabled}
                <span class="text-[10px] text-amber-500/80 ml-1.5 font-normal normal-case italic no-underline">({t("flashcards.inactiveNoteTypeField")})</span>
              {/if}
              {#if item.required && !isFieldDisabled}<span class="text-red-400">*</span>{/if}
            </span>
          </div>
          <div class="flex gap-2">
            <button
              type="button"
              class="input-modern flex-1 truncate text-left text-xs disabled:opacity-50 disabled:cursor-not-allowed"
              style="direction: rtl; text-align: left;"
              disabled={isFieldDisabled}
              title={editingEpisode[field] || placeholder}
              onclick={() => {
                const value = editingEpisode[field];
                if (value) onExpandField(field);
              }}
            >
              <span
                class={editingEpisode[field]
                  ? "text-white"
                  : "text-gray-500"}
                style="unicode-bidi: plaintext;"
              >
                {editingEpisode[field] || placeholder}
              </span>
            </button>
            <button
              type="button"
              onclick={() => onSelectFile(field)}
              disabled={isFieldDisabled}
              class="btn-secondary flex h-10 shrink-0 items-center gap-1.5 px-4 text-xs cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
              </svg>
              <span class="text-xs font-semibold">{t("flashcards.browse")}</span>
            </button>
            <button
              type="button"
              disabled={isFieldDisabled || !editingEpisode[field]}
              class={editingEpisode[field] && !isFieldDisabled
                ? "inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-red-500/30 bg-red-500/10 text-red-300 shadow-sm transition-colors hover:border-red-400/60 hover:bg-red-500/20 hover:text-red-100"
                : "inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-gray-700 bg-gray-800 text-gray-600 transition-colors cursor-default opacity-50 cursor-not-allowed"}
              title={t("common.clearField")}
              aria-label={t("common.clearField")}
              onclick={() => onClearField(field)}
            >
              <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
              </svg>
            </button>
          </div>
          <p class="mt-1.5 text-[10px] text-gray-500">
            {field === "mediaPath" ? dropMediaHint : dropSubtitleHint}
          </p>
        </div>
      {/each}
    </div>

    <div class="mt-5 flex justify-end gap-2">
      <button
        type="button"
        onclick={onRevert}
        class="btn-secondary px-4 py-2 text-sm"
      >
        {t("common.restore")}
      </button>
      <button
        type="button"
        onclick={onClose}
        class="rounded-lg border border-emerald-400/40 bg-emerald-500/20 px-4 py-2 text-sm font-semibold text-emerald-100 shadow-lg shadow-emerald-500/10 transition-all hover:border-emerald-300/60 hover:bg-emerald-500/30 disabled:cursor-not-allowed disabled:opacity-50"
      >
        {t("common.done")}
      </button>
    </div>
  </div>
</div>

<style>
  .dialog-close-button {
    border-radius: 0.45rem;
    transition:
      background-color 0.14s ease,
      box-shadow 0.14s ease,
      color 0.14s ease;
  }

  .dialog-close-button:hover {
    background: rgba(148, 163, 184, 0.1);
    box-shadow: 0 0 18px rgba(148, 163, 184, 0.26);
  }
</style>
