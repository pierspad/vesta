<script lang="ts">
  import { locale } from "$lib/i18n";
  import { contextMenu as fitContextMenu } from "$lib/utils/contextMenu";
  import type { EpisodeEntry } from "$lib/utils/seriesFileMatching";
  import type { EpisodeMediaOverrideKey } from "$lib/types/flashcardMediaTypes";
  import type { NoteTypeDef } from "$lib/types/noteTypes";
  import { episodeMediaEditorStore } from "$lib/stores/episodeMediaEditorStore.svelte";
  import EpisodeContextMenu from "$lib/components/EpisodeContextMenu.svelte";
  import MatchDiagnosticsModal from "$lib/modals/MatchDiagnosticsModal.svelte";
  import EpisodeMediaSettingsModal from "$lib/modals/EpisodeMediaSettingsModal.svelte";
  import PathPreviewModal from "$lib/modals/PathPreviewModal.svelte";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import EpisodeEditorModal, { type EpisodeFileField } from "./EpisodeEditorModal.svelte";
  import type { NoteTypeOption } from "./flashcardsHelpers";

  interface Props {
    episodes: EpisodeEntry[];
    outputDir: string;
    episodeContextMenu: { x: number; y: number; idx: number; field?: EpisodeFileField } | null;
    bottomContextMenu: { x: number; y: number; section: "overview" | "anki" } | null;
    inspectedMatch: EpisodeEntry | null;
    inspectedMatchPath: string;
    editingEpisode: EpisodeEntry | null;
    editingEpisodeIndex: number | null;
    editingEpisodeField: EpisodeFileField | null;
    expandedPathField: string | null;
    noteTypeOptions: NoteTypeOption[];
    activeNoteType: NoteTypeDef;
    dropMediaHint: string;
    dropSubtitleHint: string;
    generateAudio: boolean;
    mediaNoteTypeId: string;
    initialMediaNoteTypeId: string;
    getEpisodeNoteType: (ep: Pick<EpisodeEntry, "targetSubsPath" | "noteTypeId" | "detectedLanguage">) => NoteTypeDef;
    mediaOverrideClass: (key: EpisodeMediaOverrideKey) => string;
    onCloseEpisodeContextMenu: () => void;
    onInspectMatch: (episode: EpisodeEntry, path: string) => void;
    onCloseInspectMatch: () => void;
    onEditContextEpisode: () => void;
    onOpenEpisodeMediaSettings: (idx: number) => void;
    onPreviewEpisodeAudio: (idx: number) => void;
    onRemoveEpisode: (idx: number) => void;
    onCloseBottomContextMenu: () => void;
    onGoToSettings?: (section?: "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts", highlightItemId?: string) => void;
    onUpdateEditingNoteType: (id: string) => void;
    onCloseEpisodeEditor: () => void;
    onRevertEpisodeEditor: () => void;
    onSelectEpisodeFile: (field: EpisodeFileField) => void;
    onClearEpisodeField: (field: EpisodeFileField) => void;
    onExpandPathField: (field: string | null) => void;
    onSaveEpisodeMediaSettings: () => void;
    onResetEpisodeMediaSettings: () => void;
  }

  let {
    episodes,
    outputDir,
    episodeContextMenu,
    bottomContextMenu,
    inspectedMatch,
    inspectedMatchPath,
    editingEpisode,
    editingEpisodeIndex,
    editingEpisodeField,
    expandedPathField,
    noteTypeOptions,
    activeNoteType,
    dropMediaHint,
    dropSubtitleHint,
    generateAudio,
    mediaNoteTypeId = $bindable(),
    initialMediaNoteTypeId,
    getEpisodeNoteType,
    mediaOverrideClass,
    onCloseEpisodeContextMenu,
    onInspectMatch,
    onCloseInspectMatch,
    onEditContextEpisode,
    onOpenEpisodeMediaSettings,
    onPreviewEpisodeAudio,
    onRemoveEpisode,
    onCloseBottomContextMenu,
    onGoToSettings,
    onUpdateEditingNoteType,
    onCloseEpisodeEditor,
    onRevertEpisodeEditor,
    onSelectEpisodeFile,
    onClearEpisodeField,
    onExpandPathField,
    onSaveEpisodeMediaSettings,
    onResetEpisodeMediaSettings,
  }: Props = $props();

  let t = $derived($locale);
</script>

