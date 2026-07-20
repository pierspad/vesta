/**
 * Euristiche pure per il matching automatico di file (sottotitoli, audio,
 * video) in modalità serie di FlashcardsTab.svelte.
 *
 * Ogni funzione qui dentro è una pura funzione dei suoi argomenti — nessun
 * accesso a `$state`/`$derived` di Svelte, `localStorage` o `invoke` Tauri.
 * Questo le rende testabili unitariamente senza montare il componente, e
 * riutilizzabili se in futuro serviranno altrove (es. una CLI di sync).
 *
 * FlashcardsTab.svelte resta responsabile solo di: leggere lo stato reattivo
 * (regole di smart-matching, lingua preferita, lista `episodes`), chiamare
 * queste funzioni con quei valori, e assegnare il risultato allo stato.
 */

import { getFileName, inferLanguageFromPath } from "$lib/utils/models";
import { detectLanguageCode, languages, scoreLanguageMatch } from "$lib/config/languages";
import type { SmartMatchingRules } from "$lib/stores/smartMatchingStore.svelte";
import type { AudioTrackInfo, EpisodeMediaOverrides } from "$lib/types/flashcardMediaTypes";

export const SUBTITLE_EXTENSIONS = ["srt", "ass", "ssa", "vtt"];
export const VIDEO_EXTENSIONS = ["mp4", "mkv", "avi", "webm", "mov", "flv", "ogm", "vob"];
export const AUDIO_EXTENSIONS = ["mp3", "aac", "flac", "m4a", "ogg", "wav", "wma"];

export const KNOWN_LANGUAGE_CODES = new Set(languages.map((lang) => lang.code.toLowerCase()));

export interface EpisodeEntry {
  id: number;
  targetSubsPath: string;
  nativeSubsPath: string;
  mediaPath: string;
  mediaType: "none" | "video" | "audio";
  mediaOverrides?: EpisodeMediaOverrides;
}

export interface SeriesDraftEntry {
  baseKey: string;
  displayName: string;
  targetSubsPath: string;
  nativeSubsPath: string;
  mediaPath: string;
  mediaType: "none" | "video" | "audio";
  episodeNumber: number | null;
  mediaOverrides?: EpisodeMediaOverrides;
}

export interface ParsedSeriesSubtitle {
  path: string;
  name: string;
  baseKey: string;
  language: string | null;
  roleHint: "original" | "reference" | "unknown";
  episodeNumber: number | null;
}

export interface ParsedSeriesMedia {
  path: string;
  name: string;
  baseKey: string;
  mediaType: "none" | "video" | "audio";
  episodeNumber: number | null;
}

// ─── Building blocks per i regex di matching ────────────────────────────────

export function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function delimitedHintRegex(hints: string[], flags = "i"): RegExp | null {
  if (hints.length === 0) return null;
  return new RegExp(`(^|[._-])(${hints.map(escapeRegExp).join("|")})(?=($|[._-]))`, flags);
}

export function removableTokenRegex(tokens: string[]): RegExp | null {
  if (tokens.length === 0) return null;
  return new RegExp(`\\b(?:${tokens.map(escapeRegExp).join("|")})\\b`, "gi");
}

/** Estrae il numero di episodio da un filename usando i pattern configurabili
 * di smart-matching (vedi `SmartMatchingRules.episodeRegexes`). */
export function extractEpisodeNumber(filename: string, episodeRegexes: string[]): number | null {
  const base = filename.replace(/\.[^/.]+$/, "");
  for (const pattern of episodeRegexes) {
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

export function normalizeSeriesBaseKey(baseName: string, rules: SmartMatchingRules): string {
  let stem = baseName.toLowerCase();
  stem = stem.replace(/\([^)]*\b(?:19|20)\d{2}\b[^)]*\)/g, "");
  stem = stem.replace(/\[[^\]]*\b(?:19|20)\d{2}\b[^\]]*\]/g, "");
  stem = stem.replace(/\b(?:19|20)\d{2}\b/g, "");
  const tokenRegex = removableTokenRegex(rules.removableNameTokens);
  if (tokenRegex) stem = stem.replace(tokenRegex, "");
  stem = stem.replace(/[\s]+/g, " ");
  const roleHintRegex = delimitedHintRegex(
    [...rules.originalSubtitleHints, ...rules.referenceSubtitleHints],
    "gi",
  );
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

export function stripCompoundSubtitleSuffix(baseName: string, rules: SmartMatchingRules): string {
  return normalizeSeriesBaseKey(baseName, rules);
}

