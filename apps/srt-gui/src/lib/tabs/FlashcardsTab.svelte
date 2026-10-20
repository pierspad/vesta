<script lang="ts">
  import { RequirementFeedback } from "$lib/utils/requirementFeedback.svelte";
  import { defaultOutputDirectory } from "$lib/utils/defaultOutputDirectory";
  import { expandFlashcardFiles } from "$lib/workflows/flashcardFileDiscovery";
  import { runFlashcardSeries } from "$lib/workflows/flashcardSeries";
  import { FlashcardPreviewController } from "$lib/workflows/flashcardPreview";
  import { audioOverrideKeys, snapshotOverrideKeys, videoOverrideKeys, mediaSettingChanged, episodeMediaDiff } from "$lib/utils/episodeMediaSettings";
  import { invokeCommand as invoke } from "$lib/services/tauriClient";
  import { listen } from "@tauri-apps/api/event";
  import { guardedOpen } from "$lib/utils/dialogGuard";
  import { setupWebviewDragDrop } from "$lib/utils/dragDrop";
  import { resolveEpisodeNoteType, detectSubtitleLanguage } from "$lib/utils/episodeNoteType";
  import { onDestroy, onMount } from "svelte";
  import { currentLanguage, locale } from "$lib/i18n";
  import { getFileName, inferLanguageFromPath } from "$lib/utils/models";
  import { getLanguageSearchTerms, languages } from "$lib/config/languages";
  import {
    type EpisodeEntry,
    VIDEO_EXTENSIONS,
    AUDIO_EXTENSIONS,
    classifySubtitles,
    mergeSeriesDroppedFiles,
    autoMatchFiles,
    isSubtitleFile,
    isMediaFile,
    detectMediaType,
    pickBestAudioTrackIndex,
    deriveDeckNameFromFile,
  } from "$lib/utils/seriesFileMatching";
  import {
    CARD_TEMPLATES_UPDATED_EVENT,
    NOTE_TYPES_UPDATED_EVENT,
    NOTE_TYPE_FIELD_ORDER,
    listNoteTypes,
    findNoteTypeById,
    loadLastUsedNoteTypeId,
    saveActiveNoteTypeId,
    ACTIVE_NOTE_TYPE_CHANGED_EVENT,
    type NoteTypeDef,
    type FieldKey,
  } from "$lib/types/noteTypes";
  import PathPreviewModal from "$lib/modals/PathPreviewModal.svelte";
  import { buildFlashcardConfig } from "$lib/utils/flashcardConfig";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import CodeEditor from "$lib/components/CodeEditor.svelte";
  import { snackbar, createSnackbarNotifier } from "$lib/stores/snackbarStore.svelte";
  import { smartMatchingStore } from "$lib/stores/smartMatchingStore.svelte";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import { ankiStore } from "$lib/stores/ankiStore.svelte";
  import { fontStore } from "$lib/stores/fontStore.svelte";
  import EmptyState from "$lib/components/EmptyState.svelte";
  import FlashcardsPreviewModal from "$lib/modals/FlashcardsPreviewModal.svelte";
  import { previewStore } from "$lib/stores/previewStore.svelte";
  import type { EpisodeMediaOverrides, AudioTrackInfo, EpisodeMediaOverrideKey } from "$lib/types/flashcardMediaTypes";
  import { formatAudioTrackLabel } from "$lib/types/flashcardMediaTypes";
  import {
    loadMediaSettings,
    saveMediaSettings,
    type MediaSettings,
  } from "$lib/utils/mediaSettings";
  import type { CardFilterSettings } from "$lib/types/flashcardFilterTypes";
  import CardFiltersPanel from "$lib/panels/CardFiltersPanel.svelte";
  import { episodeMediaEditorStore } from "$lib/stores/episodeMediaEditorStore.svelte";
  import EpisodeMediaSettingsModal from "$lib/modals/EpisodeMediaSettingsModal.svelte";
  import AudioClipsPanel from "$lib/panels/AudioClipsPanel.svelte";
  import SnapshotsPanel from "$lib/panels/SnapshotsPanel.svelte";
  import VideoClipsPanel from "$lib/panels/VideoClipsPanel.svelte";
  import EpisodeContextMenu from "$lib/components/EpisodeContextMenu.svelte";
  import FilesOutputPanel from "$lib/panels/FilesOutputPanel.svelte";
  import { generationStore, SERIES_OUTPUT_MODE_KEY } from "$lib/stores/generationStore.svelte";
  import FlashcardsWorkspace from "./flashcards/FlashcardsWorkspace.svelte";
  import {
    type PanelId,
    computeEffectiveColumnCount,
  } from "$lib/utils/flashcardLayout";
  import * as vestaConfig from "$lib/config/vestaConfig";
  import { errorMessage } from "$lib/utils/errorMessage";
  import { pickEpisodeFile } from "$lib/workflows/episodeFilePicker";
  import MatchDiagnosticsModal from "$lib/modals/MatchDiagnosticsModal.svelte";
  import EpisodeEditorModal, { type EpisodeFileField } from "./flashcards/EpisodeEditorModal.svelte";
  import FlashcardsFooterBar, { type GenerationRequirement } from "./flashcards/FlashcardsFooterBar.svelte";

  interface Props {
    active?: boolean;
    onGoToSettings?: (section?: "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts", highlightItemId?: string) => void;
  }

  let { active = true, onGoToSettings }: Props = $props();

  // The tab stays mounted while hidden; its preview dialog is visibility-bound
  // and should not keep keyboard or playback ownership in the background.
  $effect(() => {
    if (!active) previewStore.close();
  });

  let t = $derived($locale);
  let dropMediaHint = $derived(t("flashcards.dropMediaHint"));
  let dropSubtitleHint = $derived(t("flashcards.dropSubtitleHint"));

  let outputDir = $state("");

  // AudioTrackInfo / EpisodeMediaOverrides live in flashcardMediaTypes.ts so
  // the extracted media-settings panel components can import them too.

  let audioTracks = $state<AudioTrackInfo[]>([]);
  let audioTrackAutoSelected = $state(true);
  let audioTracksLoading = $state(false);

  const OUTPUT_DIR_KEY = "vesta-last-output-dir";
  const DEFAULT_FLASHCARDS_LANGUAGE_KEY = "vesta-default-flashcards-language";
  const DEFAULT_NATIVE_LANGUAGE_KEY = "vesta-default-native-language";
  const DEFAULT_TARGET_LANGUAGE_KEY = "vesta-default-target-language";
  const SERIES_MODE_KEY = "vesta-flashcards-series-mode";
  const SMART_MATCHING_RULES_KEY = "vesta-flashcards-smart-matching-rules";
  const ANKI_FIELDS_PANEL_OPEN_KEY = "vesta-flashcards-anki-fields-panel-open";

  let smartFileMatchingEnabled = $derived(uiMode.easyMode || smartMatchingStore.enabled);

  let smartMatchingRules = $derived(smartMatchingStore.rules);
  let episodeContextMenu = $state<{ x: number; y: number; idx: number; field?: EpisodeFileField } | null>(null);
  let bottomContextMenu = $state<{ x: number; y: number; section: "overview" | "anki" } | null>(null);

  function openBottomContextMenu(event: MouseEvent, section: "overview" | "anki") {
    event.preventDefault();
    let x = event.clientX;
    let y = event.clientY - 52;

    const target = event.currentTarget as HTMLElement | null;
    if (target && typeof target.getBoundingClientRect === "function") {
      const rect = target.getBoundingClientRect();
      // Position the menu just above the button top edge.
      // The single-item menu height is ~42px. To have a 6px gap, we subtract 48px.
      y = rect.top - 48;
      // Align left of the menu with the left of the button
      x = rect.left;
    }

    bottomContextMenu = {
      x: Math.min(x, window.innerWidth - 220),
      y: Math.max(0, y),
      section
    };
  }

  function closeBottomContextMenu() {
    bottomContextMenu = null;
  }

  // ─── Series Mode State ───────────────────────────────────────────────────

  let episodes = $state<EpisodeEntry[]>([]);
  let seriesCurrentEpisode = $state(0);
  let seriesTotalEpisodes = $state(0);
  let editingEpisodeIndex = $state<number | null>(null);
  let editingEpisode = $state<EpisodeEntry | null>(null);
  let initialEditingEpisodeStr = $state("");
  let dismissedFontBannerLang = $state<string | null>(null);
  const showSnackbar = createSnackbarNotifier();

  function loadDefaultLanguage(key: string, fallback = ""): string {
    try {
      return vestaConfig.getItem(key) || fallback;
    } catch {
      return fallback;
    }
  }

  function getStudiedLanguagePreference(): string {
    const episode = episodes.find(ep => ep.targetSubsPath);
    return (episode ? getEpisodeNoteType(episode).language : "") || noteTypeLanguage || loadDefaultLanguage(DEFAULT_FLASHCARDS_LANGUAGE_KEY);
  }

  function isFontBannerDismissed(lang: string): boolean {
    if (!lang) return true;
    if (dismissedFontBannerLang === lang) return true;
    try {
      return vestaConfig.getItem(`vesta-font-banner-dismissed-${lang}`) === "1";
    } catch {
      return false;
    }
  }

  function dismissFontBanner(lang: string, permanent: boolean = false) {
    dismissedFontBannerLang = lang;
    if (permanent && lang) {
      try {
        vestaConfig.setItem(`vesta-font-banner-dismissed-${lang}`, "1");
        showSnackbar(t("flashcards.dontShowAgain") || "Preferenza salvata");
      } catch {
        /* ignore */
      }
    }
  }

  let proactiveLang = $derived(getStudiedLanguagePreference());
  let proactiveFont = $derived(fontStore.getFontForLanguage(proactiveLang));

  function getNativeLanguagePreference(): string {
    return (
      loadDefaultLanguage(DEFAULT_NATIVE_LANGUAGE_KEY) ||
      loadDefaultLanguage(DEFAULT_TARGET_LANGUAGE_KEY, "it")
    );
  }

  function getGenericMediaSettings(): Required<EpisodeMediaOverrides> {
    return mediaSettings;
  }

  function getEpisodeMediaSettings(ep: EpisodeEntry): Required<EpisodeMediaOverrides> {
    return {
      ...getGenericMediaSettings(),
      ...(ep.mediaOverrides || {}),
    };
  }

  function mediaOverrideValueChanged(key: EpisodeMediaOverrideKey): boolean {
    const overrides = episodeMediaEditorStore.overrides;
    const episode = episodeMediaEditorStore.episode;
    if (!overrides) return false;
    const genericSettings = getGenericMediaSettings();

    const autoTrack = key === "audioTrackIndex" && episode && genericSettings.audioTrackIndex === null
      ? pickBestAudioTrackIndex(episodeMediaEditorStore.audioTracks, getPreferredAudioLanguageCodeForEpisode(episode))
      : undefined;
    return mediaSettingChanged(key, overrides, genericSettings, autoTrack);
  }

  function mediaOverrideClass(key: EpisodeMediaOverrideKey): string {
    return mediaOverrideValueChanged(key) ? "media-override-glow" : "";
  }

  function buildEpisodeMediaOverrideDiff(settings: Required<EpisodeMediaOverrides>): EpisodeMediaOverrides {
    const episode = episodeMediaEditorStore.episode;
    const defaults = getGenericMediaSettings();
    const autoTrack = episode && defaults.audioTrackIndex === null
      ? pickBestAudioTrackIndex(episodeMediaEditorStore.audioTracks, getPreferredAudioLanguageCodeForEpisode(episode))
      : undefined;
    return episodeMediaDiff(settings, defaults, autoTrack);
  }

  function getPreferredAudioLanguageCodeForEpisode(ep: Pick<EpisodeEntry, "targetSubsPath" | "noteTypeId" | "detectedLanguage">): string {
    return getEpisodeNoteType(ep).language || noteTypeLanguage;
  }

  // Le euristiche pure di parsing/matching (regex su filename, classificazione
  // target/native, aggregazione per episodio) vivono in seriesFileMatching.ts,
  // testabili senza montare questo componente. Qui restano solo wrapper sottili
  // che passano stato reattivo (regole di smart-matching, preferenze lingua,
  // `episodes`) alle funzioni pure e assegnano il risultato allo stato.

  function mergeSeriesDroppedFilesIntoEpisodes(subtitleFiles: string[], mediaFiles: string[]) {
    episodes = mergeSeriesDroppedFiles(
      episodes,
      subtitleFiles,
      mediaFiles,
      smartMatchingRules,
      getStudiedLanguagePreference(),
      getNativeLanguagePreference(),
    );
  }

  // Non ancora cablato a nessun call site nella UI (lo era già prima di
  // questo refactor) — lasciato perché `mergeSeriesDroppedFilesIntoEpisodes`
  // copre oggi il flusso reale di drag&drop/serie. Segnalato, non rimosso.
  function autoMatchFilesIntoEpisodes(
    targetFiles: string[],
    nativeFiles: string[],
    mediaFiles: string[],
  ) {
    episodes = autoMatchFiles(targetFiles, nativeFiles, mediaFiles, smartMatchingRules.episodeRegexes, smartMatchingRules);
  }

  function expandSeriesFilesWithSmartMatches(subtitleFiles: string[], mediaFiles: string[]) {
    return expandFlashcardFiles(subtitleFiles, mediaFiles, {
      enabled: smartFileMatchingEnabled,
      rules: smartMatchingRules,
      targetLanguage: getStudiedLanguagePreference(),
      nativeLanguage: getNativeLanguagePreference(),
      invoke,
    });
  }

  // Normalize open() return: may be string | string[] | null
  function normalizeSelected(selected: unknown): string[] {
    if (!selected) return [];
    if (typeof selected === "string") return [selected];
    if (Array.isArray(selected)) return selected.filter((s): s is string => typeof s === "string" && s.length > 0);
    return [];
  }

  async function addSeriesMultipleFiles() {
    try {
      const raw = await guardedOpen({
        multiple: true,
        filters: [
          {
            name: "Subtitle and Media Files",
            extensions: ["srt", "ass", "ssa", "vtt", ...VIDEO_EXTENSIONS, ...AUDIO_EXTENSIONS],
          },
        ],
      });
      const selected = normalizeSelected(raw);
      if (selected.length > 0) {
        await handleFileDrop(selected);
      }
    } catch (e) {
      generationStore.error = `${t("flashcards.errorSelectingFile")}: ${errorMessage(e)}`;
    }
  }



  function removeEpisode(idx: number) {
    episodes = episodes
      .filter((_, i) => i !== idx)
      .map((e, i) => ({ ...e, id: i + 1 }));
    closeEpisodeContextMenu();
  }

  let inspectedMatchPath = $state("");
  let inspectedMatch = $state<EpisodeEntry | null>(null);
  let editingEpisodeField = $state<EpisodeFileField | null>(null);
  let audioClipsPanel: AudioClipsPanel | undefined = $state();
  async function previewEpisodeAudio(idx: number) {
    closeEpisodeContextMenu();
    await audioClipsPanel?.previewEpisode(idx);
  }
  async function editContextEpisode() {
    const context = episodeContextMenu;
    if (!context) return;
    if (!context.field) { openEpisodeEditor(context.idx); return; }
    const episode = episodes[context.idx];
    closeEpisodeContextMenu();
    if (!episode) return;
    try {
      const replacement = await pickEpisodeFile(episode, context.field);
      if (!replacement || episodes[context.idx] !== episode) return;
      episodes = episodes.map((item, idx) => idx === context.idx ? replacement : item);
      showSnackbar(t("flashcards.fileReplaced", { file: getFileName(replacement[context.field]) }), "success");
    } catch (error) {
      generationStore.error = `${t("flashcards.errorSelectingFile")}: ${errorMessage(error)}`;
    }
  }
  function openEpisodeEditor(idx: number, field?: EpisodeFileField) {
    editingEpisodeField = field ?? null;
    const episode = episodes[idx];
    if (!episode) return;
    closeEpisodeContextMenu();
    editingEpisodeIndex = idx;
    editingEpisode = { ...episode };
    initialEditingEpisodeStr = JSON.stringify(episode);
  }

  function openEpisodeContextMenu(event: MouseEvent, idx: number, field?: EpisodeFileField) {
    event.preventDefault();
    episodeContextMenu = { x: event.clientX, y: event.clientY, idx, field };
  }

  function closeEpisodeContextMenu() {
    episodeContextMenu = null;
  }

  let mediaNoteTypeId = $state("default");
  let initialMediaNoteTypeId = $state("default");

  async function openEpisodeMediaSettings(idx: number) {
    const episode = episodes[idx];
    if (!episode || !episode.mediaPath) return;

    closeEpisodeContextMenu();
    mediaNoteTypeId = episode.noteTypeId || "default";
    initialMediaNoteTypeId = mediaNoteTypeId;
    episodeMediaEditorStore.begin(idx, episode, getEpisodeMediaSettings(episode));

    if (episode.mediaType === "video") {
      const tracks = await listAudioTracksForEpisode(episode);
      if (episodeMediaEditorStore.episodeIndex === idx) {
        episodeMediaEditorStore.setAudioTracks(tracks);
        if (episodeMediaEditorStore.overrides && episode.mediaOverrides?.audioTrackIndex === undefined) {
          episodeMediaEditorStore.update(
            "audioTrackIndex",
            pickBestAudioTrackIndex(tracks, getPreferredAudioLanguageCodeForEpisode(episode)),
          );
        }
      }
    }

    episodeMediaEditorStore.captureBaseline();
  }

  function closeEpisodeEditor() {
    const initialId = initialEditingEpisodeStr ? JSON.parse(initialEditingEpisodeStr).noteTypeId : undefined;
    if (editingEpisode?.noteTypeId && editingEpisode.noteTypeId !== initialId) {
      saveActiveNoteTypeId(editingEpisode.noteTypeId);
    }
    editingEpisodeIndex = null;
    editingEpisode = null;
    initialEditingEpisodeStr = "";
  }

  function closeEpisodeMediaSettings() {
    episodeMediaEditorStore.close();
  }

  function syncEpisodeEditor() {
    if (editingEpisodeIndex === null || !editingEpisode) return;
    const updatedEpisode: EpisodeEntry = {
      id: editingEpisode.id,
      targetSubsPath: editingEpisode.targetSubsPath,
      nativeSubsPath: editingEpisode.nativeSubsPath,
      mediaPath: editingEpisode.mediaPath,
      mediaType: editingEpisode.mediaPath
        ? detectMediaType(getFileName(editingEpisode.mediaPath))
        : "none",
      mediaOverrides: editingEpisode.mediaPath ? editingEpisode.mediaOverrides : undefined,
      matching: editingEpisode.matching,
      noteTypeId: editingEpisode.noteTypeId,
      detectedLanguage: episodes[editingEpisodeIndex]?.targetSubsPath === editingEpisode.targetSubsPath ? editingEpisode.detectedLanguage : undefined,
    };
    episodes = episodes.map((episode, idx) =>
      idx === editingEpisodeIndex
        ? {
            ...updatedEpisode,
            id: episode.id,
          }
        : episode,
    );
  }

  function revertEpisodeEditor() {
    if (editingEpisodeIndex !== null && initialEditingEpisodeStr) {
      const restored = JSON.parse(initialEditingEpisodeStr);
      editingEpisode = restored;
      episodes = episodes.map((ep, idx) =>
        idx === editingEpisodeIndex ? restored : ep
      );
    }
    closeEpisodeEditor();
  }

  function saveEpisodeMediaSettings() {
    const idx = episodeMediaEditorStore.episodeIndex;
    const overrides = episodeMediaEditorStore.overrides;
    if (idx === null || !overrides) return;
    const mediaOverrides = buildEpisodeMediaOverrideDiff(overrides);
    episodes = episodes.map((episode, i) =>
      i === idx
        ? {
            ...episode,
            mediaOverrides: Object.keys(mediaOverrides).length > 0 ? mediaOverrides : undefined,
            noteTypeId: mediaNoteTypeId === "default" ? undefined : mediaNoteTypeId,
          }
        : episode,
    );
    if (mediaNoteTypeId !== "default" && mediaNoteTypeId !== initialMediaNoteTypeId) saveActiveNoteTypeId(mediaNoteTypeId);
    closeEpisodeMediaSettings();
  }

  function resetEpisodeMediaSettings() {
    const idx = episodeMediaEditorStore.episodeIndex;
    if (idx === null) return;
    episodes = episodes.map((episode, i) =>
      i === idx
        ? {
            ...episode,
            mediaOverrides: undefined,
            noteTypeId: undefined,
          }
        : episode,
    );
    closeEpisodeMediaSettings();
  }

  async function selectEpisodeFile(field: EpisodeFileField) {
    if (!editingEpisode) return;
    try {
      const episode = editingEpisode;
      const replacement = await pickEpisodeFile(episode, field);
      if (!replacement || editingEpisode !== episode) return;
      editingEpisode = replacement;
      syncEpisodeEditor();
      showSnackbar(
        t("flashcards.fileReplaced", { file: getFileName(replacement[field]) }),
        "success",
      );
    } catch (e) {
      generationStore.error = `${t("flashcards.errorSelectingFile")}: ${errorMessage(e)}`;
    }
  }

  function isEpisodeFileCompatible(field: EpisodeFileField, path: string) {
    const type = detectMediaType(getFileName(path));
    return field === "mediaPath"
      ? type === "video" || type === "audio"
      : /\.(srt|ass|ssa|vtt)$/i.test(path);
  }

  function replaceEpisodeFileFromDrop(paths: string[], position?: { x: number; y: number }): boolean {
    if (!editingEpisode || !position) return false;
    const target = document.elementFromPoint(position.x, position.y)?.closest<HTMLElement>("[data-episode-file-field]");
    const field = target?.dataset.episodeFileField as EpisodeFileField | undefined;
    if (!field || target?.dataset.disabled === "true") return true;

    const replacement = paths.find((path) => isEpisodeFileCompatible(field, path));
    if (!replacement) {
      showSnackbar(
        field === "mediaPath" ? dropMediaHint : dropSubtitleHint,
        "error",
      );
      return true;
    }

    editingEpisode = {
      ...editingEpisode,
      [field]: replacement,
      mediaType: field === "mediaPath" ? detectMediaType(getFileName(replacement)) : editingEpisode.mediaType,
    };
    syncEpisodeEditor();
    showSnackbar(
      t("flashcards.fileReplaced", { file: getFileName(replacement) }),
      "success",
    );
    return true;
  }

  function clearEpisodeField(field: EpisodeFileField) {
    if (!editingEpisode || !editingEpisode[field]) return;
    editingEpisode = {
      ...editingEpisode,
      [field]: "",
      mediaType: field === "mediaPath" ? "none" : editingEpisode.mediaType,
    };
    syncEpisodeEditor();
  }

  function clearAllEpisodes() {
    episodes = [];
  }

  function swapEpisodeSubs(idx: number) {
    const ep = episodes[idx];
    if (!ep) return;
    const temp = ep.targetSubsPath;
    ep.targetSubsPath = ep.nativeSubsPath;
    ep.nativeSubsPath = temp;
    ep.detectedLanguage = undefined;
    episodes = [...episodes];
  }

  function swapAllEpisodesSubs() {
    episodes = episodes.map((ep) => {
      const temp = ep.targetSubsPath;
      return {
        ...ep,
        targetSubsPath: ep.nativeSubsPath,
        nativeSubsPath: temp,
        detectedLanguage: undefined
      };
    });
  }

  let hasMedia = $derived(episodes.some((ep) => ep.mediaType !== "none"));
  let hasVideo = $derived(episodes.some((ep) => ep.mediaType === "video"));
  let hasAudio = $derived(hasMedia);

  let showAnkiFields = true;
  let hasAnyFiles = $derived(episodes.length > 0);
  const HINT_LOAD_TARGET_FIRST =
    "Load the Original subtitle track first in Files & Output to unlock this section.";
  const HINT_LOAD_MEDIA_FIRST =
    "Load a media file (audio or video) first to unlock this section.";
  const HINT_LOAD_VIDEO_FIRST =
    "Load a video file first to unlock this section.";
  const PANEL_INFO_BUTTON_CLASS =
    "text-gray-500 hover:text-emerald-300 transition-colors";

  // Movie-mode media settings — same shape as EpisodeMediaOverrides so a
  // per-episode override is just a partial diff against this object (see
  // getGenericMediaSettings / getEpisodeMediaSettings below).
  let mediaSettings = $state<MediaSettings>(loadMediaSettings());
  // "auto" = GPU encoder when available (default), "off" = force libx264 (expert mode).
  // Not part of EpisodeMediaOverrides: it's a global preference, not overridable per-episode.
  let videoHwAccel = $state(
    (() => {
      try {
        return vestaConfig.getItem("vesta-video-hw-accel") === "off" ? "off" : "auto";
      } catch {
        return "auto";
      }
    })(),
  );
  $effect(() => {
    try {
      vestaConfig.setItem("vesta-video-hw-accel", videoHwAccel);
    } catch {
      /* storage unavailable */
    }
  });

  // Tracks changes to snapshots / video clips toggles to enforce mutual exclusivity in APKG mode.
  let prevGenerateSnapshots = $state(true);
  let prevGenerateVideoClips = $state(false);

  $effect(() => {
    if (generationStore.effectiveExportFormat === "apkg") {
      if (mediaSettings.generateSnapshots && mediaSettings.generateVideoClips) {
        if (mediaSettings.generateSnapshots !== prevGenerateSnapshots) {
          mediaSettings.generateVideoClips = false;
        } else if (mediaSettings.generateVideoClips !== prevGenerateVideoClips) {
          mediaSettings.generateSnapshots = false;
        } else {
          mediaSettings.generateVideoClips = false;
        }
      }
    }
    prevGenerateSnapshots = mediaSettings.generateSnapshots;
    prevGenerateVideoClips = mediaSettings.generateVideoClips;
  });

  // ─── Card Filters ────────────────────────────────────────────────────────
  // Card-length/duration filters — single reactive object (see
  // flashcardFilterTypes.ts) so CardFiltersPanel.svelte can own the panel UI
  // via one bind:filters prop, same pattern as mediaSettings.
  let cardFilters = $state<CardFilterSettings>({
    enabled: false,
    minChars: 8,
    maxChars: 120,
    minCharsEnabled: false,
    maxCharsEnabled: false,
    minDurationMs: 500,
    maxDurationMs: 8000,
    minDurationEnabled: false,
    maxDurationEnabled: false,
    combineSentences: false,
    continuationChars: ",、→",
  });

  let prevMinChars: number | undefined = undefined;
  let prevMaxChars: number | undefined = undefined;
  let prevMinDuration: number | undefined = undefined;
  let prevMaxDuration: number | undefined = undefined;

  $effect(() => {
    // Keep within absolute bounds
    if (cardFilters.minChars < 1) cardFilters.minChars = 1;
    if (cardFilters.minChars > 2000) cardFilters.minChars = 2000;
    if (cardFilters.maxChars < 1) cardFilters.maxChars = 1;
    if (cardFilters.maxChars > 2000) cardFilters.maxChars = 2000;

    if (cardFilters.minDurationMs < 0) cardFilters.minDurationMs = 0;
    if (cardFilters.minDurationMs > 120000) cardFilters.minDurationMs = 120000;
    if (cardFilters.maxDurationMs < 0) cardFilters.maxDurationMs = 0;
    if (cardFilters.maxDurationMs > 120000) cardFilters.maxDurationMs = 120000;

    // Initialize trackers on first run
    if (prevMinChars === undefined) prevMinChars = cardFilters.minChars;
    if (prevMaxChars === undefined) prevMaxChars = cardFilters.maxChars;
    if (prevMinDuration === undefined) prevMinDuration = cardFilters.minDurationMs;
    if (prevMaxDuration === undefined) prevMaxDuration = cardFilters.maxDurationMs;

    // Enforce min <= max coherence based on which one was modified
    if (cardFilters.minChars !== prevMinChars) {
      if (cardFilters.minChars > cardFilters.maxChars) {
        cardFilters.maxChars = cardFilters.minChars;
      }
      prevMinChars = cardFilters.minChars;
      prevMaxChars = cardFilters.maxChars;
    } else if (cardFilters.maxChars !== prevMaxChars) {
      if (cardFilters.maxChars < cardFilters.minChars) {
        cardFilters.minChars = cardFilters.maxChars;
      }
      prevMinChars = cardFilters.minChars;
      prevMaxChars = cardFilters.maxChars;
    }

    if (cardFilters.minDurationMs !== prevMinDuration) {
      if (cardFilters.minDurationMs > cardFilters.maxDurationMs) {
        cardFilters.maxDurationMs = cardFilters.minDurationMs;
      }
      prevMinDuration = cardFilters.minDurationMs;
      prevMaxDuration = cardFilters.maxDurationMs;
    } else if (cardFilters.maxDurationMs !== prevMaxDuration) {
      if (cardFilters.maxDurationMs < cardFilters.minDurationMs) {
        cardFilters.minDurationMs = cardFilters.maxDurationMs;
      }
      prevMinDuration = cardFilters.minDurationMs;
      prevMaxDuration = cardFilters.maxDurationMs;
    }
  });

  // One effect for the whole blob: every media setting now survives a restart,
  // not just the two dimensions that used to have their own keys.
  $effect(() => {
    saveMediaSettings($state.snapshot(mediaSettings));
  });

  // Export format state lives in exportFormatStore (generationStore only
  // delegates to it), and its setters persist to vestaConfig on their own —
  // no per-tab sync $effects needed.
  $effect(() => {
    try { vestaConfig.setItem(SERIES_OUTPUT_MODE_KEY, generationStore.seriesOutputMode); } catch {}
  });

  let handleCpuCoresChanged = (e: Event) => {
    generationStore.cpuCores = (e as CustomEvent<number>).detail;
  };
  let handleFfmpegUpdated = () => {
    invoke<boolean>("flashcard_check_deps")
      .then((ok) => (ffmpegAvailable = ok))
      .catch(() => {});
  };

  // ─── Easy/Expert mode ──────────────────────────────────────────────────────
  // In Easy mode advanced choices collapse to sane defaults: forced .apkg
  // export, CPU cores = n-1, automatic deck name. The corresponding UI is
  // hidden while the user only decides which media to include.
  let easyMode = $derived(uiMode.easyMode);

  // Responsive columns: auto-collapse from 3 -> 2 -> 1 based on available width.
  let layoutHostEl = $state<HTMLElement | null>(null);
  let layoutWidth = $state(
    typeof window !== "undefined" ? window.innerWidth : 1700,
  );
  let effectiveColumnCount = $derived(computeEffectiveColumnCount(layoutWidth));

  let filesHelpContent = $derived(t("flashcards.filesHelp"));

  let noteTypeLanguage = $state("");

  // ── Note type selection ──────────────────────────────────────────────────
  // One select replaces the old "language + seven field pills" duplication. The
  // chosen note type drives the export name, the study language, and which of the
  // nine fields are active. Predefined note types are generated per language and
  // locked to all fields; custom ones are created and edited in Settings. The
  // active field set is read-only here — editing lives in Settings by design.
  let noteTypeList = $state<NoteTypeDef[]>(listNoteTypes());
  let fallbackNoteTypeId = $state(loadLastUsedNoteTypeId());
  let selectedNoteTypeId = $derived(episodes[0]?.noteTypeId || "default");
  let automaticNoteType = $derived(!episodes[0]?.noteTypeId);
  let activeNoteType = $derived(getEpisodeNoteType(episodes[0] || { targetSubsPath: "" }));
  let automaticEpisodeNoteType = $derived(getEpisodeNoteType({ ...episodes[0], targetSubsPath: episodes[0]?.targetSubsPath || "", noteTypeId: undefined }));
  let subtitleLanguages = $state<Record<string, string | null>>({});
  const languageRequests = new Map<string, Promise<string | null>>();

  function getEpisodeNoteType(ep: Pick<EpisodeEntry, "targetSubsPath" | "noteTypeId" | "detectedLanguage">): NoteTypeDef {
    return resolveEpisodeNoteType({ ...ep, detectedLanguage: subtitleLanguages[ep.targetSubsPath] || ep.detectedLanguage }, noteTypeList, fallbackNoteTypeId);
  }

  function detectEpisodeLanguage(path: string): Promise<string | null> {
    if (!path || inferLanguageFromPath(path)) return Promise.resolve(null);
    const pending = languageRequests.get(path);
    if (pending) return pending;
    const request = invoke<Array<{text: string}>>("flashcard_parse_subtitles", { path })
      .then(cues => detectSubtitleLanguage((cues || []).slice(0, 160).map(cue => cue.text).join("\n")))
      .catch(() => null)
      .then(language => { subtitleLanguages[path] = language; return language; });
    languageRequests.set(path, request);
    return request;
  }

  $effect(() => {
    const paths = [...new Set(episodes.map(ep => ep.targetSubsPath).filter(Boolean))];
    void Promise.all(paths.map(detectEpisodeLanguage));
  });

  function setEpisodeNoteType(index: number, id: string) {
    const noteTypeId = id === "default" || id === "auto" ? undefined : id;
    episodes = episodes.map((ep, i) => i === index ? { ...ep, noteTypeId } : ep);
    if (noteTypeId) { saveActiveNoteTypeId(noteTypeId); fallbackNoteTypeId = loadLastUsedNoteTypeId(); }
  }


  $effect(() => {
    const handler = () => {
      fallbackNoteTypeId = loadLastUsedNoteTypeId();
    };
    window.addEventListener(ACTIVE_NOTE_TYPE_CHANGED_EVENT, handler);
    return () => {
      window.removeEventListener(ACTIVE_NOTE_TYPE_CHANGED_EVENT, handler);
    };
  });

  let noteTypeOptions = $derived(
    [{ value: "default", label: `${t("flashcards.automaticNoteType")} · ${automaticEpisodeNoteType.name}`, displayLabel: automaticEpisodeNoteType.name, icon: languages.find((language) => language.code === automaticEpisodeNoteType.language)?.flag || '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" class="w-4 h-4"><path stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" d="M4 9V4h5m6 0h5v5M4 15v5h5m6 0h5v-5M8 12h8m-4-4v8" /></svg>' }, ...noteTypeList.filter((nt) => nt.id !== "default").map((nt) => ({
      value: nt.id,
      label: nt.name,
      searchTerms: [nt.name, getLanguageSearchTerms(nt.language)]
        .filter(Boolean)
        .join(" "),
      icon: languages.find((l) => l.code === nt.language)?.flag ?? "🃏",
    }))],
  );

  let activeFieldKeys = $derived(
    NOTE_TYPE_FIELD_ORDER.filter((k) => activeNoteType.included[k]),
  );

  function fieldChipLabel(key: FieldKey): string {
    switch (key) {
      case "expression":
        return `🗣️ ${t("flashcards.subs1")}`;
      case "meaning":
        return `💬 ${t("flashcards.subs2")}`;
      case "audio":
        return `🔊 ${t("flashcards.audioField")}`;
      case "snapshot":
        return `📸 ${t("flashcards.snapshotField")}`;
      case "video":
        return `🎬 ${t("flashcards.videoField")}`;
      case "tags":
        return `🏷️ ${t("flashcards.tagField")}`;
      case "sequenceMarker":
        return `🔢 ${t("flashcards.sequenceField")}`;
      case "reading":
        return `📖 ${t("flashcards.readingField")}`;
      case "notes":
        return `📝 ${t("flashcards.notesField")}`;
    }
  }

  function selectNoteType(id: string) {
    if (episodes.length === 1) setEpisodeNoteType(0, id);
    else saveActiveNoteTypeId(id);
  }

  // The automatic deck name and generation run-state (isProcessing,
  // progress, progressMessage, progressStage, logs, error, result) live in
  // generationStore now — see generationStore.svelte.ts.

  let ffmpegAvailable = $state<boolean | null>(null);
  let isDownloadingFFmpeg = $state(false);

  let previewMediaPath = $derived.by(() => {
    if (episodes.length > 0) {
      return episodes[0].mediaPath;
    }
    return "";
  });

  let previewMediaType = $derived.by(() => {
    if (episodes.length > 0) {
      return episodes[0].mediaType;
    }
    return "none";
  });

  let expandedPathField = $state<string | null>(null);

  let unlisten: (() => void) | null = null;
  let activeListener = true;

  $effect(() => {
    if (active) {
      const cleanupDragDrop = setupWebviewDragDrop({
        setDraggingOver: (v) => (isDraggingOver = editingEpisode ? false : v),
        onDrop: (paths, position) => {
          // A drop while the editor is open always belongs to the modal. It
          // either replaces the explicitly hovered field or is ignored; it
          // must never leak through and append a new Files & Output row.
          if (editingEpisode) {
            replaceEpisodeFileFromDrop(paths, position);
            return;
          }
          void handleFileDrop(paths);
        },
        onError: (e) => console.warn("Failed to set up drag-drop listener in FlashcardsTab:", e),
      });

      return () => {
        cleanupDragDrop();
      };
    }
  });
  let removeTemplateListener: (() => void) | null = null;
  let removeNoteTypesListener: (() => void) | null = null;
  let removeLanguageDefaultsListener: (() => void) | null = null;
  let removeLayoutObserver: (() => void) | null = null;
  let isDraggingOver = $state(false);
  let hasLoggedDragOver = false;

  // Note type names now come from the selected note type rather than the global
  // card template, so this just refreshes the list when custom note types change.
  function refreshNoteTypes() {
    noteTypeList = listNoteTypes();
  }
  let canRunFlashcards = $derived(
    episodes.length > 0 && episodes.every((ep) => ep.targetSubsPath) && Boolean(outputDir),
  );

  let isApkgSwitchEnabled = $derived(
    episodes.length > 1 &&
      generationStore.effectiveExportFormat === "apkg" &&
      !generationStore.isBusy &&
      !generationStore.result,
  );

  type RequirementPanelId = "files";

  let generationPromptOpen = $state(false);
  const requirementFeedback = new RequirementFeedback<RequirementPanelId>();

  let generationRequirements = $derived.by((): GenerationRequirement[] => {
    const missing: GenerationRequirement[] = [];
    if (episodes.length === 0) {
      missing.push({
        panel: "files",
        label: t("flashcards.reqAddEpisode"),
      });
    } else if (episodes.some((ep) => !ep.targetSubsPath)) {
      missing.push({
        panel: "files",
        label: t("flashcards.reqSubsAllEpisodes"),
      });
    }
    if (!outputDir) {
      missing.push({
        panel: "files",
        label: `${t("flashcards.outputDir")}`,
      });
    }
    return missing;
  });

  let generationRequirementsText = $derived(
    generationRequirements.map((item) => item.label).join(", "),
  );

  function panelHighlightClass(panelId: RequirementPanelId): string {
    return requirementFeedback.classFor(panelId);
  }

  function promptMissingGenerationRequirements() {
    if (canRunFlashcards) return;
    generationPromptOpen = true;
    requirementFeedback.pulse(generationRequirements.map(item => item.panel));
  }

  function closeGenerationPrompt() {
    generationPromptOpen = false;
  }


  function getPreferredAudioLanguageCode(): string {
    return getStudiedLanguagePreference();
  }

  // scoreAudioTrackForLanguage/pickBestAudioTrackIndex sono importate da
  // seriesFileMatching.ts (vedi cima del file).

  async function listAudioTracksForEpisode(ep: EpisodeEntry): Promise<AudioTrackInfo[]> {
    if (!ep.mediaPath || ep.mediaType !== "video") return [];
    try {
      return await invoke<AudioTrackInfo[]>("flashcard_list_audio_tracks", {
        path: ep.mediaPath,
      });
    } catch (e) {
      generationStore.addLog(`${t("flashcards.audioTracksError")}: ${errorMessage(e)}`, "warning", getFileName(ep.mediaPath));
      return [];
    }
  }

  async function pickAudioTrackIndexForEpisode(ep: EpisodeEntry, language: string): Promise<number | null> {
    const tracks = await listAudioTracksForEpisode(ep);
    return pickBestAudioTrackIndex(tracks, language);
  }


  async function loadAudioTracksForMedia(path: string) {
    audioTracks = [];
    mediaSettings.audioTrackIndex = null;
    audioTrackAutoSelected = true;

    if (detectMediaType(getFileName(path)) !== "video") return;

    audioTracksLoading = true;
    try {
      const tracks = await invoke<AudioTrackInfo[]>("flashcard_list_audio_tracks", {
        path,
      });
      audioTracks = tracks;
      mediaSettings.audioTrackIndex =
        tracks.length > 1 ? pickBestAudioTrackIndex(tracks, getPreferredAudioLanguageCode()) : null;
    } catch (e) {
      generationStore.addLog(`${t("flashcards.audioTracksError")}: ${errorMessage(e)}`, "warning");
    } finally {
      audioTracksLoading = false;
    }
  }

  $effect(() => {
    if (audioTracks.length > 1 && audioTrackAutoSelected) {
      mediaSettings.audioTrackIndex = pickBestAudioTrackIndex(audioTracks, getPreferredAudioLanguageCode());
    }
  });

  $effect(() => {
    const firstMedia = episodes[0]?.mediaPath;
    if (firstMedia && detectMediaType(getFileName(firstMedia)) === "video") {
      loadAudioTracksForMedia(firstMedia);
    }
  });

  // ─── File Drag-and-Drop Handler ───────────────────────────────────────────
  // getFileExtension/isSubtitleFile/isMediaFile/classifySubtitles/
  // deriveDeckNameFromFile is imported from
  // seriesFileMatching.ts (see top of file).

  async function handleFileDrop(paths: string[]) {
    if (!paths || paths.length === 0) {
      return;
    }

    const subtitleFiles = paths.filter(isSubtitleFile);
    const mediaFiles = paths.filter(isMediaFile);

    if (subtitleFiles.length === 0 && mediaFiles.length === 0) {
      return;
    }

    if (subtitleFiles.length > 0 || mediaFiles.length > 0) {
      try {
        const expanded = await expandSeriesFilesWithSmartMatches(
          subtitleFiles,
          mediaFiles,
        );
        mergeSeriesDroppedFilesIntoEpisodes(expanded.subtitleFiles, expanded.mediaFiles);
        generationStore.addLog(`${episodes.length} ${t("flashcards.seriesEpisodesAdded")}`, "target-subs");
      } catch (e: any) {
        console.error("[DragDrop] Errore nell'elaborazione smart della serie:", e);
      }
    }
  }

  async function handleHtmlDrop(e: DragEvent) {
    e.preventDefault();
    isDraggingOver = false;
    hasLoggedDragOver = false;

    if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const files = Array.from(e.dataTransfer.files);
      const paths: string[] = [];
      
      files.forEach((file) => {
        const path = (file as any).path;
        if (path) {
          paths.push(path);
        }
      });

      if (paths.length > 0) {
        await handleFileDrop(paths);
      }
    }
  }

  onMount(async () => {
    refreshNoteTypes();

    const handleCardTemplatesUpdated = () => {
      refreshNoteTypes();
    };
    const handleNoteTypesUpdated = () => {
      refreshNoteTypes();
    };
    window.addEventListener(NOTE_TYPES_UPDATED_EVENT, handleNoteTypesUpdated);
    removeNoteTypesListener = () =>
      window.removeEventListener(NOTE_TYPES_UPDATED_EVENT, handleNoteTypesUpdated);
    const handleLanguageDefaultsUpdated = () => {
      try {
        const defaultNoteTypeLanguage = vestaConfig.getItem(DEFAULT_FLASHCARDS_LANGUAGE_KEY);
        if (
          defaultNoteTypeLanguage &&
          languages.some((l) => l.code === defaultNoteTypeLanguage)
        ) {
          noteTypeLanguage = defaultNoteTypeLanguage;
        }
      } catch {}
    };

    const updateLayoutWidth = () => {
      const hostWidth = layoutHostEl?.getBoundingClientRect().width;
      layoutWidth = hostWidth && hostWidth > 0 ? Math.round(hostWidth) : window.innerWidth;
    };

    const resizeObserver =
      typeof ResizeObserver !== "undefined"
        ? new ResizeObserver(() => updateLayoutWidth())
        : null;

    if (resizeObserver && layoutHostEl) {
      resizeObserver.observe(layoutHostEl);
    }

    const handleResize = () => updateLayoutWidth();
    window.addEventListener("resize", handleResize);
    updateLayoutWidth();

    removeLayoutObserver = () => {
      window.removeEventListener("resize", handleResize);
      if (resizeObserver) {
        resizeObserver.disconnect();
      }
    };

    window.addEventListener(
      CARD_TEMPLATES_UPDATED_EVENT,
      handleCardTemplatesUpdated,
    );
    removeTemplateListener = () =>
      window.removeEventListener(
        CARD_TEMPLATES_UPDATED_EVENT,
        handleCardTemplatesUpdated,
      );
    window.addEventListener(
      "vesta-language-defaults-updated",
      handleLanguageDefaultsUpdated,
    );
    removeLanguageDefaultsListener = () =>
      window.removeEventListener(
        "vesta-language-defaults-updated",
        handleLanguageDefaultsUpdated,
      );

    noteTypeLanguage = loadDefaultLanguage(DEFAULT_FLASHCARDS_LANGUAGE_KEY);

    try {
      const savedDir = vestaConfig.getItem(OUTPUT_DIR_KEY);
      if (savedDir) {
        const exists = await invoke<boolean>("flashcard_check_dir_exists", {
          path: savedDir,
        });
        if (exists) {
          outputDir = savedDir;
        } else {
          vestaConfig.removeItem(OUTPUT_DIR_KEY);
        }
      }
    } catch {}

    if (!outputDir) {
      try { outputDir = await defaultOutputDirectory(); } catch {}
    }

    try {
      ffmpegAvailable = await invoke<boolean>("flashcard_check_deps");
    } catch {
      ffmpegAvailable = false;
    }

    try {
      generationStore.systemCpuCount = await invoke<number>("flashcard_get_cpu_count");
      const startupMaxCores = Math.max(2, generationStore.systemCpuCount - 1);
      const savedCores = vestaConfig.getItem("vesta_cpu_cores");
      if (savedCores) {
        generationStore.cpuCores = parseInt(savedCores);
      } else {
        generationStore.cpuCores = startupMaxCores;
      }
    } catch {
      generationStore.systemCpuCount = 4;
      const savedCores = vestaConfig.getItem("vesta_cpu_cores");
      if (savedCores) {
        generationStore.cpuCores = parseInt(savedCores);
      } else {
        generationStore.cpuCores = Math.max(2, generationStore.systemCpuCount - 1);
      }
    }

    // This async onMount can resume after its component has been destroyed.
    // Keep the progress subscription's late-resolution guard consistent with
    // these window listeners so teardown cannot be followed by a fresh leak.
    if (activeListener) {
      window.addEventListener("vesta-cpu-cores-changed", handleCpuCoresChanged);
      window.addEventListener("vesta-ffmpeg-updated", handleFfmpegUpdated);
    }

    // OS-level file drag and drop is handled dynamically via $effect

    listen<{
      stage: string;
      message: string;
      current: number;
      total: number;
      percentage: number;
      params: Record<string, string>;
    }>("flashcard-progress", (event) => {
      if (!generationStore.isBusy || cancelRequested) return;
      const p = event.payload;
      const translated = t(p.message, p.params || {});
      
      if (seriesTotalEpisodes > 0) {
        generationStore.progress = Math.round(((seriesCurrentEpisode - 1) * 100 / seriesTotalEpisodes) + (p.percentage / seriesTotalEpisodes));
        generationStore.progressMessage = `[Ep. ${seriesCurrentEpisode}/${seriesTotalEpisodes}] ${translated}`;
      } else {
        generationStore.progress = Math.round(p.percentage);
        generationStore.progressMessage = translated;
      }
      
      generationStore.progressStage = p.stage;
      generationStore.phaseProgress = p.stage === "optimizing" && p.params?.percent
        ? Number(p.params.percent) : null;
      if (p.stage !== "done") {
        generationStore.addLog(generationStore.progressMessage, "progress", undefined, p.message);
      }
    }).then((fn) => {
      if (!activeListener) fn();
      else unlisten = fn;
    }).catch(console.error);
  });

  onDestroy(() => {
    flashcardPreview.dispose();
    activeListener = false;
    window.removeEventListener("vesta-cpu-cores-changed", handleCpuCoresChanged);
    window.removeEventListener("vesta-ffmpeg-updated", handleFfmpegUpdated);
    if (unlisten) unlisten();
    if (removeTemplateListener) removeTemplateListener();
    if (removeNoteTypesListener) removeNoteTypesListener();
    if (removeLanguageDefaultsListener) removeLanguageDefaultsListener();
    if (removeLayoutObserver) removeLayoutObserver();
    requirementFeedback.clear();
  });

  // ─── AnkiConnect auto-import ─────────────────────────────────────────────
  // If Anki (with the AnkiConnect add-on) is running, push the freshly
  // generated .apkg straight into the collection instead of leaving the
  // user to import it by hand from the Experimental tab. Reuses the same
  // AnkiConnect URL configured there; silently no-ops when Anki isn't
  // reachable — the .apkg stays on disk either way.
  async function maybeAutoImportToAnki(apkgPath: string) {
    const url = (() => {
      try {
        return vestaConfig.getItem("vesta-ankiconnect-url") || "http://127.0.0.1:8765";
      } catch {
        return "http://127.0.0.1:8765";
      }
    })();

    try {
      await invoke<number>("ankiconnect_ping", { url });
    } catch {
      if (generationStore.exportFormat === "anki") {
        const fallbackUpper = generationStore.fallbackFormat.toUpperCase();
        const msg = t("flashcards.ankiNotRunningFallback", { format: fallbackUpper });
        generationStore.addLog(`ℹ ${msg}`, "info");
        snackbar.show(msg, "info");
      }
      return;
    }

    try {
      await invoke("ankiconnect_import_package", { path: apkgPath, url });
      generationStore.addLog(t("flashcards.ankiAutoImportSuccess"), "success");
      snackbar.show(t("flashcards.ankiAutoImportSuccess"), "success");
    } catch (e) {
      generationStore.addLog(`${t("flashcards.ankiAutoImportFailed")}: ${errorMessage(e)}`, "warning");
    }
  }

  // addLog/clearLogs now live on generationStore (generationStore.svelte.ts).

  function buildConfig() {
    const ep = episodes[0];
    const tPath = ep?.targetSubsPath || "";
    const nPath = ep?.nativeSubsPath || null;
    const vPath = ep?.mediaType === "video" ? ep.mediaPath : null;
    const aPath = ep?.mediaType === "audio" ? ep.mediaPath : null;

    return buildFlashcardConfig({
      targetSubsPath: tPath,
      nativeSubsPath: nPath,
      videoPath: vPath,
      audioPath: aPath,
      outputDir,
      cardFilters,
      media: mediaSettings,
      generateAudio: mediaSettings.generateAudio,
      generateSnapshots: mediaSettings.generateSnapshots,
      generateVideoClips: mediaSettings.generateVideoClips,
      audioTrackIndex: mediaSettings.audioTrackIndex,
      videoHwAccel,
      deckName: ep ? deriveDeckNameFromFile(ep) : "",
      episodeNumber: 1,
      exportFormat: generationStore.effectiveExportFormat,
      noteType: ep ? getEpisodeNoteType(ep) : activeNoteType,
      automaticNoteType: false,
      cpuCores: generationStore.effectiveCpuCores,
      targetLanguage: getStudiedLanguagePreference(),
      autoCardFont: ankiStore.autoCardFont,
      embedCardFont: ankiStore.embedCardFont,
    });
  }

  const flashcardPreview = new FlashcardPreviewController({
    isBusy: () => generationStore.isBusy,
    canPreview: () => canRunFlashcards,
    isLoading: () => previewStore.loading,
    buildConfig,
    load: (config, onLoaded) => previewStore.load(config, onLoaded),
    close: () => previewStore.close(),
    setError: (message) => { generationStore.error = message; },
    missingRequirementsMessage: () => t("flashcards.requiredFieldsMissing"),
    addLog: (message, level) => generationStore.addLog(message, level),
  });
  async function selectOutputDir() {
    try {
      const selected = await guardedOpen({ directory: true });
      if (selected) {
        outputDir = selected as string;
        vestaConfig.setItem(OUTPUT_DIR_KEY, outputDir);
        generationStore.addLog(`${t("flashcards.outputDirSet")}`, "output", outputDir);
      }
    } catch (e) {
      generationStore.error = `${t("flashcards.errorSelectingDir")}: ${errorMessage(e)}`;
    }
  }

  async function loadPreview() {
    await flashcardPreview.load();
  }

  let cancelRequested = false;

  async function startSeriesGeneration() {
    await runFlashcardSeries({
      generationStore, episodes: $state.snapshot(episodes).map(ep => ({ ...ep, detectedLanguage: subtitleLanguages[ep.targetSubsPath] || ep.detectedLanguage })), t,
      getEpisodeNoteType,
      outputDir, cardFilters: $state.snapshot(cardFilters), videoHwAccel, activeNoteType, automaticNoteType,
      ankiStore: { autoCardFont: ankiStore.autoCardFont, embedCardFont: ankiStore.embedCardFont },
      previewStore,
      getEpisodeMediaSettings: ((defaults) => (episode: EpisodeEntry) => ({ ...defaults, ...episode.mediaOverrides }))($state.snapshot(mediaSettings)),
      getAudioTrackLanguageForEpisode: getPreferredAudioLanguageCodeForEpisode,
      pickAudioTrackIndexForEpisode,
      getStudiedLanguagePreference, maybeAutoImportToAnki, invoke,
      isCancelled: () => cancelRequested,
      setEpisodeProgress: (current, total) => {
        seriesCurrentEpisode = current;
        seriesTotalEpisodes = total;
      },
    });
  }

  async function startGeneration() {
    if (generationStore.isBusy || previewStore.loading) return;
    cancelRequested = false;
    if (!canRunFlashcards) {
      promptMissingGenerationRequirements();
      generationStore.error = t("flashcards.requiredFieldsMissing");
      return;
    }

    const run = generationStore.operation.begin();
    if (!run) return;
    try {
      await Promise.all(episodes.map(ep => detectEpisodeLanguage(ep.targetSubsPath)));
      if (cancelRequested) return;
      await startSeriesGeneration();
    } finally { run.finish(); }
  }

  async function cancelGeneration() {
    if (!generationStore.isBusy || generationStore.operation.cancelRequested) return;
    cancelRequested = true;
    try {
      if (!await generationStore.operation.cancel(() => invoke("flashcard_cancel"))) return;
      generationStore.addLog(`${t("flashcards.cancelled")}`, "warning");
    } catch (e) {
      cancelRequested = false;
      generationStore.addLog(`Error cancelling: ${errorMessage(e)}`, "error");
      snackbar.show(errorMessage(e), "error");
    }
  }

  function resetGeneration() {
    if (generationStore.isBusy) return;
    generationStore.resetRun();

    // Clear files so the user can insert new ones
    audioTracks = [];
    mediaSettings.audioTrackIndex = null;
    audioTrackAutoSelected = true;
    episodes = [];
    generationStore.deckName = "";
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    if (!active || e.defaultPrevented || e.isComposing) return;
    if (bottomContextMenu && e.key.toLowerCase() === "escape") {
      closeBottomContextMenu();
      e.preventDefault();
      return;
    }
    if (episodeContextMenu) {
      if (
        document.activeElement?.tagName === "INPUT" ||
        document.activeElement?.tagName === "TEXTAREA"
      )
        return;

      const key = e.key.toLowerCase();
      if (key === "escape") {
        closeEpisodeContextMenu();
        e.preventDefault();
        return;
      }
      if (key === "e") {
        void editContextEpisode();
        closeEpisodeContextMenu();
        e.preventDefault();
        return;
      }
      if (key === "s") {
        const contextEpisode = episodes[episodeContextMenu.idx];
        if (contextEpisode?.mediaPath) {
          openEpisodeMediaSettings(episodeContextMenu.idx);
        }
        closeEpisodeContextMenu();
        e.preventDefault();
        return;
      }
      if (key === "d" || key === "delete") {
        removeEpisode(episodeContextMenu.idx);
        closeEpisodeContextMenu();
        e.preventDefault();
        return;
      }
    }
  }
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="h-full flex flex-col bg-gray-900 relative overflow-hidden"
  ondragover={(e) => {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'copy';
    }
    isDraggingOver = editingEpisode ? false : true;
  }}
  ondrop={handleHtmlDrop}
  ondragleave={(e) => {
    const rt = e.relatedTarget as HTMLElement | null;
    const ct = e.currentTarget as HTMLElement;
    if (rt && ct.contains(rt)) return;
    isDraggingOver = false;
  }}
