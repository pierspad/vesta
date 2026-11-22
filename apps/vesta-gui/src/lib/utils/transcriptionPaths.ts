import { languages } from "$lib/config/languages";

const knownLanguageCodes = new Set(languages.map(language => language.code.toLowerCase()));

export function generateTranscriptionOutputPath(input: string, language: string): string {
  return `${input.replace(/\.[^/\\.]+$/, "")}.${language}.srt`;
}

/** Preserve directory, filename and separator while replacing an existing language suffix. */
export function rewriteTranscriptionOutputLanguage(path: string, language: string): string {
  const boundary = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  const directory = path.slice(0, boundary + 1);
  const filename = path.slice(boundary + 1);
  if (!/\.srt$/i.test(filename)) return path;
  const stem = filename.replace(/\.srt$/i, "");
  const match = stem.match(/^(.*)([-._])([^-._]+)$/);
  if (match) {
    const [, prefix, separator, token] = match;
    if (knownLanguageCodes.has(token.toLowerCase()) || token.toLowerCase() === "auto" || /^[a-z]{2,3}$/i.test(token)) {
      return `${directory}${prefix}${separator}${language}.srt`;
    }
  }
  return `${directory}${stem}.${language}.srt`;
}

export function formatSubtitleTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  const millis = String(Math.floor(ms % 1000)).padStart(3, "0");
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}.${millis}` : `${minutes}:${seconds}.${millis}`;
}