export function parseSeriesSubtitle(path: string, rules: SmartMatchingRules): ParsedSeriesSubtitle {
  const name = getFileName(path);
  const baseName = name.replace(/\.[^/.]+$/, "");
  const normalized = baseName.toLowerCase();
  const language = inferLanguageFromPath(path);
  const originalHintRegex = delimitedHintRegex(rules.originalSubtitleHints);
  const referenceHintRegex = delimitedHintRegex(rules.referenceSubtitleHints);
  const roleHint = originalHintRegex?.test(normalized)
    ? "original"
    : referenceHintRegex?.test(normalized)
      ? "reference"
      : "unknown";

  return {
    path,
    name,
    baseKey: stripCompoundSubtitleSuffix(baseName, rules) || normalized,
    language,
    roleHint,
    episodeNumber: extractEpisodeNumber(name, rules.episodeRegexes),
  };
}

export function parseSeriesMedia(path: string, rules: SmartMatchingRules): ParsedSeriesMedia {
  const name = getFileName(path);
  const baseName = name.replace(/\.[^/.]+$/, "");
  return {
    path,
    name,
    baseKey: normalizeSeriesBaseKey(baseName, rules) || baseName.toLowerCase(),
    mediaType: detectMediaType(name),
    episodeNumber: extractEpisodeNumber(name, rules.episodeRegexes),
  };
}

// ─── Estensioni / tipo file ──────────────────────────────────────────────────

export function getFileExtension(path: string): string {
  return (path.split(".").pop() || "").toLowerCase();
}

export function isSubtitleFile(path: string): boolean {
  return SUBTITLE_EXTENSIONS.includes(getFileExtension(path));
}

export function isMediaFile(path: string): boolean {
  const ext = getFileExtension(path);
  return VIDEO_EXTENSIONS.includes(ext) || AUDIO_EXTENSIONS.includes(ext);
}

export function detectMediaType(filename: string): "video" | "audio" | "none" {
  const ext = filename.split(".").pop()?.toLowerCase() || "";
  if (VIDEO_EXTENSIONS.includes(ext)) return "video";
  if (AUDIO_EXTENSIONS.includes(ext)) return "audio";
  return "none";
}

/**
 * Determine which subtitle file is "target" (the one you're studying)
 * and which is "native" (your native language translation).
 * Uses language codes in filenames, keywords, and the preferred languages.
 */
export function classifySubtitleCandidates(
  paths: string[],
  preferredRole: "target" | "native" | "auto",
  rules: SmartMatchingRules,
  studiedLanguage: string,
  nativeLanguage: string,
): { target: string; native: string } {
  if (paths.length === 0) return { target: "", native: "" };

  const parsed = paths
    .map((path) => parseSeriesSubtitle(path, rules))
    .sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }));

  if (paths.length === 1) {
    return preferredRole === "native"
      ? { target: "", native: paths[0] }
      : { target: paths[0], native: "" };
  }

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
      (item) => item.path !== targetCandidate.path && item.roleHint === "reference",
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

export function classifySubtitles(
  paths: string[],
  rules: SmartMatchingRules,
  studiedLanguage: string,
  nativeLanguage: string,
): { target: string; native: string } {
  return classifySubtitleCandidates(paths, "auto", rules, studiedLanguage, nativeLanguage);
}

// ─── Serie: aggregazione di episodi da liste di file ────────────────────────

export function buildSeriesDraftMap(
  episodes: EpisodeEntry[],
  rules: SmartMatchingRules,
): Map<string, SeriesDraftEntry> {
  const map = new Map<string, SeriesDraftEntry>();

  episodes.forEach((episode) => {
    const baseKey =
      episode.targetSubsPath
        ? parseSeriesSubtitle(episode.targetSubsPath, rules).baseKey
        : episode.nativeSubsPath
          ? parseSeriesSubtitle(episode.nativeSubsPath, rules).baseKey
          : episode.mediaPath
            ? parseSeriesMedia(episode.mediaPath, rules).baseKey
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
          getFileName(episode.targetSubsPath || episode.nativeSubsPath || episode.mediaPath),
          rules.episodeRegexes,
        ) || null,
    });
  });

  return map;
}

/** Converte una draft map in una lista di episodi ordinata per numero di
 * episodio (poi per nome). Pura: non tocca lo stato del componente, il
 * chiamante deve assegnare il risultato a `episodes`. */
export function seriesDraftEntriesToEpisodes(draftMap: Map<string, SeriesDraftEntry>): EpisodeEntry[] {
  const sortedEntries = [...draftMap.values()].sort((a, b) => {
    const aEpisode = a.episodeNumber ?? Number.MAX_SAFE_INTEGER;
    const bEpisode = b.episodeNumber ?? Number.MAX_SAFE_INTEGER;
    if (aEpisode !== bEpisode) return aEpisode - bEpisode;
    return a.displayName.localeCompare(b.displayName, undefined, { numeric: true });
  });

  return sortedEntries.map((entry, index) => ({
    id: index + 1,
    targetSubsPath: entry.targetSubsPath,
    nativeSubsPath: entry.nativeSubsPath,
    mediaPath: entry.mediaPath,
    mediaType: entry.mediaType,
    mediaOverrides: entry.mediaOverrides,
  }));
}

