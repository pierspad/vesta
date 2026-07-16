<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { guardedOpen } from "./utils/dialogGuard";
  import { onDestroy, onMount } from "svelte";
  import { locale, currentLanguage } from "./i18n";
  import { getFileName, inferLanguageFromPath } from "./models";
  import { detectLanguageCode, getLanguageSearchTerms, languages, scoreLanguageMatch } from "./languages";
  import {
    CARD_TEMPLATES_UPDATED_EVENT,
    NOTE_TYPES_UPDATED_EVENT,
    NOTE_TYPE_FIELD_ORDER,
    loadCardTemplates,
    listNoteTypes,
    findNoteTypeById,
    noteTypeOutputFields,
    loadActiveNoteTypeId,
    saveActiveNoteTypeId,
    ACTIVE_NOTE_TYPE_CHANGED_EVENT,
    type NoteTypeDef,
    type FieldKey,
  } from "./noteTypes";
  import PathPreviewModal from "./PathPreviewModal.svelte";
  import SearchableSelect from "./SearchableSelect.svelte";
  import LogPanel from "./LogPanel.svelte";
  import CodeEditor from "./CodeEditor.svelte";
  import { snackbar } from "./snackbarStore.svelte";
  import { smartMatchingStore } from "./smartMatchingStore.svelte";
  import { uiMode } from "./uiModeStore.svelte";
  import { ankiStore } from "./ankiStore.svelte";
  import FooterActions from "./components/FooterActions.svelte";
  import EmptyState from "./components/EmptyState.svelte";
  import FlashcardsPreviewModal from "./FlashcardsPreviewModal.svelte";
  import { previewStore } from "./previewStore.svelte";
  import type { EpisodeMediaOverrides, AudioTrackInfo, EpisodeMediaOverrideKey } from "./flashcardMediaTypes";
  import { formatAudioTrackLabel } from "./flashcardMediaTypes";
  import type { CardFilterSettings } from "./flashcardFilterTypes";
  import CardFiltersPanel from "./CardFiltersPanel.svelte";
  import { episodeMediaEditorStore } from "./episodeMediaEditorStore.svelte";
  import EpisodeMediaSettingsModal from "./EpisodeMediaSettingsModal.svelte";
  import AudioClipsPanel from "./AudioClipsPanel.svelte";
  import SnapshotsPanel from "./SnapshotsPanel.svelte";
  import VideoClipsPanel from "./VideoClipsPanel.svelte";
  import EpisodeContextMenu from "./EpisodeContextMenu.svelte";
  import FilesOutputPanel from "./FilesOutputPanel.svelte";
  import { generationStore, EXPORT_FORMAT_KEY, SERIES_OUTPUT_MODE_KEY } from "./generationStore.svelte";
  import GenerationStatusDisplay from "./GenerationStatusDisplay.svelte";

  const SUBTITLE_EXTENSIONS = ["srt", "ass", "ssa", "vtt"];

  interface Props {
    active?: boolean;
    onGoToSettings?: (section?: "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts", highlightItemId?: string) => void;
  }

  let { active = true, onGoToSettings }: Props = $props();

  let t = $derived($locale);

  let targetSubsPath = $state("");
  let nativeSubsPath = $state("");
  let mediaPath = $state("");
  let mediaType = $state<"none" | "video" | "audio">("none");
  let outputDir = $state("");

  // AudioTrackInfo / EpisodeMediaOverrides live in flashcardMediaTypes.ts so
  // the extracted media-settings panel components can import them too.

  let audioTracks = $state<AudioTrackInfo[]>([]);
  let audioTrackAutoSelected = $state(true);
  let audioTracksLoading = $state(false);

  const OUTPUT_DIR_KEY = "vesta-last-output-dir";
  const NOTE_TYPE_LANGUAGE_KEY = "vesta-flashcards-note-type-language";
  const DEFAULT_FLASHCARDS_LANGUAGE_KEY = "vesta-default-flashcards-language";
  const DEFAULT_NATIVE_LANGUAGE_KEY = "vesta-default-native-language";
  const DEFAULT_TARGET_LANGUAGE_KEY = "vesta-default-target-language";
  const SERIES_MODE_KEY = "vesta-flashcards-series-mode";
  const SMART_MATCHING_RULES_KEY = "vesta-flashcards-smart-matching-rules";
  const ANKI_FIELDS_PANEL_OPEN_KEY = "vesta-flashcards-anki-fields-panel-open";
  const FLASHCARD_MEDIA_WIDTH_KEY = "vesta-flashcards-media-width";
  const FLASHCARD_MEDIA_HEIGHT_KEY = "vesta-flashcards-media-height";
  const DEFAULT_FLASHCARD_MEDIA_WIDTH = 240;
  const DEFAULT_FLASHCARD_MEDIA_HEIGHT = 160;

  let smartFileMatchingEnabled = $derived(uiMode.easyMode || smartMatchingStore.enabled);



  function escapeRegExp(value: string): string {
    return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  let smartMatchingRules = $derived(smartMatchingStore.rules);
  let episodeContextMenu = $state<{ x: number; y: number; idx: number } | null>(null);
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
  let seriesMode = $state(loadSeriesMode());

  function loadSeriesMode(): boolean {
    try {
      return localStorage.getItem(SERIES_MODE_KEY) === "true";
    } catch {
      return false;
    }
  }

  function toggleSeriesMode() {
    seriesMode = !seriesMode;
    localStorage.setItem(SERIES_MODE_KEY, String(seriesMode));
  }

  // Episode data for series mode
  interface EpisodeEntry {
    id: number;
    targetSubsPath: string;
    nativeSubsPath: string;
    mediaPath: string;
    mediaType: "none" | "video" | "audio";
    mediaOverrides?: EpisodeMediaOverrides;
  }

  type EpisodeFileField = "targetSubsPath" | "nativeSubsPath" | "mediaPath";

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

  let episodes = $state<EpisodeEntry[]>([]);
  let seriesCurrentEpisode = $state(0);
  let seriesTotalEpisodes = $state(0);
  let editingEpisodeIndex = $state<number | null>(null);
  let editingEpisode = $state<EpisodeEntry | null>(null);
  let initialEditingEpisodeStr = $state("");
  function showSnackbar(message: string, variant: "success" | "info" | "warning" | "error" = "info") {
    snackbar.show(message, variant, 1300);
  }

  // Extract episode number from filename using the editable smart matching patterns.
  function extractEpisodeNumber(filename: string): number | null {
    const base = filename.replace(/\.[^/.]+$/, "");
    for (const pattern of smartMatchingRules.episodeRegexes) {
      try {
        const match = base.match(new RegExp(pattern, "i"));
        const rawEpisode = match?.[1] ?? match?.[0];
        const numericEpisode = rawEpisode?.match(/\d{1,4}/)?.[0];
        if (numericEpisode) return parseInt(numericEpisode, 10);
      } catch {
        // Invalid custom regexes are blocked on save; ignore stale stored values defensively.
      }
    }
    return null;
  }

  const KNOWN_LANGUAGE_CODES = new Set(languages.map((lang) => lang.code.toLowerCase()));

  function delimitedHintRegex(hints: string[], flags = "i") {
    if (hints.length === 0) return null;
    return new RegExp(`(^|[._-])(${hints.map(escapeRegExp).join("|")})(?=($|[._-]))`, flags);
  }

  function removableTokenRegex(tokens: string[]) {
    if (tokens.length === 0) return null;
    return new RegExp(`\\b(?:${tokens.map(escapeRegExp).join("|")})\\b`, "gi");
  }

  function loadDefaultLanguage(key: string, fallback = ""): string {
    try {
      return localStorage.getItem(key) || fallback;
    } catch {
      return fallback;
    }
  }

  function loadStoredDimension(key: string, fallback: number): number {
    try {
      const value = Number.parseInt(localStorage.getItem(key) || "", 10);
      return Number.isFinite(value) && value > 0 ? value : fallback;
    } catch {
      return fallback;
    }
  }

  function persistDimension(key: string, value: number) {
    if (!Number.isFinite(value) || value <= 0) return;
    localStorage.setItem(key, String(Math.round(value)));
  }

  function getStudiedLanguagePreference(): string {
    return noteTypeLanguage || loadDefaultLanguage(DEFAULT_FLASHCARDS_LANGUAGE_KEY);
  }

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

  const audioOverrideKeys: EpisodeMediaOverrideKey[] = [
    "generateAudio",
    "audioBitrate",
    "audioTrackIndex",
    "normalizeAudio",
    "audioPadStart",
    "audioPadEnd",
  ];
  const snapshotOverrideKeys: EpisodeMediaOverrideKey[] = [
    "generateSnapshots",
    "snapshotWidth",
    "snapshotHeight",
    "cropBottom",
  ];
  const videoOverrideKeys: EpisodeMediaOverrideKey[] = [
    "generateVideoClips",
    "videoCodec",
    "h264Preset",
    "videoBitrate",
    "videoAudioBitrate",
    "videoPadStart",
    "videoPadEnd",
  ];

  function mediaOverrideValueChanged(key: EpisodeMediaOverrideKey): boolean {
    const overrides = episodeMediaEditorStore.overrides;
    const episode = episodeMediaEditorStore.episode;
    if (!overrides) return false;
    const genericSettings = getGenericMediaSettings();

    if (key === "audioTrackIndex" && episode && genericSettings.audioTrackIndex === null) {
      const autoPicked = pickBestAudioTrackIndex(
        episodeMediaEditorStore.audioTracks,
        getPreferredAudioLanguageCodeForEpisode(episode)
      );
      if (overrides.audioTrackIndex === autoPicked) {
        return false;
      }
    }

    return overrides[key] !== genericSettings[key];
  }

  function mediaOverrideClass(key: EpisodeMediaOverrideKey): string {
    return mediaOverrideValueChanged(key)
      ? "media-override-glow"
      : "";
  }

  function buildEpisodeMediaOverrideDiff(settings: Required<EpisodeMediaOverrides>): EpisodeMediaOverrides {
    const diff: EpisodeMediaOverrides = {};
    ([
      ...audioOverrideKeys,
      ...snapshotOverrideKeys,
      ...videoOverrideKeys,
    ] as EpisodeMediaOverrideKey[]).forEach((key) => {
      if (mediaOverrideValueChanged(key)) {
        diff[key] = settings[key] as never;
      }
    });
    return diff;
  }

  function getPreferredAudioLanguageCodeForEpisode(ep: { targetSubsPath: string }): string {
    return inferLanguageFromPath(ep.targetSubsPath) || noteTypeLanguage;
  }

  interface ParsedSeriesSubtitle {
    path: string;
    name: string;
    baseKey: string;
    language: string | null;
    roleHint: "original" | "reference" | "unknown";
    episodeNumber: number | null;
  }

  interface SeriesDraftEntry {
    baseKey: string;
    displayName: string;
    targetSubsPath: string;
    nativeSubsPath: string;
    mediaPath: string;
    mediaType: "none" | "video" | "audio";
    episodeNumber: number | null;
    mediaOverrides?: EpisodeMediaOverrides;
  }

  function normalizeSeriesBaseKey(baseName: string): string {
    let stem = baseName.toLowerCase();
    stem = stem.replace(/\([^)]*\b(?:19|20)\d{2}\b[^)]*\)/g, "");
    stem = stem.replace(/\[[^\]]*\b(?:19|20)\d{2}\b[^\]]*\]/g, "");
    stem = stem.replace(/\b(?:19|20)\d{2}\b/g, "");
    const tokenRegex = removableTokenRegex(smartMatchingRules.removableNameTokens);
    if (tokenRegex) stem = stem.replace(tokenRegex, "");
    stem = stem.replace(/[\s]+/g, " ");
    const roleHintRegex = delimitedHintRegex([
      ...smartMatchingRules.originalSubtitleHints,
      ...smartMatchingRules.referenceSubtitleHints,
    ], "gi");
    if (roleHintRegex) stem = stem.replace(roleHintRegex, "$1");

    const suffixParts = stem
      .split(/[._-]+/)
      .filter((part) => {
        if (!part) return false;
        if (KNOWN_LANGUAGE_CODES.has(part)) return false;
        return !detectLanguageCode(part);
      });
    // Normalize all separators to underscore for consistent matching
    stem = suffixParts.join("_");

    return stem
      .replace(/[^\p{L}\p{N}]+/gu, "_")
      .replace(/^_+|_+$/g, "")
      .replace(/_+/g, "_")
      .trim();
  }

  function stripCompoundSubtitleSuffix(baseName: string): string {
    return normalizeSeriesBaseKey(baseName);
  }

  function parseSeriesSubtitle(path: string): ParsedSeriesSubtitle {
    const name = getFileName(path);
    const baseName = name.replace(/\.[^/.]+$/, "");
    const normalized = baseName.toLowerCase();
    const language = inferLanguageFromPath(path);
    const originalHintRegex = delimitedHintRegex(smartMatchingRules.originalSubtitleHints);
    const referenceHintRegex = delimitedHintRegex(smartMatchingRules.referenceSubtitleHints);
    const roleHint = originalHintRegex?.test(normalized)
      ? "original"
      : referenceHintRegex?.test(normalized)
        ? "reference"
        : "unknown";

    return {
      path,
      name,
      baseKey: stripCompoundSubtitleSuffix(baseName) || normalized,
      language,
      roleHint,
      episodeNumber: extractEpisodeNumber(name),
    };
  }

  function parseSeriesMedia(path: string) {
    const name = getFileName(path);
    const baseName = name.replace(/\.[^/.]+$/, "");
    return {
      path,
      name,
      baseKey: normalizeSeriesBaseKey(baseName) || baseName.toLowerCase(),
      mediaType: detectMediaType(name),
      episodeNumber: extractEpisodeNumber(name),
    };
  }

  function classifySubtitleCandidates(
    paths: string[],
    preferredRole: "target" | "native" | "auto" = "auto",
  ): { target: string; native: string } {
    if (paths.length === 0) return { target: "", native: "" };

    const parsed = paths
      .map(parseSeriesSubtitle)
      .sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }));

    if (paths.length === 1) {
      return preferredRole === "native"
        ? { target: "", native: paths[0] }
        : { target: paths[0], native: "" };
    }

    const studiedLanguage = getStudiedLanguagePreference();
    const nativeLanguage = getNativeLanguagePreference();
    const byStudiedLanguage = studiedLanguage
      ? parsed.find((item) => item.language === studiedLanguage)
      : null;
    const byNativeLanguage = nativeLanguage
      ? parsed.find((item) => item.language === nativeLanguage)
      : null;

    let targetCandidate =
      (preferredRole === "auto" ? byStudiedLanguage : null) ||
      parsed.find((item) => item.roleHint === "original") ||
      parsed.find((item) => item.path !== byNativeLanguage?.path) ||
      parsed[0];

    let nativeCandidate =
      (preferredRole === "auto" && byNativeLanguage?.path !== targetCandidate.path
        ? byNativeLanguage
        : null) ||
      parsed.find(
        (item) =>
          item.path !== targetCandidate.path && item.roleHint === "reference",
      ) ||
      parsed.find(
        (item) =>
          item.path !== targetCandidate.path &&
          item.language &&
          item.language !== targetCandidate.language,
      ) ||
      parsed.find((item) => item.path !== targetCandidate.path) ||
      null;

    if (preferredRole === "native" && paths.length === 1) {
      targetCandidate = { ...targetCandidate, path: "" };
    }

    return {
      target: targetCandidate.path,
      native: nativeCandidate?.path || "",
    };
  }

  function buildSeriesDraftMap(): Map<string, SeriesDraftEntry> {
    const map = new Map<string, SeriesDraftEntry>();

    episodes.forEach((episode) => {
      const baseKey =
        episode.targetSubsPath
          ? parseSeriesSubtitle(episode.targetSubsPath).baseKey
          : episode.nativeSubsPath
            ? parseSeriesSubtitle(episode.nativeSubsPath).baseKey
            : episode.mediaPath
              ? parseSeriesMedia(episode.mediaPath).baseKey
              : `episode-${episode.id}`;

      map.set(baseKey, {
        baseKey,
        displayName:
          getFileName(episode.targetSubsPath || episode.nativeSubsPath || episode.mediaPath) ||
          baseKey,
        targetSubsPath: episode.targetSubsPath,
        nativeSubsPath: episode.nativeSubsPath,
        mediaPath: episode.mediaPath,
        mediaType: episode.mediaType,
        mediaOverrides: episode.mediaOverrides,
        episodeNumber:
          extractEpisodeNumber(
            getFileName(
              episode.targetSubsPath || episode.nativeSubsPath || episode.mediaPath,
            ),
          ) || null,
      });
    });

    return map;
  }

  function seriesDraftMapToEpisodes(draftMap: Map<string, SeriesDraftEntry>) {
    const sortedEntries = [...draftMap.values()].sort((a, b) => {
      const aEpisode = a.episodeNumber ?? Number.MAX_SAFE_INTEGER;
      const bEpisode = b.episodeNumber ?? Number.MAX_SAFE_INTEGER;
      if (aEpisode !== bEpisode) return aEpisode - bEpisode;
      return a.displayName.localeCompare(b.displayName, undefined, {
        numeric: true,
      });
    });

    episodes = sortedEntries.map((entry, index) => ({
      id: index + 1,
      targetSubsPath: entry.targetSubsPath,
      nativeSubsPath: entry.nativeSubsPath,
      mediaPath: entry.mediaPath,
      mediaType: entry.mediaType,
      mediaOverrides: entry.mediaOverrides,
    }));
  }

  function mergeSeriesSubtitleFiles(
    subtitleFiles: string[],
    preferredRole: "target" | "native" | "auto",
  ) {
    const draftMap = buildSeriesDraftMap();
    const grouped = new Map<string, string[]>();

    subtitleFiles.forEach((path) => {
      const parsed = parseSeriesSubtitle(path);
      const group = grouped.get(parsed.baseKey) || [];
      group.push(path);
      grouped.set(parsed.baseKey, group);
    });

    grouped.forEach((paths, baseKey) => {
      const parsedGroup = paths.map(parseSeriesSubtitle);
      const epNum = parsedGroup.find((item) => item.episodeNumber !== null)?.episodeNumber || null;
      let entry = draftMap.get(baseKey);

      // Fallback: match by episode number if baseKey doesn't match
      if (!entry && epNum !== null) {
        for (const [, existing] of draftMap) {
          if (existing.episodeNumber === epNum) {
            entry = existing;
            break;
          }
        }
      }

      if (!entry) {
        entry = {
          baseKey,
          displayName: parsedGroup[0]?.name || baseKey,
          targetSubsPath: "",
          nativeSubsPath: "",
          mediaPath: "",
          mediaType: "none" as const,
          episodeNumber: epNum,
        };
      }

      const classified = classifySubtitleCandidates(paths, preferredRole);

      if (preferredRole === "native" && paths.length === 1) {
        entry.nativeSubsPath = paths[0];
      } else if (preferredRole === "auto" && paths.length === 1) {
        const parsed = parsedGroup[0];
        const studiedLanguage = getStudiedLanguagePreference();
        const nativeLanguage = getNativeLanguagePreference();
        const isStudiedSubtitle = Boolean(studiedLanguage && parsed?.language === studiedLanguage);
        const isNativeSubtitle = Boolean(nativeLanguage && parsed?.language === nativeLanguage);

        if (isNativeSubtitle && !isStudiedSubtitle) {
          entry.nativeSubsPath = paths[0];
        } else if (isStudiedSubtitle) {
          entry.targetSubsPath = paths[0];
        } else if (!entry.targetSubsPath) {
          entry.targetSubsPath = paths[0];
        } else {
          entry.nativeSubsPath = paths[0];
        }
      } else {
        if (classified.target) entry.targetSubsPath = classified.target;
        if (classified.native) entry.nativeSubsPath = classified.native;
      }

      draftMap.set(entry.baseKey, entry);
    });

    seriesDraftMapToEpisodes(draftMap);
  }

  function mergeSeriesMediaFiles(mediaFiles: string[]) {
    const draftMap = buildSeriesDraftMap();

    mediaFiles.forEach((path) => {
      const parsed = parseSeriesMedia(path);
      let entry = draftMap.get(parsed.baseKey);

      // Fallback: match by episode number if baseKey doesn't match
      if (!entry && parsed.episodeNumber !== null) {
        for (const [key, existing] of draftMap) {
          if (existing.episodeNumber === parsed.episodeNumber && !existing.mediaPath) {
            entry = existing;
            break;
          }
        }
      }

      if (!entry) {
        entry = {
          baseKey: parsed.baseKey,
          displayName: parsed.name,
          targetSubsPath: "",
          nativeSubsPath: "",
          mediaPath: "",
          mediaType: "none" as const,
          episodeNumber: parsed.episodeNumber,
        };
      }

      entry.mediaPath = parsed.path;
      entry.mediaType = parsed.mediaType;
      entry.episodeNumber = entry.episodeNumber ?? parsed.episodeNumber;
      draftMap.set(entry.baseKey, entry);
    });

    seriesDraftMapToEpisodes(draftMap);
  }

  function mergeSeriesDroppedFiles(subtitleFiles: string[], mediaFiles: string[]) {
    if (subtitleFiles.length > 0) {
      mergeSeriesSubtitleFiles(subtitleFiles, "auto");
    }
    if (mediaFiles.length > 0) {
      mergeSeriesMediaFiles(mediaFiles);
    }
  }

  async function expandSeriesFilesWithSmartMatches(
    subtitleFiles: string[],
    mediaFiles: string[],
  ): Promise<{ subtitleFiles: string[]; mediaFiles: string[] }> {
    if (!smartFileMatchingEnabled || subtitleFiles.length === 0) {
      return { subtitleFiles, mediaFiles };
    }

    const subtitleSet = new Set(subtitleFiles);
    const mediaSet = new Set(mediaFiles);

    await Promise.all(
      subtitleFiles.map(async (path) => {
        try {
          const companion = await invoke<string | null>(
            "sync_suggest_companion_subtitle_for_srt",
            { srtPath: path },
          );
          if (companion && companion !== path) subtitleSet.add(companion);
        } catch {
          // Best-effort suggestion only.
        }

        try {
          const media = await invoke<string | null>("sync_suggest_media_for_srt", {
            srtPath: path,
          });
          if (media) mediaSet.add(media);
        } catch {
          // Best-effort suggestion only.
        }
      }),
    );

    return {
      subtitleFiles: [...subtitleSet],
      mediaFiles: [...mediaSet],
    };
  }

  // Auto-match files across categories by episode number, then lexicographic
  function autoMatchFiles(
    targetFiles: string[],
    nativeFiles: string[],
    mediaFiles: string[],
  ) {
    // Extract episode numbers and sort
    type FileWithEp = { path: string; ep: number | null; name: string };
    const toEntries = (files: string[]): FileWithEp[] =>
      files.map((f) => {
        const name = getFileName(f);
        return { path: f, ep: extractEpisodeNumber(name), name };
      });

    const targets = toEntries(targetFiles);
    const natives = toEntries(nativeFiles);
    const medias = toEntries(mediaFiles);

    // Try episode-number matching first
    const allHaveEps = targets.every((t) => t.ep !== null);
    if (allHaveEps) {
      targets.sort((a, b) => (a.ep ?? 0) - (b.ep ?? 0));
    } else {
      targets.sort((a, b) =>
        a.name.localeCompare(b.name, undefined, { numeric: true }),
      );
    }

    // Match natives and medias by episode number or index
    const newEpisodes: EpisodeEntry[] = targets.map((t, idx) => {
      let nativePath = "";
      let mediaPath = "";
      let mediaType: "none" | "video" | "audio" = "none";

      // Find matching native by episode number
      if (t.ep !== null) {
        const matchNative = natives.find((n) => n.ep === t.ep);
        if (matchNative) nativePath = matchNative.path;
        const matchMedia = medias.find((m) => m.ep === t.ep);
        if (matchMedia) {
          mediaPath = matchMedia.path;
          mediaType = detectMediaType(matchMedia.name);
        }
      }

      // Fall back to index matching
      if (!nativePath && idx < natives.length) {
        const sorted = [...natives].sort((a, b) =>
          a.name.localeCompare(b.name, undefined, { numeric: true }),
        );
        nativePath = sorted[idx]?.path || "";
      }
      if (!mediaPath && idx < medias.length) {
        const sorted = [...medias].sort((a, b) =>
          a.name.localeCompare(b.name, undefined, { numeric: true }),
        );
        if (sorted[idx]) {
          mediaPath = sorted[idx].path;
          mediaType = detectMediaType(sorted[idx].name);
        }
      }

      return {
        id: idx + 1,
        targetSubsPath: t.path,
        nativeSubsPath: nativePath,
        mediaPath,
        mediaType,
      };
    });

    episodes = newEpisodes;
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
      generationStore.error = `${t("flashcards.errorSelectingFile")}: ${e}`;
    }
  }

  async function addMovieFiles() {
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
      generationStore.error = `${t("flashcards.errorSelectingFile")}: ${e}`;
    }
  }

  function canClearMovieFile(field: "target" | "native" | "media" | "output") {
    return field === "target"
      ? !!targetSubsPath
      : field === "native"
        ? !!nativeSubsPath
        : field === "media"
          ? !!mediaPath
          : !!outputDir;
  }

  function clearMovieFileButtonClass(field: "target" | "native" | "media" | "output") {
    return canClearMovieFile(field)
      ? "border-red-500/30 bg-red-500/10 text-red-300 hover:border-red-400/60 hover:bg-red-500/20"
      : "cursor-not-allowed border-white/10 bg-white/5 text-gray-600 opacity-60";
  }

  function clearMovieFile(field: "target" | "native" | "media" | "output") {
    if (!canClearMovieFile(field)) return;

    if (field === "target") {
      targetSubsPath = "";
      targetSubsInfo = null;
    } else if (field === "native") {
      nativeSubsPath = "";
      nativeSubsInfo = null;
    } else if (field === "media") {
      mediaPath = "";
      mediaType = "none";
      audioTracks = [];
      mediaSettings.audioTrackIndex = null;
      audioTrackAutoSelected = true;
      mediaSettings.generateSnapshots = false;
      mediaSettings.generateVideoClips = false;
    } else if (field === "output") {
      outputDir = "";
    }
  }



  function removeEpisode(idx: number) {
    episodes = episodes
      .filter((_, i) => i !== idx)
      .map((e, i) => ({ ...e, id: i + 1 }));
    closeEpisodeContextMenu();
  }

  function openEpisodeEditor(idx: number) {
    const episode = episodes[idx];
    if (!episode) return;
    closeEpisodeContextMenu();
    editingEpisodeIndex = idx;
    editingEpisode = { ...episode };
    initialEditingEpisodeStr = JSON.stringify(episode);
  }

  function openEpisodeContextMenu(event: MouseEvent, idx: number) {
    event.preventDefault();
    episodeContextMenu = { x: event.clientX, y: event.clientY, idx };
  }

  function closeEpisodeContextMenu() {
    episodeContextMenu = null;
  }

  async function openEpisodeMediaSettings(idx: number) {
    const episode = episodes[idx];
    if (!episode || !episode.mediaPath) return;

    closeEpisodeContextMenu();
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
          }
        : episode,
    );
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
          }
        : episode,
    );
    closeEpisodeMediaSettings();
  }

  async function selectEpisodeFile(field: EpisodeFileField) {
    if (!editingEpisode) return;
    try {
      const selected = await guardedOpen({
        multiple: false,
        filters:
          field === "mediaPath"
            ? [
                {
                  name: t("flashcards.mediaFiles"),
                  extensions: [...VIDEO_EXTENSIONS, ...AUDIO_EXTENSIONS],
                },
              ]
            : [
                {
                  name: t("flashcards.subtitleFiles"),
                  extensions: ["srt", "ass", "ssa", "vtt"],
                },
              ],
      });
      if (!selected || Array.isArray(selected)) return;
      editingEpisode = {
        ...editingEpisode,
        [field]: selected,
        mediaType:
          field === "mediaPath"
            ? detectMediaType(getFileName(selected))
            : editingEpisode.mediaType,
      };
      syncEpisodeEditor();
    } catch (e) {
      generationStore.error = `${t("flashcards.errorSelectingFile")}: ${e}`;
    }
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
    episodes = [...episodes];
  }

  function swapAllEpisodesSubs() {
    episodes = episodes.map((ep) => {
      const temp = ep.targetSubsPath;
      return {
        ...ep,
        targetSubsPath: ep.nativeSubsPath,
        nativeSubsPath: temp
      };
    });
  }

  let hasMedia = $derived(
    seriesMode
      ? episodes.some((ep) => ep.mediaType !== "none")
      : mediaType !== "none"
  );
  let hasVideo = $derived(
    seriesMode
      ? episodes.some((ep) => ep.mediaType === "video")
      : mediaType === "video"
  );
  let hasAudio = $derived(hasMedia);

  let showAnkiFields = true;
  let hasAnyFiles = $derived(
    seriesMode
      ? episodes.length > 0
      : targetSubsPath !== ""
  );
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
  let mediaSettings = $state<Required<EpisodeMediaOverrides>>({
    generateAudio: true,
    audioBitrate: 128,
    audioTrackIndex: null,
    normalizeAudio: false,
    audioPadStart: 0,
    audioPadEnd: 0,
    generateSnapshots: true,
    snapshotWidth: loadStoredDimension(FLASHCARD_MEDIA_WIDTH_KEY, DEFAULT_FLASHCARD_MEDIA_WIDTH),
    snapshotHeight: loadStoredDimension(FLASHCARD_MEDIA_HEIGHT_KEY, DEFAULT_FLASHCARD_MEDIA_HEIGHT),
    cropBottom: 0,
    generateVideoClips: false,
    videoCodec: "h264",
    h264Preset: "medium",
    videoBitrate: 800,
    videoAudioBitrate: 128,
    videoPadStart: 250,
    videoPadEnd: 50,
  });
  // "auto" = GPU encoder when available (default), "off" = force libx264 (expert mode).
  // Not part of EpisodeMediaOverrides: it's a global preference, not overridable per-episode.
  let videoHwAccel = $state(
    (() => {
      try {
        return localStorage.getItem("vesta-video-hw-accel") === "off" ? "off" : "auto";
      } catch {
        return "auto";
      }
    })(),
  );
  $effect(() => {
    try {
      localStorage.setItem("vesta-video-hw-accel", videoHwAccel);
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

  $effect(() => {
    persistDimension(FLASHCARD_MEDIA_WIDTH_KEY, mediaSettings.snapshotWidth);
  });

  $effect(() => {
    persistDimension(FLASHCARD_MEDIA_HEIGHT_KEY, mediaSettings.snapshotHeight);
  });

  // exportFormat/seriesOutputMode/cpuCores/deckName state, plus generation
  // run-state (isProcessing/progress/logs/result), live in generationStore
  // (see generationStore.svelte.ts for why). These three $effects stay here
  // rather than in the store because they react to `active` (a prop of this
  // component) and to ankiStore.status over the component's lifetime — same
  // convention as every other store in this codebase (no $effect inside a
  // store class).
  $effect(() => {
    try { localStorage.setItem(EXPORT_FORMAT_KEY, generationStore.exportFormat); } catch {}
  });

  $effect(() => {
    if (active) {
      try {
        const saved = localStorage.getItem(EXPORT_FORMAT_KEY);
        if (saved === "tsv" || saved === "anki" || saved === "apkg") {
          if (saved === "anki" && ankiStore.status !== "online") {
            generationStore.exportFormat = "apkg";
          } else {
            generationStore.exportFormat = saved as any;
          }
        } else {
          if (ankiStore.status === "online") {
            generationStore.exportFormat = "anki";
          } else {
            generationStore.exportFormat = "apkg";
          }
        }
      } catch {}
    }
  });

  $effect(() => {
    if (ankiStore.status === "online") {
      try {
        const saved = localStorage.getItem(EXPORT_FORMAT_KEY);
        if (!saved) {
          generationStore.exportFormat = "anki";
        }
      } catch {}
    } else if (generationStore.exportFormat === "anki") {
      generationStore.exportFormat = "apkg";
    }
  });

  $effect(() => {
    try { localStorage.setItem(SERIES_OUTPUT_MODE_KEY, generationStore.seriesOutputMode); } catch {}
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

  const PANEL_IDS = [
    "files",
    "audioClips",
    "snapshots",
    "videoClips",
    "cardFilters",
    "naming",
    "progressResult",
    "logs",
  ] as const;

  type PanelId = (typeof PANEL_IDS)[number];

  interface ColumnLayout {
    col1: PanelId[];
    col2: PanelId[];
    col3: PanelId[];
  }

  const MOVIE_LAYOUT_KEY = "vesta-flashcards-layout-v4";
  const SERIES_LAYOUT_KEY = "vesta-flashcards-series-layout-v4";

  const DEFAULT_LAYOUT: ColumnLayout = {
    col1: ["files"],
    col2: ["audioClips", "snapshots", "videoClips"],
    col3: ["naming", "cardFilters", "progressResult"],
  };

  const DEFAULT_SERIES_LAYOUT: ColumnLayout = {
    col1: ["files"],
    col2: ["audioClips", "snapshots", "videoClips"],
    col3: ["naming", "cardFilters", "progressResult"],
  };

  function cloneLayout(layout: ColumnLayout): ColumnLayout {
    return {
      col1: [...layout.col1],
      col2: [...layout.col2],
      col3: [...layout.col3],
    };
  }

  function loadLayout(): ColumnLayout {
    return cloneLayout(DEFAULT_LAYOUT);
  }

  function loadSeriesLayout(): ColumnLayout {
    return cloneLayout(DEFAULT_SERIES_LAYOUT);
  }

  function saveLayout(layout: ColumnLayout) {
    localStorage.setItem(MOVIE_LAYOUT_KEY, JSON.stringify(layout));
  }

  function saveSeriesLayout(layout: ColumnLayout) {
    localStorage.setItem(SERIES_LAYOUT_KEY, JSON.stringify(layout));
  }

  let movieLayout = $state<ColumnLayout>(loadLayout());
  let seriesLayout = $state<ColumnLayout>(loadSeriesLayout());

  let panelLayout = $derived(seriesMode ? seriesLayout : movieLayout);

  function updatePanelLayout(newLayout: ColumnLayout) {
    if (seriesMode) {
      seriesLayout = newLayout;
      saveSeriesLayout(newLayout);
    } else {
      movieLayout = newLayout;
      saveLayout(newLayout);
    }
  }

  // Responsive columns: auto-collapse from 3 -> 2 -> 1 based on available width.
  const PREFERRED_COLUMN_COUNT = 3;
  const STACK_TO_ONE_COLUMN_WIDTH = 900;
  const STACK_TO_TWO_COLUMNS_WIDTH = 1200;
  let layoutHostEl = $state<HTMLElement | null>(null);
  let layoutWidth = $state(
    typeof window !== "undefined" ? window.innerWidth : 1700,
  );
  let effectiveColumnCount = $derived(
    layoutWidth < STACK_TO_ONE_COLUMN_WIDTH
      ? 1
      : layoutWidth < STACK_TO_TWO_COLUMNS_WIDTH
        ? 2
        : PREFERRED_COLUMN_COUNT,
  );

  let effectivePanelLayout = $derived.by((): ColumnLayout => {
    if (seriesMode) {
      if (effectiveColumnCount === 3) {
        // In easy mode (no expert mode), spread audio/snapshot/video across 3 columns
        if (easyMode) {
          return {
            col1: ["files", "audioClips"],
            col2: ["snapshots"],
            col3: ["videoClips", "progressResult"],
          };
        }
        return {
          col1: ["files", "naming", "audioClips", "snapshots"],
          col2: ["videoClips"],
          col3: ["cardFilters", "progressResult"],
        };
      }

      if (effectiveColumnCount === 2) {
        // Easy mode: skip 2-col, collapse to 1 col directly
        if (easyMode) {
          return {
            col1: [
              "files",
              "audioClips",
              "snapshots",
              "videoClips",
              "progressResult",
            ],
            col2: [],
            col3: [],
          };
        }
        return {
          col1: [
            "files",
            "audioClips",
            "snapshots",
            "videoClips",
            "progressResult",
          ],
          col2: ["naming", "cardFilters"],
          col3: [],
        };
      }

      return {
        col1: [
          "files",
          "naming",
          "audioClips",
          "snapshots",
          "videoClips",
          "cardFilters",
          "progressResult",
        ],
        col2: [],
        col3: [],
      };
    } else {
      if (effectiveColumnCount === 3) {
        return {
          col1: ["files", "naming"],
          col2: ["audioClips", "videoClips"],
          col3: ["snapshots", "cardFilters", "progressResult"],
        };
      }

      if (effectiveColumnCount === 2) {
        return {
          col1: ["files", "naming", "cardFilters"],
          col2: ["audioClips", "snapshots", "videoClips", "progressResult"],
          col3: [],
        };
      }

      return {
        col1: [
          "files",
          "naming",
          "audioClips",
          "snapshots",
          "videoClips",
          "cardFilters",
          "progressResult",
        ],
        col2: [],
        col3: [],
      };
    }
  });

  // Computed column grid class
  // In easy mode + series mode, skip 2-column layout entirely (go straight 3→1)
  let gridColClass = $derived(
    (easyMode && seriesMode && effectiveColumnCount === 2)
      ? "grid-cols-1"
      : effectiveColumnCount === 1
        ? "grid-cols-1"
        : effectiveColumnCount === 2
          ? "grid-cols-2"
          : "grid-cols-3",
  );

  let filesHelpContent = $derived(
    seriesMode ? t("flashcards.filesHelp") : t("flashcards.filesHelpMovie"),
  );

  let noteTypeLanguage = $state("");

  // ── Note type selection ──────────────────────────────────────────────────
  // One select replaces the old "language + seven field pills" duplication. The
  // chosen note type drives the export name, the study language, and which of the
  // nine fields are active. Predefined note types are generated per language and
  // locked to all fields; custom ones are created and edited in Settings. The
  // active field set is read-only here — editing lives in Settings by design.
  let noteTypeList = $state<NoteTypeDef[]>(listNoteTypes());
  let selectedNoteTypeId = $state(loadActiveNoteTypeId());
  let selectedNoteType = $derived(
    findNoteTypeById(selectedNoteTypeId)
  );
  // Never null — falls back to the default note type.
  let activeNoteType = $derived(
    selectedNoteType ?? findNoteTypeById("default")!
  );
  let noteTypeName = $derived(activeNoteType.name);

  $effect(() => {
    const handler = () => {
      selectedNoteTypeId = loadActiveNoteTypeId();
    };
    window.addEventListener(ACTIVE_NOTE_TYPE_CHANGED_EVENT, handler);
    return () => {
      window.removeEventListener(ACTIVE_NOTE_TYPE_CHANGED_EVENT, handler);
    };
  });

  let noteTypeOptions = $derived(
    noteTypeList.map((nt) => ({
      value: nt.id,
      label: nt.predefined ? nt.name : `★ ${nt.name}`,
      searchTerms: nt.predefined
        ? [nt.name, languages.find((l) => l.code === nt.language)?.nameEn ?? ""]
            .filter(Boolean)
            .join(" ")
        : nt.name,
      icon: nt.predefined
        ? languages.find((l) => l.code === nt.language)?.flag ?? "🃏"
        : "★",
    })),
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
    selectedNoteTypeId = id;
    const nt = findNoteTypeById(id);
    if (nt?.language) {
      noteTypeLanguage = nt.language;
      try {
        localStorage.setItem(NOTE_TYPE_LANGUAGE_KEY, nt.language);
      } catch {}
    }
  }

  function cycleTemplates() {
    if (noteTypeList.length === 0) return;
    const currentIndex = noteTypeList.findIndex((nt) => nt.id === selectedNoteTypeId);
    const nextIndex = (currentIndex + 1) % noteTypeList.length;
    const nextTemplate = noteTypeList[nextIndex];
    if (nextTemplate) {
      selectedNoteTypeId = nextTemplate.id;
      saveActiveNoteTypeId(nextTemplate.id);
    }
  }

  // deckName/deckNameAuto and the generation run-state (isProcessing,
  // progress, progressMessage, progressStage, logs, error, result) live in
  // generationStore now — see generationStore.svelte.ts.

  let targetSubsInfo = $state<{
    count: number;
    first_text: string;
    format: string;
    actors: string[];
    duration_ms: number;
  } | null>(null);
  let nativeSubsInfo = $state<{
    count: number;
    first_text: string;
    format: string;
  } | null>(null);
  let ffmpegAvailable = $state<boolean | null>(null);
  let isDownloadingFFmpeg = $state(false);

  let previewMediaPath = $derived.by(() => {
    if (seriesMode && episodes.length > 0) {
      return episodes[0].mediaPath;
    }
    return mediaPath;
  });

  let previewMediaType = $derived.by(() => {
    if (seriesMode && episodes.length > 0) {
      return episodes[0].mediaType;
    }
    return hasVideo ? "video" : (hasAudio ? "audio" : "none");
  });

  let expandedPathField = $state<string | null>(null);

  let unlisten: (() => void) | null = null;
  let activeListener = true;

  $effect(() => {
    if (active) {
      let unlistenDragDropLocal: (() => void) | null = null;
      getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "over") {
          isDraggingOver = true;
        } else if (event.payload.type === "drop") {
          isDraggingOver = false;
          if (event.payload.paths && event.payload.paths.length > 0) {
            handleFileDrop(event.payload.paths);
          }
        } else if (event.payload.type === "leave") {
          isDraggingOver = false;
        }
      }).then((fn) => {
        unlistenDragDropLocal = fn;
      }).catch((e) => {
        console.warn("Failed to set up drag-drop listener in FlashcardsTab:", e);
      });

      return () => {
        if (unlistenDragDropLocal) {
          unlistenDragDropLocal();
        }
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
  let needsDeckName = $derived(
    !seriesMode || generationStore.seriesOutputMode === "single",
  );
  let canRunFlashcards = $derived(
    seriesMode
      ? Boolean(
          episodes.length > 0 &&
            episodes.every((ep) => ep.targetSubsPath) &&
            outputDir &&
            (needsDeckName ? Boolean(generationStore.deckName || (easyMode && episodes.length > 0)) : true) &&
            noteTypeLanguage,
        )
      : Boolean(targetSubsPath && outputDir && generationStore.deckName && noteTypeLanguage),
  );

  type RequirementPanelId = "files" | "naming";
  type GenerationRequirement = {
    panel: RequirementPanelId;
    label: string;
  };

  let generationPromptOpen = $state(false);
  let highlightedRequirementPanels = $state<Set<RequirementPanelId>>(new Set());
  let requirementPulseTimer: ReturnType<typeof setTimeout> | null = null;

  let generationRequirements = $derived.by((): GenerationRequirement[] => {
    const missing: GenerationRequirement[] = [];
    if (seriesMode) {
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
    } else if (!targetSubsPath) {
      missing.push({
        panel: "files",
        label: `${t("flashcards.targetLangSubs")}`,
      });
    }
    if (!outputDir) {
      missing.push({
        panel: "files",
        label: `${t("flashcards.outputDir")}`,
      });
    }
    if (!easyMode && needsDeckName && !generationStore.deckName.trim()) {
      missing.push({
        panel: "naming",
        label: `${t("flashcards.deckNameLabel")}`,
      });
    }
    return missing;
  });

  let generationRequirementsText = $derived(
    generationRequirements.map((item) => item.label).join(", "),
  );

  /** Panels rendered in the column layout. Easy mode hides the advanced ones. */
  function isPanelVisible(panelId: PanelId): boolean {
    if (panelId === "progressResult") return false;
    if (seriesMode && panelId === "files") return false; // rendered full-width above
    if (easyMode && (panelId === "cardFilters" || panelId === "naming")) return false;
    return true;
  }

  function panelHighlightClass(panelId: RequirementPanelId): string {
    return highlightedRequirementPanels.has(panelId)
      ? "flashcard-requirement-pulse"
      : "";
  }

  function promptMissingGenerationRequirements() {
    if (canRunFlashcards) return;
    generationPromptOpen = true;
    highlightedRequirementPanels = new Set(
      generationRequirements.map((item) => item.panel),
    );
    if (requirementPulseTimer) clearTimeout(requirementPulseTimer);
    requirementPulseTimer = setTimeout(() => {
      highlightedRequirementPanels = new Set();
    }, 1800);
  }

  function closeGenerationPrompt() {
    generationPromptOpen = false;
  }


  function getPreferredAudioLanguageCode(): string {
    return inferLanguageFromPath(targetSubsPath) || noteTypeLanguage;
  }

  function scoreAudioTrackForLanguage(track: AudioTrackInfo, languageCode: string): number {
    if (!languageCode) return 0;
    return Math.max(
      scoreLanguageMatch(track.language || "", languageCode),
      Math.max(0, scoreLanguageMatch(track.title || "", languageCode) - 12),
    );
  }

  function pickBestAudioTrackIndex(tracks: AudioTrackInfo[], languageCode: string): number | null {
    if (tracks.length <= 1) return null;
    let bestTrack = tracks[0];
    let bestScore = -1;

    for (const track of tracks) {
      const score = scoreAudioTrackForLanguage(track, languageCode);
      if (score > bestScore) {
        bestScore = score;
        bestTrack = track;
      }
    }

    return bestTrack.index;
  }

  async function listAudioTracksForEpisode(ep: EpisodeEntry): Promise<AudioTrackInfo[]> {
    if (!ep.mediaPath || ep.mediaType !== "video") return [];
    try {
      return await invoke<AudioTrackInfo[]>("flashcard_list_audio_tracks", {
        path: ep.mediaPath,
      });
    } catch (e) {
      generationStore.addLog(`${t("flashcards.audioTracksError")}: ${e}`, "warning", getFileName(ep.mediaPath));
      return [];
    }
  }

  async function pickAudioTrackIndexForEpisode(ep: EpisodeEntry): Promise<number | null> {
    const tracks = await listAudioTracksForEpisode(ep);
    return pickBestAudioTrackIndex(tracks, getPreferredAudioLanguageCodeForEpisode(ep));
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
      generationStore.addLog(`${t("flashcards.audioTracksError")}: ${e}`, "warning");
    } finally {
      audioTracksLoading = false;
    }
  }

  $effect(() => {
    if (audioTracks.length > 1 && audioTrackAutoSelected) {
      mediaSettings.audioTrackIndex = pickBestAudioTrackIndex(audioTracks, getPreferredAudioLanguageCode());
    }
  });

  // ─── File Drag-and-Drop Handler ───────────────────────────────────────────
  function getFileExtension(path: string): string {
    return (path.split(".").pop() || "").toLowerCase();
  }


  function isSubtitleFile(path: string): boolean {
    return SUBTITLE_EXTENSIONS.includes(getFileExtension(path));
  }

  function isMediaFile(path: string): boolean {
    const ext = getFileExtension(path);
    return VIDEO_EXTENSIONS.includes(ext) || AUDIO_EXTENSIONS.includes(ext);
  }

  /**
   * Determine which subtitle file is "target" (the one you're studying)
   * and which is "native" (your native language translation).
   * Uses language codes in filenames, keywords, and the selected noteTypeLanguage.
   */
  function classifySubtitles(paths: string[]): { target: string; native: string } {
    return classifySubtitleCandidates(paths, "auto");
  }
  function generateDefaultDeckName(filename: string): string {
    let base = filename.replace(/\.[^/.]+$/, "");
    
    // Remove known suffixes
    base = base.replace(/[._-](native|original|orig|source|translated|translation|tradotto|traduzione|reference|ref)(?=(\.|_|-|$))/gi, "");
    
    // Remove language code
    const parts = base.split(/[._-]/);
    if (parts.length > 1) {
      const lastPart = parts[parts.length - 1].toLowerCase();
      if (KNOWN_LANGUAGE_CODES.has(lastPart) || detectLanguageCode(lastPart)) {
        parts.pop();
        base = parts.join(" ");
      } else {
        base = parts.join(" ");
      }
    }
    
    // Remove episode numbers
    base = base.replace(/[._\-\s]*[Ss]\d{1,2}[Ee]\d{1,4}[._\-\s]*/gi, " ");
    base = base.replace(/[._\-\s]*[Ee][Pp]?\.?\s*\d{1,4}[._\-\s]*/gi, " ");
    base = base.replace(/[._\-\s]*[Ee]pisode\.?\s*\d{1,4}[._\-\s]*/gi, " ");
    base = base.replace(/[._\-\s]*[Xx]\d{1,4}[._\-\s]*/gi, " ");
    
    // Isolated numbers
    base = base.replace(/[\s_\-\.]\d{1,4}$/, "");
    base = base.replace(/[\s_\-\.]\d{1,4}[\s_\-\.]/, " ");
    base = base.replace(/^\d{1,4}[\s_\-\.]/, "");
    
    return base.replace(/[._-]/g, " ").replace(/\s+/g, " ").trim() || "Default Deck";
  }

  /** Derive a deck name from an episode file path for "separate" mode.
   *  Returns the filename without extension, with known language suffixes
   *  like -en, -it etc. stripped. */
  function deriveDeckNameFromFile(ep: EpisodeEntry): string {
    // Prefer media file, then target subs
    const filePath = ep.mediaPath || ep.targetSubsPath;
    const filename = getFileName(filePath);
    let base = filename.replace(/\.[^/.]+$/, "");

    // Strip known language suffixes like -en, _it, .ja etc.
    const langParts = base.split(/[._-]/);
    if (langParts.length > 1) {
      const lastPart = langParts[langParts.length - 1].toLowerCase();
      if (KNOWN_LANGUAGE_CODES.has(lastPart) || detectLanguageCode(lastPart)) {
        langParts.pop();
        base = langParts.join(" ");
      }
    }

    return base.replace(/[._-]/g, " ").replace(/\s+/g, " ").trim() || `Episode`;
  }

  async function handleFileDrop(paths: string[]) {
    if (!paths || paths.length === 0) {
      return;
    }

    const subtitleFiles = paths.filter(isSubtitleFile);
    const mediaFiles = paths.filter(isMediaFile);

    if (subtitleFiles.length === 0 && mediaFiles.length === 0) {
      return;
    }

    if (seriesMode) {
      if (subtitleFiles.length > 0 || mediaFiles.length > 0) {
        try {
          const expanded = await expandSeriesFilesWithSmartMatches(
            subtitleFiles,
            mediaFiles,
          );
          mergeSeriesDroppedFiles(expanded.subtitleFiles, expanded.mediaFiles);
          generationStore.addLog(`${episodes.length} ${t("flashcards.seriesEpisodesAdded")}`, "target-subs");
        } catch (e: any) {
          console.error("[DragDrop] Errore nell'elaborazione smart della serie:", e);
        }
      }
    } else {
      // Single-episode mode
      if (subtitleFiles.length >= 2) {
        const { target, native } = classifySubtitles(subtitleFiles);
        if (target) {
          try {
            await loadTargetSubtitle(target);
            await tryAutoSelectMediaForSubtitle(target, smartFileMatchingEnabled);
          } catch (e: any) {
            generationStore.error = `Error parsing subtitles: ${e}`;
          }
        }
        if (native) {
          try {
            await loadNativeSubtitle(native);
          } catch (e: any) {
            generationStore.error = `Error parsing native subtitles: ${e}`;
          }
        }
      } else if (subtitleFiles.length === 1) {
        const subPath = subtitleFiles[0];
        if (!targetSubsPath) {
          try {
            await loadTargetSubtitle(subPath);
            await tryAutoSelectCompanionSubtitle(
              subPath,
              "target",
              smartFileMatchingEnabled,
            );
            await tryAutoSelectMediaForSubtitle(
              subPath,
              smartFileMatchingEnabled,
            );
          } catch (e: any) {
            generationStore.error = `Error parsing subtitles: ${e}`;
          }
        } else {
          try {
            await loadNativeSubtitle(subPath);
            await tryAutoSelectCompanionSubtitle(
              subPath,
              "native",
              smartFileMatchingEnabled,
            );
            await tryAutoSelectMediaForSubtitle(
              subPath,
              smartFileMatchingEnabled,
            );
          } catch (e: any) {
            generationStore.error = `Error parsing native subtitles: ${e}`;
          }
        }
      }

      // Handle media files
      if (mediaFiles.length > 0) {
        const mediaPath = mediaFiles[0];
        await applyMediaSelection(mediaPath);
        await tryAutoSelectSubtitlesForMedia(mediaPath, subtitleFiles.length === 0);
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
        const defaultNoteTypeLanguage = localStorage.getItem(DEFAULT_FLASHCARDS_LANGUAGE_KEY);
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

    try {
      const savedNoteTypeLanguage = localStorage.getItem(
        NOTE_TYPE_LANGUAGE_KEY,
      );
      if (
        savedNoteTypeLanguage &&
        languages.some((l) => l.code === savedNoteTypeLanguage)
      ) {
        noteTypeLanguage = savedNoteTypeLanguage;
      } else {
        const defaultNoteTypeLanguage = localStorage.getItem(DEFAULT_FLASHCARDS_LANGUAGE_KEY);
        if (
          defaultNoteTypeLanguage &&
          languages.some((l) => l.code === defaultNoteTypeLanguage)
        ) {
          noteTypeLanguage = defaultNoteTypeLanguage;
        }
      }
    } catch {}

    try {
      const savedDir = localStorage.getItem(OUTPUT_DIR_KEY);
      if (savedDir) {
        const exists = await invoke<boolean>("flashcard_check_dir_exists", {
          path: savedDir,
        });
        if (exists) {
          outputDir = savedDir;
        } else {
          localStorage.removeItem(OUTPUT_DIR_KEY);
        }
      }
    } catch {}

    try {
      ffmpegAvailable = await invoke<boolean>("flashcard_check_deps");
    } catch {
      ffmpegAvailable = false;
    }

    try {
      generationStore.systemCpuCount = await invoke<number>("flashcard_get_cpu_count");
      const startupMaxCores = Math.max(2, generationStore.systemCpuCount - 1);
      const savedCores = localStorage.getItem("vesta_cpu_cores");
      if (savedCores) {
        generationStore.cpuCores = parseInt(savedCores);
      } else {
        generationStore.cpuCores = startupMaxCores;
      }
    } catch {
      generationStore.systemCpuCount = 4;
      const savedCores = localStorage.getItem("vesta_cpu_cores");
      if (savedCores) {
        generationStore.cpuCores = parseInt(savedCores);
      } else {
        generationStore.cpuCores = Math.max(2, generationStore.systemCpuCount - 1);
      }
    }

    window.addEventListener("vesta-cpu-cores-changed", handleCpuCoresChanged);
    window.addEventListener("vesta-ffmpeg-updated", handleFfmpegUpdated);

    // OS-level file drag and drop is handled dynamically via $effect

    listen<{
      stage: string;
      message: string;
      current: number;
      total: number;
      percentage: number;
      params: Record<string, string>;
    }>("flashcard-progress", (event) => {
      const p = event.payload;
      const translated = t(p.message, p.params || {});
      
      if (seriesMode && seriesTotalEpisodes > 0) {
        generationStore.progress = Math.round(((seriesCurrentEpisode - 1) * 100 / seriesTotalEpisodes) + (p.percentage / seriesTotalEpisodes));
        generationStore.progressMessage = `[Ep. ${seriesCurrentEpisode}/${seriesTotalEpisodes}] ${translated}`;
      } else {
        generationStore.progress = Math.round(p.percentage);
        generationStore.progressMessage = translated;
      }
      
      generationStore.progressStage = p.stage;
      if (p.stage !== "done") {
        generationStore.addLog(generationStore.progressMessage, "progress", undefined, p.message);
      }
    }).then((fn) => {
      if (!activeListener) fn();
      else unlisten = fn;
    }).catch(console.error);
  });

  onDestroy(() => {
    activeListener = false;
    window.removeEventListener("vesta-cpu-cores-changed", handleCpuCoresChanged);
    window.removeEventListener("vesta-ffmpeg-updated", handleFfmpegUpdated);
    if (unlisten) unlisten();
    if (removeTemplateListener) removeTemplateListener();
    if (removeNoteTypesListener) removeNoteTypesListener();
    if (removeLanguageDefaultsListener) removeLanguageDefaultsListener();
    if (removeLayoutObserver) removeLayoutObserver();
    if (requirementPulseTimer) clearTimeout(requirementPulseTimer);
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
        return localStorage.getItem("vesta-ankiconnect-url") || "http://127.0.0.1:8765";
      } catch {
        return "http://127.0.0.1:8765";
      }
    })();

    try {
      await invoke<number>("ankiconnect_ping", { url });
    } catch {
      return; // Anki not running / AnkiConnect not installed.
    }

    try {
      await invoke("ankiconnect_import_package", { path: apkgPath, url });
      generationStore.addLog(t("flashcards.ankiAutoImportSuccess"), "success");
      snackbar.show(t("flashcards.ankiAutoImportSuccess"), "success");
    } catch (e) {
      generationStore.addLog(`${t("flashcards.ankiAutoImportFailed")}: ${e}`, "warning");
    }
  }

  // addLog/clearLogs now live on generationStore (generationStore.svelte.ts).

  function parseTimeToMs(time: string): number | null {
    if (!time || !time.trim()) return null;
    const parts = time.split(":").map(Number);
    if (parts.length === 3 && parts.every((p) => !isNaN(p))) {
      return (parts[0] * 3600 + parts[1] * 60 + parts[2]) * 1000;
    }
    return null;
  }

  function buildConfig() {
    let tPath = targetSubsPath;
    let nPath = nativeSubsPath || null;
    let vPath = hasVideo ? mediaPath : null;
    let aPath = hasAudio && !hasVideo ? mediaPath : null;

    if (seriesMode && episodes.length > 0) {
      const ep = episodes[0];
      tPath = ep.targetSubsPath;
      nPath = ep.nativeSubsPath || null;
      vPath = ep.mediaType === "video" ? ep.mediaPath : null;
      aPath = ep.mediaType === "audio" ? ep.mediaPath : null;
    }

    return {
      target_subs_path: tPath,
      native_subs_path: nPath,
      video_path: vPath,
      audio_path: aPath,
      output_dir: outputDir,
      use_timings_from: "target",
      span_start_ms: null,
      span_end_ms: null,
      time_shift_target_ms: 0,
      time_shift_native_ms: 0,
      filters: {
        include_words: null,
        exclude_words: null,
        exclude_duplicates_subs1: false,
        exclude_duplicates_subs2: false,
        min_chars: cardFilters.enabled && cardFilters.minCharsEnabled ? cardFilters.minChars : null,
        max_chars: cardFilters.enabled && cardFilters.maxCharsEnabled ? cardFilters.maxChars : null,
        min_duration_ms: cardFilters.enabled && cardFilters.minDurationEnabled ? cardFilters.minDurationMs : null,
        max_duration_ms: cardFilters.enabled && cardFilters.maxDurationEnabled ? cardFilters.maxDurationMs : null,
        exclude_styled: false,
        actor_filter: null,
        only_cjk: false,
        remove_no_match: false,
      },
      context: {
        leading: 0,
        trailing: 0,
        max_gap_seconds: 15.0,
      },
      combine_sentences: cardFilters.enabled && cardFilters.combineSentences,
      continuation_chars: cardFilters.continuationChars,
      generate_audio: mediaSettings.generateAudio,
      audio_bitrate: mediaSettings.audioBitrate,
      audio_track_index: mediaSettings.audioTrackIndex,
      normalize_audio: mediaSettings.normalizeAudio,
      audio_pad_start_ms: mediaSettings.audioPadStart,
      audio_pad_end_ms: mediaSettings.audioPadEnd,
      generate_snapshots: mediaSettings.generateSnapshots,
      snapshot_width: mediaSettings.snapshotWidth,
      snapshot_height: mediaSettings.snapshotHeight,
      crop_bottom: mediaSettings.cropBottom,
      generate_video_clips: mediaSettings.generateVideoClips,
      video_codec: mediaSettings.videoCodec,
      h264_preset: mediaSettings.h264Preset,
      video_hw_accel: videoHwAccel,
      video_bitrate: mediaSettings.videoBitrate,
      video_audio_bitrate: mediaSettings.videoAudioBitrate,
      video_pad_start_ms: mediaSettings.videoPadStart,
      video_pad_end_ms: mediaSettings.videoPadEnd,
      deck_name: generationStore.deckName,
      episode_number: 1,
      export_format: generationStore.effectiveExportFormat,
      note_type_name: activeNoteType.name,
      field_names: activeNoteType.fields,
      output_fields: noteTypeOutputFields(activeNoteType),
      cpu_cores: generationStore.effectiveCpuCores,
      card_front_html: loadCardTemplates().frontHtml,
      card_back_html: loadCardTemplates().backHtml,
      card_css: loadCardTemplates().css,
    };
  }

  async function loadTargetSubtitle(path: string) {
    targetSubsPath = path;
    const filename = getFileName(targetSubsPath);

    if (!noteTypeLanguage) {
      const inferredLanguage = inferLanguageFromPath(targetSubsPath);
      if (inferredLanguage) {
        noteTypeLanguage = inferredLanguage;
        localStorage.setItem(NOTE_TYPE_LANGUAGE_KEY, inferredLanguage);
      }
    }

    const info = await invoke<any>("flashcard_load_subs", {
      path: targetSubsPath,
    });
    targetSubsInfo = info;
    generationStore.addLog(
      `${info.count} ${t("flashcards.subtitlesLoaded")} (${info.format.toUpperCase()})`,
      "target-subs",
      filename,
    );

    if (generationStore.deckNameAuto || !generationStore.deckName.trim()) {
      generationStore.deckName = generateDefaultDeckName(filename);
      generationStore.deckNameAuto = true;
    }
  }

  async function loadNativeSubtitle(path: string) {
    nativeSubsPath = path;
    const filename = getFileName(nativeSubsPath);

    const info = await invoke<any>("flashcard_load_subs", {
      path: nativeSubsPath,
    });
    nativeSubsInfo = info;
    generationStore.addLog(
      `${info.count} ${t("flashcards.subtitlesLoaded")} (${info.format.toUpperCase()})`,
      "native-subs",
      filename,
    );
  }

  async function applyMediaSelection(path: string, autoSelected = false) {
    mediaPath = path;
    const filename = getFileName(mediaPath);
    mediaType = detectMediaType(filename);
    await loadAudioTracksForMedia(path);

    if (mediaType === "video") {
      mediaSettings.generateAudio = true;
      mediaSettings.generateSnapshots = true;
      generationStore.addLog(`${t("flashcards.mediaTypeVideo")}`, "media", filename);
    } else if (mediaType === "audio") {
      mediaSettings.generateAudio = true;
      mediaSettings.generateSnapshots = false;
      mediaSettings.generateVideoClips = false;
      generationStore.addLog(`${t("flashcards.mediaTypeAudio")}`, "media", filename);
    }

    if (autoSelected) {
      generationStore.addLog(`Auto-selected media: ${filename}`, "media");
    }
  }

  async function tryAutoSelectCompanionSubtitle(
    path: string,
    selectedRole: "target" | "native",
    force = false,
  ) {
    if (!smartFileMatchingEnabled) return;
    const needsTarget = selectedRole === "native" && !targetSubsPath;
    const needsNative = selectedRole === "target" && !nativeSubsPath;
    if (!force && !needsTarget && !needsNative) return;

    try {
      const suggested = await invoke<string | null>("sync_suggest_companion_subtitle_for_srt", {
        srtPath: path,
      });
      if (!suggested || suggested === path) {
        if (force) {
          if (selectedRole === "target") {
            nativeSubsPath = "";
            nativeSubsInfo = null;
          } else {
            targetSubsPath = "";
            targetSubsInfo = null;
          }
        }
        return;
      }

      if (selectedRole === "target") {
        await loadNativeSubtitle(suggested);
      } else {
        await loadTargetSubtitle(suggested);
      }
    } catch {
      // Best-effort suggestion only.
    }
  }

  async function tryAutoSelectMediaForSubtitle(path: string, force = false) {
    if (!smartFileMatchingEnabled) return;
    if (!force && mediaPath) return;
    try {
      const suggestedPath = await invoke<string | null>("sync_suggest_media_for_srt", {
        srtPath: path,
      });
      if (!suggestedPath) {
        if (force) {
          mediaPath = "";
          mediaType = "none";
        }
        return;
      }
      await applyMediaSelection(suggestedPath, true);
    } catch {
      // Best-effort suggestion only.
    }
  }

  async function tryAutoSelectSubtitlesForMedia(path: string, force = false) {
    if (!smartFileMatchingEnabled) return;
    const needsTarget = !targetSubsPath;
    const needsNative = !nativeSubsPath;
    if (!force && !needsTarget && !needsNative) return;

    try {
      const defaultTargetLang = getStudiedLanguagePreference();
      const defaultNativeLang = getNativeLanguagePreference();

      const suggested = await invoke<{ target: string | null; native: string | null }>(
        "sync_suggest_subtitles_for_media",
        {
          mediaPath: path,
          defaultTargetLang: defaultTargetLang || null,
          defaultNativeLang: defaultNativeLang || null,
        }
      );

      if (suggested) {
        if (suggested.target && (force || !targetSubsPath)) {
          await loadTargetSubtitle(suggested.target);
        }
        if (suggested.native && (force || !nativeSubsPath)) {
          await loadNativeSubtitle(suggested.native);
        }
      }
    } catch (e) {
      console.error("[SmartMatching] Error suggesting subtitles for media:", e);
    }
  }

  async function selectTargetSubs() {
    try {
      const selected = await guardedOpen({
        multiple: false,
        filters: [
          {
            name: t("flashcards.subtitleFiles"),
            extensions: ["srt", "ass", "ssa", "vtt"],
          },
        ],
      });
      if (selected) {
        try {
          const selectedPath = selected as string;
          await loadTargetSubtitle(selectedPath);
          await tryAutoSelectCompanionSubtitle(
            selectedPath,
            "target",
            smartFileMatchingEnabled,
          );
          await tryAutoSelectMediaForSubtitle(
            selectedPath,
            smartFileMatchingEnabled,
          );
        } catch (e) {
          generationStore.error = `Error parsing subtitles: ${e}`;
        }
      }
    } catch (e) {
      generationStore.error = `${t("flashcards.errorSelectingFile")}: ${e}`;
    }
  }

  async function selectNativeSubs() {
    try {
      const selected = await guardedOpen({
        multiple: false,
        filters: [
          {
            name: t("flashcards.subtitleFiles"),
            extensions: ["srt", "ass", "ssa", "vtt"],
          },
        ],
      });
      if (selected) {
        try {
          const selectedPath = selected as string;
          await loadNativeSubtitle(selectedPath);
          await tryAutoSelectCompanionSubtitle(
            selectedPath,
            "native",
            smartFileMatchingEnabled,
          );
          await tryAutoSelectMediaForSubtitle(
            selectedPath,
            smartFileMatchingEnabled,
          );
        } catch (e) {
          generationStore.error = `Error parsing native subtitles: ${e}`;
        }
      }
    } catch (e) {
      generationStore.error = `${t("flashcards.errorSelectingFile")}: ${e}`;
    }
  }

  const VIDEO_EXTENSIONS = [
    "mp4",
    "mkv",
    "avi",
    "webm",
    "mov",
    "flv",
    "ogm",
    "vob",
  ];
  const AUDIO_EXTENSIONS = ["mp3", "aac", "flac", "m4a", "ogg", "wav", "wma"];

  function detectMediaType(filename: string): "video" | "audio" | "none" {
    const ext = filename.split(".").pop()?.toLowerCase() || "";
    if (VIDEO_EXTENSIONS.includes(ext)) return "video";
    if (AUDIO_EXTENSIONS.includes(ext)) return "audio";
    return "none";
  }

  async function selectMedia() {
    try {
      const selected = await guardedOpen({
        multiple: false,
        filters: [
          {
            name: t("flashcards.mediaFiles"),
            extensions: [...VIDEO_EXTENSIONS, ...AUDIO_EXTENSIONS],
          },
        ],
      });
      if (selected) {
        await applyMediaSelection(selected as string);
        await tryAutoSelectSubtitlesForMedia(selected as string, true);
      }
    } catch (e) {
      generationStore.error = `${t("flashcards.errorSelectingFile")}: ${e}`;
    }
  }

  async function selectOutputDir() {
    try {
      const selected = await guardedOpen({ directory: true });
      if (selected) {
        outputDir = selected as string;
        localStorage.setItem(OUTPUT_DIR_KEY, outputDir);
        generationStore.addLog(`${t("flashcards.outputDirSet")}`, "output", outputDir);
      }
    } catch (e) {
      generationStore.error = `${t("flashcards.errorSelectingDir")}: ${e}`;
    }
  }

  async function loadPreview() {
    if (!canRunFlashcards) {
      generationStore.error = t("flashcards.requiredFieldsMissing");
      return;
    }

    generationStore.error = null;
    try {
      const config = buildConfig();
      await previewStore.load(config, (lines) =>
        generationStore.addLog(
          `Preview: ${lines.length} total, ${lines.filter((l) => l.active).length} active`,
          "info",
        ),
      );
    } catch (e) {
      generationStore.error = `Preview error: ${e}`;
    }
  }

  async function startSeriesGeneration() {
    if (easyMode && needsDeckName && !generationStore.deckName.trim() && episodes.length > 0) {
      generationStore.deckName = deriveDeckNameFromFile(episodes[0]);
      generationStore.deckNameAuto = true;
    }
    generationStore.error = null;
    generationStore.result = null;
    generationStore.progress = 0;
    generationStore.isProcessing = true;
    seriesTotalEpisodes = episodes.length;
    seriesCurrentEpisode = 0;

    generationStore.addLog(
      `${t("flashcards.starting")}... (${t("flashcards.modeSeries")}: ${episodes.length} ${t("flashcards.seriesEpisodes")})`,
      "info",
    );
    generationStore.addLog(`${t("flashcards.deckName")}: ${generationStore.deckName}`, "info");

    const startTime = Date.now();
    let totalCards = 0;
    let totalAudio = 0;
    let totalSnapshots = 0;
    let totalVideoClips = 0;
    const apkgPaths: string[] = [];
    let hadError = false;
    let errorMessages: string[] = [];

    try {
      for (let i = 0; i < episodes.length; i++) {
        seriesCurrentEpisode = i + 1;
        const ep = episodes[i];
        const epNum = i + 1;

        generationStore.addLog(
          `${t("flashcards.processingEpisode", { current: String(epNum), total: String(episodes.length) })}`,
          "info",
        );

        // Determine media availability for this episode
        const epMediaType = ep.mediaType;
        const epHasVideo = epMediaType === "video";
        const epHasMedia = epMediaType !== "none";
        const epMediaSettings = getEpisodeMediaSettings(ep);
        const epAudioTrackIndex =
          ep.mediaOverrides?.audioTrackIndex !== undefined
            ? ep.mediaOverrides.audioTrackIndex
            : await pickAudioTrackIndexForEpisode(ep);

        const epConfig = {
          target_subs_path: ep.targetSubsPath,
          native_subs_path: ep.nativeSubsPath || null,
          video_path: epHasVideo ? ep.mediaPath : null,
          audio_path: epHasMedia && !epHasVideo ? ep.mediaPath : null,
          output_dir: outputDir,
          use_timings_from: "target",
          span_start_ms: null,
          span_end_ms: null,
          time_shift_target_ms: 0,
          time_shift_native_ms: 0,
          filters: {
            include_words: null,
            exclude_words: null,
            exclude_duplicates_subs1: false,
            exclude_duplicates_subs2: false,
            min_chars: cardFilters.enabled && cardFilters.minCharsEnabled ? cardFilters.minChars : null,
            max_chars: cardFilters.enabled && cardFilters.maxCharsEnabled ? cardFilters.maxChars : null,
            min_duration_ms: cardFilters.enabled && cardFilters.minDurationEnabled ? cardFilters.minDurationMs : null,
            max_duration_ms: cardFilters.enabled && cardFilters.maxDurationEnabled ? cardFilters.maxDurationMs : null,
            exclude_styled: false,
            actor_filter: null,
            only_cjk: false,
            remove_no_match: false,
          },
          context: {
            leading: 0,
            trailing: 0,
            max_gap_seconds: 15.0,
          },
          combine_sentences: cardFilters.enabled && cardFilters.combineSentences,
          continuation_chars: cardFilters.continuationChars,
          generate_audio: ep.mediaPath ? epMediaSettings.generateAudio : false,
          audio_bitrate: epMediaSettings.audioBitrate,
          audio_track_index: epAudioTrackIndex,
          normalize_audio: epMediaSettings.normalizeAudio,
          audio_pad_start_ms: epMediaSettings.audioPadStart,
          audio_pad_end_ms: epMediaSettings.audioPadEnd,
          generate_snapshots: epHasVideo ? epMediaSettings.generateSnapshots : false,
          snapshot_width: epMediaSettings.snapshotWidth,
          snapshot_height: epMediaSettings.snapshotHeight,
          crop_bottom: epMediaSettings.cropBottom,
          generate_video_clips: epHasVideo ? epMediaSettings.generateVideoClips : false,
          video_codec: epMediaSettings.videoCodec,
          h264_preset: epMediaSettings.h264Preset,
          video_hw_accel: videoHwAccel,
          video_bitrate: epMediaSettings.videoBitrate,
          video_audio_bitrate: epMediaSettings.videoAudioBitrate,
          video_pad_start_ms: epMediaSettings.videoPadStart,
          video_pad_end_ms: epMediaSettings.videoPadEnd,
          deck_name: generationStore.seriesOutputMode === "separate" ? deriveDeckNameFromFile(ep) : generationStore.deckName,
          episode_number: epNum,
          export_format: generationStore.effectiveExportFormat,
          note_type_name: activeNoteType.name,
          field_names: activeNoteType.fields,
          output_fields: noteTypeOutputFields(activeNoteType),
          cpu_cores: generationStore.effectiveCpuCores,
          card_front_html: loadCardTemplates().frontHtml,
          card_back_html: loadCardTemplates().backHtml,
          card_css: loadCardTemplates().css,
        };

        await previewStore.applyOverrides(epConfig);

        try {
          const res = await invoke<any>("flashcard_generate", {
            config: epConfig,
          });
          if (res.success) {
            totalCards += res.cards_generated;
            totalAudio += res.audio_clips;
            totalSnapshots += res.snapshots;
            totalVideoClips += res.video_clips;
            if (res.apkg_path) apkgPaths.push(res.apkg_path);
            generationStore.addLog(
              `✓ Ep ${epNum}: ${res.cards_generated} ${t("flashcards.cardsGenerated")}`,
              "success",
            );
          } else {
            generationStore.addLog(`⚠ Ep ${epNum}: ${res.message}`, "warning");
            errorMessages.push(`Ep ${epNum}: ${res.message}`);
          }
        } catch (e: any) {
          const errMsg = e ? e.toString() : "Unknown error";
          generationStore.addLog(`✗ Ep ${epNum}: ${errMsg}`, "error");
          hadError = true;
          errorMessages.push(`Ep ${epNum}: ${errMsg}`);
        }
      }

      let finalApkgPath: string | null = null;
      if (apkgPaths.length > 0) {
        finalApkgPath = apkgPaths[apkgPaths.length - 1];
      }

      // Merge APKGs if single mode selected
      if (
        generationStore.seriesOutputMode === "single" &&
        apkgPaths.length > 1 &&
        generationStore.effectiveExportFormat === "apkg"
      ) {
        generationStore.addLog(t("flashcards.mergingApkg"), "info");
        try {
          const mergedPath = await invoke<string>("flashcard_merge_apkg", {
            apkgPaths,
            outputPath: `${outputDir}/${generationStore.deckName.replace(/[^a-zA-Z0-9_\-\. ]/g, "_")}.apkg`,
          });
          finalApkgPath = mergedPath;
          generationStore.addLog(`APKG: ${mergedPath}`, "success");
        } catch (e) {
          generationStore.addLog(`${t("flashcards.mergeFailed")}: ${e}`, "error");
          hadError = true;
        }
      }

      if (finalApkgPath && generationStore.exportFormat === "anki" && !hadError) {
        await maybeAutoImportToAnki(finalApkgPath);
      }

      generationStore.result = {
        success: !hadError && totalCards > 0,
        message: errorMessages.length > 0 ? errorMessages.join(", ") : (totalCards === 0 ? "No active subtitle lines after filtering" : null),
        cardsGenerated: totalCards,
        audioClips: totalAudio,
        snapshots: totalSnapshots,
        videoClips: totalVideoClips,
        tsvPath: null,
        apkgPath: finalApkgPath,
      };

      generationStore.addLog(
        `${t("flashcards.seriesComplete", { total: String(episodes.length) })}`,
        "success",
      );

    } catch (e: any) {
      const errMsg = e ? e.toString() : "Unknown error";
      generationStore.error = `${t("flashcards.errorGenerating")}: ${errMsg}`;
      generationStore.addLog(`${generationStore.error}`, "error");
      generationStore.result = {
        success: false,
        message: errMsg,
        cardsGenerated: 0,
        audioClips: 0,
        snapshots: 0,
        videoClips: 0,
        tsvPath: null,
        apkgPath: null,
      };
    } finally {
      generationStore.isProcessing = false;
      seriesCurrentEpisode = 0;
      seriesTotalEpisodes = 0;
      generationStore.progress = 0;
      generationStore.progressMessage = "";
      generationStore.progressStage = "";
      const elapsed = Math.floor((Date.now() - startTime) / 1000);
      const hh = String(Math.floor(elapsed / 3600)).padStart(2, "0");
      const mm = String(Math.floor((elapsed % 3600) / 60)).padStart(2, "0");
      const ss = String(elapsed % 60).padStart(2, "0");
      generationStore.addLog(`⏱ ${hh}:${mm}:${ss}`, "info");
    }
  }

  async function startGeneration() {
    if (!canRunFlashcards) {
      promptMissingGenerationRequirements();
      generationStore.error = t("flashcards.requiredFieldsMissing");
      return;
    }

    if (seriesMode) {
      await startSeriesGeneration();
      return;
    }

    generationStore.error = null;
    generationStore.result = null;
    generationStore.progress = 0;
    generationStore.isProcessing = true;
    generationStore.addLog(`${t("flashcards.starting")}...`, "info");
    generationStore.addLog(`${t("flashcards.deckName")}: ${generationStore.deckName}`, "info");

    const startTime = Date.now();

    try {
      const config = buildConfig();

      await previewStore.applyOverrides(config);

      const res = await invoke<any>("flashcard_generate", { config });
      generationStore.result = {
        success: res.success,
        message: res.message || null,
        cardsGenerated: res.cards_generated,
        audioClips: res.audio_clips,
        snapshots: res.snapshots,
        videoClips: res.video_clips,
        tsvPath: res.tsv_path,
        apkgPath: res.apkg_path,
      };


      if (res.success) {
        generationStore.addLog(
          `${res.cards_generated} ${t("flashcards.cardsGenerated")}`,
          "success",
        );
        if (res.tsv_path) {
          generationStore.addLog(`TSV: ${res.tsv_path}`, "success");
        }
        if (res.apkg_path) {
          generationStore.addLog(`APKG: ${res.apkg_path}`, "success");
          if (generationStore.exportFormat === "anki") {
            await maybeAutoImportToAnki(res.apkg_path);
          }
        }
      } else {
        generationStore.addLog(res.message, "warning");
      }
    } catch (e: any) {
      const errMsg = e ? e.toString() : "Unknown error";
      generationStore.error = `${t("flashcards.errorGenerating")}: ${errMsg}`;
      generationStore.addLog(`${generationStore.error}`, "error");
      generationStore.result = {
        success: false,
        message: errMsg,
        cardsGenerated: 0,
        audioClips: 0,
        snapshots: 0,
        videoClips: 0,
        tsvPath: null,
        apkgPath: null,
      };
    } finally {
      generationStore.isProcessing = false;
      generationStore.progress = 0;
      generationStore.progressMessage = "";
      generationStore.progressStage = "";
      const elapsed = Math.floor((Date.now() - startTime) / 1000);
      const hh = String(Math.floor(elapsed / 3600)).padStart(2, "0");
      const mm = String(Math.floor((elapsed % 3600) / 60)).padStart(2, "0");
      const ss = String(elapsed % 60).padStart(2, "0");
      generationStore.addLog(`⏱ ${hh}:${mm}:${ss}`, "info");
    }
  }

  async function cancelGeneration() {
    try {
      await invoke("flashcard_cancel");
      generationStore.cancelRun();
      generationStore.addLog(`${t("flashcards.cancelled")}`, "warning");
    } catch (e) {
      generationStore.addLog(`Error cancelling: ${e}`, "error");
    }
  }

  function resetGeneration() {
    generationStore.resetRun();

    // Clear files so the user can insert new ones
    targetSubsPath = "";
    nativeSubsPath = "";
    mediaPath = "";
    mediaType = "none";
    audioTracks = [];
    mediaSettings.audioTrackIndex = null;
    audioTrackAutoSelected = true;
    targetSubsInfo = null;
    nativeSubsInfo = null;
    episodes = [];
    generationStore.deckName = "";
    generationStore.deckNameAuto = true;
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
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
        openEpisodeEditor(episodeContextMenu.idx);
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
    isDraggingOver = true;
  }}
  ondrop={handleHtmlDrop}
  ondragleave={(e) => {
    const rt = e.relatedTarget as HTMLElement | null;
    const ct = e.currentTarget as HTMLElement;
    if (rt && ct.contains(rt)) return;
    isDraggingOver = false;
  }}
>
  <div class="flex-1 overflow-y-auto overflow-x-hidden p-6 flashcards-scroll min-h-0 flex flex-col gap-4 {generationStore.isProcessing ? 'pointer-events-none opacity-60 select-none' : ''}">
  {#if isDraggingOver}
    <div
      class="absolute inset-0 z-50 {seriesMode ? 'bg-violet-500/10 border-violet-400/80 text-violet-400' : 'bg-emerald-500/10 border-emerald-400/80 text-emerald-400'} border-2 border-dashed rounded-2xl flex items-center justify-center pointer-events-none"
    >
      <div class="text-center">
        <svg
          class="w-16 h-16 mx-auto mb-3 {seriesMode ? 'text-violet-400' : 'text-emerald-400'}"
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
        <p class="text-lg font-medium {seriesMode ? 'text-violet-300' : 'text-emerald-300'}">
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
            generationStore.error = `${t("flashcards.ffmpegDownloadFailed")}: ${e}`;
          } finally {
            isDownloadingFFmpeg = false;
          }
        }}
        class="flex-shrink-0 px-3 py-1.5 rounded-lg bg-amber-500/20 border border-amber-500/40 text-amber-300 text-xs font-semibold hover:bg-amber-500/30 transition-colors flex items-center gap-1.5 disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {#if isDownloadingFFmpeg}
          <svg class="animate-spin w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
          {t("flashcards.downloading") || "Downloading..."}
        {:else}
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" /></svg>
          {t("flashcards.downloadAuto")}
        {/if}
      </button>
    </div>
  {/if}

  {#if previewStore.visible}
    <FlashcardsPreviewModal mediaPath={previewMediaPath} mediaType={previewMediaType} nativeSubsPath={nativeSubsPath} />
  {/if}

  {#snippet panelContent(panelId: PanelId)}
    {#if panelId === "files"}
      <FilesOutputPanel
        highlightClass={panelHighlightClass('files')}
        {seriesMode}
        onToggleSeriesMode={toggleSeriesMode}
        {episodes}
        onAddFiles={addSeriesMultipleFiles}
        onClearAll={clearAllEpisodes}
        {showSnackbar}
        onSwapAll={swapAllEpisodesSubs}
        onSwap={swapEpisodeSubs}
        onEdit={openEpisodeEditor}
        onMediaSettings={openEpisodeMediaSettings}
        onRemove={removeEpisode}
        onContextMenu={openEpisodeContextMenu}
        {targetSubsPath}
        {nativeSubsPath}
        {mediaPath}
        {outputDir}
        {activeNoteType}
        onExpand={(field) => (expandedPathField = field)}
        onSelectTarget={selectTargetSubs}
        onSelectNative={selectNativeSubs}
        onSelectMedia={selectMedia}
        onSelectOutput={selectOutputDir}
        onClearField={clearMovieFile}
      />
    {:else if panelId === "audioClips"}
      <AudioClipsPanel
        bind:settings={mediaSettings}
        {hasAudio}
        {mediaType}
        {audioTracks}
        {audioTracksLoading}
        hintLoadMediaFirst={HINT_LOAD_MEDIA_FIRST}
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
        bind:videoHwAccel
        {hasVideo}
        effectiveExportFormat={generationStore.effectiveExportFormat}
        hintLoadVideoFirst={HINT_LOAD_VIDEO_FIRST}
      />
    {:else if panelId === "cardFilters"}
      <CardFiltersPanel
        bind:filters={cardFilters}
        {hasAnyFiles}
        hintLoadTargetFirst={HINT_LOAD_TARGET_FIRST}
      />

    {:else if panelId === "naming"}
      <div
        inert={!hasAnyFiles}
        title={!hasAnyFiles ? HINT_LOAD_TARGET_FIRST : undefined}
        class="glass-card p-5 {panelHighlightClass('naming')} {!hasAnyFiles
          ? 'opacity-50'
          : ''}"
      >
        <h3
          class="text-lg font-semibold mb-4 flex items-center gap-2 text-amber-400"
        >
          <svg
            class="w-5 h-5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"
            />
          </svg>
          {t("flashcards.naming")}
        </h3>

        <div class="space-y-3">
          {#if needsDeckName}
            <div>
              <span class="block text-xs text-gray-400 mb-1">
                {t("flashcards.deckNameLabel")}
                <span class="text-red-400">*</span>
              </span>
              <input
                type="text"
                bind:value={generationStore.deckName}
                oninput={(event) => {
                  generationStore.deckNameAuto =
                    (event.currentTarget as HTMLInputElement).value.trim().length === 0;
                }}
                class="input-modern w-full text-sm"
                placeholder={t("flashcards.deckNamePlaceholder")}
              />
            </div>
          {:else}
            <div class="rounded-lg bg-violet-500/10 border border-violet-500/20 p-3">
              <div class="flex items-center gap-2 mb-1">
                <svg class="w-3.5 h-3.5 text-violet-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span class="text-xs font-medium text-violet-300">{t("flashcards.deckNameAutoLabel")}</span>
              </div>
              <p class="text-[10px] text-gray-400">{t("flashcards.deckNameAutoDesc")}</p>
            </div>
          {/if}

        </div>
      </div>

    {:else if panelId === "progressResult"}
      <div class="space-y-3">
        {#if generationStore.isProcessing || generationStore.progress > 0}
          <div
            class="glass-card p-5 {generationStore.isProcessing ? 'animate-pulse-glow' : ''}"
          >
            <div class="flex items-center gap-4">
              <div class="flex-1">
                <div class="progress-modern h-2">
                  <div
                    class="progress-modern-bar bg-gradient-to-r from-emerald-500 to-teal-500"
                    style="width: {generationStore.progress}%"
                  ></div>
                </div>
              </div>
              <span class="text-lg font-bold text-emerald-400">{generationStore.progress}%</span
              >
            </div>
            {#if generationStore.progressMessage}
              <p class="text-gray-400 text-xs mt-2">{generationStore.progressMessage}</p>
            {/if}
            {#if generationStore.progressStage}
              <div class="flex gap-1.5 mt-2">
                {#each Array(10) as _, i}
                  {@const threshold = (i + 1) * 10}
                  <div
                    class="h-1 flex-1 rounded-full transition-colors duration-300 {generationStore.progress >=
                    threshold
                      ? 'bg-emerald-700'
                      : generationStore.progress >= threshold - 10
                        ? 'bg-emerald-400'
                        : 'bg-gray-700'}"
                  ></div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
        {#if generationStore.result}
          <div
            class="glass-card p-5 border-l-4 {generationStore.result.success
              ? 'border-green-500 bg-green-500/5'
              : 'border-red-500 bg-red-500/5'}"
          >
            {#if generationStore.result.success}
              <div class="space-y-2">
                <div class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-400"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  <p class="text-green-400 font-medium">
                    {generationStore.result.cardsGenerated}
                    {t("flashcards.cardsGenerated")}
                  </p>
                </div>
                <div class="flex gap-4 text-xs text-gray-400">
                  {#if generationStore.result.audioClips > 0}
                    <span>🔊 {generationStore.result.audioClips} {t("flashcards.countAudio")}</span>
                  {/if}
                  {#if generationStore.result.snapshots > 0}
                    <span>📸 {generationStore.result.snapshots} {t("flashcards.countSnapshots")}</span>
                  {/if}
                  {#if generationStore.result.videoClips > 0}
                    <span>🎬 {generationStore.result.videoClips} {t("flashcards.countVideo")}</span>
                  {/if}
                </div>
                {#if generationStore.result.tsvPath}
                  <p
                    class="text-xs text-gray-500 break-words"
                    title={generationStore.result.tsvPath}
                  >
                    📄 {generationStore.result.tsvPath}
                  </p>
                {/if}
                {#if generationStore.result.apkgPath}
                  <p
                    class="text-xs text-gray-500 break-words"
                    title={generationStore.result.apkgPath}
                  >
                    📦 {generationStore.result.apkgPath}
                  </p>

                {/if}
              </div>
            {:else}
              <div class="flex items-center gap-3">
                <svg
                  class="w-5 h-5 text-red-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
                <p class="text-red-300">
                  {generationStore.result.message
                    ? (generationStore.result.message.includes("No active")
                      ? t("flashcards.noActiveLines")
                      : generationStore.result.message)
                    : t("flashcards.errorGenerating")}
                </p>
              </div>
            {/if}
          </div>
        {/if}
        {#if generationStore.error}
          <div class="glass-card p-5 border border-red-500/30 bg-red-500/10">
            <div class="flex items-center gap-3">
              <svg
                class="w-5 h-5 text-red-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <p class="text-red-300 flex-1 text-sm break-words">{generationStore.error}</p>
              <button
                onclick={() => (generationStore.error = null)}
                class="text-red-400 hover:text-red-300">✕</button
              >
            </div>
          </div>
        {/if}
      </div>
    {:else if panelId === "logs"}
      <LogPanel
        title={t("flashcards.logs")}
        clearLogText={t("flashcards.clearLog")}
        noLogText={t("translate.noLog")}
        logs={generationStore.logs}
        onclear={() => generationStore.clearLogs()}
        minHeight="180px"
        maxHeightContent="16rem"
      />
    {/if}
  {/snippet}

  <div bind:this={layoutHostEl} class="grid {gridColClass} gap-4 min-w-0">
    {#if seriesMode}
      <!-- In series mode, render the files panel full-width above the columns -->
      <div class="{effectiveColumnCount >= 2 ? 'col-span-2' : ''} {effectiveColumnCount >= 3 ? 'col-span-3' : ''} mb-1">
        {@render panelContent("files")}
      </div>
    {/if}
    <div class="space-y-3 min-w-0 pr-1 min-h-[100px]" role="list">
      {#each effectivePanelLayout.col1 as panelId, idx}
        {#if isPanelVisible(panelId)}
        <div class="relative" role="listitem">
          {@render panelContent(panelId)}
        </div>
        {/if}
      {/each}
    </div>

    {#if effectiveColumnCount >= 2}
      <div class="space-y-3 min-w-0 pr-1 min-h-[100px]" role="list">
        {#each effectivePanelLayout.col2 as panelId, idx}
          {#if isPanelVisible(panelId)}
          <div class="relative" role="listitem">
            {@render panelContent(panelId)}
          </div>
          {/if}
        {/each}
      </div>
    {/if}

    {#if effectiveColumnCount >= 3}
      <div class="space-y-3 min-w-0 pr-1 min-h-[100px]" role="list">
        {#each effectivePanelLayout.col3 as panelId, idx}
          {#if isPanelVisible(panelId)}
          <div class="relative" role="listitem">
            {@render panelContent(panelId)}
          </div>
          {/if}
        {/each}
      </div>
    {/if}
  </div>

  {#if episodeContextMenu}
    <EpisodeContextMenu
      x={episodeContextMenu.x}
      y={episodeContextMenu.y}
      hasMedia={Boolean(episodes[episodeContextMenu.idx]?.mediaPath)}
      onEdit={() => openEpisodeEditor(episodeContextMenu!.idx)}
      onMediaSettings={() => openEpisodeMediaSettings(episodeContextMenu!.idx)}
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

  {#if editingEpisode}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/65 p-6"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={closeEpisodeEditor}
      onkeydown={(e) => {
        if (e.key === "Escape") closeEpisodeEditor();
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
              {t("common.edit")} {getFileName(editingEpisode.targetSubsPath || editingEpisode.nativeSubsPath || editingEpisode.mediaPath) || `episodio ${editingEpisode.id}`}
            </h3>
          </div>
          <button
            type="button"
            onclick={closeEpisodeEditor}
	            class="dialog-close-button text-xl leading-none text-gray-400 hover:text-white"
          >×</button>
        </div>

        <div class="space-y-3">
          {#each episodeEditorFields as item}
            {@const field = item.field}
            {@const label = t(`flashcards.${item.labelKey}`)}
            {@const placeholder = t(`flashcards.${item.placeholderKey}`)}
            {@const isFieldDisabled = field === "nativeSubsPath" ? !activeNoteType.included.meaning : field === "mediaPath" ? (!activeNoteType.included.audio && !activeNoteType.included.snapshot && !activeNoteType.included.video) : false}
            <div>
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
                    const value = editingEpisode?.[field];
                    if (value) expandedPathField = field as string;
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
                  onclick={() => selectEpisodeFile(field)}
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
                    onclick={() => {
                      if (!editingEpisode || !editingEpisode[field]) return;
                      editingEpisode = {
                        ...editingEpisode,
                        [field]: "",
                        mediaType: field === "mediaPath" ? "none" : editingEpisode.mediaType,
                      };
                      syncEpisodeEditor();
                    }}
                  >
                    <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                    </svg>
                  </button>
              </div>
            </div>
          {/each}
        </div>

        <div class="mt-5 flex justify-end gap-2">
          <button
            type="button"
            onclick={revertEpisodeEditor}
            class="btn-secondary px-4 py-2 text-sm"
          >
            {t("common.restore")}
          </button>
          <button
            type="button"
            onclick={closeEpisodeEditor}
            class="rounded-lg border border-emerald-400/40 bg-emerald-500/20 px-4 py-2 text-sm font-semibold text-emerald-100 shadow-lg shadow-emerald-500/10 transition-all hover:border-emerald-300/60 hover:bg-emerald-500/30 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {t("common.done")}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <EpisodeMediaSettingsModal {mediaOverrideClass} onSave={saveEpisodeMediaSettings} onReset={resetEpisodeMediaSettings} />

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
    value={expandedPathField === "targetSubs"
      ? targetSubsPath
      : expandedPathField === "targetSubsPath"
        ? editingEpisode?.targetSubsPath || ""
      : expandedPathField === "output"
        ? outputDir
        : expandedPathField === "nativeSubs"
          ? nativeSubsPath
          : expandedPathField === "nativeSubsPath"
            ? editingEpisode?.nativeSubsPath || ""
            : expandedPathField === "mediaPath"
              ? editingEpisode?.mediaPath || ""
              : mediaPath}
    onclose={() => (expandedPathField = null)}
  />
  </div>

  <!-- Fixed Bottom Band with Action Buttons -->
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
      onCycleTemplates={cycleTemplates}
      onNoteTypeContextMenu={(e) => openBottomContextMenu(e, "anki")}
      onNoteTypeMiddleClick={() => onGoToSettings?.("anki")}
      {showSnackbar}
    />
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
              oncontextmenu={(e) => openBottomContextMenu(e, "overview")}
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
              {generationStore.exportFormat === 'apkg' ? 'APKG' : generationStore.exportFormat === 'tsv' ? 'TSV' : 'Anki Connect'}
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

        <!-- Series output mode inline selector (only visible in series mode + apkg) -->
        {#if seriesMode && generationStore.effectiveExportFormat === "apkg"}
          <div class="flex items-center bg-gray-800/60 border border-gray-700/60 rounded-lg p-0.5 select-none relative group/sw {generationStore.isProcessing || generationStore.result ? 'opacity-50 cursor-not-allowed pointer-events-none' : ''}">
            <!-- Sliding indicator background -->
            <div 
              class="absolute top-0.5 bottom-0.5 left-0.5 rounded-md bg-violet-500/20 border border-violet-500/50 transition-all duration-200 ease-out"
              style="width: 160px; transform: translateX({generationStore.seriesOutputMode === 'separate' ? '0px' : '160px'});"
            ></div>

            <button
              onclick={() => { if(!generationStore.isProcessing && !generationStore.result) generationStore.seriesOutputMode = generationStore.seriesOutputMode === 'separate' ? 'single' : 'separate'; }}
              disabled={generationStore.isProcessing || !!generationStore.result}
              class="w-[160px] py-1 rounded-md text-xs font-semibold transition-colors duration-200 flex items-center justify-center cursor-pointer select-none relative z-10 disabled:cursor-not-allowed
                {generationStore.seriesOutputMode === 'separate' ? 'text-violet-200' : 'text-gray-500 hover:text-gray-300'}"
            >
              {t("flashcards.outputPerEpisode")}
            </button>
            <button
              onclick={() => { if(!generationStore.isProcessing && !generationStore.result) generationStore.seriesOutputMode = generationStore.seriesOutputMode === 'separate' ? 'single' : 'separate'; }}
              disabled={generationStore.isProcessing || !!generationStore.result}
              class="w-[160px] py-1 rounded-md text-xs font-semibold transition-colors duration-200 flex items-center justify-center cursor-pointer select-none relative z-10 disabled:cursor-not-allowed
                {generationStore.seriesOutputMode === 'single' ? 'text-violet-200' : 'text-gray-500 hover:text-gray-300'}"
            >
              {t("flashcards.outputSingleApkg")}
            </button>

            <!-- Custom premium tooltip -->
            {#if !generationStore.isProcessing && !generationStore.result}
              <div 
                class="pointer-events-none absolute bottom-full z-50 mb-3 -translate-x-1/2 rounded-xl border border-violet-500/30 bg-gray-950/95 p-3 text-center text-xs text-violet-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover/sw:delay-300 opacity-0 group-hover/sw:opacity-100 group-hover/sw:translate-y-0 translate-y-1 whitespace-normal max-w-[280px] w-max"
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
            onclick={loadPreview}
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
          onclick={cancelGeneration}
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
              if (!canRunFlashcards) promptMissingGenerationRequirements();
            }}
            onmouseleave={closeGenerationPrompt}
            onfocusout={closeGenerationPrompt}
            onkeydown={(event) => {
              if (!canRunFlashcards && (event.key === "Enter" || event.key === " ")) {
                event.preventDefault();
                promptMissingGenerationRequirements();
              }
            }}
          >
            <button
              onclick={startGeneration}
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
              onclick={resetGeneration}
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
</div>

<style>
  /* Responsive button labels: hide text on narrow screens */
  @media (max-width: 900px) {
    .episode-btn-label {
      display: none;
    }
  }

  /* Media settings panels: side by side on wide, stacked on narrow */
  .media-settings-panels {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 1.25rem;
  }

  .filter-pill-check {
    align-items: center;
    border: 1px solid rgba(148, 163, 184, 0.16);
    border-radius: 7px;
    cursor: pointer;
    display: flex;
    min-height: 2.15rem;
    padding: 0.48rem 0.65rem;
    text-align: center;
    transition:
      background-color 0.16s ease,
      border-color 0.16s ease,
      color 0.16s ease;
  }

  .filter-pill-check:hover {
    background: rgba(148, 163, 184, 0.08);
    border-color: rgba(148, 163, 184, 0.28);
  }

  .filter-pill-check:has(input:checked) {
    background: rgba(99, 102, 241, 0.14);
    border-color: rgba(129, 140, 248, 0.4);
  }

  .filter-pill-check:has(input:checked) span {
    color: rgb(199 210 254);
  }

  .vesta-check-row {
    align-items: center;
    border: 1px solid rgba(148, 163, 184, 0.12);
    border-radius: 8px;
    cursor: pointer;
    display: flex;
    gap: 0.5rem;
    min-height: 2.1rem;
    padding: 0.4rem 0.55rem;
    transition:
      background-color 0.16s ease,
      border-color 0.16s ease;
  }

  .vesta-check-row:hover {
    background: rgba(148, 163, 184, 0.07);
    border-color: rgba(148, 163, 184, 0.22);
  }

  .vesta-check-row:has(.vesta-check-input:checked) {
    background: rgba(99, 102, 241, 0.1);
    border-color: rgba(129, 140, 248, 0.32);
  }

  .vesta-check-input {
    appearance: none;
    background: rgba(15, 23, 42, 0.85);
    border: 1px solid rgba(148, 163, 184, 0.38);
    border-radius: 5px;
    display: grid;
    height: 0.95rem;
    margin: 0;
    place-content: center;
    width: 0.95rem;
  }

  .vesta-check-input::before {
    background: rgb(199 210 254);
    clip-path: polygon(14% 44%, 0 60%, 40% 100%, 100% 16%, 84% 0, 38% 62%);
    content: "";
    height: 0.55rem;
    opacity: 0;
    transform: scale(0.75);
    transition: opacity 0.12s ease, transform 0.12s ease;
    width: 0.55rem;
  }

  .vesta-check-input:checked {
    background: rgba(79, 70, 229, 0.35);
    border-color: rgba(165, 180, 252, 0.72);
  }

	  .vesta-check-input:checked::before {
	    opacity: 1;
	    transform: scale(1);
	  }

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

	  .media-override-glow,
	  :global(.media-override-glow) {
	    border-color: rgba(167, 139, 250, 0.72) !important;
	    box-shadow:
	      0 0 0 1px rgba(167, 139, 250, 0.28),
	      0 0 18px rgba(167, 139, 250, 0.22) !important;
	  }

	  :global(.media-override-glow .searchable-select-input) {
	    border-color: rgba(167, 139, 250, 0.72) !important;
	    box-shadow:
	      0 0 0 1px rgba(167, 139, 250, 0.28),
	      0 0 18px rgba(167, 139, 250, 0.22) !important;
	  }

	  .media-tab-dot {
	    background: rgb(167 139 250);
	    border-radius: 9999px;
	    box-shadow: 0 0 10px rgba(167, 139, 250, 0.8);
	    height: 0.42rem;
	    position: absolute;
	    right: 0.35rem;
	    top: 0.35rem;
	    width: 0.42rem;
	  }

	  :global(.flashcard-requirement-pulse) {
	    animation: flashcard-requirement-pulse 0.9s ease-in-out 2;
	    border-color: rgba(251, 191, 36, 0.75) !important;
    box-shadow:
      0 0 0 1px rgba(251, 191, 36, 0.3),
      0 0 24px rgba(251, 191, 36, 0.24);
  }

  @keyframes flashcard-requirement-pulse {
    0%,
    100% {
      border-color: rgba(251, 191, 36, 0.35);
      box-shadow: 0 0 0 0 rgba(251, 191, 36, 0);
    }

    45% {
      border-color: rgba(251, 191, 36, 0.9);
      box-shadow:
        0 0 0 1px rgba(251, 191, 36, 0.45),
        0 0 28px rgba(251, 191, 36, 0.36);
    }
  }

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

