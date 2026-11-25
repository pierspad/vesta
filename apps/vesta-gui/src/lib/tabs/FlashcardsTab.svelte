<script lang="ts">
  import { FlashcardAudioSelection } from "$lib/workflows/flashcardAudioSelection.svelte";
  import { RequirementFeedback } from "$lib/utils/requirementFeedback.svelte";
  import { expandFlashcardFiles } from "$lib/workflows/flashcardFileDiscovery";
  import { runFlashcardSeries } from "$lib/workflows/flashcardSeries";
  import { FlashcardPreviewController } from "$lib/workflows/flashcardPreview";
  import { invokeCommand as invoke } from "$lib/services/tauriClient";
  import { guardedOpen, guardedSave } from "$lib/utils/dialogGuard";
  import { join } from "@tauri-apps/api/path";
  import { setupWebviewDragDrop } from "$lib/utils/dragDrop";
  import { resolveEpisodeNoteType } from "$lib/utils/episodeNoteType";
  import { onDestroy, onMount, untrack } from "svelte";
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import {
    type EpisodeEntry,
    VIDEO_EXTENSIONS,
    AUDIO_EXTENSIONS,
    deriveDeckNameFromFile,
    mergeSeriesDroppedFiles,
    isSubtitleFile,
    isMediaFile,
    detectMediaType,
    pickBestAudioTrackIndex,
  } from "$lib/utils/seriesFileMatching";
  import {
    listNoteTypes,
    loadLastUsedNoteTypeId,
    saveActiveNoteTypeId,
    ACTIVE_NOTE_TYPE_CHANGED_EVENT,
    type NoteTypeDef,
  } from "$lib/types/noteTypes";
  import { snackbar, createSnackbarNotifier } from "$lib/stores/snackbarStore.svelte";
  import { smartMatchingStore } from "$lib/stores/smartMatchingStore.svelte";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import { ankiStore } from "$lib/stores/ankiStore.svelte";
  import { fontStore } from "$lib/stores/fontStore.svelte";
  import FlashcardsPreviewModal from "$lib/modals/FlashcardsPreviewModal.svelte";
  import { previewStore } from "$lib/stores/previewStore.svelte";
  import type { EpisodeMediaOverrides, AudioTrackInfo, EpisodeMediaOverrideKey } from "$lib/types/flashcardMediaTypes";
  import {
    loadMediaSettings,
    saveMediaSettings,
    type MediaSettings,
  } from "$lib/utils/mediaSettings";
  import type { CardFilterSettings } from "$lib/types/flashcardFilterTypes";
  import CardFiltersPanel from "$lib/panels/CardFiltersPanel.svelte";
  import { episodeMediaEditorStore } from "$lib/stores/episodeMediaEditorStore.svelte";
  import AudioClipsPanel from "$lib/panels/AudioClipsPanel.svelte";
  import SnapshotsPanel from "$lib/panels/SnapshotsPanel.svelte";
  import VideoClipsPanel from "$lib/panels/VideoClipsPanel.svelte";
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
  import { type EpisodeFileField } from "./flashcards/EpisodeEditorModal.svelte";
  import FlashcardsFooterBar, { type GenerationRequirement } from "./flashcards/FlashcardsFooterBar.svelte";
  import FlashcardsBanners from "./flashcards/FlashcardsBanners.svelte";
  import FlashcardsOverlays from "./flashcards/FlashcardsOverlays.svelte";
  import { handleFlashcardsKeydown } from "./flashcards/flashcardsKeyboard";
  import {
    buildFlashcardNoteTypeOptions,
    type CardFilterTracker,
    computeBottomContextMenuPos,
    enforceCardFilterCoherence,
    isEpisodeFileCompatible,
    mergePluginHandoffEpisode,
    normalizeSelectedPaths,
  } from "./flashcards/flashcardsHelpers";
  import {
    OUTPUT_DIR_KEY,
    buildFirstEpisodeFlashcardConfig,
    loadInitialFlashcardsOutputDir,
    loadInitialFlashcardsCpuCores,
    maybeAutoImportApkgToAnki,
    setupFlashcardsWindowSubscriptions,
    subscribeFlashcardProgressEvents,
  } from "./flashcards/flashcardsRunner";
  import {
    buildEpisodeMediaOverrideDiffFor,
    clearEpisodesMediaOverrides,
    detectEpisodeLanguageCached,
    isEpisodeMediaOverrideChanged,
    isFontBannerDismissedPref,
    loadDefaultLanguagePref,
    mergeEditedEpisodeIntoList,
    persistFontBannerDismissal,
    removeEpisodeInList,
    swapAllEpisodesSubsInList,
    swapEpisodeSubsInList,
    updateEpisodesMediaOverrides,
  } from "./flashcards/flashcardsEpisodes";

  interface Props {
    active?: boolean;
    onGoToSettings?: (section?: "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts", highlightItemId?: string) => void;
    pluginHandoff?: { nonce: number; videoPath: string; subtitlePath: string } | null;
  }

  let { active = true, onGoToSettings, pluginHandoff = null }: Props = $props();

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

  const audioSelection = new FlashcardAudioSelection(
    path => invoke<AudioTrackInfo[]>("flashcard_list_audio_tracks", { path }),
    () => getPreferredAudioLanguageCode(),
    index => { mediaSettings.audioTrackIndex = index; },
    error => generationStore.addLog(`${t("flashcards.audioTracksError")}: ${errorMessage(error)}`, "warning"),
  );

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
    bottomContextMenu = computeBottomContextMenuPos(event, section);
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

  function getStudiedLanguagePreference(): string {
    const episode = episodes.find(ep => ep.targetSubsPath);
    return (episode ? getEpisodeNoteType(episode).language : "") || noteTypeLanguage || loadDefaultLanguagePref(DEFAULT_FLASHCARDS_LANGUAGE_KEY);
  }

  function isFontBannerDismissed(lang: string): boolean {
    return isFontBannerDismissedPref(lang, dismissedFontBannerLang);
  }

  function dismissFontBanner(lang: string, permanent: boolean = false) {
    dismissedFontBannerLang = lang;
    if (permanent && persistFontBannerDismissal(lang)) {
      showSnackbar(t("flashcards.dontShowAgain"));
    }
  }

  let proactiveLang = $derived(getStudiedLanguagePreference());
  let proactiveFont = $derived(fontStore.getFontForLanguage(proactiveLang));

  function getNativeLanguagePreference(): string {
    return (
      loadDefaultLanguagePref(DEFAULT_NATIVE_LANGUAGE_KEY) ||
      loadDefaultLanguagePref(DEFAULT_TARGET_LANGUAGE_KEY, "it")
    );
  }

  function getEpisodeMediaSettings(ep: EpisodeEntry): Required<EpisodeMediaOverrides> {
    return { ...mediaSettings, ...(ep.mediaOverrides || {}) };
  }

  function mediaOverrideClass(key: EpisodeMediaOverrideKey): string {
    const episode = episodeMediaEditorStore.episode;
    return isEpisodeMediaOverrideChanged(
      key,
      episodeMediaEditorStore.overrides,
      episode,
      mediaSettings,
      episodeMediaEditorStore.audioTracks,
      episode ? getPreferredAudioLanguageCodeForEpisode(episode) : "",
    )
      ? "media-override-glow"
      : "";
  }

  function getPreferredAudioLanguageCodeForEpisode(ep: Pick<EpisodeEntry, "targetSubsPath" | "noteTypeId" | "detectedLanguage">): string {
    return getEpisodeNoteType(ep).language || noteTypeLanguage;
  }

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

  function expandSeriesFilesWithSmartMatches(subtitleFiles: string[], mediaFiles: string[]) {
    return expandFlashcardFiles(subtitleFiles, mediaFiles, {
      enabled: smartFileMatchingEnabled,
      rules: smartMatchingRules,
      targetLanguage: getStudiedLanguagePreference(),
      nativeLanguage: getNativeLanguagePreference(),
      invoke,
    });
  }

  async function addSeriesMultipleFiles() {
    try {
      const raw = await guardedOpen({
        multiple: true,
        filters: [
          {
            name: `SRT / ${t("flashcards.mediaFiles")}`,
            extensions: ["srt", "ass", "ssa", "vtt", ...VIDEO_EXTENSIONS, ...AUDIO_EXTENSIONS],
          },
        ],
      });
      const selected = normalizeSelectedPaths(raw);
      if (selected.length > 0) {
        await handleFileDrop(selected);
      }
    } catch (e) {
      generationStore.error = `${t("flashcards.errorSelectingFile")}: ${errorMessage(e)}`;
    }
  }



  function removeEpisode(idx: number) {
    episodes = removeEpisodeInList(episodes, idx);
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
    episodes = mergeEditedEpisodeIntoList(episodes, editingEpisodeIndex, editingEpisode);
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
    const episode = episodeMediaEditorStore.episode;
    const mediaOverrides = buildEpisodeMediaOverrideDiffFor(
      overrides,
      episode,
      mediaSettings,
      episodeMediaEditorStore.audioTracks,
      episode ? getPreferredAudioLanguageCodeForEpisode(episode) : "",
    );
    episodes = updateEpisodesMediaOverrides(episodes, idx, mediaOverrides, mediaNoteTypeId);
    if (mediaNoteTypeId !== "default" && mediaNoteTypeId !== initialMediaNoteTypeId) saveActiveNoteTypeId(mediaNoteTypeId);
    closeEpisodeMediaSettings();
  }

  function resetEpisodeMediaSettings() {
    const idx = episodeMediaEditorStore.episodeIndex;
    if (idx === null) return;
    episodes = clearEpisodesMediaOverrides(episodes, idx);
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
    episodes = swapEpisodeSubsInList(episodes, idx);
  }

  function swapAllEpisodesSubs() {
    episodes = swapAllEpisodesSubsInList(episodes);
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

  const cardFilterTracker: CardFilterTracker = {
    prevMinChars: undefined,
    prevMaxChars: undefined,
    prevMinDuration: undefined,
    prevMaxDuration: undefined,
  };

  $effect(() => {
    enforceCardFilterCoherence(cardFilters, cardFilterTracker);
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
    return detectEpisodeLanguageCached(path, languageRequests, (resolvedPath, language) => {
      subtitleLanguages[resolvedPath] = language;
    });
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
    buildFlashcardNoteTypeOptions(noteTypeList, automaticEpisodeNoteType, t),
  );

  function selectNoteType(id: string) {
    if (episodes.length === 1) setEpisodeNoteType(0, id);
    else saveActiveNoteTypeId(id);
  }

  // The automatic deck name and generation run-state (isProcessing,
  // progress, progressMessage, progressStage, logs, error, result) live in
  // generationStore now — see generationStore.svelte.ts.

  let ffmpegAvailable = $state<boolean | null>(null);

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
  let removeWindowSubscriptions: (() => void) | null = null;
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


  let loadedAudioMedia: string | undefined;

  $effect(() => { audioSelection.updateLanguage(); });

  $effect(() => {
    const firstMedia = episodes[0]?.mediaPath;
    if (firstMedia === loadedAudioMedia) return;
    loadedAudioMedia = firstMedia;
    untrack(() => { void audioSelection.load(firstMedia || ""); });
  });

  let lastAppliedHandoffNonce = 0;

  function applyPluginHandoff(handoff: { videoPath: string; subtitlePath: string }) {
    if (generationStore.isBusy) return;
    const merged = mergePluginHandoffEpisode(episodes, handoff);
    if (!merged) return;

    if (merged.mediaOrTargetChanged) {
      previewStore.close();
      if (editingEpisodeIndex === 0) closeEpisodeEditor();
      if (episodeMediaEditorStore.episodeIndex === 0) closeEpisodeMediaSettings();
      if (generationStore.result) {
        generationStore.result = null;
        generationStore.error = null;
      }
    }

    episodes = merged.nextEpisodes;

    if (
      !merged.mediaChanged &&
      merged.nextMediaType === "video" &&
      merged.nextMediaPath &&
      !audioSelection.loading &&
      audioSelection.tracks.length === 0
    ) {
      loadedAudioMedia = merged.nextMediaPath;
      void audioSelection.load(merged.nextMediaPath);
    }
  }

  $effect(() => {
    const handoff = pluginHandoff;
    if (!handoff || handoff.nonce === lastAppliedHandoffNonce) return;
    if (generationStore.isBusy) return;
    lastAppliedHandoffNonce = handoff.nonce;
    untrack(() => applyPluginHandoff(handoff));
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
    removeWindowSubscriptions = setupFlashcardsWindowSubscriptions({
      defaultLanguageKey: DEFAULT_FLASHCARDS_LANGUAGE_KEY,
      getLayoutHost: () => layoutHostEl,
      onNoteTypesChanged: refreshNoteTypes,
      onLanguageDefaultChanged: (code) => { noteTypeLanguage = code; },
      onLayoutWidthChanged: (width) => { layoutWidth = width; },
    });

    noteTypeLanguage = loadDefaultLanguagePref(DEFAULT_FLASHCARDS_LANGUAGE_KEY);
    outputDir = await loadInitialFlashcardsOutputDir();

    try {
      ffmpegAvailable = await invoke<boolean>("flashcard_check_deps");
    } catch {
      ffmpegAvailable = false;
    }

    await loadInitialFlashcardsCpuCores();

    // This async onMount can resume after its component has been destroyed.
    // Keep the progress subscription's late-resolution guard consistent with
    // these window listeners so teardown cannot be followed by a fresh leak.
    if (activeListener) {
      window.addEventListener("vesta-cpu-cores-changed", handleCpuCoresChanged);
      window.addEventListener("vesta-ffmpeg-updated", handleFfmpegUpdated);
    }

    subscribeFlashcardProgressEvents({
      isActive: () => activeListener,
      isCancelled: () => cancelRequested,
      getSeriesProgress: () => ({ current: seriesCurrentEpisode, total: seriesTotalEpisodes }),
      translate: (key, params) => t(key, params),
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(console.error);
  });

  onDestroy(() => {
    flashcardPreview.dispose();
    audioSelection.dispose();
    activeListener = false;
    window.removeEventListener("vesta-cpu-cores-changed", handleCpuCoresChanged);
    window.removeEventListener("vesta-ffmpeg-updated", handleFfmpegUpdated);
    unlisten?.();
    removeWindowSubscriptions?.();
    requirementFeedback.clear();
  });

  async function maybeAutoImportToAnki(apkgPath: string) {
    await maybeAutoImportApkgToAnki(apkgPath, t);
  }

  function buildConfig() {
    return buildFirstEpisodeFlashcardConfig({
      ep: episodes[0],
      outputDir,
      cardFilters,
      mediaSettings,
      videoHwAccel,
      activeNoteType,
      getEpisodeNoteType,
      targetLanguage: getStudiedLanguagePreference(),
    });
  }

  const flashcardPreview = new FlashcardPreviewController({
    isBusy: () => generationStore.isBusy,
    canPreview: () => canRunFlashcards,
    isLoading: () => previewStore.loading,
    buildConfig,
    load: (config, onLoaded) => previewStore.load(config, onLoaded),
    close: () => previewStore.close(),
    setError: (message) => {
      generationStore.error = message;
      if (message) snackbar.show(message, "error");
    },
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

  let exportingDueling = $state(false);

  async function exportDuelingForEpisode(ep: EpisodeEntry | undefined, fromPreview = false) {
    if (!ep?.targetSubsPath || !ep?.nativeSubsPath || exportingDueling) return;
    const baseName = (deriveDeckNameFromFile(ep) || "dueling").replace(/[\\/:*?"<>|]/g, "_");
    const defaultFilename = `${baseName}.dueling.ass`;
    const defaultPath = outputDir ? await join(outputDir, defaultFilename) : defaultFilename;
    const selected = await guardedSave({
      defaultPath,
      filters: [
        { name: "ASS Subtitles", extensions: ["ass"] },
        { name: "SRT Subtitles", extensions: ["srt"] },
      ],
    });
    if (!selected) return;

    const format = selected.toLowerCase().endsWith(".srt") ? "srt" : "ass";
    const config = buildFirstEpisodeFlashcardConfig({
      ep,
      outputDir,
      cardFilters: $state.snapshot(cardFilters),
      mediaSettings: { ...$state.snapshot(mediaSettings), ...ep.mediaOverrides },
      videoHwAccel,
      activeNoteType,
      getEpisodeNoteType,
      targetLanguage: getStudiedLanguagePreference(),
    });
    if (fromPreview) {
      await previewStore.applyOverrides(config);
    }

    exportingDueling = true;
    try {
      const result = await invoke<{
        output_path: string;
        format: "ass" | "srt";
        line_count: number;
        matched_count: number;
      }>("flashcard_export_dueling", {
        config,
        options: {
          format,
          output_path: selected,
          target_first: true,
          include_unmatched: true,
        },
      });
      const msg = t("flashcards.duelingExported", {
        count: result.line_count,
        file: getFileName(result.output_path),
      });
      generationStore.addLog(msg, "output", result.output_path);
      snackbar.show(msg, "success");
    } catch (e) {
      const msg = errorMessage(e);
      generationStore.error = msg;
      snackbar.show(msg, "error");
    } finally {
      exportingDueling = false;
    }
  }

  let cancelRequested = false;

  async function startSeriesGeneration() {
    await runFlashcardSeries({
      generationStore, episodes: $state.snapshot(episodes).map(ep => ({ ...ep, detectedLanguage: subtitleLanguages[ep.targetSubsPath] || ep.detectedLanguage })), t,
      getEpisodeNoteType,
      outputDir, cardFilters: $state.snapshot(cardFilters), videoHwAccel, activeNoteType, automaticNoteType,
      ankiStore: { dictionaryLookup: ankiStore.dictionaryLookup, autoCardFont: ankiStore.autoCardFont, embedCardFont: ankiStore.embedCardFont },
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
      generationStore.addLog(`${t("sync.cancelFailed")} — ${errorMessage(e)}`, "error");
      snackbar.show(errorMessage(e), "error");
    }
  }

  function resetGeneration() {
    if (generationStore.isBusy) return;
    generationStore.resetRun();

    // Clear files so the user can insert new ones
    audioSelection.reset();
    episodes = [];
    generationStore.deckName = "";
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    handleFlashcardsKeydown(e, {
      active,
      bottomContextMenuOpen: !!bottomContextMenu,
      episodeContextMenuIdx: episodeContextMenu?.idx ?? null,
      episodeHasMedia: episodeContextMenu ? Boolean(episodes[episodeContextMenu.idx]?.mediaPath) : false,
      previewVisible: previewStore.visible,
      canRunFlashcards,
      isBusy: generationStore.isBusy,
      isPreviewLoading: previewStore.loading,
      onCloseBottomMenu: closeBottomContextMenu,
      onCloseEpisodeMenu: closeEpisodeContextMenu,
      onEditContextEpisode: () => { void editContextEpisode(); },
      onOpenEpisodeMediaSettings: openEpisodeMediaSettings,
      onRemoveEpisode: removeEpisode,
      onStartGeneration: () => { void startGeneration(); },
      onCancelGeneration: () => { void cancelGeneration(); },
      onLoadPreview: () => { void loadPreview(); },
    });
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
  <div class="flex-1 overflow-hidden overflow-x-hidden p-6 flashcards-scroll min-h-0 flex flex-col {generationStore.isBusy ? 'pointer-events-none opacity-60 select-none' : ''}">
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
  <FlashcardsBanners
    {ffmpegAvailable}
    {hasAnyFiles}
    {proactiveLang}
    {proactiveFont}
    {isFontBannerDismissed}
    onFfmpegInstalled={(ok) => (ffmpegAvailable = ok)}
    onDismissFontBanner={dismissFontBanner}
  />

  {#if previewStore.visible}
    <FlashcardsPreviewModal
      mediaPath={previewMediaPath}
      mediaType={previewMediaType}
      nativeSubsPath={episodes[0]?.nativeSubsPath || ""}
      {active}
      {exportingDueling}
      onExportDueling={() => void exportDuelingForEpisode(episodes[0], true)}
    />
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
        {active}
        {hasAudio}
        mediaType={previewMediaType}
        audioTracks={audioSelection.tracks}
        audioTracksLoading={audioSelection.loading}
        hintLoadMediaFirst={HINT_LOAD_MEDIA_FIRST}
        firstEpisodeMediaPath={episodes[0]?.mediaPath || ""}
        firstEpisodeSubsPath={episodes[0]?.targetSubsPath || ""}
        {episodes}
        onTrackPicked={() => (audioSelection.automatic = false)}
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

  <FlashcardsOverlays
    {episodes}
    {outputDir}
    {episodeContextMenu}
    {bottomContextMenu}
    {inspectedMatch}
    {inspectedMatchPath}
    {editingEpisode}
    {editingEpisodeIndex}
    {editingEpisodeField}
    {expandedPathField}
    {noteTypeOptions}
    {activeNoteType}
    {dropMediaHint}
    {dropSubtitleHint}
    generateAudio={mediaSettings.generateAudio}
    bind:mediaNoteTypeId
    {initialMediaNoteTypeId}
    {getEpisodeNoteType}
    {mediaOverrideClass}
    onCloseEpisodeContextMenu={closeEpisodeContextMenu}
    onInspectMatch={(ep, path) => { inspectedMatch = ep; inspectedMatchPath = path; }}
    onCloseInspectMatch={() => (inspectedMatch = null)}
    onEditContextEpisode={editContextEpisode}
    onOpenEpisodeMediaSettings={openEpisodeMediaSettings}
    onPreviewEpisodeAudio={previewEpisodeAudio}
    onExportDuelingEpisode={(idx) => void exportDuelingForEpisode(episodes[idx], false)}
    onRemoveEpisode={removeEpisode}
    onCloseBottomContextMenu={closeBottomContextMenu}
    {onGoToSettings}
    onUpdateEditingNoteType={(id) => {
      if (editingEpisode) {
        editingEpisode.noteTypeId = id === "default" ? undefined : id;
        syncEpisodeEditor();
      }
    }}
    onCloseEpisodeEditor={closeEpisodeEditor}
    onRevertEpisodeEditor={revertEpisodeEditor}
    onSelectEpisodeFile={selectEpisodeFile}
    onClearEpisodeField={clearEpisodeField}
    onExpandPathField={(field) => (expandedPathField = field)}
    onSaveEpisodeMediaSettings={saveEpisodeMediaSettings}
    onResetEpisodeMediaSettings={resetEpisodeMediaSettings}
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
