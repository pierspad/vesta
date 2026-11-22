import { buildFlashcardConfig } from "$lib/utils/flashcardConfig";
import { deriveDeckNameFromFile, type EpisodeEntry } from "$lib/utils/seriesFileMatching";
import type { EpisodeMediaOverrides } from "$lib/types/flashcardMediaTypes";
import type { CardFilterSettings } from "$lib/types/flashcardFilterTypes";
import type { NoteTypeDef } from "$lib/types/noteTypes";

export interface FlashcardEpisodePreparationOptions {
  episode: EpisodeEntry;
  episodeNumber: number;
  outputDir: string;
  seriesOutputMode: "single" | "separate";
  deckName: string;
  exportFormat: string;
  cpuCores: number;
  targetLanguage: string;
  cardFilters: CardFilterSettings;
  videoHwAccel: string;
  activeNoteType: NoteTypeDef;
  automaticNoteType?: boolean;
  autoCardFont: boolean;
  embedCardFont: boolean;
  getMediaSettings: (episode: EpisodeEntry) => Required<EpisodeMediaOverrides>;
  getAudioTrackLanguage: (episode: EpisodeEntry) => string;
  pickAudioTrackIndex: (episode: EpisodeEntry, language: string) => Promise<number | null>;
}

/** Captures episode media and language preferences before asynchronous track discovery. */
export async function prepareFlashcardEpisodeConfig(options: FlashcardEpisodePreparationOptions) {
  const {
    episode, episodeNumber, outputDir, seriesOutputMode, deckName, exportFormat, cpuCores,
    targetLanguage, cardFilters, videoHwAccel, activeNoteType, automaticNoteType,
    autoCardFont, embedCardFont, getMediaSettings, getAudioTrackLanguage, pickAudioTrackIndex,
  } = options;
  // This workflow intentionally awaits track discovery. Capture every mutable input
  // first so edits made while the picker is open apply to the next run, not this one.
  const ep = { ...episode, mediaOverrides: episode.mediaOverrides ? { ...episode.mediaOverrides } : undefined };
  const filters = { ...cardFilters };
  const noteType = {
    ...activeNoteType,
    fields: { ...activeNoteType.fields },
    included: { ...activeNoteType.included },
  };
  const mediaSettings = { ...getMediaSettings(ep) };
  const preferredAudioLanguage = getAudioTrackLanguage(ep);
  const audioTrackIndex = ep.mediaOverrides?.audioTrackIndex !== undefined
    ? ep.mediaOverrides.audioTrackIndex
    : await pickAudioTrackIndex(ep, preferredAudioLanguage);
  const hasVideo = ep.mediaType === "video";
  const hasMedia = ep.mediaType !== "none";

  return buildFlashcardConfig({
    targetSubsPath: ep.targetSubsPath,
    nativeSubsPath: ep.nativeSubsPath || null,
    videoPath: hasVideo ? ep.mediaPath : null,
    audioPath: hasMedia && !hasVideo ? ep.mediaPath : null,
    outputDir: seriesOutputMode === "single" ? `${outputDir}/episodes/episode-${episodeNumber}` : outputDir,
    cardFilters: filters,
    media: mediaSettings,
    generateAudio: ep.mediaPath ? mediaSettings.generateAudio : false,
    generateSnapshots: hasVideo ? mediaSettings.generateSnapshots : false,
    generateVideoClips: hasVideo ? mediaSettings.generateVideoClips : false,
    audioTrackIndex,
    videoHwAccel,
    deckName: seriesOutputMode === "separate" ? deriveDeckNameFromFile(ep) : deckName,
    episodeNumber,
    exportFormat,
    noteType,
    automaticNoteType,
    cpuCores,
    targetLanguage,
    autoCardFont,
    embedCardFont,
  });
}
