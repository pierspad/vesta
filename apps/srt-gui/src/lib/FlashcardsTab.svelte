<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { guardedOpen } from "./dialogGuard";
  import { onDestroy, onMount } from "svelte";
  import { locale } from "./i18n";
  import {
    CARD_TEMPLATES_UPDATED_EVENT,
    detectLanguageCode,
    getLanguageSearchTerms,
    languages,
    loadCardTemplates,
    loadFieldNames,
    scoreLanguageMatch,
  } from "./models";
  import PathPreviewModal from "./PathPreviewModal.svelte";
  import SearchableSelect from "./SearchableSelect.svelte";
  import LogPanel, { type LogEntry } from "./LogPanel.svelte";
  import InfoModal from "./InfoModal.svelte";
  import InfoButton from "./InfoButton.svelte";
  import CodeEditor from "./CodeEditor.svelte";
  import Snackbar from "./Snackbar.svelte";
  import { flashcardsSections } from "./info";

  const SUBTITLE_EXTENSIONS = ["srt", "ass", "ssa", "vtt"];

  interface Props {
    active?: boolean;
  }

  let { active = true }: Props = $props();

  let t = $derived($locale);

  let targetSubsPath = $state("");
  let nativeSubsPath = $state("");
  let mediaPath = $state("");
  let mediaType = $state<"none" | "video" | "audio">("none");
  let outputDir = $state("");

  interface AudioTrackInfo {
    index: number;
    stream_index: number;
    codec: string | null;
    language: string | null;
    title: string | null;
    channels: number | null;
  }

  let audioTracks = $state<AudioTrackInfo[]>([]);
  let selectedAudioTrackIndex = $state<number | null>(null);
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

  let smartFileMatchingEnabled = $state(true);

  interface SmartMatchingRules {
    episodeRegexes: string[];
    originalSubtitleHints: string[];
    referenceSubtitleHints: string[];
    removableNameTokens: string[];
  }

  const DEFAULT_SMART_MATCHING_RULES: SmartMatchingRules = {
    episodeRegexes: [
      "[Ss]\\d{1,2}[Ee](\\d{1,4})",
      "[Pp]art(?:e)?[\\s_\\-.]?(\\d{1,4})",
      "[Ee][Pp]?\\.?\\s*(\\d{1,4})",
      "[Ee]pisode\\.?\\s*(\\d{1,4})",
      "[Xx](\\d{1,4})",
      "[\\s_\\-.](\\d{1,4})[\\s_\\-.]",
      "^(\\d{1,4})[\\s_\\-.]",
      "[\\s_\\-.](\\d{1,4})$",
    ],
    originalSubtitleHints: ["native", "original", "orig", "source"],
    referenceSubtitleHints: [
      "translated",
      "translation",
      "tradotto",
      "traduzione",
      "reference",
      "ref",
    ],
    removableNameTokens: [
      "720p",
      "1080p",
      "2160p",
      "4k",
      "bluray",
      "brrip",
      "webrip",
      "web-dl",
      "webdl",
      "hdtv",
      "dvdrip",
      "x264",
      "x265",
      "h264",
      "h265",
      "aac",
      "dts",
    ],
  };

  function normalizeSmartMatchingRules(value: unknown): SmartMatchingRules {
    const fallback = DEFAULT_SMART_MATCHING_RULES;
    const obj = value && typeof value === "object" ? value as Partial<SmartMatchingRules> : {};
    const cleanStringArray = (items: unknown, fallbackItems: string[]) =>
      Array.isArray(items)
        ? items.filter((item): item is string => typeof item === "string" && item.trim().length > 0)
        : fallbackItems;

    return {
      episodeRegexes: cleanStringArray(obj.episodeRegexes, fallback.episodeRegexes),
      originalSubtitleHints: cleanStringArray(obj.originalSubtitleHints, fallback.originalSubtitleHints),
      referenceSubtitleHints: cleanStringArray(obj.referenceSubtitleHints, fallback.referenceSubtitleHints),
      removableNameTokens: cleanStringArray(obj.removableNameTokens, fallback.removableNameTokens),
    };
  }

  function loadSmartMatchingRules(): SmartMatchingRules {
    try {
      const saved = localStorage.getItem(SMART_MATCHING_RULES_KEY);
      return saved
        ? normalizeSmartMatchingRules(JSON.parse(stripJsonComments(saved)))
        : normalizeSmartMatchingRules(DEFAULT_SMART_MATCHING_RULES);
    } catch {
      return normalizeSmartMatchingRules(DEFAULT_SMART_MATCHING_RULES);
    }
  }

  function parseSmartMatchingRulesDraft(): SmartMatchingRules {
    const parsed = normalizeSmartMatchingRules(JSON.parse(stripJsonComments(smartMatchingRulesDraft)));
    parsed.episodeRegexes.forEach((pattern) => new RegExp(pattern, "i"));
    return parsed;
  }

  function getSmartMatchingRulesDraftError(): string | null {
    try {
      parseSmartMatchingRulesDraft();
      return null;
    } catch (e) {
      return `${t("flashcards.smartMatchingInvalid")}: ${e}`;
    }
  }

  function formatJsonArray(items: string[], indent = 2): string {
    const spaces = " ".repeat(indent);
    return items
      .map((item) => `${spaces}${JSON.stringify(item)}`)
      .join(",\n");
  }

  function formatSmartMatchingRules(rules: SmartMatchingRules): string {
    const episodeRegexDescriptions = [
      "S01E02 / S1E2 -> captures 02 or 2",
      "Part 02 / Parte 02 -> captures 02",
      "Ep. 02 / E02 -> captures 02",
      "Episode 02 -> captures 02",
      "x02 -> captures 02, useful for names like Show x02",
      "Episode number between separators: space, underscore, hyphen, or dot",
      "Episode number at the beginning of the filename",
      "Episode number at the end of the filename",
    ];
    const episodeRegexes = rules.episodeRegexes
      .map((pattern, index) => {
        const description =
          episodeRegexDescriptions[index] || "Custom pattern: it must capture the episode number";
        return `    // ${description}\n    ${JSON.stringify(pattern)}`;
      })
      .join(",\n");

    return `{
  // episodeRegexes: regexes tested from top to bottom on the filename.
  // The first capture group (\\d...) must be the episode number to match.
  "episodeRegexes": [
${episodeRegexes}
  ],

  // originalSubtitleHints: words that mark a subtitle file as the original track.
  "originalSubtitleHints": [
${formatJsonArray(rules.originalSubtitleHints, 4)}
  ],

  // referenceSubtitleHints: words that mark a subtitle file as the reference translation.
  "referenceSubtitleHints": [
${formatJsonArray(rules.referenceSubtitleHints, 4)}
  ],

  // removableNameTokens: technical tokens removed before comparing series/episode names.
  "removableNameTokens": [
${formatJsonArray(rules.removableNameTokens, 4)}
  ]
}`;
  }

  function stripJsonComments(value: string): string {
    let output = "";
    let inString = false;
    let inLineComment = false;
    let inBlockComment = false;
    let escaped = false;

    for (let i = 0; i < value.length; i += 1) {
      const char = value[i];
      const next = value[i + 1];

      if (inLineComment) {
        if (char === "\n") {
          inLineComment = false;
          output += char;
        }
        continue;
      }

      if (inBlockComment) {
        if (char === "*" && next === "/") {
          inBlockComment = false;
          i += 1;
        } else if (char === "\n") {
          output += "\n";
        }
        continue;
      }

      if (!inString && char === "/" && next === "/") {
        inLineComment = true;
        i += 1;
        continue;
      }

      if (!inString && char === "/" && next === "*") {
        inBlockComment = true;
        i += 1;
        continue;
      }

      output += char;

      if (char === '"' && !escaped) {
        inString = !inString;
      }

      escaped = inString && char === "\\" && !escaped;
      if (char !== "\\") escaped = false;
    }

    return output;
  }

  function escapeRegExp(value: string): string {
    return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  let smartMatchingRules = $state<SmartMatchingRules>(loadSmartMatchingRules());
  let showSmartMatchingDialog = $state(false);
  let smartMatchingRulesDraft = $state("");
  let smartMatchingRulesError = $state<string | null>(null);
  let episodeContextMenu = $state<{ x: number; y: number; idx: number } | null>(null);

  function openSmartMatchingDialog() {
    smartMatchingRulesDraft = formatSmartMatchingRules(smartMatchingRules);
    smartMatchingRulesError = null;
    showSmartMatchingDialog = true;
  }

  function closeSmartMatchingDialog() {
    showSmartMatchingDialog = false;
    smartMatchingRulesError = null;
  }

  function saveSmartMatchingRules() {
    try {
      const parsed = parseSmartMatchingRulesDraft();
      smartMatchingRules = parsed;
      localStorage.setItem(SMART_MATCHING_RULES_KEY, formatSmartMatchingRules(parsed));
      closeSmartMatchingDialog();
    } catch (e) {
      smartMatchingRulesError = `${t("flashcards.smartMatchingInvalid")}: ${e}`;
    }
  }

  function resetSmartMatchingRules() {
    if (!window.confirm(t("flashcards.smartMatchingResetConfirm"))) return;
    smartMatchingRules = normalizeSmartMatchingRules(DEFAULT_SMART_MATCHING_RULES);
    smartMatchingRulesDraft = formatSmartMatchingRules(smartMatchingRules);
    localStorage.removeItem(SMART_MATCHING_RULES_KEY);
    smartMatchingRulesError = null;
  }

  function copySmartMatchingRules() {
    navigator.clipboard.writeText(smartMatchingRulesDraft);
    showSnackbar(t("flashcards.copiedSmartMatching"), "success");
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
  interface EpisodeMediaOverrides {
    generateAudio?: boolean;
    audioBitrate?: number;
    audioTrackIndex?: number | null;
    normalizeAudio?: boolean;
    audioPadStart?: number;
    audioPadEnd?: number;
    generateSnapshots?: boolean;
    snapshotWidth?: number;
    snapshotHeight?: number;
    cropBottom?: number;
    generateVideoClips?: boolean;
    videoCodec?: string;
    h264Preset?: string;
    videoBitrate?: number;
    videoAudioBitrate?: number;
    videoPadStart?: number;
    videoPadEnd?: number;
  }

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
  let seriesOutputMode = $state<"single" | "separate">("separate");
  let seriesCurrentEpisode = $state(0);
  let seriesTotalEpisodes = $state(0);
  let editingEpisodeIndex = $state<number | null>(null);
  let editingEpisode = $state<EpisodeEntry | null>(null);
  let editingMediaEpisodeIndex = $state<number | null>(null);
  let editingMediaEpisode = $state<EpisodeEntry | null>(null);
  let editingMediaOverrides = $state<Required<EpisodeMediaOverrides> | null>(null);
  let editingMediaTab = $state<"audio" | "snapshot" | "video">("audio");
  let episodeAudioTracks = $state<AudioTrackInfo[]>([]);
  let episodeAudioTracksLoading = $state(false);
  let initialEditingMediaOverridesStr = $state("");
  let initialEditingEpisodeStr = $state("");
  let snackbarMessage = $state<string | null>(null);
  let snackbarTimeout = $state<ReturnType<typeof setTimeout> | null>(null);
  let snackbarKey = $state(0);

  let snackbarVariant = $state<"success" | "info" | "warning" | "error">("info");

  function showSnackbar(message: string, variant: "success" | "info" | "warning" | "error" = "info") {
    if (snackbarTimeout) clearTimeout(snackbarTimeout);
    snackbarKey += 1;
    snackbarMessage = message;
    snackbarVariant = variant;
    snackbarTimeout = setTimeout(() => {
      snackbarMessage = null;
    }, 1300);
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
    return {
      generateAudio,
      audioBitrate,
      audioTrackIndex: selectedAudioTrackIndex,
      normalizeAudio,
      audioPadStart,
      audioPadEnd,
      generateSnapshots,
      snapshotWidth,
      snapshotHeight,
      cropBottom,
      generateVideoClips,
      videoCodec,
      h264Preset,
      videoBitrate,
      videoAudioBitrate,
      videoPadStart,
      videoPadEnd,
    };
  }

  function getEpisodeMediaSettings(ep: EpisodeEntry): Required<EpisodeMediaOverrides> {
    return {
      ...getGenericMediaSettings(),
      ...(ep.mediaOverrides || {}),
    };
  }

  function episodeHasMediaOverrides(ep: EpisodeEntry): boolean {
    return Boolean(ep.mediaOverrides && Object.keys(ep.mediaOverrides).length > 0);
  }

  type EpisodeMediaOverrideKey = keyof EpisodeMediaOverrides;

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
    if (!editingMediaOverrides) return false;
    const genericSettings = getGenericMediaSettings();
    
    if (key === "audioTrackIndex" && editingMediaEpisode && genericSettings.audioTrackIndex === null) {
      const autoPicked = pickBestAudioTrackIndex(
        episodeAudioTracks,
        getPreferredAudioLanguageCodeForEpisode(editingMediaEpisode)
      );
      if (editingMediaOverrides.audioTrackIndex === autoPicked) {
        return false;
      }
    }

    return editingMediaOverrides[key] !== genericSettings[key];
  }

  function mediaOverrideClass(key: EpisodeMediaOverrideKey): string {
    return mediaOverrideValueChanged(key)
      ? "media-override-glow"
      : "";
  }

  function mediaOverrideGroupHasChanges(keys: EpisodeMediaOverrideKey[]): boolean {
    return keys.some((key) => mediaOverrideValueChanged(key));
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

  function getPreferredAudioLanguageCodeForEpisode(ep: EpisodeEntry): string {
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
        const name = f.split("/").pop() || f;
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
      error = `${t("flashcards.errorSelectingFile")}: ${e}`;
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
      error = `${t("flashcards.errorSelectingFile")}: ${e}`;
    }
  }

  function canClearMovieFile(field: "target" | "native" | "media") {
    return field === "target"
      ? !!targetSubsPath
      : field === "native"
        ? !!nativeSubsPath
        : !!mediaPath;
  }

  function clearMovieFileButtonClass(field: "target" | "native" | "media") {
    return canClearMovieFile(field)
      ? "border-red-500/30 bg-red-500/10 text-red-300 hover:border-red-400/60 hover:bg-red-500/20"
      : "cursor-not-allowed border-white/10 bg-white/5 text-gray-600 opacity-60";
  }

  function timingSourceFieldClass(field: "target" | "native") {
    return showTimingFlash && useTimingsFrom === field ? "timing-source-flash" : "";
  }

  function clearMovieFile(field: "target" | "native" | "media") {
    if (!canClearMovieFile(field)) return;

    if (field === "target") {
      targetSubsPath = "";
      targetSubsInfo = null;
    } else if (field === "native") {
      nativeSubsPath = "";
      nativeSubsInfo = null;
      if (useTimingsFrom === "native") useTimingsFrom = "target";
    } else {
      mediaPath = "";
      mediaType = "none";
      audioTracks = [];
      selectedAudioTrackIndex = null;
      audioTrackAutoSelected = true;
      generateSnapshots = false;
      generateVideoClips = false;
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
    editingMediaEpisodeIndex = idx;
    editingMediaEpisode = { ...episode };
    editingMediaTab = "audio";
    episodeAudioTracks = [];
    episodeAudioTracksLoading = episode.mediaType === "video";
    editingMediaOverrides = getEpisodeMediaSettings(episode);

    if (episode.mediaType === "video") {
      const tracks = await listAudioTracksForEpisode(episode);
      episodeAudioTracks = tracks;
      if (
        editingMediaEpisodeIndex === idx &&
        editingMediaOverrides &&
        episode.mediaOverrides?.audioTrackIndex === undefined
      ) {
        editingMediaOverrides = {
          ...editingMediaOverrides,
          audioTrackIndex: pickBestAudioTrackIndex(
            tracks,
            getPreferredAudioLanguageCodeForEpisode(episode),
          ),
        };
      }
      episodeAudioTracksLoading = false;
    }
    
    if (editingMediaOverrides) {
      initialEditingMediaOverridesStr = JSON.stringify(editingMediaOverrides);
    }
  }

  function closeEpisodeEditor() {
    editingEpisodeIndex = null;
    editingEpisode = null;
    initialEditingEpisodeStr = "";
  }

  function closeEpisodeMediaSettings() {
    editingMediaEpisodeIndex = null;
    editingMediaEpisode = null;
    editingMediaOverrides = null;
    episodeAudioTracks = [];
    episodeAudioTracksLoading = false;
    initialEditingMediaOverridesStr = "";
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

  function updateEditingMediaOverride<K extends keyof EpisodeMediaOverrides>(
    key: K,
    value: EpisodeMediaOverrides[K],
  ) {
    if (!editingMediaOverrides) return;
    editingMediaOverrides = {
      ...editingMediaOverrides,
      [key]: value,
    };
  }

  function saveEpisodeMediaSettings() {
    if (editingMediaEpisodeIndex === null || !editingMediaOverrides) return;
    const mediaOverrides = buildEpisodeMediaOverrideDiff(editingMediaOverrides);
    episodes = episodes.map((episode, idx) =>
      idx === editingMediaEpisodeIndex
        ? {
            ...episode,
            mediaOverrides: Object.keys(mediaOverrides).length > 0 ? mediaOverrides : undefined,
          }
        : episode,
    );
    closeEpisodeMediaSettings();
  }

  function resetEpisodeMediaSettings() {
    if (editingMediaEpisodeIndex === null) return;
    episodes = episodes.map((episode, idx) =>
      idx === editingMediaEpisodeIndex
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
      error = `${t("flashcards.errorSelectingFile")}: ${e}`;
    }
  }

  function clearAllEpisodes() {
    episodes = [];
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

  let useTimingsFrom = $state<"target" | "native">("target");
  let hasReferenceSubs = $derived(
    seriesMode
      ? episodes.some((ep) => ep.nativeSubsPath !== "")
      : nativeSubsPath !== "",
  );
  let spanStart = $state("");
  let spanEnd = $state("");
  let timeShiftTarget = $state(0);
  let timeShiftNative = $state(0);

  $effect(() => {
    if (!hasReferenceSubs && useTimingsFrom === "native") {
      useTimingsFrom = "target";
    }
  });

  let showSubtitleOptions = $state(false);
  let showContextLines = $state(false);
  let showFilters = $state(false);
  let showAnkiFields = $state(loadAnkiFieldsPanelOpen());

  function loadAnkiFieldsPanelOpen(): boolean {
    try {
      const saved = localStorage.getItem(ANKI_FIELDS_PANEL_OPEN_KEY);
      return saved === null ? true : saved === "true";
    } catch {
      return true;
    }
  }

  function toggleAnkiFieldsPanel() {
    showAnkiFields = !showAnkiFields;
    localStorage.setItem(ANKI_FIELDS_PANEL_OPEN_KEY, String(showAnkiFields));
  }
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
  let includeWords = $state("");
  let excludeWords = $state("");
  let excludeDuplicatesSubs1 = $state(false);
  let excludeDuplicatesSubs2 = $state(false);
  let minChars = $state<number | null>(null);
  let maxChars = $state<number | null>(null);
  let minDurationMs = $state<number | null>(null);
  let maxDurationMs = $state<number | null>(null);
  let excludeStyled = $state(false);
  let actorFilter = $state("");
  let onlyCjk = $state(false);
  let removeNoMatch = $state(false);

  let timingFlashTimer: ReturnType<typeof setTimeout> | null = null;
  let showTimingFlash = $state(false);
  let timingFlashKey = $state(0);

  function triggerTimingFlash() {
    timingFlashKey += 1;
    showTimingFlash = true;
    if (timingFlashTimer) clearTimeout(timingFlashTimer);
    timingFlashTimer = setTimeout(() => {
      showTimingFlash = false;
    }, 1200);
  }

  function handleTimingSourceChange(nextSource: "target" | "native") {
    const previousSource = useTimingsFrom;
    if (previousSource === nextSource) return;

    useTimingsFrom = nextSource;
    triggerTimingFlash();
  }

  let contextLeading = $state(0);
  let contextTrailing = $state(0);
  let contextMaxGap = $state(15.0);

  let combineSentences = $state(false);
  let continuationChars = $state(",、→");

  let generateAudio = $state(true);
  let audioBitrate = $state(128);
  let audioPadStart = $state(0);
  let audioPadEnd = $state(0);
  let normalizeAudio = $state(false);

  let generateSnapshots = $state(true);
  let snapshotWidth = $state(loadStoredDimension(FLASHCARD_MEDIA_WIDTH_KEY, DEFAULT_FLASHCARD_MEDIA_WIDTH));
  let snapshotHeight = $state(loadStoredDimension(FLASHCARD_MEDIA_HEIGHT_KEY, DEFAULT_FLASHCARD_MEDIA_HEIGHT));
  let cropBottom = $state(0);

  let generateVideoClips = $state(false);
  let videoCodec = $state("h264");
  let h264Preset = $state("medium");
  let videoBitrate = $state(800);
  let videoAudioBitrate = $state(128);
  let videoPadStart = $state(250);
  let videoPadEnd = $state(50);

  $effect(() => {
    persistDimension(FLASHCARD_MEDIA_WIDTH_KEY, snapshotWidth);
  });

  $effect(() => {
    persistDimension(FLASHCARD_MEDIA_HEIGHT_KEY, snapshotHeight);
  });

  let exportFormat = $state<"tsv" | "apkg">("apkg");

  let systemCpuCount = $state(4);
  let cpuCores = $state(2); // will be set properly onMount
  let minCpuCores = $derived(2);
  let maxCpuCores = $derived(Math.max(2, systemCpuCount - 1));

  // CPU preset definitions (evenly spaced between min and max cores)
  let cpuPresets = $derived([
    { id: "eco", threads: minCpuCores },
    {
      id: "balanced",
      threads: minCpuCores + Math.ceil((maxCpuCores - minCpuCores) / 3),
    },
    {
      id: "performance",
      threads: minCpuCores + Math.ceil(((maxCpuCores - minCpuCores) * 2) / 3),
    },
    { id: "full", threads: maxCpuCores },
  ] as const);

  let activeCpuPreset = $derived(
    cpuPresets.find((p) => p.threads === cpuCores)?.id ?? null,
  );

  function setCpuPreset(presetId: string) {
    const preset = cpuPresets.find((p) => p.id === presetId);
    if (preset) cpuCores = preset.threads;
  }

  const PANEL_IDS = [
    "files",
    "subtitleOptions",
    "filters",
    "contextLines",
    "audioClips",
    "snapshots",
    "videoClips",
    "ankiFields",
    "exportFormat",
    "naming",
    "cpuCores",
    "actions",
    "progressResult",
    "logs",
  ] as const;

  type PanelId = (typeof PANEL_IDS)[number];

  interface ColumnLayout {
    col1: PanelId[];
    col2: PanelId[];
    col3: PanelId[];
  }

  const MOVIE_LAYOUT_KEY = "vesta-flashcards-layout-v3";
  const SERIES_LAYOUT_KEY = "vesta-flashcards-series-layout-v3";

  const DEFAULT_LAYOUT: ColumnLayout = {
    col1: ["files", "subtitleOptions", "contextLines", "filters"],
    col2: ["naming", "audioClips", "snapshots", "videoClips", "ankiFields"],
    col3: ["exportFormat", "cpuCores", "actions", "progressResult", "logs"],
  };

  const DEFAULT_SERIES_LAYOUT: ColumnLayout = {
    col1: ["files", "cpuCores", "ankiFields", "subtitleOptions", "contextLines", "filters"],
    col2: ["naming", "audioClips", "snapshots", "videoClips"],
    col3: ["exportFormat", "actions", "progressResult", "logs"],
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
  const STACK_TO_ONE_COLUMN_WIDTH = 1040;
  const STACK_TO_TWO_COLUMNS_WIDTH = 1240;
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

  let effectivePanelLayout = $derived.by(() => {
    if (effectiveColumnCount === 3) {
      return {
        col1: [...panelLayout.col1],
        col2: [...panelLayout.col2],
        col3: [...panelLayout.col3],
      };
    }

    if (effectiveColumnCount === 2) {
      const orderedPanels = [
        ...panelLayout.col1,
        ...panelLayout.col2,
        ...panelLayout.col3,
      ];
      const balancedCol1: PanelId[] = [];
      const balancedCol2: PanelId[] = [];

      orderedPanels.forEach((panelId, idx) => {
        if (idx % 2 === 0) {
          balancedCol1.push(panelId);
        } else {
          balancedCol2.push(panelId);
        }
      });

      return {
        col1: balancedCol1,
        col2: balancedCol2,
        col3: [],
      };
    }

    return {
      col1: [...panelLayout.col1, ...panelLayout.col2, ...panelLayout.col3],
      col2: [],
      col3: [],
    };
  });

  // Computed column grid class
  let gridColClass = $derived(
    effectiveColumnCount === 1
      ? "grid-cols-1"
      : effectiveColumnCount === 2
        ? "grid-cols-2"
        : "grid-cols-3",
  );

  let filesHelpContent = $derived(
    seriesMode ? t("flashcards.filesHelp") : t("flashcards.filesHelpMovie"),
  );

  let helpSection = $state<string | null>(null);

  let noteTypeLanguage = $state("");
  let noteTypeName = $state(loadCardTemplates().noteTypeName);

  // Auto-update noteTypeName when language changes
  $effect(() => {
    if (noteTypeLanguage) {
      const lang = languages.find(l => l.code === noteTypeLanguage);
      noteTypeName = lang ? `${lang.nameEn}_Vesta` : `Vesta_${noteTypeLanguage}`;
    } else {
      noteTypeName = loadCardTemplates().noteTypeName;
    }
  });

  let includeTag = $state(true);
  let includeSequence = $state(true);
  let includeAudioField = $state(true);
  let includeSnapshotField = $state(true);
  let includeVideoField = $state(true);
  let includeSubs1Field = $state(true);
  let includeSubs2Field = $state(true);

  let deckName = $state("");
  let deckNameAuto = $state(true);
  let firstEpisode = $state(1);

  let isProcessing = $state(false);
  let progress = $state(0);
  let progressMessage = $state("");
  let progressStage = $state("");

  let logIdCounter = 0;
  let logs = $state<LogEntry[]>([]);
  let error = $state<string | null>(null);
  let result = $state<{
    success: boolean;
    cardsGenerated: number;
    audioClips: number;
    snapshots: number;
    videoClips: number;
    tsvPath: string | null;
    apkgPath: string | null;
  } | null>(null);

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

  let showPreview = $state(false);
  let previewLines = $state<any[]>([]);
  let previewLoading = $state(false);
  let previewFilter = $state<"all" | "active" | "inactive">("all");
  let previewSearch = $state("");
  let previewPage = $state(1);
  let expandedPathField = $state<string | null>(null);
  const previewPerPage = 50;

  let unlisten: (() => void) | null = null;
  let unlistenDragDrop: (() => void) | null = null;
  let removeTemplateListener: (() => void) | null = null;
  let removeLanguageDefaultsListener: (() => void) | null = null;
  let removeLayoutObserver: (() => void) | null = null;
  let isDraggingOver = $state(false);

  function syncNoteTypeNameFromTemplates() {
    noteTypeName = loadCardTemplates().noteTypeName;
  }
  let needsDeckName = $derived(
    !seriesMode || seriesOutputMode === "single",
  );
  let canRunFlashcards = $derived(
    seriesMode
      ? Boolean(
          episodes.length > 0 && outputDir && (needsDeckName ? deckName : true) && noteTypeLanguage,
        )
      : Boolean(targetSubsPath && outputDir && deckName && noteTypeLanguage),
  );

  type RequirementPanelId = "files" | "naming" | "ankiFields";
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
          label: `Aggiungi almeno un episodio in ${t("flashcards.files")}`,
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
    if (needsDeckName && !deckName.trim()) {
      missing.push({
        panel: "naming",
        label: `${t("flashcards.deckNameLabel")}`,
      });
    }
    if (!noteTypeLanguage) {
      missing.push({
        panel: "ankiFields",
        label: `${t("flashcards.noteTypeLanguage")}`,
      });
    }
    return missing;
  });

  let generationRequirementsText = $derived(
    generationRequirements.map((item) => item.label).join(", "),
  );

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

  function inferLanguageFromPath(filePath: string): string | null {
    const filename = filePath.split("/").pop()?.toLowerCase() || "";
    const base = filename.replace(/\.[^/.]+$/, "");
    const tokens = base.split(/[^a-z0-9-]+/i).filter(Boolean);
    const suffixMatches = tokens
      .slice(-3)
      .reverse()
      .map((token) => detectLanguageCode(token))
      .filter(Boolean);
    return suffixMatches[0] || detectLanguageCode(base);
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
      addLog(`${t("flashcards.audioTracksError")}: ${e}`, "warning", getFileName(ep.mediaPath));
      return [];
    }
  }

  async function pickAudioTrackIndexForEpisode(ep: EpisodeEntry): Promise<number | null> {
    const tracks = await listAudioTracksForEpisode(ep);
    return pickBestAudioTrackIndex(tracks, getPreferredAudioLanguageCodeForEpisode(ep));
  }

  function formatAudioTrackLabel(track: AudioTrackInfo): string {
    const parts = [`#${track.index + 1}`];
    if (track.language) parts.push(track.language.toUpperCase());
    if (track.title) parts.push(track.title);
    if (track.codec) parts.push(track.codec);
    if (track.channels) parts.push(`${track.channels} ch`);
    return parts.join(" - ");
  }

  async function loadAudioTracksForMedia(path: string) {
    audioTracks = [];
    selectedAudioTrackIndex = null;
    audioTrackAutoSelected = true;

    if (detectMediaType(getFileName(path)) !== "video") return;

    audioTracksLoading = true;
    try {
      const tracks = await invoke<AudioTrackInfo[]>("flashcard_list_audio_tracks", {
        path,
      });
      audioTracks = tracks;
      selectedAudioTrackIndex =
        tracks.length > 1 ? pickBestAudioTrackIndex(tracks, getPreferredAudioLanguageCode()) : null;
    } catch (e) {
      addLog(`${t("flashcards.audioTracksError")}: ${e}`, "warning");
    } finally {
      audioTracksLoading = false;
    }
  }

  $effect(() => {
    if (audioTracks.length > 1 && audioTrackAutoSelected) {
      selectedAudioTrackIndex = pickBestAudioTrackIndex(audioTracks, getPreferredAudioLanguageCode());
    }
  });

  // ─── File Drag-and-Drop Handler ───────────────────────────────────────────
  function getFileExtension(path: string): string {
    return (path.split(".").pop() || "").toLowerCase();
  }

  function getFileName(path: string): string {
    return path.replace(/\\/g, "/").split("/").pop() || path;
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
    const filename = filePath.replace(/\\/g, "/").split("/").pop() || "";
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
    if (!paths || paths.length === 0) return;

    const subtitleFiles = paths.filter(isSubtitleFile);
    const mediaFiles = paths.filter(isMediaFile);

    if (subtitleFiles.length === 0 && mediaFiles.length === 0) {
      addLog(t("flashcards.dropNoValidFiles") || "No valid subtitle or media files dropped", "warning");
      return;
    }

    if (seriesMode) {
      if (subtitleFiles.length > 0 || mediaFiles.length > 0) {
        const expanded = await expandSeriesFilesWithSmartMatches(
          subtitleFiles,
          mediaFiles,
        );
        mergeSeriesDroppedFiles(expanded.subtitleFiles, expanded.mediaFiles);
        addLog(`${episodes.length} ${t("flashcards.seriesEpisodesAdded")}`, "target-subs");
      }
    } else {
      // Single-episode mode
      if (subtitleFiles.length >= 2) {
        const { target, native } = classifySubtitles(subtitleFiles);
        if (target) {
          try {
            await loadTargetSubtitle(target);
            await tryAutoSelectMediaForSubtitle(target, smartFileMatchingEnabled);
          } catch (e) {
            error = `Error parsing subtitles: ${e}`;
          }
        }
        if (native) {
          try {
            await loadNativeSubtitle(native);
          } catch (e) {
            error = `Error parsing native subtitles: ${e}`;
          }
        }
      } else if (subtitleFiles.length === 1) {
        // Single subtitle: assign to target if empty, otherwise to native
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
          } catch (e) {
            error = `Error parsing subtitles: ${e}`;
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
          } catch (e) {
            error = `Error parsing native subtitles: ${e}`;
          }
        }
      }

      // Handle media files
      if (mediaFiles.length > 0) {
        await applyMediaSelection(mediaFiles[0]);
      }
    }
  }

  onMount(async () => {
    syncNoteTypeNameFromTemplates();

    const handleCardTemplatesUpdated = () => {
      syncNoteTypeNameFromTemplates();
    };
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
      systemCpuCount = await invoke<number>("flashcard_get_cpu_count");
      const startupMaxCores = Math.max(2, systemCpuCount - 1);
      cpuCores = startupMaxCores;
    } catch {
      systemCpuCount = 4;
      cpuCores = Math.max(2, systemCpuCount - 1);
    }

    // Listen for OS-level file drag and drop
    try {
      unlistenDragDrop = await getCurrentWebview().onDragDropEvent((event) => {
        if (!active) return;
        if (event.payload.type === "over") isDraggingOver = true;
        else if (event.payload.type === "drop") {
          isDraggingOver = false;
          if (event.payload.paths) handleFileDrop(event.payload.paths);
        } else if (event.payload.type === "leave") isDraggingOver = false;
      });
    } catch (e) {
      console.warn("Failed to set up drag-drop listener:", e);
    }

    unlisten = await listen<{
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
        progress = Math.round(((seriesCurrentEpisode - 1) * 100 / seriesTotalEpisodes) + (p.percentage / seriesTotalEpisodes));
        progressMessage = `[Ep. ${seriesCurrentEpisode}/${seriesTotalEpisodes}] ${translated}`;
      } else {
        progress = Math.round(p.percentage);
        progressMessage = translated;
      }
      
      progressStage = p.stage;
      if (p.stage !== "done") {
        addLog(progressMessage, "progress", undefined, p.message);
      }
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    if (unlistenDragDrop) unlistenDragDrop();
    if (removeTemplateListener) removeTemplateListener();
    if (removeLanguageDefaultsListener) removeLanguageDefaultsListener();
    if (removeLayoutObserver) removeLayoutObserver();
    if (requirementPulseTimer) clearTimeout(requirementPulseTimer);
  });

  // Track the i18n key of the last progress log so sequential updates
  // (e.g. "Extracting media… 1/100", "2/100", …) replace the previous
  // entry instead of appending thousands of lines.
  let lastProgressKey: string | null = null;

  function addLog(
    message: string,
    type: LogEntry["type"] = "info",
    details?: string,
    progressKey?: string,
  ) {
    const timestamp = new Date().toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });

    // For sequential progress messages with the same key, update in-place
    if (
      type === "progress" &&
      progressKey &&
      progressKey === lastProgressKey &&
      logs.length > 0
    ) {
      const last = logs[logs.length - 1];
      if (last.type === "progress") {
        const updated = { ...last, timestamp, message };
        logs = [...logs.slice(0, -1), updated];
        return;
      }
    }

    if (type === "progress" && progressKey) {
      lastProgressKey = progressKey;
    } else if (type !== "progress") {
      lastProgressKey = null;
    }

    logs = [...logs, { id: ++logIdCounter, timestamp, message, type, details }];
  }

  function parseTimeToMs(time: string): number | null {
    if (!time || !time.trim()) return null;
    const parts = time.split(":").map(Number);
    if (parts.length === 3 && parts.every((p) => !isNaN(p))) {
      return (parts[0] * 3600 + parts[1] * 60 + parts[2]) * 1000;
    }
    return null;
  }

  function buildConfig() {
    return {
      target_subs_path: targetSubsPath,
      native_subs_path: nativeSubsPath || null,
      video_path: hasVideo ? mediaPath : null,
      audio_path: hasAudio && !hasVideo ? mediaPath : null,
      output_dir: outputDir,
      use_timings_from: useTimingsFrom,
      span_start_ms: parseTimeToMs(spanStart),
      span_end_ms: parseTimeToMs(spanEnd),
      time_shift_target_ms: timeShiftTarget,
      time_shift_native_ms: timeShiftNative,
      filters: {
        include_words: includeWords || null,
        exclude_words: excludeWords || null,
        exclude_duplicates_subs1: excludeDuplicatesSubs1,
        exclude_duplicates_subs2: excludeDuplicatesSubs2,
        min_chars: minChars,
        max_chars: maxChars,
        min_duration_ms: minDurationMs,
        max_duration_ms: maxDurationMs,
        exclude_styled: excludeStyled,
        actor_filter: actorFilter || null,
        only_cjk: onlyCjk,
        remove_no_match: removeNoMatch,
      },
      context: {
        leading: contextLeading,
        trailing: contextTrailing,
        max_gap_seconds: contextMaxGap,
      },
      combine_sentences: combineSentences,
      continuation_chars: continuationChars,
      generate_audio: generateAudio,
      audio_bitrate: audioBitrate,
      audio_track_index: selectedAudioTrackIndex,
      normalize_audio: normalizeAudio,
      audio_pad_start_ms: audioPadStart,
      audio_pad_end_ms: audioPadEnd,
      generate_snapshots: generateSnapshots,
      snapshot_width: snapshotWidth,
      snapshot_height: snapshotHeight,
      crop_bottom: cropBottom,
      generate_video_clips: generateVideoClips,
      video_codec: videoCodec,
      h264_preset: h264Preset,
      video_bitrate: videoBitrate,
      video_audio_bitrate: videoAudioBitrate,
      video_pad_start_ms: videoPadStart,
      video_pad_end_ms: videoPadEnd,
      deck_name: deckName,
      episode_number: 1,
      export_format: exportFormat,
      note_type_name: noteTypeName,
      field_names: loadFieldNames(),
      output_fields: {
        include_tag: includeTag,
        include_sequence: includeSequence,
        include_audio: includeAudioField,
        include_snapshot: includeSnapshotField,
        include_video: includeVideoField,
        include_subs1: includeSubs1Field,
        include_subs2: includeSubs2Field,
      },
      cpu_cores: cpuCores,
      card_front_html: loadCardTemplates().frontHtml,
      card_back_html: loadCardTemplates().backHtml,
      card_css: loadCardTemplates().css,
    };
  }

  async function loadTargetSubtitle(path: string) {
    targetSubsPath = path;
    const filename = targetSubsPath.split("/").pop() || "";

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
    addLog(
      `${info.count} ${t("flashcards.subtitlesLoaded")} (${info.format.toUpperCase()})`,
      "target-subs",
      filename,
    );

    if (deckNameAuto || !deckName.trim()) {
      deckName = generateDefaultDeckName(filename);
      deckNameAuto = true;
    }
  }

  async function loadNativeSubtitle(path: string) {
    nativeSubsPath = path;
    const filename = nativeSubsPath.split("/").pop() || "";

    const info = await invoke<any>("flashcard_load_subs", {
      path: nativeSubsPath,
    });
    nativeSubsInfo = info;
    addLog(
      `${info.count} ${t("flashcards.subtitlesLoaded")} (${info.format.toUpperCase()})`,
      "native-subs",
      filename,
    );
  }

  async function applyMediaSelection(path: string, autoSelected = false) {
    mediaPath = path;
    const filename = mediaPath.split("/").pop() || "";
    mediaType = detectMediaType(filename);
    await loadAudioTracksForMedia(path);

    if (mediaType === "video") {
      generateAudio = true;
      generateSnapshots = true;
      addLog(`${t("flashcards.mediaTypeVideo")}`, "media", filename);
    } else if (mediaType === "audio") {
      generateAudio = true;
      generateSnapshots = false;
      generateVideoClips = false;
      addLog(`${t("flashcards.mediaTypeAudio")}`, "media", filename);
    }

    if (autoSelected) {
      addLog(`Auto-selected media: ${filename}`, "media");
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
          error = `Error parsing subtitles: ${e}`;
        }
      }
    } catch (e) {
      error = `${t("flashcards.errorSelectingFile")}: ${e}`;
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
          error = `Error parsing native subtitles: ${e}`;
        }
      }
    } catch (e) {
      error = `${t("flashcards.errorSelectingFile")}: ${e}`;
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
      }
    } catch (e) {
      error = `${t("flashcards.errorSelectingFile")}: ${e}`;
    }
  }

  async function selectOutputDir() {
    try {
      const selected = await guardedOpen({ directory: true });
      if (selected) {
        outputDir = selected as string;
        localStorage.setItem(OUTPUT_DIR_KEY, outputDir);
        addLog(`${t("flashcards.outputDirSet")}`, "output", outputDir);
      }
    } catch (e) {
      error = `${t("flashcards.errorSelectingDir")}: ${e}`;
    }
  }

  async function loadPreview() {
    if (!canRunFlashcards) {
      error = t("flashcards.requiredFieldsMissing");
      return;
    }

    previewLoading = true;
    showPreview = true;
    error = null;

    try {
      const config = buildConfig();
      previewLines = await invoke<any[]>("flashcard_preview", { config });
      addLog(
        `Preview: ${previewLines.length} total, ${previewLines.filter((l: any) => l.active).length} active`,
        "info",
      );
    } catch (e) {
      error = `Preview error: ${e}`;
    } finally {
      previewLoading = false;
    }
  }

  let filteredPreview = $derived(
    previewLines.filter((line: any) => {
      const matchFilter =
        previewFilter === "all" ||
        (previewFilter === "active" && line.active) ||
        (previewFilter === "inactive" && !line.active);
      const matchSearch =
        !previewSearch ||
        line.subs1_text.toLowerCase().includes(previewSearch.toLowerCase()) ||
        (line.subs2_text &&
          line.subs2_text.toLowerCase().includes(previewSearch.toLowerCase()));
      return matchFilter && matchSearch;
    }),
  );

  let previewTotalPages = $derived(
    Math.max(1, Math.ceil(filteredPreview.length / previewPerPage)),
  );
  let previewPaged = $derived(
    filteredPreview.slice(
      (previewPage - 1) * previewPerPage,
      previewPage * previewPerPage,
    ),
  );

  $effect(() => {
    // reactive dependencies: previewFilter, previewSearch
    void previewFilter;
    void previewSearch;
    previewPage = 1;
  });

  async function startSeriesGeneration() {
    error = null;
    result = null;
    progress = 0;
    isProcessing = true;
    seriesTotalEpisodes = episodes.length;
    seriesCurrentEpisode = 0;

    addLog(
      `${t("flashcards.starting")}... (${t("flashcards.modeSeries")}: ${episodes.length} ${t("flashcards.seriesEpisodes")})`,
      "info",
    );
    addLog(`${t("flashcards.deckName")}: ${deckName}`, "info");

    const startTime = Date.now();
    let totalCards = 0;
    let totalAudio = 0;
    let totalSnapshots = 0;
    let totalVideoClips = 0;
    const apkgPaths: string[] = [];
    let hadError = false;

    try {
      for (let i = 0; i < episodes.length; i++) {
        seriesCurrentEpisode = i + 1;
        const ep = episodes[i];
        const epNum = i + 1;

        addLog(
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
          use_timings_from: useTimingsFrom,
          span_start_ms: parseTimeToMs(spanStart),
          span_end_ms: parseTimeToMs(spanEnd),
          time_shift_target_ms: timeShiftTarget,
          time_shift_native_ms: timeShiftNative,
          filters: {
            include_words: includeWords || null,
            exclude_words: excludeWords || null,
            exclude_duplicates_subs1: excludeDuplicatesSubs1,
            exclude_duplicates_subs2: excludeDuplicatesSubs2,
            min_chars: minChars,
            max_chars: maxChars,
            min_duration_ms: minDurationMs,
            max_duration_ms: maxDurationMs,
            exclude_styled: excludeStyled,
            actor_filter: actorFilter || null,
            only_cjk: onlyCjk,
            remove_no_match: removeNoMatch,
          },
          context: {
            leading: contextLeading,
            trailing: contextTrailing,
            max_gap_seconds: contextMaxGap,
          },
          combine_sentences: combineSentences,
          continuation_chars: continuationChars,
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
          video_bitrate: epMediaSettings.videoBitrate,
          video_audio_bitrate: epMediaSettings.videoAudioBitrate,
          video_pad_start_ms: epMediaSettings.videoPadStart,
          video_pad_end_ms: epMediaSettings.videoPadEnd,
          deck_name: seriesOutputMode === "separate" ? deriveDeckNameFromFile(ep) : deckName,
          episode_number: epNum,
          export_format: exportFormat,
          note_type_name: noteTypeName,
          field_names: loadFieldNames(),
          output_fields: {
            include_tag: includeTag,
            include_sequence: includeSequence,
            include_audio: includeAudioField,
            include_snapshot: includeSnapshotField,
            include_video: includeVideoField,
            include_subs1: includeSubs1Field,
            include_subs2: includeSubs2Field,
          },
          cpu_cores: cpuCores,
          card_front_html: loadCardTemplates().frontHtml,
          card_back_html: loadCardTemplates().backHtml,
          card_css: loadCardTemplates().css,
        };

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
            addLog(
              `✓ Ep ${epNum}: ${res.cards_generated} ${t("flashcards.cardsGenerated")}`,
              "success",
            );
          } else {
            addLog(`⚠ Ep ${epNum}: ${res.message}`, "warning");
          }
        } catch (e) {
          addLog(`✗ Ep ${epNum}: ${e}`, "error");
          hadError = true;
        }
      }

      // Merge APKGs if single mode selected
      if (
        seriesOutputMode === "single" &&
        apkgPaths.length > 1 &&
        exportFormat === "apkg"
      ) {
        addLog(t("flashcards.mergingApkg"), "info");
        try {
          const mergedPath = await invoke<string>("flashcard_merge_apkg", {
            apkgPaths,
            outputPath: `${outputDir}/${deckName.replace(/[^a-zA-Z0-9_\-\. ]/g, "_")}.apkg`,
          });
          addLog(`APKG: ${mergedPath}`, "success");
        } catch (e) {
          addLog(`${t("flashcards.mergeFailed")}: ${e}`, "error");
          hadError = true;
        }
      }

      result = {
        success: !hadError,
        cardsGenerated: totalCards,
        audioClips: totalAudio,
        snapshots: totalSnapshots,
        videoClips: totalVideoClips,
        tsvPath: null,
        apkgPath: apkgPaths.length > 0 ? apkgPaths[apkgPaths.length - 1] : null,
      };

      addLog(
        `${t("flashcards.seriesComplete", { total: String(episodes.length) })}`,
        "success",
      );
    } catch (e) {
      error = `${t("flashcards.errorGenerating")}: ${e}`;
      addLog(`${error}`, "error");
    } finally {
      isProcessing = false;
      seriesCurrentEpisode = 0;
      seriesTotalEpisodes = 0;
      const elapsed = Math.floor((Date.now() - startTime) / 1000);
      const hh = String(Math.floor(elapsed / 3600)).padStart(2, "0");
      const mm = String(Math.floor((elapsed % 3600) / 60)).padStart(2, "0");
      const ss = String(elapsed % 60).padStart(2, "0");
      addLog(`⏱ ${hh}:${mm}:${ss}`, "info");
    }
  }

  async function startGeneration() {
    if (!canRunFlashcards) {
      promptMissingGenerationRequirements();
      error = t("flashcards.requiredFieldsMissing");
      return;
    }

    if (seriesMode) {
      await startSeriesGeneration();
      return;
    }

    error = null;
    result = null;
    progress = 0;
    isProcessing = true;
    addLog(`${t("flashcards.starting")}...`, "info");
    addLog(`${t("flashcards.deckName")}: ${deckName}`, "info");

    const startTime = Date.now();

    try {
      const config = buildConfig();
      const res = await invoke<any>("flashcard_generate", { config });
      result = {
        success: res.success,
        cardsGenerated: res.cards_generated,
        audioClips: res.audio_clips,
        snapshots: res.snapshots,
        videoClips: res.video_clips,
        tsvPath: res.tsv_path,
        apkgPath: res.apkg_path,
      };

      if (res.success) {
        addLog(
          `${res.cards_generated} ${t("flashcards.cardsGenerated")}`,
          "success",
        );
        if (res.tsv_path) {
          addLog(`TSV: ${res.tsv_path}`, "success");
        }
        if (res.apkg_path) {
          addLog(`APKG: ${res.apkg_path}`, "success");
        }
      } else {
        addLog(res.message, "warning");
      }
    } catch (e) {
      error = `${t("flashcards.errorGenerating")}: ${e}`;
      addLog(`${error}`, "error");
    } finally {
      isProcessing = false;
      const elapsed = Math.floor((Date.now() - startTime) / 1000);
      const hh = String(Math.floor(elapsed / 3600)).padStart(2, "0");
      const mm = String(Math.floor((elapsed % 3600) / 60)).padStart(2, "0");
      const ss = String(elapsed % 60).padStart(2, "0");
      addLog(`⏱ ${hh}:${mm}:${ss}`, "info");
    }
  }

  async function cancelGeneration() {
    try {
      await invoke("flashcard_cancel");
      isProcessing = false;
      progress = 0;
      progressMessage = "";
      addLog(`${t("flashcards.cancelled")}`, "warning");
    } catch (e) {
      addLog(`Error cancelling: ${e}`, "error");
    }
  }

  function clearLogs() {
    logs = [];
    lastProgressKey = null;
  }

  function resetGeneration() {
    result = null;
    error = null;
    progress = 0;
    progressMessage = "";
    progressStage = "";
    logs = [];
    logIdCounter = 0;
    lastProgressKey = null;
    
    // Clear files so the user can insert new ones
    targetSubsPath = "";
    nativeSubsPath = "";
    mediaPath = "";
    mediaType = "none";
    audioTracks = [];
    selectedAudioTrackIndex = null;
    audioTrackAutoSelected = true;
    targetSubsInfo = null;
    nativeSubsInfo = null;
    episodes = [];
    deckName = "";
    deckNameAuto = true;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="h-full flex flex-col p-6 overflow-y-auto flashcards-scroll bg-gradient-to-br from-gray-900 via-gray-900 to-gray-950 relative"
  ondragover={(e) => { e.preventDefault(); if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy'; }}
  ondrop={(e) => { e.preventDefault(); isDraggingOver = false; }}
  ondragleave={(e) => {
    const rt = e.relatedTarget as HTMLElement | null;
    const ct = e.currentTarget as HTMLElement;
    if (rt && ct.contains(rt)) return;
    isDraggingOver = false;
  }}
>
  {#if isDraggingOver}
    <div
      class="absolute inset-0 z-50 bg-purple-500/10 border-2 border-dashed border-purple-400 rounded-2xl flex items-center justify-center pointer-events-none"
    >
      <div class="text-center">
        <svg
          class="w-16 h-16 mx-auto mb-3 text-purple-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          ><path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
          /></svg
        >
        <p class="text-lg font-medium text-purple-300">
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
            ffmpegAvailable = true;
          } catch (e) {
            error = "Download failed: " + e;
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
          Scarica Automaticamente
        {/if}
      </button>
    </div>
  {/if}

  {#if showPreview}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-6"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={() => (showPreview = false)}
      onkeydown={(e) => {
        if (e.key === "Escape") showPreview = false;
      }}
    >
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="bg-gray-900 border border-gray-700 rounded-xl w-full max-w-5xl max-h-[85vh] flex flex-col"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.stopPropagation()}
      >
        <div
          class="flex items-center justify-between p-4 border-b border-gray-700"
        >
          <div class="flex items-center gap-3">
            <h2 class="text-lg font-bold text-emerald-400">
              {t("flashcards.preview")}
            </h2>
          </div>
          <div class="flex items-center gap-3">
            <div class="relative">
              <svg
                class="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-gray-500 pointer-events-none"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                />
              </svg>
              <input
                type="text"
                bind:value={previewSearch}
                class="input-modern text-xs w-48 pl-8"
                placeholder={t("flashcards.previewSearch")}
                style="text-indent: 0;"
              />
            </div>
            <button
              onclick={() => (showPreview = false)}
	              class="dialog-close-button text-gray-400 hover:text-white text-xl leading-none p-1"
            >
              ✕
            </button>
          </div>
        </div>

        <div
          class="px-4 py-2 border-b border-gray-700 flex items-center justify-between"
        >
          <div class="flex items-center gap-2">
            <div class="flex rounded-lg overflow-hidden border border-gray-700">
              {#each [["all", t("flashcards.previewAll"), "All subtitle lines"], ["active", t("flashcards.previewActive"), "Lines that will become flashcards"], ["inactive", t("flashcards.previewInactive"), "Lines excluded by your filters"]] as [val, label, tooltip]}
                <button
                  class="px-3 py-1 text-xs font-medium transition-colors {previewFilter ===
                  val
                    ? 'bg-emerald-500/20 text-emerald-300'
                    : 'text-gray-400 hover:bg-gray-800'}"
                  onclick={() => (previewFilter = val as any)}
                  title={tooltip}
                >
                  {label}
                </button>
              {/each}
            </div>
            <span class="text-xs text-gray-500">
              {filteredPreview.length}
              {t("flashcards.linesShown")}
            </span>
          </div>
          {#if previewTotalPages > 1}
            <div class="flex items-center gap-1">
              <button
                disabled={previewPage <= 1}
                onclick={() => (previewPage = 1)}
                class="px-2 py-1 text-xs rounded text-gray-400 hover:text-white hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed"
                >«</button
              >
              <button
                disabled={previewPage <= 1}
                onclick={() => previewPage--}
                class="px-2 py-1 text-xs rounded text-gray-400 hover:text-white hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed"
                >‹</button
              >
              <span class="text-xs text-gray-400 px-2">
                {previewPage} / {previewTotalPages}
              </span>
              <button
                disabled={previewPage >= previewTotalPages}
                onclick={() => previewPage++}
                class="px-2 py-1 text-xs rounded text-gray-400 hover:text-white hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed"
                >›</button
              >
              <button
                disabled={previewPage >= previewTotalPages}
                onclick={() => (previewPage = previewTotalPages)}
                class="px-2 py-1 text-xs rounded text-gray-400 hover:text-white hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed"
                >»</button
              >
            </div>
          {/if}
        </div>

        <div class="flex-1 overflow-y-auto p-2">
          {#if previewLoading}
            <div class="flex items-center justify-center h-32">
              <div
                class="animate-spin w-8 h-8 border-2 border-emerald-500 border-t-transparent rounded-full"
              ></div>
            </div>
          {:else}
            <table class="w-full text-xs">
              <thead class="sticky top-0 z-10">
                <tr class="text-gray-400 bg-gray-800 shadow-sm">
                  <th class="p-2 text-left w-12">#</th>
                  <th class="p-2 text-left w-20"
                    >{t("flashcards.previewTime")}</th
                  >
                  <th class="p-2 text-left">{t("flashcards.subs1")}</th>
                  {#if nativeSubsPath}
                    <th class="p-2 text-left">{t("flashcards.subs2")}</th>
                  {/if}
                  <th class="p-2 text-center w-16"
                    >{t("flashcards.previewStatus")}</th
                  >
                </tr>
              </thead>
              <tbody>
                {#each previewPaged as line, i}
                  <tr
                    class="border-t border-gray-800 {line.active
                      ? 'bg-emerald-500/5 hover:bg-emerald-500/10'
                      : 'bg-red-500/5 opacity-60 hover:bg-red-500/10'}"
                  >
                    <td class="p-2 text-gray-500 font-mono">{line.index + 1}</td
                    >
                    <td class="p-2 text-gray-400 font-mono">
                      {Math.floor(line.start_ms / 60000)}:{String(
                        Math.floor((line.start_ms % 60000) / 1000),
                      ).padStart(2, "0")}
                    </td>
                    <td class="p-2">
                      <span
                        contenteditable="true"
                        class="text-gray-200 outline-none focus:bg-gray-800/50 focus:ring-1 focus:ring-emerald-500/30 rounded px-1 -mx-1 block"
                        onblur={(e) => {
                          line.subs1_text =
                            (e.target as HTMLElement).textContent || "";
                        }}>{line.subs1_text}</span
                      >
                    </td>
                    {#if nativeSubsPath}
                      <td class="p-2">
                        <span
                          contenteditable="true"
                          class="text-gray-300 outline-none focus:bg-gray-800/50 focus:ring-1 focus:ring-emerald-500/30 rounded px-1 -mx-1 block"
                          onblur={(e) => {
                            line.subs2_text =
                              (e.target as HTMLElement).textContent || "";
                          }}>{line.subs2_text || "—"}</span
                        >
                      </td>
                    {/if}
                    <td class="p-2 text-center">
                      {#if line.active}
                        <span
                          class="inline-block w-2 h-2 bg-emerald-400 rounded-full"
                        ></span>
                      {:else}
                        <span
                          class="inline-block w-2 h-2 bg-red-400 rounded-full"
                        ></span>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  {#snippet panelContent(panelId: PanelId)}
    {#if panelId === "files"}
      <div class="glass-card p-4 {panelHighlightClass('files')}">
        <div class="mb-3 flex items-center gap-3">
          <h3
            class="flex min-w-0 items-center gap-2 text-sm font-semibold {seriesMode ? 'text-violet-300' : 'text-emerald-400'}"
          >
            <svg
              class="w-4 h-4 shrink-0"
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
            {t("flashcards.files")}
          </h3>
          <span class="flex shrink-0 items-center gap-1.5 rounded-full border border-gray-700/60 bg-gray-900/60 px-2 py-1">
            <button
              type="button"
              onclick={toggleSeriesMode}
              class="flex items-center gap-1 text-xs font-semibold transition-colors {!seriesMode
                ? 'text-emerald-300'
                : 'text-gray-500 hover:text-gray-300'}"
              title={t("flashcards.modeMovie")}
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
                <rect x="2" y="4" width="20" height="16" rx="2" />
                <path d="M2 8h20M7 4v4M17 4v4" stroke-linecap="round" />
              </svg>
              Film
            </button>
            <button
              type="button"
              class="relative h-5 w-9 shrink-0 rounded-full transition-colors {seriesMode ? 'bg-violet-500/60' : 'bg-emerald-500/50'}"
              onclick={toggleSeriesMode}
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
              onclick={toggleSeriesMode}
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
              Serie TV
            </button>
          </span>
          <div class="ml-auto">
            <InfoButton onclick={() => (helpSection = "files")} />
          </div>
        </div>

        {#if !seriesMode}
          <div class="space-y-2.5">
            <div>
              <span class="block text-xs text-gray-400 mb-1">
                {t("flashcards.targetLangSubs")}
                <span class="text-red-400">*</span>
              </span>
              <span class="block text-[10px] text-gray-500 mb-1"
                >{t("flashcards.targetLangSubsDesc")}</span
              >
              <div class="flex gap-2">
                <button
                  type="button"
                  onclick={() => {
                    if (targetSubsPath) expandedPathField = "targetSubs";
                  }}
                  class="input-modern flex-1 text-xs text-left transition-colors truncate {timingSourceFieldClass('target')} {targetSubsPath
                    ? 'cursor-pointer hover:bg-white/10'
                    : 'cursor-default hover:bg-transparent'}"
                  style="direction: rtl; text-align: left;"
                  title={targetSubsPath || t("flashcards.selectFile")}
                >
                  <span
                    class={targetSubsPath ? "text-white" : "text-gray-500"}
                    style="unicode-bidi: plaintext;"
                  >
                    {targetSubsPath || t("flashcards.selectFile")}
                  </span>
                </button>
                <button
                  onclick={selectTargetSubs}
                  class="btn-primary py-1.5 px-3 text-xs flex-shrink-0 flex items-center gap-1"
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
                      d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                    /></svg
                  >
                  {t("flashcards.browse")}
                </button>
                <button
                  type="button"
                  onclick={() => clearMovieFile("target")}
                  disabled={!canClearMovieFile("target")}
                  class="inline-flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-lg border transition-colors {clearMovieFileButtonClass('target')}"
                  title="Rimuovi file"
                  aria-label="Rimuovi file"
                >
                  <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            </div>

            <div>
              <span class="block text-xs text-gray-400 mb-1"
                >{t("flashcards.nativeLangSubs")}</span
              >
              <span class="block text-[10px] text-gray-500 mb-1"
                >{t("flashcards.nativeLangSubsDesc")}</span
              >
              <div class="flex gap-2">
                <button
                  type="button"
                  onclick={() => {
                    if (nativeSubsPath) expandedPathField = "nativeSubs";
                  }}
                  class="input-modern flex-1 text-xs text-left transition-colors truncate {timingSourceFieldClass('native')} {nativeSubsPath
                    ? 'cursor-pointer hover:bg-white/10'
                    : 'cursor-default hover:bg-transparent'}"
                  style="direction: rtl; text-align: left;"
                  title={nativeSubsPath || t("flashcards.optional")}
                >
                  <span
                    class={nativeSubsPath ? "text-white" : "text-gray-500"}
                    style="unicode-bidi: plaintext;"
                  >
                    {nativeSubsPath || t("flashcards.optional")}
                  </span>
                </button>
                <button
                  onclick={selectNativeSubs}
                  class="btn-secondary py-1.5 px-3 text-xs flex-shrink-0 flex items-center gap-1"
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
                      d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                    /></svg
                  >
                  {t("flashcards.browse")}
                </button>
                <button
                  type="button"
                  onclick={() => clearMovieFile("native")}
                  disabled={!canClearMovieFile("native")}
                  class="inline-flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-lg border transition-colors {clearMovieFileButtonClass('native')}"
                  title="Rimuovi file"
                  aria-label="Rimuovi file"
                >
                  <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            </div>

            <div>
              <span class="block text-xs text-gray-400 mb-1"
                >{t("flashcards.mediaFile")}</span
              >
              <span class="block text-[10px] text-gray-500 mb-1"
                >{t("flashcards.mediaFileDesc")}</span
              >
              <div class="flex gap-2">
                <button
                  type="button"
                  onclick={() => {
                    if (mediaPath) expandedPathField = "media";
                  }}
                  class="input-modern flex-1 text-xs text-left transition-colors truncate {mediaPath
                    ? 'cursor-pointer hover:bg-white/10'
                    : 'cursor-default hover:bg-transparent'}"
                  style="direction: rtl; text-align: left;"
                  title={mediaPath || t("flashcards.mediaPlaceholder")}
                >
                  <span
                    class={mediaPath ? "text-white" : "text-gray-500"}
                    style="unicode-bidi: plaintext;"
                  >
                    {mediaPath || t("flashcards.mediaPlaceholder")}
                  </span>
                </button>
                <button
                  onclick={selectMedia}
                  class="btn-secondary py-1.5 px-3 text-xs flex-shrink-0 flex items-center gap-1"
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
                      d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                    /></svg
                  >
                  {t("flashcards.browse")}
                </button>
                <button
                  type="button"
                  onclick={() => clearMovieFile("media")}
                  disabled={!canClearMovieFile("media")}
                  class="inline-flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-lg border transition-colors {clearMovieFileButtonClass('media')}"
                  title="Rimuovi file"
                  aria-label="Rimuovi file"
                >
                  <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            </div>

            <div>
              <span class="block text-xs text-gray-400 mb-1">
                {t("flashcards.outputDir")} <span class="text-red-400">*</span>
              </span>
              <div class="flex gap-2">
                <button
                  type="button"
                  onclick={() => {
                    if (outputDir) expandedPathField = "output";
                  }}
                  class="input-modern flex-1 text-xs text-left transition-colors truncate {outputDir
                    ? 'cursor-pointer hover:bg-white/10'
                    : 'cursor-default hover:bg-transparent'}"
                  style="direction: rtl; text-align: left;"
                  title={outputDir || t("flashcards.selectDir")}
                >
                  <span
                    class={outputDir ? "text-white" : "text-gray-500"}
                    style="unicode-bidi: plaintext;"
                  >
                    {outputDir || t("flashcards.selectDir")}
                  </span>
                </button>
                <button
                  onclick={selectOutputDir}
                  class="btn-primary py-1.5 px-3 text-xs flex-shrink-0 flex items-center gap-1"
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
                      d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                    /></svg
                  >
                  {t("flashcards.browse")}
                </button>
              </div>
            </div>
          </div>
        {:else}
          <!-- Series mode: batch file management -->
          <div class="space-y-3">
            <!-- Add files buttons -->
            <div class="flex flex-wrap gap-2">
              <button
                onclick={addSeriesMultipleFiles}
                class="btn-primary py-1.5 px-3 text-xs flex items-center gap-1.5"
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
                Add Files
              </button>
              {#if episodes.length > 0}
                <button
                  onclick={clearAllEpisodes}
                  class="ml-auto text-xs text-red-400 hover:text-red-300 transition-colors flex items-center gap-1"
                >
                  <svg
                    class="w-3 h-3"
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

            <!-- Episode table -->
            {#if episodes.length === 0}
              <div
                class="p-6 text-center text-gray-500 text-xs border border-dashed border-gray-700 rounded-lg"
              >
                <svg
                  class="w-8 h-8 mx-auto mb-2 text-gray-600"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                  ><path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="1.5"
                    d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
                  /></svg
                >
                {t("flashcards.noFilesAdded")}
              </div>
            {:else}
              <div class="border border-gray-700/50 rounded-lg overflow-hidden">
                <div class="overflow-y-auto max-h-[400px]">
                  <table class="w-full text-xs table-fixed">
                    <thead class="bg-gray-800/80 sticky top-0">
                      <tr>
                        <th class="p-1.5 text-left text-gray-400 w-10">#</th>
                        <th class="p-1.5 text-left text-gray-400"
                          >{t("flashcards.targetLangSubs")}</th
                        >
                        <th class="p-1.5 text-left text-gray-400"
                          >{t("flashcards.nativeLangSubs")}</th
                        >
                        <th class="p-1.5 text-left text-gray-400"
                          >{t("flashcards.mediaFile")}</th
                        >
                        <th class="p-1.5 w-28"></th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each episodes as ep, idx}
	                        <tr
	                          class="border-t border-gray-800 cursor-default {idx % 2 === 0
	                            ? 'bg-gray-900/30'
	                            : 'bg-gray-800/20'} hover:bg-gray-700/20"
	                          oncontextmenu={(e) => openEpisodeContextMenu(e, idx)}
	                        >
                          <td class="p-1.5 text-gray-500 font-mono">{ep.id}</td>
                          <td
                            class="p-1.5 cursor-pointer truncate text-emerald-300 transition-colors hover:bg-violet-500/12 hover:text-emerald-100 rounded-md"
                            title={ep.targetSubsPath}
                            onclick={() => { navigator.clipboard.writeText(ep.targetSubsPath); showSnackbar(t("flashcards.copiedTargetSubs") || "Percorso originale copiato", "success"); }}
                          >
                            <span class="px-1.5 py-0.5">{ep.targetSubsPath.split("/").pop()}</span>
                          </td>
                          <td
                            class="p-1.5 cursor-pointer truncate transition-colors hover:bg-violet-500/12 rounded-md {ep.nativeSubsPath
                              ? 'text-blue-300 hover:text-blue-100'
                              : 'text-gray-600 hover:text-gray-400'}"
                            title={ep.nativeSubsPath || "—"}
                            onclick={() => { if(ep.nativeSubsPath) { navigator.clipboard.writeText(ep.nativeSubsPath); showSnackbar(t("flashcards.copiedNativeSubs") || "Percorso riferimento copiato", "success"); } }}
                          >
                            <span class="px-1.5 py-0.5">{ep.nativeSubsPath
                              ? ep.nativeSubsPath.split("/").pop()
                              : "—"}</span>
                          </td>
	                          <td
                              class="p-1.5 cursor-pointer truncate transition-colors hover:bg-violet-500/12 rounded-md {ep.mediaPath ? 'text-purple-300 hover:text-purple-100' : 'text-gray-600 hover:text-gray-400'}"
                              title={ep.mediaPath || "—"}
                              onclick={() => { if(ep.mediaPath) { navigator.clipboard.writeText(ep.mediaPath); showSnackbar(t("flashcards.copiedMediaPath") || "Percorso media copiato", "success"); } }}
                            >
	                            {#if ep.mediaPath}
	                              <span class="group inline-flex max-w-full items-center gap-1.5 text-left px-1.5 py-0.5">
	                                {#if episodeHasMediaOverrides(ep)}
	                                  <span
	                                    class="h-2 w-2 shrink-0 rounded-full bg-violet-400 shadow-[0_0_10px_rgba(167,139,250,0.75)]"
	                                    title={t("flashcards.hasPerMovieOverrides")}
	                                    aria-label={t("flashcards.hasPerMovieOverrides")}
	                                  ></span>
	                                {/if}
	                                <span class="truncate">
	                                  {ep.mediaPath.split("/").pop()}
	                                </span>
	                              </span>
	                            {:else}
	                              <span class="px-1.5 py-0.5">—</span>
	                            {/if}
	                          </td>
	                          <td class="p-1.5">
	                            <div class="flex items-center justify-end gap-1">
	                            <button
	                              onclick={(e) => { e.stopPropagation(); openEpisodeEditor(idx); }}
	                              class="inline-flex h-7 w-7 cursor-pointer items-center justify-center rounded-md text-amber-400 transition-colors hover:bg-amber-400/10 hover:text-amber-300"
	                              title={t("common.edit")}
	                              aria-label={t("common.edit")}
	                            >
                              <svg
                                class="w-4 h-4"
                                fill="none"
                                stroke="currentColor"
                                viewBox="0 0 24 24"
                                ><path
                                  stroke-linecap="round"
                                  stroke-linejoin="round"
                                  stroke-width="2"
                                  d="M11 4H6a2 2 0 00-2 2v12a2 2 0 002 2h12a2 2 0 002-2v-5M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"
	                                /></svg
	                              >
	                            </button>
	                            <button
	                              onclick={(e) => { e.stopPropagation(); openEpisodeMediaSettings(idx); }}
	                              disabled={!ep.mediaPath}
	                              class="inline-flex h-7 w-7 cursor-pointer items-center justify-center rounded-md text-violet-300 transition-colors hover:bg-violet-400/10 hover:text-violet-200 disabled:cursor-not-allowed disabled:opacity-35 disabled:hover:bg-transparent disabled:hover:text-violet-300"
	                              title={t("common.settings")}
	                              aria-label={t("common.settings")}
	                            >
	                              <svg
	                                class="h-4 w-4"
	                                fill="none"
	                                stroke="currentColor"
	                                viewBox="0 0 24 24"
	                              >
	                                <path
	                                  stroke-linecap="round"
	                                  stroke-linejoin="round"
	                                  stroke-width="2"
	                                  d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
	                                />
	                                <path
	                                  stroke-linecap="round"
	                                  stroke-linejoin="round"
	                                  stroke-width="2"
	                                  d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
	                                />
	                              </svg>
	                            </button>
	                            <button
	                              onclick={(e) => { e.stopPropagation(); removeEpisode(idx); }}
	                              class="inline-flex h-7 w-7 cursor-pointer items-center justify-center rounded-md text-red-400 transition-colors hover:bg-red-400/10 hover:text-red-300"
                              title={t("common.delete")}
                              aria-label={t("common.delete")}
                            >
                              <svg
                                class="w-4 h-4"
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
                            </button>
                            </div>
                          </td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
                <div
                  class="bg-gray-800/40 px-2 py-1 text-[10px] text-gray-500 flex items-center justify-between border-t border-gray-700/50"
                >
                  <span>{episodes.length} {t("flashcards.seriesEpisodes")}</span
                  >
                  <button
                    type="button"
                    onclick={openSmartMatchingDialog}
                    class="inline-flex items-center gap-1.5 rounded-md px-1.5 py-0.5 text-gray-500 transition-colors hover:bg-white/5 hover:text-violet-300"
                    title={t("flashcards.smartMatchingTitle")}
                  >
                    <svg class="h-3 w-3 text-violet-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3zM19 14l.75 2.25L22 17l-2.25.75L19 20l-.75-2.25L16 17l2.25-.75L19 14zM5 14l.75 2.25L8 17l-2.25.75L5 20l-.75-2.25L2 17l2.25-.75L5 14z" />
                    </svg>
                    {t("flashcards.autoMatched")}
                  </button>
                </div>
              </div>

              <!-- Output dir (shared with movie mode) -->
              <div>
                <span class="block text-xs text-gray-400 mb-1">
                  {t("flashcards.outputDir")}
                  <span class="text-red-400">*</span>
                </span>
                <div class="flex gap-2">
                  <button
                    type="button"
                    onclick={() => {
                      if (outputDir) expandedPathField = "output";
                    }}
                    class="input-modern flex-1 text-xs text-left transition-colors truncate {outputDir
                      ? 'cursor-pointer hover:bg-white/10'
                      : 'cursor-default hover:bg-transparent'}"
                    style="direction: rtl; text-align: left;"
                    title={outputDir || t("flashcards.selectDir")}
                  >
                    <span
                      class={outputDir ? "text-white" : "text-gray-500"}
                      style="unicode-bidi: plaintext;"
                    >
                      {outputDir || t("flashcards.selectDir")}
                    </span>
                  </button>
                  <button
                    onclick={selectOutputDir}
                    class="btn-primary py-1.5 px-3 text-xs flex-shrink-0 flex items-center gap-1"
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
                        d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                      /></svg
                    >
                    {t("flashcards.browse")}
                  </button>
                </div>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {:else if panelId === "subtitleOptions"}
      <div
        inert={!hasAnyFiles}
        title={!hasAnyFiles ? HINT_LOAD_TARGET_FIRST : undefined}
        class="glass-card p-4 {!hasAnyFiles
          ? 'opacity-40'
          : ''}"
      >
        <div class="flex items-center gap-2">
          <button
            onclick={() => {
              if (hasAnyFiles) {
                showSubtitleOptions = !showSubtitleOptions;
              }
            }}
            class="flex-1 flex items-center justify-between text-sm font-semibold text-teal-400"
          >
            <span class="flex items-center gap-2">
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
                  d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z"
                />
              </svg>
              {t("flashcards.subtitleOptions")}
            </span>
            <svg
              class="w-4 h-4 transition-transform {showSubtitleOptions
                ? 'rotate-180'
                : ''}"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M19 9l-7 7-7-7"
              />
            </svg>
          </button>
          <InfoButton onclick={() => (helpSection = "subtitleOptions")} />
        </div>
        {#if showSubtitleOptions}
          <div class="mt-3 space-y-2.5 animate-fade-in">
            <div class="flex flex-col gap-2">
              <span class="text-xs text-gray-400">{t("flashcards.useTimingsFrom")}</span>
              <div
                class="timing-source-toggle {useTimingsFrom === 'native' ? 'timing-source-toggle-native' : ''} {showTimingFlash ? 'timing-source-toggle-flash' : ''}"
              >
                {#key timingFlashKey}
                  <div class="timing-source-slider"></div>
                {/key}
                <label class="flex-1 cursor-pointer">
                  <input
                    type="radio"
                    checked={useTimingsFrom === "target"}
                    value="target"
                    class="peer sr-only"
                    onchange={() => handleTimingSourceChange("target")}
                  />
                  <div class="timing-source-choice">
                    {t("flashcards.timingsOriginal")}
                  </div>
                </label>
                <label class="flex-1 cursor-pointer { !hasReferenceSubs ? 'opacity-50 cursor-not-allowed' : '' }">
                  <input
                    type="radio"
                    checked={useTimingsFrom === "native"}
                    value="native"
                    disabled={!hasReferenceSubs}
                    class="peer sr-only"
                    onchange={() => handleTimingSourceChange("native")}
                  />
                  <div class="timing-source-choice">
                    {t("flashcards.timingsReference")}
                  </div>
                </label>
              </div>
            </div>

            <div class="grid grid-cols-2 gap-2">
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.spanStart")}</span
                >
                <input
                  type="text"
                  bind:value={spanStart}
                  class="input-modern w-full text-xs"
                  placeholder="h:mm:ss"
                />
              </div>
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.spanEnd")}</span
                >
                <input
                  type="text"
                  bind:value={spanEnd}
                  class="input-modern w-full text-xs"
                  placeholder="h:mm:ss"
                />
              </div>
            </div>

            <div class="grid grid-cols-2 gap-2">
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.timeShiftOriginal")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={timeShiftTarget}
                    class="input-modern w-full text-xs"
                  />
                  <span class="text-xs text-gray-500">ms</span>
                </div>
              </div>
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.timeShiftReference")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={timeShiftNative}
                    class="input-modern w-full text-xs"
                    disabled={!hasReferenceSubs}
                  />
                  <span class="text-xs text-gray-500">ms</span>
                </div>
              </div>
            </div>

            <div class="flex items-center gap-3 pt-1">
              <label class="vesta-check-row flex items-center gap-2">
                <input
                  type="checkbox"
                  bind:checked={combineSentences}
                  class="vesta-check-input text-emerald-500"
                />
                <span class="text-xs text-gray-300"
                  >{t("flashcards.combineSentences")}</span
                >
              </label>
              {#if combineSentences}
                <input
                  type="text"
                  bind:value={continuationChars}
                  class="input-modern w-28 text-xs"
                  placeholder=",、→"
                  title={t("flashcards.continuationCharsHint")}
                />
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {:else if panelId === "filters"}
      <div
        inert={!hasAnyFiles}
        title={!hasAnyFiles ? HINT_LOAD_TARGET_FIRST : undefined}
        class="glass-card p-4 {!hasAnyFiles
          ? 'opacity-40'
          : ''}"
      >
        <div class="flex items-center gap-2">
          <button
            onclick={() => {
              if (!hasAnyFiles) return;
              showFilters = !showFilters;
            }}
            class="flex-1 flex items-center justify-between text-sm font-semibold text-orange-400"
          >
            <span class="flex items-center gap-2">
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
                  d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z"
                />
              </svg>
              {t("flashcards.filters")}
            </span>
            <svg
              class="w-4 h-4 transition-transform {showFilters
                ? 'rotate-180'
                : ''}"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M19 9l-7 7-7-7"
              />
            </svg>
          </button>
          <InfoButton onclick={() => (helpSection = "filters")} />
        </div>

        {#if showFilters}
          <div class="mt-3 space-y-2.5 animate-fade-in">
            <div>
              <span class="block text-xs text-gray-500 mb-1"
                >{t("flashcards.includeWords")}</span
              >
              <input
                type="text"
                bind:value={includeWords}
                class="input-modern w-full text-xs"
                placeholder={t("flashcards.includeWordsHint")}
              />
            </div>
            <div>
              <span class="block text-xs text-gray-500 mb-1"
                >{t("flashcards.excludeWords")}</span
              >
              <input
                type="text"
                bind:value={excludeWords}
                class="input-modern w-full text-xs"
                placeholder={t("flashcards.excludeWordsHint")}
              />
            </div>

            <div class="flex flex-col gap-2">
              <div class="grid grid-cols-2 gap-2">
                <label class="filter-pill-check justify-center">
                  <input
                    type="checkbox"
                    bind:checked={excludeDuplicatesSubs1}
                    class="sr-only"
                  />
                  <span class="text-xs font-medium text-gray-300"
                    >{t("flashcards.excludeDupSubs1")}</span
                  >
                </label>
                <label class="filter-pill-check justify-center { !nativeSubsPath ? 'opacity-50 cursor-not-allowed' : '' }">
                  <input
                    type="checkbox"
                    bind:checked={excludeDuplicatesSubs2}
                    disabled={!nativeSubsPath}
                    class="sr-only"
                  />
                  <span
                    class="text-xs font-medium text-gray-300">{t("flashcards.excludeDupSubs2")}</span
                  >
                </label>
              </div>
            </div>

            <div class="grid grid-cols-2 gap-2">
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.minChars")}</span
                >
                <input
                  type="number"
                  bind:value={minChars}
                  class="input-modern w-full text-xs"
                  min="0"
                  placeholder="—"
                />
              </div>
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.maxChars")}</span
                >
                <input
                  type="number"
                  bind:value={maxChars}
                  class="input-modern w-full text-xs"
                  min="0"
                  placeholder="—"
                />
              </div>
            </div>

            <div class="grid grid-cols-2 gap-2">
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.minDuration")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={minDurationMs}
                    class="input-modern w-full text-xs"
                    min="0"
                    placeholder="—"
                  />
                  <span class="text-xs text-gray-500">ms</span>
                </div>
              </div>
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.maxDuration")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={maxDurationMs}
                    class="input-modern w-full text-xs"
                    min="0"
                    placeholder="—"
                  />
                  <span class="text-xs text-gray-500">ms</span>
                </div>
              </div>
            </div>

            <div class="grid gap-1.5">
              <label class="vesta-check-row">
                <input
                  type="checkbox"
                  bind:checked={excludeStyled}
                  class="vesta-check-input text-orange-500"
                />
                <span class="text-xs text-gray-300"
                  >{t("flashcards.excludeStyled")}</span
                >
              </label>
              <label class="vesta-check-row">
                <input
                  type="checkbox"
                  bind:checked={onlyCjk}
                  class="vesta-check-input text-orange-500"
                />
                <span class="text-xs text-gray-300"
                  >{t("flashcards.onlyCjk")}</span
                >
              </label>
              <label class="vesta-check-row {!nativeSubsPath ? 'opacity-50 cursor-not-allowed' : ''}">
                <input
                  type="checkbox"
                  bind:checked={removeNoMatch}
                  class="vesta-check-input text-orange-500"
                  disabled={!nativeSubsPath}
                />
                <span
                  class="text-xs text-gray-300">{t("flashcards.removeNoMatch")}</span
                >
              </label>
            </div>

            {#if targetSubsInfo && targetSubsInfo.actors.length > 0}
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.actorFilter")}</span
                >
                <input
                  type="text"
                  bind:value={actorFilter}
                  class="input-modern w-full text-xs"
                  placeholder={targetSubsInfo.actors.join(", ")}
                />
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {:else if panelId === "contextLines"}
      <div
        inert={!hasAnyFiles}
        title={!hasAnyFiles ? HINT_LOAD_TARGET_FIRST : undefined}
        class="glass-card p-4 {!hasAnyFiles
          ? 'opacity-40'
          : ''}"
      >
        <div class="flex items-center gap-2">
          <button
            onclick={() => {
              if (hasAnyFiles) showContextLines = !showContextLines;
            }}
            class="flex-1 flex items-center justify-between text-sm font-semibold text-indigo-400"
          >
            <span class="flex items-center gap-2">
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
                  d="M4 6h16M4 10h16M4 14h16M4 18h16"
                />
              </svg>
              {t("flashcards.contextLines")}
            </span>
            <svg
              class="w-4 h-4 transition-transform {showContextLines
                ? 'rotate-180'
                : ''}"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M19 9l-7 7-7-7"
              />
            </svg>
          </button>
          <InfoButton onclick={() => (helpSection = "contextLines")} />
        </div>
        {#if showContextLines}
          <div class="mt-3 grid grid-cols-3 gap-2 animate-fade-in">
            <div>
              <span class="block text-xs text-gray-500 mb-1"
                >{t("flashcards.leading")}</span
              >
              <input
                type="number"
                bind:value={contextLeading}
                class="input-modern w-full text-xs"
                min="0"
                max="10"
              />
            </div>
            <div>
              <span class="block text-xs text-gray-500 mb-1"
                >{t("flashcards.trailing")}</span
              >
              <input
                type="number"
                bind:value={contextTrailing}
                class="input-modern w-full text-xs"
                min="0"
                max="10"
              />
            </div>
            <div>
              <span class="block text-xs text-gray-500 mb-1"
                >{t("flashcards.maxGap")}</span
              >
              <div class="flex items-center gap-1">
                <input
                  type="number"
                  bind:value={contextMaxGap}
                  class="input-modern w-full text-xs"
                  min="0"
                  step="0.5"
                />
                <span class="text-xs text-gray-500">s</span>
              </div>
            </div>
          </div>
        {/if}
      </div>
    {:else if panelId === "audioClips"}
      <div
        inert={!hasAudio}
        title={!hasAudio ? HINT_LOAD_MEDIA_FIRST : undefined}
        class="glass-card p-4 {!hasAudio
          ? 'opacity-40'
          : ''}"
        style="overflow: visible; position: relative; z-index: 10;"
      >
        <div class="flex items-center justify-between mb-3">
          <h3
            class="text-sm font-semibold flex items-center gap-2 text-cyan-400"
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
                d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z"
              />
            </svg>
            {t("flashcards.generateAudioClips")}
            <InfoButton onclick={() => (helpSection = "audioClips")} />
          </h3>
          <button
            onclick={() => {
              if (hasAudio) generateAudio = !generateAudio;
            }}
            class="w-10 h-5 rounded-full transition-all duration-200 relative
              {generateAudio && hasAudio ? 'bg-cyan-500' : 'bg-gray-600'}"
            aria-label="Toggle audio clips"
            disabled={!hasAudio}
          >
            <div
              class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200
              {generateAudio && hasAudio ? 'left-5' : 'left-0.5'}"
            ></div>
          </button>
        </div>

        {#if generateAudio && hasAudio}
          <div class="space-y-2 animate-fade-in">
            <div class="grid grid-cols-2 gap-2">
              {#if mediaType === "video" && (audioTracksLoading || audioTracks.length >= 1)}
                <div>
                  <span class="block text-xs text-gray-500 mb-1"
                    >{t("flashcards.audioTrack")}</span
                  >
                  {#if audioTracksLoading}
                    <div class="input-modern text-xs text-gray-500">
                      {t("flashcards.audioTracksLoading")}
                    </div>
                  {:else if audioTracks.length > 1}
                    <SearchableSelect
                      noResultsText={t("common.noResults")}
                      options={audioTracks.map((track) => ({
                        value: String(track.index),
                        label: formatAudioTrackLabel(track),
                      }))}
                      value={selectedAudioTrackIndex === null ? "" : String(selectedAudioTrackIndex)}
                      onchange={(value) => {
                        selectedAudioTrackIndex = value === "" ? null : Number(value);
                        audioTrackAutoSelected = false;
                      }}
                      placeholder={t("flashcards.audioTrack")}
                    />
                  {:else}
                    <div class="input-modern text-xs text-gray-500 opacity-60 cursor-not-allowed">
                      {formatAudioTrackLabel(audioTracks[0])}
                    </div>
                  {/if}
                </div>
              {/if}

              <div class={mediaType === "video" && (audioTracksLoading || audioTracks.length >= 1) ? "" : "col-span-2"}>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.bitrate")}</span
                >
                <SearchableSelect
                  noResultsText={t("common.noResults")}
                  options={[
                    { value: "64", label: "64 kb/s" },
                    { value: "128", label: "128 kb/s" },
                    { value: "192", label: "192 kb/s" },
                    { value: "256", label: "256 kb/s" },
                    { value: "320", label: "320 kb/s" },
                  ]}
                  value={String(audioBitrate)}
                  onchange={(v) => (audioBitrate = parseInt(v))}
                  placeholder="Bitrate"
                />
              </div>
            </div>
            <div class="grid grid-cols-3 gap-2 items-end">
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.padStart")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={audioPadStart}
                    class="input-modern w-full text-xs"
                  />
                  <span class="text-xs text-gray-500">ms</span>
                </div>
              </div>
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.padEnd")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={audioPadEnd}
                    class="input-modern w-full text-xs"
                  />
                  <span class="text-xs text-gray-500">ms</span>
                </div>
              </div>
              <div class="flex justify-center">
                <label class="vesta-check-row min-h-[42px] w-full">
                  <input
                    type="checkbox"
                    bind:checked={normalizeAudio}
                    class="vesta-check-input shrink-0"
                  />
                  <span class="min-w-0 text-left text-xs font-medium text-gray-300"
                    >{t("flashcards.normalizeAudio")}</span
                  >
                </label>
              </div>
            </div>
          </div>
        {/if}
      </div>
    {:else if panelId === "snapshots"}
      <div
        inert={!hasVideo}
        title={!hasVideo ? HINT_LOAD_VIDEO_FIRST : undefined}
        class="glass-card p-4 {!hasVideo
          ? 'opacity-40'
          : ''}"
      >
        <div class="flex items-center justify-between mb-3">
          <h3
            class="text-sm font-semibold flex items-center gap-2 text-purple-400"
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
                d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
              />
            </svg>
            {t("flashcards.generateSnapshots")}
            <InfoButton onclick={() => (helpSection = "snapshots")} />
          </h3>
          <button
            onclick={() => {
              if (hasVideo) {
                generateSnapshots = !generateSnapshots;
              }
            }}
            class="w-10 h-5 rounded-full transition-all duration-200 relative
              {generateSnapshots && hasVideo ? 'bg-purple-500' : 'bg-gray-600'}"
            aria-label="Toggle snapshots"
            disabled={!hasVideo}
          >
            <div
              class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200
              {generateSnapshots && hasVideo ? 'left-5' : 'left-0.5'}"
            ></div>
          </button>
        </div>

        {#if generateSnapshots && hasVideo}
          <div class="space-y-2 animate-fade-in">
            <div class="grid grid-cols-3 gap-2">
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.width")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={snapshotWidth}
                    class="input-modern w-full text-xs"
                  />
                  <span class="text-xs text-gray-500">px</span>
                </div>
              </div>
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.height")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={snapshotHeight}
                    class="input-modern w-full text-xs"
                  />
                  <span class="text-xs text-gray-500">px</span>
                </div>
              </div>
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.cropBottom")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={cropBottom}
                    class="input-modern w-full text-xs"
                  />
                  <span class="text-xs text-gray-500">px</span>
                </div>
              </div>
            </div>
          </div>
        {/if}
      </div>
    {:else if panelId === "videoClips"}
      <div
        inert={!hasVideo}
        title={!hasVideo ? HINT_LOAD_VIDEO_FIRST : undefined}
        class="glass-card p-4 {!hasVideo
          ? 'opacity-40'
          : ''}"
        style="overflow: visible; position: relative; z-index: 5;"
      >
        <div class="flex items-center justify-between mb-3">
          <h3
            class="text-sm font-semibold flex items-center gap-2 text-rose-400"
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
                d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"
              />
            </svg>
            {t("flashcards.generateVideoClips")}
            <InfoButton onclick={() => (helpSection = "videoClips")} />
          </h3>
          <button
            onclick={() => {
              if (hasVideo) {
                generateVideoClips = !generateVideoClips;
              }
            }}
            class="w-10 h-5 rounded-full transition-all duration-200 relative
              {generateVideoClips && hasVideo ? 'bg-rose-500' : 'bg-gray-600'}"
            aria-label="Toggle video clips"
            disabled={!hasVideo}
          >
            <div
              class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200
              {generateVideoClips && hasVideo ? 'left-5' : 'left-0.5'}"
            ></div>
          </button>
        </div>

        {#if generateVideoClips && hasVideo}
          <div class="space-y-2 animate-fade-in">
            <div class="grid grid-cols-2 gap-2">
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.width")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={snapshotWidth}
                    class="input-modern w-full text-xs"
                  />
                  <span class="text-xs text-gray-500">px</span>
                </div>
              </div>
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.height")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={snapshotHeight}
                    class="input-modern w-full text-xs"
                  />
                  <span class="text-xs text-gray-500">px</span>
                </div>
              </div>
            </div>
            <div class="grid grid-cols-2 gap-2">
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.videoCodec")}</span
                >
                <SearchableSelect
                  className="compact-select"
                  noResultsText={t("common.noResults")}
                  options={[
                    { value: "h264", label: "H.264 (MP4)" },
                    { value: "mpeg4", label: "MPEG-4 (AVI)" },
                  ]}
                  value={videoCodec}
                  onchange={(v) => (videoCodec = v)}
                  placeholder="Codec"
                />
              </div>
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.h264Preset")}</span
                >
                <SearchableSelect
                  className="compact-select"
                  noResultsText={t("common.noResults")}
                  options={[
                    { value: "ultrafast", label: "Ultrafast" },
                    { value: "fast", label: "Fast" },
                    { value: "medium", label: "Medium" },
                    { value: "slow", label: "Slow" },
                    { value: "veryslow", label: "Very slow" },
                  ]}
                  value={h264Preset}
                  onchange={(v) => (h264Preset = v)}
                  placeholder="Preset"
                />
              </div>
            </div>
            <div class="grid grid-cols-2 gap-2">
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.videoBitrate")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={videoBitrate}
                    class="input-modern w-full text-xs"
                  />
                  <span class="text-xs text-gray-500">kb/s</span>
                </div>
              </div>
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.audioBitrate")}</span
                >
                <SearchableSelect
                  className="compact-select"
                  noResultsText={t("common.noResults")}
                  options={[
                    { value: "64", label: "64 kb/s" },
                    { value: "128", label: "128 kb/s" },
                    { value: "192", label: "192 kb/s" },
                    { value: "256", label: "256 kb/s" },
                  ]}
                  value={String(videoAudioBitrate)}
                  onchange={(v) => (videoAudioBitrate = parseInt(v))}
                  placeholder="Bitrate"
                />
              </div>
            </div>
            <div class="grid grid-cols-2 gap-2">
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.padStart")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={videoPadStart}
                    class="input-modern w-full text-xs"
                  />
                  <span class="text-xs text-gray-500">ms</span>
                </div>
              </div>
              <div>
                <span class="block text-xs text-gray-500 mb-1"
                  >{t("flashcards.padEnd")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={videoPadEnd}
                    class="input-modern w-full text-xs"
                  />
                  <span class="text-xs text-gray-500">ms</span>
                </div>
              </div>
            </div>
          </div>
        {/if}
      </div>
    {:else if panelId === "ankiFields"}
      <div
        inert={!hasAnyFiles}
        title={!hasAnyFiles ? HINT_LOAD_TARGET_FIRST : undefined}
        class="glass-card p-4 {panelHighlightClass('ankiFields')} {!hasAnyFiles
          ? 'opacity-50'
          : ''}"
      >
        <div class="flex items-center gap-2">
          <button
            type="button"
            onclick={toggleAnkiFieldsPanel}
            class="flex-1 flex items-center justify-between text-sm font-semibold text-lime-400"
          >
            <span class="flex items-center gap-2">
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
                  d="M9 17V7m0 10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h2a2 2 0 012 2m0 10a2 2 0 002 2h2a2 2 0 002-2M9 7a2 2 0 012-2h2a2 2 0 012 2m0 10V7m0 10a2 2 0 002 2h2a2 2 0 002-2V7a2 2 0 00-2-2h-2a2 2 0 00-2 2"
                />
              </svg>
              {t("flashcards.ankiFields")}
            </span>
            <svg
              class="w-4 h-4 transition-transform {showAnkiFields ? 'rotate-180' : ''}"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
            </svg>
          </button>
          <InfoButton onclick={() => (helpSection = "ankiFields")} />
        </div>

        {#if showAnkiFields}
        <div class="mt-3 mb-3">
          <span class="block text-xs text-gray-400 mb-1"
            >{t("flashcards.noteTypeLanguage")}</span
          >
          <SearchableSelect
            noResultsText={t("common.noResults")}
            options={languages.map((lang) => ({
              value: lang.code,
              label:
                lang.nameEn === lang.name
                  ? lang.name
                  : `${lang.nameEn} — ${lang.name}`,
              searchTerms: getLanguageSearchTerms(lang.code),
              icon: lang.flag,
            }))}
            value={noteTypeLanguage}
            onchange={(v) => {
              noteTypeLanguage = v;
              if (v) {
                localStorage.setItem(NOTE_TYPE_LANGUAGE_KEY, v);
              } else {
                localStorage.removeItem(NOTE_TYPE_LANGUAGE_KEY);
              }
            }}
            placeholder={t("flashcards.noteTypeLanguagePlaceholder")}
          />
        </div>

        <div class="mb-3 flex items-center gap-1.5">
          <span class="text-xs text-gray-400"
            >{t("flashcards.noteTypeName")}:</span
          >
          <span
            class="text-xs text-white font-mono bg-white/10 px-2 py-0.5 rounded font-medium"
            >{noteTypeName || "—"}</span
          >
        </div>

        <span class="block text-xs text-gray-500 mb-2"
          >{t("flashcards.fieldsLabel")}</span
        >
        <div class="flex flex-wrap gap-2">
          <button
            type="button"
            onclick={() => (includeSubs1Field = !includeSubs1Field)}
            class="px-3 py-1.5 rounded-full text-xs font-medium border transition-all {includeSubs1Field
              ? 'bg-lime-500/20 border-lime-500/50 text-lime-300'
              : 'bg-gray-800/50 border-gray-700/50 text-gray-500 hover:border-gray-600'}"
          >
            🗣️ {t("flashcards.subs1")}
          </button>
          <button
            type="button"
            onclick={() => {
              if (nativeSubsPath) includeSubs2Field = !includeSubs2Field;
            }}
            class="px-3 py-1.5 rounded-full text-xs font-medium border transition-all {!nativeSubsPath
              ? 'opacity-40 cursor-not-allowed'
              : ''} {includeSubs2Field && nativeSubsPath
              ? 'bg-lime-500/20 border-lime-500/50 text-lime-300'
              : 'bg-gray-800/50 border-gray-700/50 text-gray-500 hover:border-gray-600'}"
          >
            💬 {t("flashcards.subs2")}
          </button>
          <button
            type="button"
            onclick={() => (includeAudioField = !includeAudioField)}
            class="px-3 py-1.5 rounded-full text-xs font-medium border transition-all {includeAudioField
              ? 'bg-lime-500/20 border-lime-500/50 text-lime-300'
              : 'bg-gray-800/50 border-gray-700/50 text-gray-500 hover:border-gray-600'}"
          >
            🔊 {t("flashcards.audioField")}
          </button>
          <button
            type="button"
            onclick={() => (includeSnapshotField = !includeSnapshotField)}
            class="px-3 py-1.5 rounded-full text-xs font-medium border transition-all {includeSnapshotField
              ? 'bg-lime-500/20 border-lime-500/50 text-lime-300'
              : 'bg-gray-800/50 border-gray-700/50 text-gray-500 hover:border-gray-600'}"
          >
            📸 {t("flashcards.snapshotField")}
          </button>
          <button
            type="button"
            onclick={() => (includeVideoField = !includeVideoField)}
            class="px-3 py-1.5 rounded-full text-xs font-medium border transition-all {includeVideoField
              ? 'bg-lime-500/20 border-lime-500/50 text-lime-300'
              : 'bg-gray-800/50 border-gray-700/50 text-gray-500 hover:border-gray-600'}"
          >
            🎬 {t("flashcards.videoField")}
          </button>
          <button
            type="button"
            onclick={() => (includeTag = !includeTag)}
            class="px-3 py-1.5 rounded-full text-xs font-medium border transition-all {includeTag
              ? 'bg-lime-500/20 border-lime-500/50 text-lime-300'
              : 'bg-gray-800/50 border-gray-700/50 text-gray-500 hover:border-gray-600'}"
          >
            🏷️ {t("flashcards.tagField")}
          </button>
          <button
            type="button"
            onclick={() => (includeSequence = !includeSequence)}
            class="px-3 py-1.5 rounded-full text-xs font-medium border transition-all {includeSequence
              ? 'bg-lime-500/20 border-lime-500/50 text-lime-300'
              : 'bg-gray-800/50 border-gray-700/50 text-gray-500 hover:border-gray-600'}"
          >
            🔢 {t("flashcards.sequenceField")}
          </button>
        </div>
        {/if}
      </div>
    {:else if panelId === "exportFormat"}
      <div
        inert={!hasAnyFiles}
        title={!hasAnyFiles ? HINT_LOAD_TARGET_FIRST : undefined}
        class="glass-card p-4 {!hasAnyFiles
          ? 'opacity-50'
          : ''}"
      >
        <h3
          class="text-sm font-semibold mb-3 flex items-center gap-2 text-sky-400"
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
              d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
          {t("flashcards.exportFormat")}
          <InfoButton onclick={() => (helpSection = "exportFormat")} />
        </h3>
        <div class="space-y-2">
          <label
            class="flex items-start gap-2 p-2 rounded-lg cursor-pointer transition-colors
            {exportFormat === 'apkg'
              ? 'bg-emerald-500/10 border border-emerald-500/30'
              : 'bg-transparent border border-transparent hover:bg-gray-800/50'}"
          >
            <input
              type="radio"
              bind:group={exportFormat}
              value="apkg"
              class="mt-0.5 text-emerald-500"
            />
            <div class="flex-1">
              <span class="text-xs font-medium text-gray-200"
                >{t("flashcards.exportAPKG")}</span
              >
              <span
                class="ml-1.5 text-[9px] bg-emerald-500/20 text-emerald-400 px-1.5 py-0.5 rounded-full font-semibold uppercase"
                >{t("flashcards.exportAPKGBadge")}</span
              >
              <p class="text-[10px] text-gray-500">
                {t("flashcards.exportAPKGDesc")}
              </p>
            </div>
          </label>
          <label
            class="flex items-start gap-2 p-2 rounded-lg cursor-pointer transition-colors
            {exportFormat === 'tsv'
              ? 'bg-sky-500/10 border border-sky-500/30'
              : 'bg-transparent border border-transparent hover:bg-gray-800/50'}"
          >
            <input
              type="radio"
              bind:group={exportFormat}
              value="tsv"
              class="mt-0.5 text-sky-500"
            />
            <div class="flex-1">
              <span class="text-xs font-medium text-gray-200"
                >{t("flashcards.exportTSV")}</span
              >
              <span
                class="ml-1.5 text-[9px] bg-gray-500/20 text-gray-400 px-1.5 py-0.5 rounded-full font-semibold uppercase"
                >{t("flashcards.exportTSVBadge")}</span
              >
              <p class="text-[10px] text-gray-500">
                {t("flashcards.exportTSVDesc")}
              </p>
            </div>
          </label>

          {#if seriesMode && exportFormat === "apkg"}
            <!-- Series output mode (only for APKG) -->
            <div class="mt-4 pt-3 border-t border-gray-700/50">
              <span class="block text-xs text-gray-400 mb-2"
                >{t("flashcards.seriesOutputFormat")}</span
              >
              <div class="flex gap-2">
                <button
                  onclick={() => (seriesOutputMode = "separate")}
                  class="flex-1 py-2 px-3 text-xs rounded-lg border transition-colors {seriesOutputMode ===
                  'separate'
                    ? 'border-violet-500/50 bg-violet-500/10 text-violet-300'
                    : 'border-gray-700/50 text-gray-400 hover:border-gray-600 bg-gray-900/40'}"
                >
                  <div
                    class="font-medium mb-0.5 text-gray-200 {seriesOutputMode ===
                    'separate'
                      ? 'text-violet-200'
                      : ''}"
                  >
                    {t("flashcards.outputPerEpisode")}
                  </div>
                  <div class="text-[10px] opacity-80">
                    {t("flashcards.outputPerEpisodeDesc")}
                  </div>
                </button>
                <button
                  onclick={() => (seriesOutputMode = "single")}
                  class="flex-1 py-2 px-3 text-xs rounded-lg border transition-colors {seriesOutputMode ===
                  'single'
                    ? 'border-violet-500/50 bg-violet-500/10 text-violet-300'
                    : 'border-gray-700/50 text-gray-400 hover:border-gray-600 bg-gray-900/40'}"
                >
                  <div
                    class="font-medium mb-0.5 text-gray-200 {seriesOutputMode ===
                    'single'
                      ? 'text-violet-200'
                      : ''}"
                  >
                    {t("flashcards.outputSingleApkg")}
                  </div>
                  <div class="text-[10px] opacity-80">
                    {t("flashcards.outputSingleApkgDesc")}
                  </div>
                </button>
              </div>
            </div>
          {/if}
        </div>
      </div>
    {:else if panelId === "naming"}
      <div
        inert={!hasAnyFiles}
        title={!hasAnyFiles ? HINT_LOAD_TARGET_FIRST : undefined}
        class="glass-card p-4 {panelHighlightClass('naming')} {!hasAnyFiles
          ? 'opacity-50'
          : ''}"
      >
        <h3
          class="text-sm font-semibold mb-3 flex items-center gap-2 text-amber-400"
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
              d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"
            />
          </svg>
          {t("flashcards.naming")}
          <InfoButton onclick={() => (helpSection = "naming")} />
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
                bind:value={deckName}
                oninput={(event) => {
                  deckNameAuto =
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
    {:else if panelId === "cpuCores"}
      <div
        inert={!hasAnyFiles}
        title={!hasAnyFiles ? HINT_LOAD_TARGET_FIRST : undefined}
        class="glass-card p-4 {!hasAnyFiles
          ? 'opacity-50'
          : ''}"
      >
        <h3
          class="text-sm font-semibold mb-3 flex items-center gap-2 text-orange-400"
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
              d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"
            />
          </svg>
          {t("flashcards.cpuCores")}
          <InfoButton onclick={() => (helpSection = "cpuCores")} />
        </h3>
        <div class="grid grid-cols-4 gap-2 mb-3">
          <button
            onclick={() => setCpuPreset("eco")}
            class="p-2 rounded-lg text-center transition-all duration-200 border text-xs {activeCpuPreset ===
            'eco'
              ? 'bg-orange-500/20 border-orange-500/50 text-white'
              : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
          >
            <span class="block mb-1 text-white">
              <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M5 14c0-5.523 4.477-10 10-10h4v4c0 5.523-4.477 10-10 10H5v-4z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M7 17c2.5-2.5 5.5-4.5 9-6" />
              </svg>
            </span>
            <span class="font-semibold block">{t("flashcards.cpuEco")}</span>
          </button>
          <button
            onclick={() => setCpuPreset("balanced")}
            class="p-2 rounded-lg text-center transition-all duration-200 border text-xs {activeCpuPreset ===
            'balanced'
              ? 'bg-orange-500/20 border-orange-500/50 text-white'
              : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
          >
            <span class="block mb-1 text-white">
              <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 4v16" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M6 7h12" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M8 7l-3 5h6L8 7z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M16 7l-3 5h6l-3-5z" />
              </svg>
            </span>
            <span class="font-semibold block"
              >{t("flashcards.cpuBalanced")}</span
            >
          </button>
          <button
            onclick={() => setCpuPreset("performance")}
            class="p-2 rounded-lg text-center transition-all duration-200 border text-xs {activeCpuPreset ===
            'performance'
              ? 'bg-orange-500/20 border-orange-500/50 text-white'
              : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
          >
            <span class="block mb-1 text-white">
              <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M5 16l5-5 3 3 6-7" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M14 7h5v5" />
              </svg>
            </span>
            <span class="font-semibold block"
              >{t("flashcards.cpuPerformance")}</span
            >
          </button>
          <button
            onclick={() => setCpuPreset("full")}
            class="p-2 rounded-lg text-center transition-all duration-200 border text-xs {activeCpuPreset ===
            'full'
              ? 'bg-orange-500/20 border-orange-500/50 text-white'
              : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
          >
            <span class="block mb-1 text-white">
              <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M11 3L6 13h5l-1 8 8-12h-5l2-6h-4z" />
              </svg>
            </span>
            <span class="font-semibold block"
              >{t("flashcards.cpuFullPower")}</span
            >
          </button>
        </div>
        <div class="flex items-center justify-between text-xs">
          <span class="text-gray-500">{t("flashcards.cpuCoresUsage")}</span>
          <span
            class="text-white font-mono bg-white/10 px-2 py-0.5 rounded text-sm"
            >{cpuCores} / {systemCpuCount}</span
          >
        </div>
      </div>
    {:else if panelId === "actions"}
      <div class="space-y-3">
        {#if isProcessing}
          <button
            onclick={cancelGeneration}
            class="btn-danger w-full py-4 text-lg"
          >
            <svg
              class="w-5 h-5 inline mr-2"
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
              class="btn-success w-full py-4 text-lg disabled:cursor-help disabled:opacity-55 {!canRunFlashcards ? 'pointer-events-none saturate-75' : ''}"
            >
              <svg
                class="w-5 h-5 inline mr-2"
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
                class="pointer-events-none absolute bottom-full left-1/2 z-40 mb-3 w-[min(22rem,calc(100vw-3rem))] -translate-x-1/2 rounded-xl border border-amber-400/30 bg-gray-950/95 p-3 text-left text-xs text-gray-200 opacity-0 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 group-hover:opacity-100 group-hover:translate-y-0 {generationPromptOpen ? 'opacity-100 translate-y-0' : 'translate-y-1'}"
              >
                <p class="mb-2 font-semibold text-amber-200">
                  Mancano ancora questi passaggi:
                </p>
                <ol class="list-decimal space-y-1 pl-4 text-gray-300">
                  {#each generationRequirements as requirement}
                    <li>{requirement.label}</li>
                  {/each}
                </ol>
              </div>
            {/if}
          </div>

          <button
            class="btn-secondary w-full py-2 disabled:opacity-50 disabled:cursor-not-allowed"
            disabled={!canRunFlashcards}
            title={!canRunFlashcards
              ? `Completa: ${generationRequirementsText}`
              : undefined}
            onclick={loadPreview}
          >
            <svg
              class="w-4 h-4 inline mr-2"
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
        {/if}

        {#if result}
          <button
            onclick={resetGeneration}
            class="w-full py-2 rounded-lg border border-amber-500/30 bg-amber-500/10 text-amber-300 hover:bg-amber-500/20 transition-colors text-sm font-medium"
            title={t("flashcards.newGenerationDesc")}
          >
            <svg
              class="w-4 h-4 inline mr-2"
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
        {/if}
      </div>
    {:else if panelId === "progressResult"}
      <div class="space-y-3">
        {#if isProcessing || progress > 0}
          <div
            class="glass-card p-4 {isProcessing ? 'animate-pulse-glow' : ''}"
          >
            <div class="flex items-center gap-4">
              <div class="flex-1">
                <div class="progress-modern h-2">
                  <div
                    class="progress-modern-bar bg-gradient-to-r from-emerald-500 to-teal-500"
                    style="width: {progress}%"
                  ></div>
                </div>
              </div>
              <span class="text-lg font-bold text-emerald-400">{progress}%</span
              >
            </div>
            {#if progressMessage}
              <p class="text-gray-400 text-xs mt-2">{progressMessage}</p>
            {/if}
            {#if progressStage}
              <div class="flex gap-1.5 mt-2">
                {#each Array(10) as _, i}
                  {@const threshold = (i + 1) * 10}
                  <div
                    class="h-1 flex-1 rounded-full transition-colors duration-300 {progress >=
                    threshold
                      ? 'bg-emerald-700'
                      : progress >= threshold - 10
                        ? 'bg-emerald-400'
                        : 'bg-gray-700'}"
                  ></div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
        {#if result}
          <div
            class="glass-card p-4 border-l-4 {result.success
              ? 'border-green-500 bg-green-500/5'
              : 'border-red-500 bg-red-500/5'}"
          >
            {#if result.success}
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
                    {result.cardsGenerated}
                    {t("flashcards.cardsGenerated")}
                  </p>
                </div>
                <div class="flex gap-4 text-xs text-gray-400">
                  {#if result.audioClips > 0}
                    <span>🔊 {result.audioClips} audio</span>
                  {/if}
                  {#if result.snapshots > 0}
                    <span>📸 {result.snapshots} snapshots</span>
                  {/if}
                  {#if result.videoClips > 0}
                    <span>🎬 {result.videoClips} video</span>
                  {/if}
                </div>
                {#if result.tsvPath}
                  <p
                    class="text-xs text-gray-500 break-words"
                    title={result.tsvPath}
                  >
                    📄 {result.tsvPath}
                  </p>
                {/if}
                {#if result.apkgPath}
                  <p
                    class="text-xs text-gray-500 break-words"
                    title={result.apkgPath}
                  >
                    📦 {result.apkgPath}
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
                <p class="text-red-300">{t("flashcards.noActiveLines")}</p>
              </div>
            {/if}
          </div>
        {/if}
        {#if error}
          <div class="glass-card p-4 border border-red-500/30 bg-red-500/10">
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
              <p class="text-red-300 flex-1 text-sm break-words">{error}</p>
              <button
                onclick={() => (error = null)}
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
        {logs}
        onclear={clearLogs}
        minHeight="180px"
        maxHeightContent="16rem"
      />
    {/if}
  {/snippet}

  <div bind:this={layoutHostEl} class="flex-1 grid {gridColClass} gap-4 min-h-0 overflow-y-auto">
    {#if seriesMode}
      <!-- In series mode, render the files panel full-width above the columns -->
      <div class="{effectiveColumnCount >= 2 ? 'col-span-2' : ''} {effectiveColumnCount >= 3 ? 'col-span-3' : ''} mb-1">
        {@render panelContent("files")}
      </div>
    {/if}
    <div class="space-y-3 {seriesMode ? '' : 'overflow-y-auto'} pr-1 min-h-[100px]" role="list">
      {#each effectivePanelLayout.col1 as panelId, idx (panelId)}
        {#if !(seriesMode && panelId === "files")}
        <div class="relative transition-all duration-150" role="listitem">
          {@render panelContent(panelId)}
        </div>
        {/if}
      {/each}
    </div>

    {#if effectiveColumnCount >= 2}
      <div class="space-y-3 {seriesMode ? '' : 'overflow-y-auto'} pr-1 min-h-[100px]" role="list">
        {#each effectivePanelLayout.col2 as panelId, idx (panelId)}
          {#if !(seriesMode && panelId === "files")}
          <div class="relative transition-all duration-150" role="listitem">
            {@render panelContent(panelId)}
          </div>
          {/if}
        {/each}
      </div>
    {/if}

    {#if effectiveColumnCount >= 3}
      <div class="space-y-3 {seriesMode ? '' : 'overflow-y-auto'} pr-1 min-h-[100px]" role="list">
        {#each effectivePanelLayout.col3 as panelId, idx (panelId)}
          {#if !(seriesMode && panelId === "files")}
          <div class="transition-all duration-150" role="listitem">
            {@render panelContent(panelId)}
          </div>
          {/if}
        {/each}
      </div>
    {/if}
  </div>

  {#if episodeContextMenu}
    {@const contextEpisode = episodes[episodeContextMenu.idx]}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50"
      onclick={closeEpisodeContextMenu}
      oncontextmenu={(e) => {
        e.preventDefault();
        closeEpisodeContextMenu();
      }}
      onkeydown={(e) => {
        if (e.key === "Escape") closeEpisodeContextMenu();
      }}
      role="presentation"
      tabindex="-1"
    >
      <div
        class="vesta-context-menu animate-fade-in"
        style="left: {episodeContextMenu.x}px; top: {episodeContextMenu.y}px;"
      >
        <button
          type="button"
          class="vesta-context-menu-item"
          onclick={() => openEpisodeEditor(episodeContextMenu!.idx)}
        >
          <span class="inline-flex items-center gap-2">
            <svg class="h-4 w-4 text-amber-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4H6a2 2 0 00-2 2v12a2 2 0 002 2h12a2 2 0 002-2v-5M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z" />
            </svg>
            {t("common.edit")}
          </span>
        </button>
        <button
          type="button"
          class="vesta-context-menu-item"
          disabled={!contextEpisode?.mediaPath}
          onclick={() => openEpisodeMediaSettings(episodeContextMenu!.idx)}
        >
          <span class="inline-flex items-center gap-2">
            <svg class="h-4 w-4 text-violet-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
            {t("common.settings")}
          </span>
        </button>
        <div class="vesta-context-menu-separator"></div>
        <button
          type="button"
          class="vesta-context-menu-item"
          onclick={() => removeEpisode(episodeContextMenu!.idx)}
        >
          <span class="inline-flex items-center gap-2 text-red-300">
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
            </svg>
            {t("common.delete")}
          </span>
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
            <div>
              <div class="mb-1 flex items-center gap-3">
                <span class="text-xs font-medium text-gray-400">
                  {label}
                  {#if item.required}<span class="text-red-400">*</span>{/if}
                </span>
              </div>
              <div class="flex gap-2">
                <button
                  type="button"
                  class="input-modern flex-1 truncate text-left text-xs"
                  style="direction: rtl; text-align: left;"
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
                  class="btn-primary flex h-10 shrink-0 items-center gap-1.5 px-4 text-xs"
                >
                  <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7a2 2 0 012-2h5l2 2h7a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z" />
                  </svg>
                  {t("flashcards.browse")}
                </button>
                  <button
                    type="button"
                    class={editingEpisode[field]
                      ? "inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-red-500/30 bg-red-500/10 text-red-300 shadow-sm transition-colors hover:border-red-400/60 hover:bg-red-500/20 hover:text-red-100"
                      : "inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-gray-700 bg-gray-800 text-gray-600 transition-colors cursor-default"}
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

  {#if editingMediaEpisode && editingMediaOverrides}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/65 p-6"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={closeEpisodeMediaSettings}
      onkeydown={(e) => {
        if (e.key === "Escape") closeEpisodeMediaSettings();
      }}
    >
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="flex max-h-[92vh] w-[96vw] flex-col rounded-xl border border-gray-700 bg-gray-900 shadow-2xl"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.stopPropagation()}
      >
        <div class="flex items-center justify-between gap-3 border-b border-gray-700 px-5 py-4">
          <div class="min-w-0">
            <p class="text-xs uppercase tracking-wide text-violet-300">
              {t("flashcards.perMovieSettings")}
            </p>
            <h3 class="truncate text-lg font-bold text-white" title={editingMediaEpisode.mediaPath}>
              {getFileName(editingMediaEpisode.mediaPath)}
            </h3>
          </div>
          <button
            type="button"
            onclick={closeEpisodeMediaSettings}
	            class="dialog-close-button p-1 text-xl leading-none text-gray-400 hover:text-white"
            aria-label={t("common.close")}
          >×</button>
        </div>

        <div class="flex-1 overflow-y-auto p-5">
          <div class="media-settings-panels">
          <!-- AUDIO PANEL -->
            <div class="space-y-4 rounded-xl border border-gray-800 bg-gray-800/30 p-5 shadow-inner">
              <div class="flex items-center justify-between rounded-lg border border-cyan-500/20 bg-cyan-500/10 p-3">
                <span class="text-sm font-semibold text-cyan-200">
                  {t("flashcards.generateAudioClips")}
                </span>
                <button
                  type="button"
                  aria-label={t("flashcards.generateAudioClips")}
	                  class="relative h-5 w-10 rounded-full transition-colors {editingMediaOverrides.generateAudio ? 'bg-cyan-500' : 'bg-gray-600'} {mediaOverrideClass('generateAudio')}"
                  onclick={() => updateEditingMediaOverride("generateAudio", !editingMediaOverrides?.generateAudio)}
                >
                  <span class="absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all {editingMediaOverrides.generateAudio ? 'left-5' : 'left-0.5'}"></span>
                </button>
              </div>

              {#if editingMediaOverrides.generateAudio}
                <div class="space-y-4 animate-fade-in">
                  {#if editingMediaEpisode.mediaType === "video"}
                <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.audioTrack")}</span>
                  {#if episodeAudioTracksLoading}
                    <div class="input-modern text-xs text-gray-500">{t("flashcards.audioTracksLoading")}</div>
                  {:else if episodeAudioTracks.length > 1}
	                    <SearchableSelect
	                      className={mediaOverrideClass("audioTrackIndex")}
	                      noResultsText={t("common.noResults")}
                      options={episodeAudioTracks.map((track) => ({
                        value: String(track.index),
                        label: formatAudioTrackLabel(track),
                      }))}
                      value={editingMediaOverrides.audioTrackIndex === null ? "" : String(editingMediaOverrides.audioTrackIndex)}
                      onchange={(value) => updateEditingMediaOverride("audioTrackIndex", value === "" ? null : Number(value))}
                      placeholder={t("flashcards.audioTrack")}
                    />
                  {:else if episodeAudioTracks.length === 1}
	                    <div class="input-modern text-xs text-gray-500 opacity-60 cursor-not-allowed {mediaOverrideClass('audioTrackIndex')}">
                      {formatAudioTrackLabel(episodeAudioTracks[0])}
                    </div>
                  {:else}
	                    <div class="input-modern text-xs text-gray-500 {mediaOverrideClass('audioTrackIndex')}">
                      {t("flashcards.audioTrackAuto")}
                    </div>
                  {/if}
                </div>
              {/if}

              <div class="grid grid-cols-2 gap-3">
                <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.bitrate")}</span>
	                  <SearchableSelect
	                    className={mediaOverrideClass("audioBitrate")}
	                    noResultsText={t("common.noResults")}
                    options={[
                      { value: "64", label: "64 kb/s" },
                      { value: "128", label: "128 kb/s" },
                      { value: "192", label: "192 kb/s" },
                      { value: "256", label: "256 kb/s" },
                      { value: "320", label: "320 kb/s" },
                    ]}
                    value={String(editingMediaOverrides.audioBitrate)}
                    onchange={(v) => updateEditingMediaOverride("audioBitrate", parseInt(v))}
                    placeholder="Bitrate"
                  />
                </div>
                <label class="vesta-check-row mt-5">
                  <input
                    type="checkbox"
                    checked={!!editingMediaOverrides.normalizeAudio}
                    onchange={(event) => updateEditingMediaOverride("normalizeAudio", (event.currentTarget as HTMLInputElement).checked)}
	                    class="vesta-check-input shrink-0 {mediaOverrideClass('normalizeAudio')}"
                  />
                  <span class="text-xs font-medium text-gray-300">{t("flashcards.normalizeAudio")}</span>
                </label>
              </div>

              <div class="grid grid-cols-2 gap-3">
                <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.padStart")}</span>
                  <div class="flex items-center gap-1">
                    <input
                      type="number"
                      value={editingMediaOverrides.audioPadStart}
                      oninput={(event) => updateEditingMediaOverride("audioPadStart", Number((event.currentTarget as HTMLInputElement).value))}
	                      class="input-modern w-full text-xs {mediaOverrideClass('audioPadStart')}"
                    />
                    <span class="text-xs text-gray-500">ms</span>
                  </div>
                </div>
                <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.padEnd")}</span>
                  <div class="flex items-center gap-1">
                    <input
                      type="number"
                      value={editingMediaOverrides.audioPadEnd}
                      oninput={(event) => updateEditingMediaOverride("audioPadEnd", Number((event.currentTarget as HTMLInputElement).value))}
	                      class="input-modern w-full text-xs {mediaOverrideClass('audioPadEnd')}"
                    />
                    <span class="text-xs text-gray-500">ms</span>
                  </div>
                </div>
              </div>
            </div>
            {/if}
            </div>
          <!-- SNAPSHOT PANEL -->
            <div class="space-y-4 rounded-xl border border-gray-800 bg-gray-800/30 p-5 shadow-inner {editingMediaEpisode.mediaType !== 'video' ? 'opacity-45' : ''}">
              <div class="flex items-center justify-between rounded-lg border border-purple-500/20 bg-purple-500/10 p-3">
                <span class="text-sm font-semibold text-purple-200">
                  {t("flashcards.generateSnapshots")}
                </span>
                <button
                  type="button"
                  aria-label={t("flashcards.generateSnapshots")}
                  disabled={editingMediaEpisode.mediaType !== "video"}
	                  class="relative h-5 w-10 rounded-full transition-colors {editingMediaOverrides.generateSnapshots && editingMediaEpisode.mediaType === 'video' ? 'bg-purple-500' : 'bg-gray-600'} {mediaOverrideClass('generateSnapshots')}"
                  onclick={() => updateEditingMediaOverride("generateSnapshots", !editingMediaOverrides?.generateSnapshots)}
                >
                  <span class="absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all {editingMediaOverrides.generateSnapshots && editingMediaEpisode.mediaType === 'video' ? 'left-5' : 'left-0.5'}"></span>
                </button>
              </div>
              
              {#if editingMediaOverrides.generateSnapshots && editingMediaEpisode.mediaType === "video"}
                <div class="grid grid-cols-3 gap-3 animate-fade-in">
                  <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.width")}</span>
                  <div class="flex items-center gap-1">
	                    <input type="number" value={editingMediaOverrides.snapshotWidth} oninput={(event) => updateEditingMediaOverride("snapshotWidth", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('snapshotWidth')}" />
                    <span class="text-xs text-gray-500">px</span>
                  </div>
                </div>
                <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.height")}</span>
                  <div class="flex items-center gap-1">
	                    <input type="number" value={editingMediaOverrides.snapshotHeight} oninput={(event) => updateEditingMediaOverride("snapshotHeight", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('snapshotHeight')}" />
                    <span class="text-xs text-gray-500">px</span>
                  </div>
                </div>
                <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.cropBottom")}</span>
                  <div class="flex items-center gap-1">
	                    <input type="number" value={editingMediaOverrides.cropBottom} oninput={(event) => updateEditingMediaOverride("cropBottom", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('cropBottom')}" />
                    <span class="text-xs text-gray-500">px</span>
                  </div>
                </div>
              </div>
              {/if}
            </div>
          <!-- VIDEO PANEL -->
            <div class="space-y-4 rounded-xl border border-gray-800 bg-gray-800/30 p-5 shadow-inner {editingMediaEpisode.mediaType !== 'video' ? 'opacity-45' : ''}">
              <div class="flex items-center justify-between rounded-lg border border-rose-500/20 bg-rose-500/10 p-3">
                <span class="text-sm font-semibold text-rose-200">
                  {t("flashcards.generateVideoClips")}
                </span>
                <button
                  type="button"
                  aria-label={t("flashcards.generateVideoClips")}
                  disabled={editingMediaEpisode.mediaType !== "video"}
	                  class="relative h-5 w-10 rounded-full transition-colors {editingMediaOverrides.generateVideoClips && editingMediaEpisode.mediaType === 'video' ? 'bg-rose-500' : 'bg-gray-600'} {mediaOverrideClass('generateVideoClips')}"
                  onclick={() => updateEditingMediaOverride("generateVideoClips", !editingMediaOverrides?.generateVideoClips)}
                >
                  <span class="absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all {editingMediaOverrides.generateVideoClips && editingMediaEpisode.mediaType === 'video' ? 'left-5' : 'left-0.5'}"></span>
                </button>
              </div>
              
              {#if editingMediaOverrides.generateVideoClips && editingMediaEpisode.mediaType === "video"}
                <div class="space-y-4 animate-fade-in">
                  <div class="grid grid-cols-2 gap-3">
                    <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.videoCodec")}</span>
                  <SearchableSelect
	                    className="compact-select {mediaOverrideClass('videoCodec')}"
                    noResultsText={t("common.noResults")}
                    options={[
                      { value: "h264", label: "H.264 (MP4)" },
                      { value: "mpeg4", label: "MPEG-4 (AVI)" },
                    ]}
                    value={editingMediaOverrides.videoCodec}
                    onchange={(v) => updateEditingMediaOverride("videoCodec", v)}
                    placeholder="Codec"
                  />
                </div>
                <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.h264Preset")}</span>
                  <SearchableSelect
	                    className="compact-select {mediaOverrideClass('h264Preset')}"
                    noResultsText={t("common.noResults")}
                    options={[
                      { value: "ultrafast", label: "Ultrafast" },
                      { value: "fast", label: "Fast" },
                      { value: "medium", label: "Medium" },
                      { value: "slow", label: "Slow" },
                      { value: "veryslow", label: "Very slow" },
                    ]}
                    value={editingMediaOverrides.h264Preset}
                    onchange={(v) => updateEditingMediaOverride("h264Preset", v)}
                    placeholder="Preset"
                  />
                </div>
              </div>
              <div class="grid grid-cols-2 gap-3">
                <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.videoBitrate")}</span>
                  <div class="flex items-center gap-1">
	                    <input type="number" value={editingMediaOverrides.videoBitrate} oninput={(event) => updateEditingMediaOverride("videoBitrate", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('videoBitrate')}" />
                    <span class="text-xs text-gray-500">kb/s</span>
                  </div>
                </div>
                <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.audioBitrate")}</span>
                  <SearchableSelect
	                    className="compact-select {mediaOverrideClass('videoAudioBitrate')}"
                    noResultsText={t("common.noResults")}
                    options={[
                      { value: "64", label: "64 kb/s" },
                      { value: "128", label: "128 kb/s" },
                      { value: "192", label: "192 kb/s" },
                      { value: "256", label: "256 kb/s" },
                    ]}
                    value={String(editingMediaOverrides.videoAudioBitrate)}
                    onchange={(v) => updateEditingMediaOverride("videoAudioBitrate", parseInt(v))}
                    placeholder="Bitrate"
                  />
                </div>
              </div>
              <div class="grid grid-cols-2 gap-3">
                <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.padStart")}</span>
                  <div class="flex items-center gap-1">
	                    <input type="number" value={editingMediaOverrides.videoPadStart} oninput={(event) => updateEditingMediaOverride("videoPadStart", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('videoPadStart')}" />
                    <span class="text-xs text-gray-500">ms</span>
                  </div>
                </div>
                <div>
                  <span class="mb-1 block text-xs text-gray-500">{t("flashcards.padEnd")}</span>
                  <div class="flex items-center gap-1">
	                    <input type="number" value={editingMediaOverrides.videoPadEnd} oninput={(event) => updateEditingMediaOverride("videoPadEnd", Number((event.currentTarget as HTMLInputElement).value))} class="input-modern w-full text-xs {mediaOverrideClass('videoPadEnd')}" />
                    <span class="text-xs text-gray-500">ms</span>
                  </div>
                </div>
              </div>
              </div>
              {/if}
            </div>
          </div>
        </div>

        <div class="flex items-center justify-between gap-3 border-t border-gray-700 px-5 py-4">
          <button type="button" onclick={resetEpisodeMediaSettings} class="btn-secondary px-4 py-2 text-sm">
            {t("flashcards.useGenericSettings")}
          </button>
          <div class="flex gap-2">
            <button type="button" onclick={closeEpisodeMediaSettings} class="btn-secondary px-4 py-2 text-sm">
              {t("settings.modal.cancel")}
            </button>
            <button
              type="button"
              disabled={!editingMediaOverrides || JSON.stringify(editingMediaOverrides) === initialEditingMediaOverridesStr}
              onclick={saveEpisodeMediaSettings}
              class="rounded-lg border border-violet-400/40 bg-violet-500/20 px-4 py-2 text-sm font-semibold text-violet-100 shadow-lg shadow-violet-500/10 transition-all hover:border-violet-300/60 hover:bg-violet-500/30 disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:border-violet-400/40 disabled:hover:bg-violet-500/20"
            >
              {t("settings.modal.save")}
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}

  {#if showSmartMatchingDialog}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/65 p-6"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={closeSmartMatchingDialog}
      onkeydown={(e) => {
        if (e.key === "Escape") closeSmartMatchingDialog();
      }}
    >
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="flex max-h-[94vh] w-full max-w-4xl flex-col rounded-xl border border-gray-700 bg-gray-900 shadow-2xl"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.stopPropagation()}
      >
        <div class="flex items-center justify-between gap-3 border-b border-gray-700 px-5 py-4">
          <div>
            <h3 class="flex items-center gap-2 text-lg font-bold text-violet-300">
              <svg class="h-4 w-4 text-violet-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3zM19 14l.75 2.25L22 17l-2.25.75L19 20l-.75-2.25L16 17l2.25-.75L19 14z" />
              </svg>
              {t("flashcards.smartMatchingTitle")}
            </h3>
          </div>
          <button
            type="button"
            onclick={closeSmartMatchingDialog}
	            class="dialog-close-button p-1 text-xl leading-none text-gray-400 hover:text-white"
            aria-label={t("common.close")}
          >×</button>
        </div>

        <div class="flex-1 overflow-y-auto p-5">
          <div class="relative" id="smart-matching-rules">
            <button
              type="button"
              onclick={copySmartMatchingRules}
              class="absolute right-3 top-3 z-10 inline-flex h-8 w-8 items-center justify-center rounded-md border border-white/10 bg-gray-950/85 text-gray-300 shadow-lg transition-colors hover:bg-white/10 hover:text-white"
              title={t("common.copy")}
              aria-label={t("common.copy")}
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
              </svg>
            </button>
	            <CodeEditor
	              bind:value={smartMatchingRulesDraft}
	              language="jsonc"
	              heightClass="h-[34rem]"
	              onchange={() => {
	                smartMatchingRulesError = getSmartMatchingRulesDraftError();
	              }}
	            />
          </div>
          {#if smartMatchingRulesError}
            <p class="mt-2 rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-200">
              {smartMatchingRulesError}
            </p>
          {/if}
        </div>

        <div class="flex items-center justify-between gap-3 border-t border-gray-700 px-5 py-4">
          <button
            type="button"
            onclick={resetSmartMatchingRules}
            class="btn-secondary px-4 py-2 text-sm"
          >
            {t("flashcards.smartMatchingReset")}
          </button>
          <div class="flex gap-2">
            <button
              type="button"
              onclick={closeSmartMatchingDialog}
              class="btn-secondary px-4 py-2 text-sm"
            >
              {t("settings.modal.cancel")}
            </button>
	            <button
	              type="button"
	              onclick={saveSmartMatchingRules}
	              disabled={!!getSmartMatchingRulesDraftError()}
	              class="rounded-lg border border-violet-400/40 bg-violet-500/20 px-4 py-2 text-sm font-semibold text-violet-100 shadow-lg shadow-violet-500/10 transition-all hover:border-violet-300/60 hover:bg-violet-500/30 disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:border-violet-400/40 disabled:hover:bg-violet-500/20"
	            >
              {t("settings.modal.save")}
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <InfoModal 
    section={helpSection} 
    sections={flashcardsSections} 
    onclose={() => (helpSection = null)} 
  />

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

  .timing-source-toggle {
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 0.5rem;
    display: flex;
    gap: 0.25rem;
    padding: 0.25rem;
    position: relative;
  }

  .timing-source-toggle label {
    position: relative;
    z-index: 1;
  }

  .timing-source-slider {
    background: rgba(148, 163, 184, 0.11);
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 7px;
    bottom: 0.25rem;
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.06),
      0 8px 18px rgba(0, 0, 0, 0.16);
    left: 0.25rem;
    position: absolute;
    top: 0.25rem;
    transform: translateX(0);
    transition:
      transform 0.28s cubic-bezier(0.22, 1, 0.36, 1),
      border-color 0.2s ease,
      box-shadow 0.2s ease;
    width: calc((100% - 0.75rem) / 2);
  }

  .timing-source-toggle-native .timing-source-slider {
    transform: translateX(calc(100% + 0.25rem));
  }

  .timing-source-toggle-flash .timing-source-slider {
    animation: timing-source-flash 0.42s ease-in-out 2;
  }

  .timing-source-choice {
    border: 1px solid transparent;
    border-radius: 7px;
    color: rgb(156 163 175);
    font-size: 0.75rem;
    font-weight: 650;
    line-height: 1.1;
    min-height: 2rem;
    padding: 0.5rem 0.65rem;
    text-align: center;
    transition:
      background-color 0.16s ease,
      border-color 0.16s ease,
      color 0.16s ease;
  }

  input:checked + .timing-source-choice {
    color: rgb(229 231 235);
  }

  label:hover .timing-source-choice {
    color: rgb(209 213 219);
  }

  input:disabled + .timing-source-choice {
    pointer-events: none;
  }

  .timing-source-flash {
    animation: timing-source-flash 0.42s ease-in-out 2;
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

  @keyframes timing-source-flash {
    0%,
    100% {
      border-color: rgba(52, 211, 153, 0.2);
      box-shadow: 0 0 0 0 rgba(52, 211, 153, 0);
    }

    45% {
      border-color: rgba(52, 211, 153, 0.9);
      box-shadow:
        0 0 0 2px rgba(52, 211, 153, 0.18),
        0 0 14px rgba(52, 211, 153, 0.25);
    }
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
</style>

{#if snackbarMessage}
  <Snackbar
    message={snackbarMessage}
    variant={snackbarVariant}
    duration={1300}
    animationKey={snackbarKey}
    onclose={() => (snackbarMessage = null)}
  />
{/if}
