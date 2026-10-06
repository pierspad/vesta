import { detectLanguageCode } from "$lib/config/languages";

/** Preserve the source directory and replace only a language suffix in the filename. */
export function generateTranslationOutputPath(input: string, language: string): string {
  const boundary = Math.max(input.lastIndexOf("/"), input.lastIndexOf("\\"));
  const directory = input.slice(0, boundary + 1);
  const filename = input.slice(boundary + 1);
  const stem = filename.replace(/\.srt$/i, "");
  const suffix = stem.match(/^(.*)([-._])([^-._]+)$/);
  const outputStem = suffix && detectLanguageCode(suffix[3])
    ? `${suffix[1]}${suffix[2]}${language}`
    : `${stem}.${language}`;
  const candidate = `${directory}${outputStem}.srt`;
  // Translating into the source language still needs a distinct destination.
  return candidate.toLowerCase() === input.toLowerCase()
    ? `${directory}${stem}.translated.${language}.srt`
    : candidate;
}
