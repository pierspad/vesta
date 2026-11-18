import * as vestaConfig from "$lib/config/vestaConfig";
import { invokeCommand as invoke } from "$lib/services/tauriClient";
import type { AudioTrackInfo, EpisodeMediaOverrideKey, EpisodeMediaOverrides } from "$lib/types/flashcardMediaTypes";
import { episodeMediaDiff, mediaSettingChanged } from "$lib/utils/episodeMediaSettings";
import { detectSubtitleLanguage } from "$lib/utils/episodeNoteType";
import { getFileName, inferLanguageFromPath } from "$lib/utils/models";
import { detectMediaType, pickBestAudioTrackIndex, type EpisodeEntry } from "$lib/utils/seriesFileMatching";

export function loadDefaultLanguagePref(key: string, fallback = ""): string {
  try {
    return vestaConfig.getItem(key) || fallback;
  } catch {
    return fallback;
  }
}

export function isFontBannerDismissedPref(lang: string, sessionDismissedLang: string | null): boolean {
  if (!lang) return true;
  if (sessionDismissedLang === lang) return true;
  try {
    return vestaConfig.getItem(`vesta-font-banner-dismissed-${lang}`) === "1";
  } catch {
    return false;
  }
}

export function persistFontBannerDismissal(lang: string): boolean {
  if (!lang) return false;
  try {
    vestaConfig.setItem(`vesta-font-banner-dismissed-${lang}`, "1");
    return true;
  } catch {
    return false;
  }
}

export function detectEpisodeLanguageCached(
  path: string,
  cache: Map<string, Promise<string | null>>,
  onDetected: (path: string, language: string | null) => void,
): Promise<string | null> {
  if (!path || inferLanguageFromPath(path)) return Promise.resolve(null);
  const pending = cache.get(path);
  if (pending) return pending;
  const request = invoke<Array<{ text: string }>>("flashcard_parse_subtitles", { path })
    .then((cues) => detectSubtitleLanguage((cues || []).slice(0, 160).map((cue) => cue.text).join("\n")))
    .catch(() => null)
    .then((language) => {
      onDetected(path, language);
      return language;
    });
  cache.set(path, request);
  return request;
}

export function swapEpisodeSubsInList(episodes: EpisodeEntry[], idx: number): EpisodeEntry[] {
  const ep = episodes[idx];
  if (!ep) return episodes;
  return episodes.map((item, i) =>
    i === idx
      ? {
          ...item,
          targetSubsPath: item.nativeSubsPath,
          nativeSubsPath: item.targetSubsPath,
          detectedLanguage: undefined,
        }
      : item,
  );
}

export function swapAllEpisodesSubsInList(episodes: EpisodeEntry[]): EpisodeEntry[] {
  return episodes.map((ep) => ({
    ...ep,
    targetSubsPath: ep.nativeSubsPath,
    nativeSubsPath: ep.targetSubsPath,
    detectedLanguage: undefined,
  }));
}

export function removeEpisodeInList(episodes: EpisodeEntry[], idx: number): EpisodeEntry[] {
  return episodes.filter((_, i) => i !== idx).map((e, i) => ({ ...e, id: i + 1 }));
}

export function mergeEditedEpisodeIntoList(
  episodes: EpisodeEntry[],
  index: number,
  edited: EpisodeEntry,
): EpisodeEntry[] {
  const updatedEpisode: EpisodeEntry = {
    id: edited.id,
    targetSubsPath: edited.targetSubsPath,
    nativeSubsPath: edited.nativeSubsPath,
    mediaPath: edited.mediaPath,
    mediaType: edited.mediaPath ? detectMediaType(getFileName(edited.mediaPath)) : "none",
    mediaOverrides: edited.mediaPath ? edited.mediaOverrides : undefined,
    matching: edited.matching,
    noteTypeId: edited.noteTypeId,
    detectedLanguage:
      episodes[index]?.targetSubsPath === edited.targetSubsPath ? edited.detectedLanguage : undefined,
  };
  return episodes.map((episode, idx) =>
    idx === index ? { ...updatedEpisode, id: episode.id } : episode,
  );
}

export function isEpisodeMediaOverrideChanged(
  key: EpisodeMediaOverrideKey,
  overrides: Required<EpisodeMediaOverrides> | null,
  episode: { mediaType?: string } | null,
  defaults: Required<EpisodeMediaOverrides>,
  audioTracks: AudioTrackInfo[],
  preferredLang: string,
): boolean {
  if (!overrides) return false;
  const autoTrack =
    key === "audioTrackIndex" && episode && defaults.audioTrackIndex === null
      ? pickBestAudioTrackIndex(audioTracks, preferredLang)
      : undefined;
  return mediaSettingChanged(key, overrides, defaults, autoTrack);
}

export function buildEpisodeMediaOverrideDiffFor(
  settings: Required<EpisodeMediaOverrides>,
  episode: { mediaType?: string } | null,
  defaults: Required<EpisodeMediaOverrides>,
  audioTracks: AudioTrackInfo[],
  preferredLang: string,
): EpisodeMediaOverrides {
  const autoTrack =
    episode && defaults.audioTrackIndex === null
      ? pickBestAudioTrackIndex(audioTracks, preferredLang)
      : undefined;
  return episodeMediaDiff(settings, defaults, autoTrack);
}

export function updateEpisodesMediaOverrides(
  episodes: EpisodeEntry[],
  idx: number,
  mediaOverrides: EpisodeMediaOverrides,
  mediaNoteTypeId: string,
): EpisodeEntry[] {
  return episodes.map((episode, i) =>
    i === idx
      ? {
          ...episode,
          mediaOverrides: Object.keys(mediaOverrides).length > 0 ? mediaOverrides : undefined,
          noteTypeId: mediaNoteTypeId === "default" ? undefined : mediaNoteTypeId,
        }
      : episode,
  );
}

export function clearEpisodesMediaOverrides(episodes: EpisodeEntry[], idx: number): EpisodeEntry[] {
  return episodes.map((episode, i) =>
    i === idx
      ? {
          ...episode,
          mediaOverrides: undefined,
          noteTypeId: undefined,
        }
      : episode,
  );
}