>
  <div class="flex-1 overflow-hidden overflow-x-hidden p-6 flashcards-scroll min-h-0 flex flex-col gap-4 {generationStore.isBusy ? 'pointer-events-none opacity-60 select-none' : ''}">
  {#if isDraggingOver && !editingEpisode}
    <div
      class="absolute inset-0 z-50 bg-violet-500/10 border-violet-400/80 text-violet-400 border-2 border-dashed rounded-2xl flex items-center justify-center pointer-events-none"
    >
      <div class="text-center">
        <svg
          class="w-16 h-16 mx-auto mb-3 text-violet-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
          />
        </svg>
        <p class="text-lg font-medium text-violet-300">
          {t("flashcards.dropFileHere")}
        </p>
        <p class="text-sm text-gray-400 mt-1">{t("flashcards.dropFileHint")}</p>
      </div>
    </div>
  {/if}
  {#if ffmpegAvailable === false}
    <div
      class="mb-4 p-3 bg-amber-500/10 border border-amber-500/30 rounded-lg flex items-center gap-3"
    >
      <svg
        class="w-5 h-5 text-amber-400 flex-shrink-0"
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
      <p class="text-amber-300 text-sm flex-1">
        {t("flashcards.ffmpegMissing")}
      </p>
      <button
        type="button"
        disabled={isDownloadingFFmpeg}
        onclick={async () => {
          isDownloadingFFmpeg = true;
          try {
            await invoke("flashcard_download_ffmpeg");
            ffmpegAvailable = await invoke<boolean>("flashcard_check_deps");
            window.dispatchEvent(new CustomEvent("vesta-ffmpeg-updated"));
          } catch (e) {
            generationStore.error = `${t("flashcards.ffmpegDownloadFailed")}: ${errorMessage(e)}`;
          } finally {
            isDownloadingFFmpeg = false;
          }
        }}
        class="flex-shrink-0 px-3 py-1.5 rounded-lg bg-amber-500/20 border border-amber-500/40 text-amber-300 text-xs font-semibold hover:bg-amber-500/30 transition-colors flex items-center gap-1.5 disabled:opacity-50 disabled:cursor-not-allowed"
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
    {@const proactiveLang = getStudiedLanguagePreference()}
    {@const proactiveFont = fontStore.getFontForLanguage(proactiveLang)}
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
              {t("flashcards.fontSuggestionTitle") || "Caratteri per"} {proactiveFont.language_name}
            </p>
            <p class="text-[11px] text-gray-300 mt-0.5 leading-relaxed">
              {t("flashcards.fontSuggestionDesc") || "Scarica ed embedda il font"} ({proactiveFont.approx_size}) {t("flashcards.fontSuggestionDescEnd") || "per visualizzare correttamente i caratteri su dispositivi senza font preinstallati,"} {t("flashcards.fontSuggestionDescLight") || "oppure continua con i font di sistema per un mazzo più leggero."}
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
              <span>{t("settings.download") || "Scarica ed embedda"} ({proactiveFont.approx_size})</span>
            {/if}
          </button>
          <button
            type="button"
            onclick={() => dismissFontBanner(proactiveLang, true)}
            class="px-2.5 py-1.5 rounded-lg border border-white/10 bg-white/5 text-gray-300 text-xs font-medium hover:bg-white/10 hover:text-white transition-colors"
            title={t("flashcards.dontShowAgain") || "Non mostrare più"}
          >
            <span>{t("flashcards.dontShowAgain") || "Non mostrare più"}</span>
          </button>
          <button
            type="button"
            onclick={() => dismissFontBanner(proactiveLang, false)}
            class="p-1.5 rounded-lg text-gray-400 hover:text-white hover:bg-white/10 transition-colors"
            title={t("common.close") || "Chiudi"}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>
    {/if}
  {/if}

  {#if previewStore.visible}
    <FlashcardsPreviewModal mediaPath={previewMediaPath} mediaType={previewMediaType} nativeSubsPath={episodes[0]?.nativeSubsPath || "" } {active} />
  {/if}

  {#snippet panelContent(panelId: PanelId)}
    {#if panelId === "files"}
      <FilesOutputPanel
        highlightClass={panelHighlightClass('files')}
        {episodes}
        {getEpisodeNoteType}
        {noteTypeOptions}
        contextTarget={episodeContextMenu}
        onAddFiles={addSeriesMultipleFiles}
        onClearAll={clearAllEpisodes}
        {showSnackbar}
        onSwapAll={swapAllEpisodesSubs}
        onSwap={swapEpisodeSubs}
        onEdit={openEpisodeEditor}
        onMediaSettings={openEpisodeMediaSettings}
        onRemove={removeEpisode}
        onContextMenu={openEpisodeContextMenu}
        {outputDir}
        onExpand={(field) => (expandedPathField = field)}
        onSelectOutput={selectOutputDir}
        onClearField={(field) => { if (field === "output") outputDir = ""; }}
      />
    {:else if panelId === "audioClips"}
      <AudioClipsPanel
        bind:this={audioClipsPanel}
        bind:settings={mediaSettings}
        {hasAudio}
        mediaType={previewMediaType}
        {audioTracks}
        {audioTracksLoading}
        hintLoadMediaFirst={HINT_LOAD_MEDIA_FIRST}
        firstEpisodeMediaPath={episodes[0]?.mediaPath || ""}
        firstEpisodeSubsPath={episodes[0]?.targetSubsPath || ""}
        {episodes}
        onTrackPicked={() => (audioTrackAutoSelected = false)}
      />
    {:else if panelId === "snapshots"}
      <SnapshotsPanel
        bind:settings={mediaSettings}
        {hasVideo}
        effectiveExportFormat={generationStore.effectiveExportFormat}
        hintLoadVideoFirst={HINT_LOAD_VIDEO_FIRST}
      />
    {:else if panelId === "videoClips"}
      <VideoClipsPanel
        bind:settings={mediaSettings}
        {hasVideo}
        effectiveExportFormat={generationStore.effectiveExportFormat}
        hintLoadVideoFirst={HINT_LOAD_VIDEO_FIRST}
      />
    {:else if panelId === "cardFilters"}
      <div class="space-y-4">
        <CardFiltersPanel
          bind:filters={cardFilters}
          {hasAnyFiles}
          hintLoadTargetFirst={HINT_LOAD_TARGET_FIRST}
        />
      </div>

    {/if}
  {/snippet}

  <div bind:this={layoutHostEl} class="flex min-h-0 flex-1 flex-col">
    <FlashcardsWorkspace columns={effectiveColumnCount} {easyMode} panel={panelContent}>
      {#snippet files()}{@render panelContent("files")}{/snippet}
    </FlashcardsWorkspace>
  </div>

  {#if episodeContextMenu}
    <EpisodeContextMenu
      x={episodeContextMenu.x}
      y={episodeContextMenu.y}
      hasMedia={Boolean(episodes[episodeContextMenu.idx]?.mediaPath)}
      onInspectMatch={() => { const context = episodeContextMenu!; inspectedMatch = episodes[context.idx]; inspectedMatchPath = context.field ? inspectedMatch[context.field] : ""; closeEpisodeContextMenu(); }}
      onEdit={editContextEpisode}
      onMediaSettings={() => openEpisodeMediaSettings(episodeContextMenu!.idx)}
      canPreviewAudio={mediaSettings.generateAudio}
      onPreviewAudio={() => previewEpisodeAudio(episodeContextMenu!.idx)}
      onRemove={() => removeEpisode(episodeContextMenu!.idx)}
      onClose={closeEpisodeContextMenu}
    />
  {/if}

  {#if bottomContextMenu}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50"
      onclick={closeBottomContextMenu}
      oncontextmenu={(e) => {
        e.preventDefault();
        closeBottomContextMenu();
      }}
      onkeydown={(e) => {
        if (e.key === "Escape") closeBottomContextMenu();
      }}
      role="presentation"
      tabindex="-1"
    >
      <div
        class="vesta-context-menu animate-fade-in"
        style="left: {bottomContextMenu.x}px; top: {bottomContextMenu.y}px;"
      >
        <button
          type="button"
          class="vesta-context-menu-item"
          onclick={() => {
            onGoToSettings?.(bottomContextMenu!.section);
            closeBottomContextMenu();
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
    <MatchDiagnosticsModal episode={inspectedMatch} focusedPath={inspectedMatchPath} onclose={() => inspectedMatch = null}
      onrules={() => { inspectedMatch = null; onGoToSettings?.("language", "smart-matching-rules"); }} />
  {/if}
  {#if editingEpisode}
    <EpisodeEditorModal
      {editingEpisode}
      {noteTypeOptions}
      {getEpisodeNoteType}
      episodeIndex={editingEpisodeIndex ?? undefined}
      onNoteTypeChange={(_index, id) => { if (editingEpisode) { editingEpisode.noteTypeId = id === "default" ? undefined : id; syncEpisodeEditor(); } }}
      onlyField={editingEpisodeField}
      {activeNoteType}
      {dropMediaHint}
      {dropSubtitleHint}
      onClose={closeEpisodeEditor}
      onRevert={revertEpisodeEditor}
      onSelectFile={selectEpisodeFile}
      onClearField={clearEpisodeField}
      onExpandField={(field) => (expandedPathField = field)}
    />
  {/if}

  <EpisodeMediaSettingsModal {mediaOverrideClass} noteTypeDirty={mediaNoteTypeId !== initialMediaNoteTypeId} onSave={saveEpisodeMediaSettings} onReset={resetEpisodeMediaSettings}>
    {#snippet noteTypeControl()}
      <label class="mb-4 block space-y-2 text-sm text-gray-300">
        <span>{t("settings.noteType")}</span>
        <SearchableSelect options={noteTypeOptions.map(option => option.value === "default" && episodeMediaEditorStore.episode ? { ...option, label: `${t("flashcards.automaticNoteType")} · ${getEpisodeNoteType({ ...episodeMediaEditorStore.episode, noteTypeId: undefined }).name}`, displayLabel: getEpisodeNoteType({ ...episodeMediaEditorStore.episode, noteTypeId: undefined }).name } : option)} value={mediaNoteTypeId} onchange={(id) => mediaNoteTypeId = id} ariaLabel={t("settings.noteType")} placeholder={t("settings.noteType")} noResultsText={t("common.noResults")} />
      </label>
    {/snippet}
  </EpisodeMediaSettingsModal>

  <PathPreviewModal
    isOpen={!!expandedPathField}
    title={expandedPathField === "targetSubs"
      || expandedPathField === "targetSubsPath"
      ? t("flashcards.targetLangSubs")
      : expandedPathField === "output"
        ? t("flashcards.outputDir")
        : expandedPathField === "nativeSubs"
          || expandedPathField === "nativeSubsPath"
          ? t("flashcards.nativeLangSubs")
          : t("flashcards.mediaFile")}
    value={expandedPathField === "targetSubs" || expandedPathField === "targetSubsPath"
      ? editingEpisode?.targetSubsPath || episodes[0]?.targetSubsPath || ""
      : expandedPathField === "output"
        ? outputDir
        : expandedPathField === "nativeSubs" || expandedPathField === "nativeSubsPath"
          ? editingEpisode?.nativeSubsPath || episodes[0]?.nativeSubsPath || ""
          : editingEpisode?.mediaPath || episodes[0]?.mediaPath || ""}
    onclose={() => (expandedPathField = null)}
  />
  </div>

  <!-- Fixed Bottom Band with Action Buttons -->
  <FlashcardsFooterBar
    {easyMode}
    {activeNoteType}
    {noteTypeOptions}
    {automaticNoteType}
    {selectedNoteTypeId}
    {selectNoteType}
    {episodes}
    {isApkgSwitchEnabled}
    {canRunFlashcards}
    {generationPromptOpen}
    {generationRequirements}
    {generationRequirementsText}
    onOpenBottomContextMenu={openBottomContextMenu}
    {onGoToSettings}
    onPreview={loadPreview}
    onStartGeneration={startGeneration}
    onCancelGeneration={cancelGeneration}
    onResetGeneration={resetGeneration}
    onPromptMissingRequirements={promptMissingGenerationRequirements}
    onCloseGenerationPrompt={closeGenerationPrompt}
  />
</div>

<style>
  :global(.media-override-glow) {
    border-color: rgba(167, 139, 250, 0.72) !important;
    box-shadow: 0 0 0 1px rgba(167, 139, 250, 0.28), 0 0 18px rgba(167, 139, 250, 0.22) !important;
  }
  :global(.media-override-glow .searchable-select-input) {
    border-color: rgba(167, 139, 250, 0.72) !important;
    box-shadow: 0 0 0 1px rgba(167, 139, 250, 0.28), 0 0 18px rgba(167, 139, 250, 0.22) !important;
  }
</style>
