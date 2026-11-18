import { getLanguageSearchTerms, languages } from "$lib/config/languages";
import type { CardFilterSettings } from "$lib/types/flashcardFilterTypes";
import type { NoteTypeDef } from "$lib/types/noteTypes";
import { getFileName } from "$lib/utils/models";
import { detectMediaType, type EpisodeEntry } from "$lib/utils/seriesFileMatching";
import type { EpisodeFileField } from "./EpisodeEditorModal.svelte";

export interface NoteTypeOption {
  value: string;
  label: string;
  displayLabel?: string;
  group?: string;
  searchTerms?: string;
  icon?: string;
}

export interface CardFilterTracker {
  prevMinChars: number | undefined;
  prevMaxChars: number | undefined;
  prevMinDuration: number | undefined;
  prevMaxDuration: number | undefined;
}

export function buildFlashcardNoteTypeOptions(
  noteTypeList: NoteTypeDef[],
  automaticEpisodeNoteType: NoteTypeDef,
  t: (key: string, params?: Record<string, string | number>) => string,
): NoteTypeOption[] {
  return [
    {
      value: "default",
      label: `${t("flashcards.automaticNoteType")} · ${automaticEpisodeNoteType.name}`,
      displayLabel: automaticEpisodeNoteType.name,
      icon:
        languages.find((language) => language.code === automaticEpisodeNoteType.language)?.flag ||
        '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" class="w-4 h-4"><path stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" d="M4 9V4h5m6 0h5v5M4 15v5h5m6 0h5v-5M8 12h8m-4-4v8" /></svg>',
    },
    ...[...noteTypeList]
      .filter((nt) => nt.id !== "default")
      .sort((a, b) => Number(a.predefined) - Number(b.predefined) || a.name.localeCompare(b.name))
      .map((nt) => ({
        group: t(nt.predefined ? "settings.anki.builtinTemplates" : "settings.anki.customTemplates"),
        value: nt.id,
        label: nt.name,
        searchTerms: [nt.name, getLanguageSearchTerms(nt.language)].filter(Boolean).join(" "),
        icon: languages.find((l) => l.code === nt.language)?.flag ?? "🃏",
      })),
  ];
}

export function enforceCardFilterCoherence(cardFilters: CardFilterSettings, tracker: CardFilterTracker): void {
  if (cardFilters.minChars < 1) cardFilters.minChars = 1;
  if (cardFilters.minChars > 2000) cardFilters.minChars = 2000;
  if (cardFilters.maxChars < 1) cardFilters.maxChars = 1;
  if (cardFilters.maxChars > 2000) cardFilters.maxChars = 2000;

  if (cardFilters.minDurationMs < 0) cardFilters.minDurationMs = 0;
  if (cardFilters.minDurationMs > 120000) cardFilters.minDurationMs = 120000;
  if (cardFilters.maxDurationMs < 0) cardFilters.maxDurationMs = 0;
  if (cardFilters.maxDurationMs > 120000) cardFilters.maxDurationMs = 120000;

  if (tracker.prevMinChars === undefined) tracker.prevMinChars = cardFilters.minChars;
  if (tracker.prevMaxChars === undefined) tracker.prevMaxChars = cardFilters.maxChars;
  if (tracker.prevMinDuration === undefined) tracker.prevMinDuration = cardFilters.minDurationMs;
  if (tracker.prevMaxDuration === undefined) tracker.prevMaxDuration = cardFilters.maxDurationMs;

  if (cardFilters.minChars !== tracker.prevMinChars) {
    if (cardFilters.minChars > cardFilters.maxChars) cardFilters.maxChars = cardFilters.minChars;
    tracker.prevMinChars = cardFilters.minChars;
    tracker.prevMaxChars = cardFilters.maxChars;
  } else if (cardFilters.maxChars !== tracker.prevMaxChars) {
    if (cardFilters.maxChars < cardFilters.minChars) cardFilters.minChars = cardFilters.maxChars;
    tracker.prevMinChars = cardFilters.minChars;
    tracker.prevMaxChars = cardFilters.maxChars;
  }

  if (cardFilters.minDurationMs !== tracker.prevMinDuration) {
    if (cardFilters.minDurationMs > cardFilters.maxDurationMs) cardFilters.maxDurationMs = cardFilters.minDurationMs;
    tracker.prevMinDuration = cardFilters.minDurationMs;
    tracker.prevMaxDuration = cardFilters.maxDurationMs;
  } else if (cardFilters.maxDurationMs !== tracker.prevMaxDuration) {
    if (cardFilters.maxDurationMs < cardFilters.minDurationMs) cardFilters.minDurationMs = cardFilters.maxDurationMs;
    tracker.prevMinDuration = cardFilters.minDurationMs;
    tracker.prevMaxDuration = cardFilters.maxDurationMs;
  }
}

