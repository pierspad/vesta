<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { guardedOpen } from "./dialogGuard";
  import { onDestroy, onMount, tick } from "svelte";
  import { locale } from "./i18n";
  import {
    CARD_TEMPLATES_UPDATED_EVENT,
    languages,
    loadCardTemplates,
  } from "./models";
  import PathPreviewModal from "./PathPreviewModal.svelte";
  import SearchableSelect from "./SearchableSelect.svelte";
  import LogPanel, { type LogEntry } from "./LogPanel.svelte";
  import InfoModal from "./InfoModal.svelte";
  import InfoButton from "./InfoButton.svelte";
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

  const OUTPUT_DIR_KEY = "vesta-last-output-dir";
  const NOTE_TYPE_LANGUAGE_KEY = "vesta-flashcards-note-type-language";
  const DEFAULT_FLASHCARDS_LANGUAGE_KEY = "vesta-default-flashcards-language";
  const SERIES_MODE_KEY = "vesta-flashcards-series-mode";
  const ANKI_FIELDS_PANEL_OPEN_KEY = "vesta-flashcards-anki-fields-panel-open";

  let smartFileMatchingEnabled = $state(true);

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

  // Extract episode number from filename using common patterns
  function extractEpisodeNumber(filename: string): number | null {
    const base = filename.replace(/\.[^/.]+$/, "");
    // Match patterns: S01E03, E03, Ep03, Episode 03, x03, - 03, _03
    const patterns = [
      /[Ss]\d{1,2}[Ee](\d{1,4})/,
      /[Ee][Pp]?\.?\s*(\d{1,4})/i,
      /[Ee]pisode\.?\s*(\d{1,4})/i,
      /[Xx](\d{1,4})/,
      /[\s_\-\.](\d{1,4})[\s_\-\.]/,
      /^(\d{1,4})[\s_\-\.]/,
      /[\s_\-\.](\d{1,4})$/,
    ];
    for (const pat of patterns) {
      const m = base.match(pat);
      if (m) return parseInt(m[1], 10);
    }
    return null;
  }

  const ORIGINAL_SUBTITLE_HINTS = [
    "native",
    "original",
    "orig",
    "source",
  ];
  const REFERENCE_SUBTITLE_HINTS = [
    "translated",
    "translation",
    "tradotto",
    "traduzione",
    "reference",
    "ref",
  ];
  const KNOWN_LANGUAGE_CODES = new Set(
    languages.map((lang) => lang.code.toLowerCase()),
  );

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
  }

  function normalizeSeriesBaseKey(baseName: string): string {
    let stem = baseName.toLowerCase();
    stem = stem.replace(/\([^)]*\b(?:19|20)\d{2}\b[^)]*\)/g, "");
    stem = stem.replace(/\[[^\]]*\b(?:19|20)\d{2}\b[^\]]*\]/g, "");
    stem = stem.replace(/\b(?:19|20)\d{2}\b/g, "");
    stem = stem.replace(/\b(?:720p|1080p|2160p|4k|bluray|brrip|webrip|web-dl|webdl|hdtv|dvdrip|x264|x265|h264|h265|aac|dts)\b/g, "");
    stem = stem.replace(/[\s]+/g, " ");
    stem = stem.replace(/[._-](native|original|orig|source)(?=($|[._-]))/gi, "");
    // Also strip reference/translation hints for consistent matching
    stem = stem.replace(/[._-](translated|translation|tradotto|traduzione|reference|ref)(?=($|[._-]))/gi, "");

    const suffixParts = stem.split(/[._-]+/).filter(Boolean);
    if (suffixParts.length > 1) {
      const lastPart = suffixParts[suffixParts.length - 1];
      if (KNOWN_LANGUAGE_CODES.has(lastPart)) {
        suffixParts.pop();
      }
    }
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
    const roleHint = ORIGINAL_SUBTITLE_HINTS.some((hint) =>
      new RegExp(`(^|[._-])${hint}([._-]|$)`, "i").test(normalized),
    )
      ? "original"
      : REFERENCE_SUBTITLE_HINTS.some((hint) =>
            new RegExp(`(^|[._-])${hint}([._-]|$)`, "i").test(normalized),
          )
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

    let targetCandidate =
      parsed.find((item) => item.roleHint === "original") ||
      parsed[0];

    let nativeCandidate =
      parsed.find(
        (item) =>
          item.path !== targetCandidate.path && item.roleHint === "reference",
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

  function clearMovieFile(field: "target" | "native" | "media") {
    if (field === "target") {
      targetSubsPath = "";
      targetSubsInfo = null;
    } else if (field === "native") {
      nativeSubsPath = "";
      nativeSubsInfo = null;
    } else {
      mediaPath = "";
      mediaType = "none";
      generateSnapshots = false;
      generateVideoClips = false;
    }
  }



  function removeEpisode(idx: number) {
    episodes = episodes
      .filter((_, i) => i !== idx)
      .map((e, i) => ({ ...e, id: i + 1 }));
  }

  function openEpisodeEditor(idx: number) {
    const episode = episodes[idx];
    if (!episode) return;
    editingEpisodeIndex = idx;
    editingEpisode = { ...episode };
  }

  function closeEpisodeEditor() {
    editingEpisodeIndex = null;
    editingEpisode = null;
  }

  function saveEpisodeEditor() {
    if (editingEpisodeIndex === null || !editingEpisode) return;
    const updatedEpisode: EpisodeEntry = {
      id: editingEpisode.id,
      targetSubsPath: editingEpisode.targetSubsPath,
      nativeSubsPath: editingEpisode.nativeSubsPath,
      mediaPath: editingEpisode.mediaPath,
      mediaType: editingEpisode.mediaPath
        ? detectMediaType(getFileName(editingEpisode.mediaPath))
        : "none",
    };
    episodes = episodes.map((episode, idx) =>
      idx === editingEpisodeIndex
        ? {
            ...updatedEpisode,
            id: episode.id,
          }
        : episode,
    );
    closeEpisodeEditor();
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
  let spanStart = $state("");
  let spanEnd = $state("");
  let timeShiftTarget = $state(0);
  let timeShiftNative = $state(0);

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

  let contextLeading = $state(0);
  let contextTrailing = $state(0);
  let contextMaxGap = $state(15.0);

  let combineSentences = $state(false);
  let continuationChars = $state(",、→");

  let generateAudio = $state(true);
  let audioBitrate = $state(128);
  let audioPadStart = $state(250);
  let audioPadEnd = $state(250);
  let normalizeAudio = $state(false);

  let generateSnapshots = $state(true);
  let snapshotWidth = $state(384);
  let snapshotHeight = $state(216);
  let cropBottom = $state(0);

  let generateVideoClips = $state(false);
  let videoCodec = $state("h264");
  let h264Preset = $state("medium");
  let videoBitrate = $state(800);
  let videoAudioBitrate = $state(128);
  let videoPadStart = $state(250);
  let videoPadEnd = $state(50);

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
  let removeLayoutObserver: (() => void) | null = null;
  let isDraggingOver = $state(false);

  function syncNoteTypeNameFromTemplates() {
    noteTypeName = loadCardTemplates().noteTypeName;
  }
  let canRunFlashcards = $derived(
    seriesMode
      ? Boolean(
          episodes.length > 0 && outputDir && deckName && noteTypeLanguage,
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
    if (!deckName.trim()) {
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
    const tokens = base.split(/[^a-z0-9-]+/).filter(Boolean);
    const tokenSet = new Set(tokens);

    for (const lang of languages) {
      const code = lang.code.toLowerCase();
      if (code.includes("-") && tokenSet.has(code)) return lang.code;
    }

    for (const lang of languages) {
      const code = lang.code.toLowerCase();
      if (code.length !== 2) continue;
      const index = tokens.lastIndexOf(code);
      if (index !== -1 && index >= tokens.length - 2) return lang.code;
    }

    const normalized = ` ${base.replace(/[^a-z0-9]+/g, " ")} `;
    for (const lang of languages) {
      const languageName = lang.nameEn
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, " ")
        .trim();
      if (languageName && normalized.includes(` ${languageName} `)) {
        return lang.code;
      }
    }

    return null;
  }

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
      if (KNOWN_LANGUAGE_CODES.has(lastPart)) {
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
        applyMediaSelection(mediaFiles[0]);
      }
    }
  }

  onMount(async () => {
    syncNoteTypeNameFromTemplates();

    const handleCardTemplatesUpdated = () => {
      syncNoteTypeNameFromTemplates();
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

    if (!noteTypeLanguage) {
      const inferredLanguage = inferLanguageFromPath(nativeSubsPath);
      if (inferredLanguage) {
        noteTypeLanguage = inferredLanguage;
        localStorage.setItem(NOTE_TYPE_LANGUAGE_KEY, inferredLanguage);
      }
    }

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

  function applyMediaSelection(path: string, autoSelected = false) {
    mediaPath = path;
    const filename = mediaPath.split("/").pop() || "";
    mediaType = detectMediaType(filename);

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
      applyMediaSelection(suggestedPath, true);
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
        applyMediaSelection(selected as string);
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
          generate_audio: ep.mediaPath ? generateAudio : false,
          audio_bitrate: audioBitrate,
          normalize_audio: normalizeAudio,
          audio_pad_start_ms: audioPadStart,
          audio_pad_end_ms: audioPadEnd,
          generate_snapshots: epHasVideo ? generateSnapshots : false,
          snapshot_width: snapshotWidth,
          snapshot_height: snapshotHeight,
          crop_bottom: cropBottom,
          generate_video_clips: epHasVideo ? generateVideoClips : false,
          video_codec: videoCodec,
          h264_preset: h264Preset,
          video_bitrate: videoBitrate,
          video_audio_bitrate: videoAudioBitrate,
          video_pad_start_ms: videoPadStart,
          video_pad_end_ms: videoPadEnd,
          deck_name: seriesOutputMode === "separate" ? `${deckName}_${epNum}` : deckName,
          episode_number: epNum,
          export_format: exportFormat,
          note_type_name: noteTypeName,
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
        onclick={async () => {
          const { open } = await import("@tauri-apps/plugin-shell");
          await open("https://www.ffmpeg.org/download.html");
        }}
        class="flex-shrink-0 px-3 py-1.5 rounded-lg bg-amber-500/20 border border-amber-500/40 text-amber-300 text-xs font-semibold hover:bg-amber-500/30 transition-colors flex items-center gap-1.5"
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" /></svg>
        {t("transcribe.ffmpegDownload")}
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
              class="text-gray-400 hover:text-white text-xl leading-none p-1"
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
                  class="input-modern flex-1 text-xs text-left transition-colors truncate {targetSubsPath
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
                  class="inline-flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-lg border border-red-500/30 bg-red-500/10 text-red-300 transition-colors hover:border-red-400/60 hover:bg-red-500/20 {targetSubsPath ? '' : 'invisible'}"
                  title="Rimuovi file"
                  aria-label="Rimuovi file"
                >
                  <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
                  class="input-modern flex-1 text-xs text-left transition-colors truncate {nativeSubsPath
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
                  class="inline-flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-lg border border-red-500/30 bg-red-500/10 text-red-300 transition-colors hover:border-red-400/60 hover:bg-red-500/20 {nativeSubsPath ? '' : 'invisible'}"
                  title="Rimuovi file"
                  aria-label="Rimuovi file"
                >
                  <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
                  class="inline-flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-lg border border-red-500/30 bg-red-500/10 text-red-300 transition-colors hover:border-red-400/60 hover:bg-red-500/20 {mediaPath ? '' : 'invisible'}"
                  title="Rimuovi file"
                  aria-label="Rimuovi file"
                >
                  <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
                        <th class="p-1.5 w-20"></th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each episodes as ep, idx}
                        <tr
                          class="border-t border-gray-800 cursor-pointer {idx % 2 === 0
                            ? 'bg-gray-900/30'
                            : 'bg-gray-800/20'} hover:bg-gray-700/20"
                          onclick={() => openEpisodeEditor(idx)}
                        >
                          <td class="p-1.5 text-gray-500 font-mono">{ep.id}</td>
                          <td
                            class="p-1.5 text-emerald-300 truncate"
                            title={ep.targetSubsPath}
                          >
                            {ep.targetSubsPath.split("/").pop()}
                          </td>
                          <td
                            class="p-1.5 truncate {ep.nativeSubsPath
                              ? 'text-blue-300'
                              : 'text-gray-600'}"
                            title={ep.nativeSubsPath || "—"}
                          >
                            {ep.nativeSubsPath
                              ? ep.nativeSubsPath.split("/").pop()
                              : "—"}
                          </td>
                          <td
                            class="p-1.5 truncate {ep.mediaPath
                              ? 'text-purple-300'
                              : 'text-gray-600'}"
                            title={ep.mediaPath || "—"}
                          >
                            {ep.mediaPath ? ep.mediaPath.split("/").pop() : "—"}
                          </td>
                          <td class="p-1.5">
                            <div class="flex items-center justify-end gap-5">
                            <button
                              onclick={(e) => { e.stopPropagation(); openEpisodeEditor(idx); }}
                              class="text-amber-400 hover:text-amber-300 transition-colors"
                              title="Modifica episodio"
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
                                  d="M11 4H6a2 2 0 00-2 2v12a2 2 0 002 2h12a2 2 0 002-2v-5M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"
                                /></svg
                              >
                            </button>
                            <button
                              onclick={(e) => { e.stopPropagation(); removeEpisode(idx); }}
                              class="text-red-400 hover:text-red-300 transition-colors"
                              title={t("flashcards.removeEpisode")}
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
                  <span class="text-gray-600"
                    >{t("flashcards.autoMatched")}</span
                  >
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
              if (hasAnyFiles) showSubtitleOptions = !showSubtitleOptions;
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
            <div class="flex items-center gap-4">
              <span class="text-xs text-gray-400"
                >{t("flashcards.useTimingsFrom")}:</span
              >
              <label class="flex items-center gap-1.5">
                <input
                  type="radio"
                  bind:group={useTimingsFrom}
                  value="target"
                  class="text-emerald-500"
                />
                <span class="text-xs text-gray-300"
                  >{t("flashcards.subs1")}</span
                >
              </label>
              <label class="flex items-center gap-1.5">
                <input
                  type="radio"
                  bind:group={useTimingsFrom}
                  value="native"
                  class="text-emerald-500"
                  disabled={!nativeSubsPath}
                />
                <span
                  class="text-xs text-gray-300 {!nativeSubsPath
                    ? 'opacity-50'
                    : ''}">{t("flashcards.subs2")}</span
                >
              </label>
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
                  >{t("flashcards.timeShift")} {t("flashcards.subs1")}</span
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
                  >{t("flashcards.timeShift")} {t("flashcards.subs2")}</span
                >
                <div class="flex items-center gap-1">
                  <input
                    type="number"
                    bind:value={timeShiftNative}
                    class="input-modern w-full text-xs"
                    disabled={!nativeSubsPath}
                  />
                  <span class="text-xs text-gray-500">ms</span>
                </div>
              </div>
            </div>

            <div class="flex items-center gap-3 pt-1">
              <label class="flex items-center gap-1.5">
                <input
                  type="checkbox"
                  bind:checked={combineSentences}
                  class="rounded text-emerald-500"
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
            onclick={async (e) => {
              if (!hasAnyFiles) return;
              showFilters = !showFilters;
              const column = e.currentTarget.closest(".overflow-y-auto");
              await tick();
              if (column) {
                if (showFilters) {
                  column.scrollTo({
                    top: column.scrollHeight,
                    behavior: "smooth",
                  });
                } else {
                  column.scrollTo({ top: 0, behavior: "smooth" });
                }
              }
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

            <div class="flex flex-wrap gap-3">
              <label class="flex items-center gap-1.5">
                <input
                  type="checkbox"
                  bind:checked={excludeDuplicatesSubs1}
                  class="rounded text-orange-500"
                />
                <span class="text-xs text-gray-300"
                  >{t("flashcards.excludeDupSubs1")}</span
                >
              </label>
              <label class="flex items-center gap-1.5">
                <input
                  type="checkbox"
                  bind:checked={excludeDuplicatesSubs2}
                  class="rounded text-orange-500"
                  disabled={!nativeSubsPath}
                />
                <span
                  class="text-xs text-gray-300 {!nativeSubsPath
                    ? 'opacity-50'
                    : ''}">{t("flashcards.excludeDupSubs2")}</span
                >
              </label>
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

            <div class="space-y-1.5">
              <label class="flex items-center gap-1.5">
                <input
                  type="checkbox"
                  bind:checked={excludeStyled}
                  class="rounded text-orange-500"
                />
                <span class="text-xs text-gray-300"
                  >{t("flashcards.excludeStyled")}</span
                >
              </label>
              <label class="flex items-center gap-1.5">
                <input
                  type="checkbox"
                  bind:checked={onlyCjk}
                  class="rounded text-orange-500"
                />
                <span class="text-xs text-gray-300"
                  >{t("flashcards.onlyCjk")}</span
                >
              </label>
              <label class="flex items-center gap-1.5">
                <input
                  type="checkbox"
                  bind:checked={removeNoMatch}
                  class="rounded text-orange-500"
                  disabled={!nativeSubsPath}
                />
                <span
                  class="text-xs text-gray-300 {!nativeSubsPath
                    ? 'opacity-50'
                    : ''}">{t("flashcards.removeNoMatch")}</span
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
              <div>
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
              <div class="flex items-end">
                <label class="flex items-center gap-1.5">
                  <input
                    type="checkbox"
                    bind:checked={normalizeAudio}
                    class="rounded text-cyan-500"
                  />
                  <span class="text-xs text-gray-300"
                    >{t("flashcards.normalizeAudio")}</span
                  >
                </label>
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
                if (generateSnapshots) generateVideoClips = false;
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
                if (generateVideoClips) generateSnapshots = false;
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
                  >{t("flashcards.videoCodec")}</span
                >
                <SearchableSelect
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
              searchTerms: `${lang.nameEn} ${lang.name}`,
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
            <p class="text-xs uppercase tracking-wide text-gray-500">Modalità Serie</p>
            <h3 class="text-lg font-bold text-white">
              Modifica episodio {editingEpisode.id}
            </h3>
          </div>
          <button
            type="button"
            onclick={closeEpisodeEditor}
            class="text-xl leading-none text-gray-400 hover:text-white"
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
                {#if editingEpisode[field]}
                  <button
                    type="button"
                    class="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-red-500/30 bg-red-500/10 text-red-300 shadow-sm transition-colors hover:border-red-400/60 hover:bg-red-500/20 hover:text-red-100"
                    title="Svuota campo"
                    aria-label="Svuota campo"
                    onclick={() => {
                      if (!editingEpisode) return;
                      editingEpisode = {
                        ...editingEpisode,
                        [field]: "",
                        mediaType: field === "mediaPath" ? "none" : editingEpisode.mediaType,
                      };
                    }}
                  >
                    <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                    </svg>
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>

        <div class="mt-5 flex justify-end gap-2">
          <button
            type="button"
            onclick={closeEpisodeEditor}
            class="btn-secondary px-4 py-2 text-sm"
          >
            {t("settings.modal.cancel")}
          </button>
          <button
            type="button"
            onclick={saveEpisodeEditor}
            disabled={!editingEpisode.targetSubsPath}
            class="rounded-lg border border-emerald-400/40 bg-emerald-500/20 px-4 py-2 text-sm font-semibold text-emerald-100 shadow-lg shadow-emerald-500/10 transition-all hover:border-emerald-300/60 hover:bg-emerald-500/30 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {t("settings.modal.save")}
          </button>
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
</style>