export function mergeSeriesSubtitleFiles(
  episodes: EpisodeEntry[],
  subtitleFiles: string[],
  preferredRole: "target" | "native" | "auto",
  rules: SmartMatchingRules,
  studiedLanguage: string,
  nativeLanguage: string,
): EpisodeEntry[] {
  const draftMap = buildSeriesDraftMap(episodes, rules);
  const grouped = new Map<string, string[]>();

  subtitleFiles.forEach((path) => {
    const parsed = parseSeriesSubtitle(path, rules);
    const group = grouped.get(parsed.baseKey) || [];
    group.push(path);
    grouped.set(parsed.baseKey, group);
  });

  grouped.forEach((paths, baseKey) => {
    const parsedGroup = paths.map((path) => parseSeriesSubtitle(path, rules));
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

    const classified = classifySubtitleCandidates(paths, preferredRole, rules, studiedLanguage, nativeLanguage);

    if (preferredRole === "native" && paths.length === 1) {
      entry.nativeSubsPath = paths[0];
    } else if (preferredRole === "auto" && paths.length === 1) {
      const parsed = parsedGroup[0];
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

  return seriesDraftEntriesToEpisodes(draftMap);
}

export function mergeSeriesMediaFiles(
  episodes: EpisodeEntry[],
  mediaFiles: string[],
  rules: SmartMatchingRules,
): EpisodeEntry[] {
  const draftMap = buildSeriesDraftMap(episodes, rules);

  mediaFiles.forEach((path) => {
    const parsed = parseSeriesMedia(path, rules);
    let entry = draftMap.get(parsed.baseKey);

    // Fallback: match by episode number if baseKey doesn't match
    if (!entry && parsed.episodeNumber !== null) {
      for (const [, existing] of draftMap) {
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

  return seriesDraftEntriesToEpisodes(draftMap);
}

export function mergeSeriesDroppedFiles(
  episodes: EpisodeEntry[],
  subtitleFiles: string[],
  mediaFiles: string[],
  rules: SmartMatchingRules,
  studiedLanguage: string,
  nativeLanguage: string,
): EpisodeEntry[] {
  let result = episodes;
  if (subtitleFiles.length > 0) {
    result = mergeSeriesSubtitleFiles(result, subtitleFiles, "auto", rules, studiedLanguage, nativeLanguage);
  }
  if (mediaFiles.length > 0) {
    result = mergeSeriesMediaFiles(result, mediaFiles, rules);
  }
  return result;
}

/** Auto-match files across categories by episode number, then lexicographic. */
export function autoMatchFiles(
  targetFiles: string[],
  nativeFiles: string[],
  mediaFiles: string[],
  episodeRegexes: string[],
): EpisodeEntry[] {
  type FileWithEp = { path: string; ep: number | null; name: string };
  const toEntries = (files: string[]): FileWithEp[] =>
    files.map((f) => {
      const name = getFileName(f);
      return { path: f, ep: extractEpisodeNumber(name, episodeRegexes), name };
    });

  const targets = toEntries(targetFiles);
  const natives = toEntries(nativeFiles);
  const medias = toEntries(mediaFiles);

  // Try episode-number matching first
  const allHaveEps = targets.every((t) => t.ep !== null);
  if (allHaveEps) {
    targets.sort((a, b) => (a.ep ?? 0) - (b.ep ?? 0));
  } else {
    targets.sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }));
  }

  // Match natives and medias by episode number or index
  return targets.map((t, idx) => {
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
      const sorted = [...natives].sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }));
      nativePath = sorted[idx]?.path || "";
    }
    if (!mediaPath && idx < medias.length) {
      const sorted = [...medias].sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }));
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
}

// ─── Audio track scoring ─────────────────────────────────────────────────────

export function scoreAudioTrackForLanguage(track: AudioTrackInfo, languageCode: string): number {
  if (!languageCode) return 0;
  return Math.max(
    scoreLanguageMatch(track.language || "", languageCode),
    Math.max(0, scoreLanguageMatch(track.title || "", languageCode) - 12),
  );
}

export function pickBestAudioTrackIndex(tracks: AudioTrackInfo[], languageCode: string): number | null {
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

// ─── Deck naming ─────────────────────────────────────────────────────────────

export function generateDefaultDeckName(filename: string): string {
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
export function deriveDeckNameFromFile(ep: { mediaPath: string; targetSubsPath: string }): string {
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