export function isEpisodeFileCompatible(field: EpisodeFileField, path: string): boolean {
  const type = detectMediaType(getFileName(path));
  return field === "mediaPath"
    ? type === "video" || type === "audio"
    : /\.(srt|ass|ssa|vtt)$/i.test(path);
}

export function normalizeSelectedPaths(selected: unknown): string[] {
  if (!selected) return [];
  if (typeof selected === "string") return [selected];
  if (Array.isArray(selected)) return selected.filter((s): s is string => typeof s === "string" && s.length > 0);
  return [];
}

export function computeBottomContextMenuPos(event: MouseEvent, section: "overview" | "anki") {
  let x = event.clientX;
  let y = event.clientY - 52;
  const target = event.currentTarget as HTMLElement | null;
  if (target && typeof target.getBoundingClientRect === "function") {
    const rect = target.getBoundingClientRect();
    y = rect.top - 48;
    x = rect.left;
  }
  return {
    x: Math.min(x, window.innerWidth - 220),
    y: Math.max(0, y),
    section,
  };
}

export function mergePluginHandoffEpisode(
  episodes: EpisodeEntry[],
  handoff: { videoPath: string; subtitlePath: string },
): {
  nextEpisodes: EpisodeEntry[];
  mediaOrTargetChanged: boolean;
  mediaChanged: boolean;
  nextMediaPath: string;
  nextMediaType: EpisodeEntry["mediaType"];
} | null {
  const subtitlePath = handoff.subtitlePath.trim();
  const mediaCandidate = handoff.videoPath.trim();
  const detectedType = mediaCandidate ? detectMediaType(getFileName(mediaCandidate)) : "none";
  const hasValidMedia = Boolean(mediaCandidate && detectedType !== "none");
  if (!subtitlePath && !hasValidMedia) return null;

  const existing = episodes[0];
  const nextTargetSubsPath = subtitlePath || existing?.targetSubsPath || "";
  const targetSubsChanged = nextTargetSubsPath !== (existing?.targetSubsPath ?? "");
  const preserveExistingMedia = !targetSubsChanged && !mediaCandidate;
  const nextMediaPath = hasValidMedia
    ? mediaCandidate
    : preserveExistingMedia
      ? (existing?.mediaPath || "")
      : "";
  const nextMediaType: EpisodeEntry["mediaType"] = hasValidMedia
    ? detectedType
    : preserveExistingMedia
      ? (existing?.mediaType ?? "none")
      : "none";

  const mediaChanged = nextMediaPath !== (existing?.mediaPath ?? "");
  const updatedEpisode: EpisodeEntry = {
    id: existing?.id ?? 1,
    targetSubsPath: nextTargetSubsPath,
    nativeSubsPath: mediaChanged ? "" : (existing?.nativeSubsPath ?? ""),
    mediaPath: nextMediaPath,
    mediaType: nextMediaType,
    mediaOverrides: mediaChanged ? undefined : existing?.mediaOverrides,
    noteTypeId: existing?.noteTypeId,
    detectedLanguage: targetSubsChanged ? undefined : existing?.detectedLanguage,
    matching: mediaChanged ? undefined : existing?.matching,
  };

  const nextEpisodes =
    episodes.length === 0
      ? [updatedEpisode]
      : episodes.map((ep, idx) => (idx === 0 ? updatedEpisode : ep));

  return {
    nextEpisodes,
    mediaOrTargetChanged: mediaChanged || targetSubsChanged,
    mediaChanged,
    nextMediaPath,
    nextMediaType,
  };
}