{#if episodeContextMenu}
  <EpisodeContextMenu
    x={episodeContextMenu.x}
    y={episodeContextMenu.y}
    hasMedia={Boolean(episodes[episodeContextMenu.idx]?.mediaPath)}
    onInspectMatch={() => {
      const context = episodeContextMenu!;
      const ep = episodes[context.idx];
      const path = context.field ? ep[context.field] : "";
      onCloseEpisodeContextMenu();
      onInspectMatch(ep, path);
    }}
    onEdit={onEditContextEpisode}
    onMediaSettings={() => onOpenEpisodeMediaSettings(episodeContextMenu!.idx)}
    canPreviewAudio={generateAudio}
    onPreviewAudio={() => onPreviewEpisodeAudio(episodeContextMenu!.idx)}
    onRemove={() => onRemoveEpisode(episodeContextMenu!.idx)}
    onClose={onCloseEpisodeContextMenu}
  />
{/if}

{#if bottomContextMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50"
    onclick={onCloseBottomContextMenu}
    oncontextmenu={(e) => {
      e.preventDefault();
      onCloseBottomContextMenu();
    }}
    onkeydown={(e) => {
      if (e.key === "Escape") onCloseBottomContextMenu();
    }}
    role="presentation"
    tabindex="-1"
  >
    <div
      use:fitContextMenu={{ x: bottomContextMenu.x, y: bottomContextMenu.y, close: onCloseBottomContextMenu }}
      class="vesta-context-menu animate-fade-in"
    >
      <button
        type="button"
        class="vesta-context-menu-item"
        onclick={() => {
          onGoToSettings?.(bottomContextMenu!.section);
          onCloseBottomContextMenu();
        }}
      >
        <span class="inline-flex items-center gap-2">
          <svg class="h-4 w-4 text-sky-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
          {t("translate.goToSettings")}
        </span>
        <kbd>{t("keys.middleClick")}</kbd>
      </button>
    </div>
  </div>
{/if}

{#if inspectedMatch}
  <MatchDiagnosticsModal
    episode={inspectedMatch}
    focusedPath={inspectedMatchPath}
    onclose={onCloseInspectMatch}
    onrules={() => {
      onCloseInspectMatch();
      onGoToSettings?.("language", "smart-matching-rules");
    }}
  />
{/if}

{#if editingEpisode}
  <EpisodeEditorModal
    {editingEpisode}
    {noteTypeOptions}
    {getEpisodeNoteType}
    episodeIndex={editingEpisodeIndex ?? undefined}
    onNoteTypeChange={(_index, id) => onUpdateEditingNoteType(id)}
    onlyField={editingEpisodeField}
    {activeNoteType}
    {dropMediaHint}
    {dropSubtitleHint}
    onClose={onCloseEpisodeEditor}
    onRevert={onRevertEpisodeEditor}
    onSelectFile={onSelectEpisodeFile}
    onClearField={onClearEpisodeField}
    onExpandField={(field) => onExpandPathField(field)}
  />
{/if}

<EpisodeMediaSettingsModal
  {mediaOverrideClass}
  noteTypeDirty={mediaNoteTypeId !== initialMediaNoteTypeId}
  onSave={onSaveEpisodeMediaSettings}
  onReset={onResetEpisodeMediaSettings}
>
  {#snippet noteTypeControl()}
    <label class="mb-4 block space-y-2 text-sm text-gray-300">
      <span>{t("settings.noteType")}</span>
      <SearchableSelect
        options={noteTypeOptions.map(option => option.value === "default" && episodeMediaEditorStore.episode ? { ...option, label: `${t("flashcards.automaticNoteType")} · ${getEpisodeNoteType({ ...episodeMediaEditorStore.episode, noteTypeId: undefined }).name}`, displayLabel: getEpisodeNoteType({ ...episodeMediaEditorStore.episode, noteTypeId: undefined }).name } : option)}
        value={mediaNoteTypeId}
        onchange={(id) => (mediaNoteTypeId = id)}
        ariaLabel={t("settings.noteType")}
        placeholder={t("settings.noteType")}
        noResultsText={t("common.noResults")}
      />
    </label>
  {/snippet}
</EpisodeMediaSettingsModal>

<PathPreviewModal
  isOpen={!!expandedPathField}
  title={expandedPathField === "targetSubs" || expandedPathField === "targetSubsPath"
    ? t("flashcards.targetLangSubs")
    : expandedPathField === "output"
      ? t("flashcards.outputDir")
      : expandedPathField === "nativeSubs" || expandedPathField === "nativeSubsPath"
        ? t("flashcards.nativeLangSubs")
        : t("flashcards.mediaFile")}
  value={expandedPathField === "targetSubs" || expandedPathField === "targetSubsPath"
    ? editingEpisode?.targetSubsPath || episodes[0]?.targetSubsPath || ""
    : expandedPathField === "output"
      ? outputDir
      : expandedPathField === "nativeSubs" || expandedPathField === "nativeSubsPath"
        ? editingEpisode?.nativeSubsPath || episodes[0]?.nativeSubsPath || ""
        : editingEpisode?.mediaPath || episodes[0]?.mediaPath || ""}
  onclose={() => onExpandPathField(null)}
/>
