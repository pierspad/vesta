import { detectLanguageCode, getLanguageSearchTerms, languages, normalizeLanguageText, localizedLanguageName } from "$lib/config/languages";

export interface SubtitleTrack { index: number; codec: string; language: string; title: string; text_based: boolean }
export const SUBTITLE_PAGE_SIZE = 10;
export function describeSubtitleTrack(track: SubtitleTrack) {
  // Explicit title language is more useful than contradictory container tags.
  // Keep the conflict visible: this is a label inference, not content detection.
  const metadataCode = detectLanguageCode(track.language);
  const titleCode = detectLanguageCode(track.title);
  const code = titleCode ?? metadataCode;
  const language = languages.find((item) => item.code === code);
  return { ...track, code, languageConflict: Boolean(metadataCode && titleCode && metadataCode.split("-")[0] !== titleCode.split("-")[0]), languageName: language?.nameEn ?? track.language, nativeName: language?.name ?? "" };
}
export function prepareSubtitleTracks(tracks: SubtitleTrack[], locale = "en") {
  const collator = new Intl.Collator(locale);
  return tracks.map((track) => {
    const described = describeSubtitleTrack(track);
    if (described.code) described.languageName = localizedLanguageName(described.code, locale);
    const searchText = normalizeLanguageText(`${described.languageName} ${described.nativeName} ${track.language} ${track.title} ${track.codec} ${described.code ? getLanguageSearchTerms(described.code) : ""}`);
    return { ...described, searchText };
  }).sort((a, b) => Number(b.text_based) - Number(a.text_based) || collator.compare(a.languageName, b.languageName) || a.index - b.index);
}
export function filterPreparedSubtitleTracks(tracks: ReturnType<typeof prepareSubtitleTracks>, search: string) {
  const tokens = normalizeLanguageText(search).split(/\s+/).filter(Boolean);
  if (!tokens.length) return tracks;
  return tracks.filter((track) => tokens.every((token) => track.searchText.includes(token)));
}
export function filterSubtitleTracks(tracks: SubtitleTrack[], search: string, locale = "en") {
  return filterPreparedSubtitleTracks(prepareSubtitleTracks(tracks, locale), search);
}

/** Suggested filename; the save dialog still lets users rename duplicate-language tracks. */
export function subtitleOutputName(mediaPath: string, track: SubtitleTrack): string {
  const basename = mediaPath.split(/[\\/]/).pop() || "film";
  const stem = basename.replace(/\.[^.]+$/, "").replace(/[<>:"/\\|?*\x00-\x1f]/g, "_").replace(/\s+/g, "_").replace(/[. ]+$/, "") || "film";
  const code = describeSubtitleTrack(track).code?.split("-")[0] || "xx";
  return `${stem}_${code}.srt`;
}

export type PreparedSubtitleTrack = ReturnType<typeof prepareSubtitleTracks>[number];
export interface SubtitleLanguageGroup {
  key: string;
  code: string | null;
  languageName: string;
  nativeName: string;
  tracks: PreparedSubtitleTrack[];
  downloadable: boolean;
}

/** One card per language; ordinary text tracks precede special/bitmap variants. */
export function groupSubtitleTracks(tracks: PreparedSubtitleTrack[], locale = "en"): SubtitleLanguageGroup[] {
  const groups = new Map<string, SubtitleLanguageGroup>();
  for (const track of tracks) {
    const key = track.code || `unknown:${track.language}`;
    let group = groups.get(key);
    if (!group) {
      group = { key, code: track.code, languageName: track.languageName, nativeName: track.nativeName, tracks: [], downloadable: false };
      groups.set(key, group);
    }
    group.tracks.push(track);
    group.downloadable ||= track.text_based;
  }
  const special = (track: PreparedSubtitleTrack) => /commentary|forced|sdh|hearing|commento/i.test(track.title) ? 1 : 0;
  for (const group of groups.values()) group.tracks.sort((a, b) => Number(b.text_based) - Number(a.text_based) || special(a) - special(b) || a.index - b.index);
  const collator = new Intl.Collator(locale);
  return [...groups.values()].sort((a, b) => Number(b.downloadable) - Number(a.downloadable) || collator.compare(a.languageName, b.languageName) || a.key.localeCompare(b.key));
}
