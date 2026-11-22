import { guardedOpen } from "$lib/utils/dialogGuard";
import { t } from "$lib/i18n";
import { getFileName } from "$lib/utils/models";
import { AUDIO_EXTENSIONS, VIDEO_EXTENSIONS, SUBTITLE_EXTENSIONS, detectMediaType, type EpisodeEntry } from "$lib/utils/seriesFileMatching";

export type EpisodeFileField = "targetSubsPath" | "nativeSubsPath" | "mediaPath";

/** Opens only the native chooser; cancellation leaves the episode untouched. */
export async function pickEpisodeFile(episode: EpisodeEntry, field: EpisodeFileField): Promise<EpisodeEntry | null> {
  const media = field === "mediaPath";
  const selected = await guardedOpen({
    multiple: false,
    defaultPath: episode[field] || undefined,
    filters: [{
      name: t(media ? "flashcards.mediaFiles" : "flashcards.subtitleFiles"),
      extensions: media ? [...VIDEO_EXTENSIONS, ...AUDIO_EXTENSIONS] : [...SUBTITLE_EXTENSIONS],
    }],
  });
  if (!selected || Array.isArray(selected)) return null;
  return {
    ...episode,
    [field]: selected,
    detectedLanguage: field === "targetSubsPath" && selected !== episode.targetSubsPath ? undefined : episode.detectedLanguage,
    mediaType: media ? detectMediaType(getFileName(selected)) : episode.mediaType,
  };
}
