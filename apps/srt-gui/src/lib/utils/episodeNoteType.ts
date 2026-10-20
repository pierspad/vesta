import { normalizeLanguageCode } from "$lib/config/languages";
import {
  predefinedNoteTypeForLanguage,
  type NoteTypeDef,
} from "$lib/types/noteTypes";
import { inferLanguageFromPath } from "$lib/utils/models";
import type { EpisodeEntry } from "$lib/utils/seriesFileMatching";

/**
 * Conservatively infer a subtitle language from its text. This is deliberately
 * a small offline heuristic: it only returns a result when script or several
 * distinctive words provide reasonable evidence. Short cues and timestamps
 * return null rather than defaulting to English.
 */
export function detectSubtitleLanguage(text: string): string | null {
  const cleanText = text
    .replace(/<[^>]*>/g, " ")
    .replace(/\{\\[^}]*\}/g, " ")
    .replace(/\b\d{1,2}:\d{2}(?::\d{2})?[,.]\d{1,3}\b/g, " ")
    .replace(/-->/g, " ");
  const letters = Array.from(cleanText).filter((char) => /\p{L}/u.test(char));
  if (letters.length < 12) return null;

  const count = (pattern: RegExp) =>
    letters.filter((char) => pattern.test(char)).length;
  const kanaCount = count(/[\u3040-\u30ff]/u);
  if (kanaCount >= 4 && kanaCount >= letters.length * 0.08) return "ja";
  const scriptEvidence: Array<[string, number]> = [
    ["ja", count(/[\u3040-\u30ff]/u)],
    ["ko", count(/[\uac00-\ud7af]/u)],
    ["ar", count(/[\u0600-\u06ff]/u)],
    ["he", count(/[\u0590-\u05ff]/u)],
    ["hi", count(/[\u0900-\u097f]/u)],
    ["th", count(/[\u0e00-\u0e7f]/u)],
    ["el", count(/[\u0370-\u03ff]/u)],
    ["zh", count(/[\u4e00-\u9fff]/u)],
  ];
  scriptEvidence.sort((a, b) => b[1] - a[1]);
  const [scriptLanguage, scriptCount] = scriptEvidence[0];
  if (scriptCount >= Math.max(8, letters.length * 0.25)) return scriptLanguage;

  const cyrillic = count(/[\u0400-\u04ff]/u);
  if (cyrillic >= Math.max(8, letters.length * 0.25)) {
    if (/[їієґ]/iu.test(cleanText)) return "uk";
    // Most Cyrillic subtitles in our supported language set are Russian, but
    // the shared alphabet alone is not enough to distinguish Russian/Ukrainian.
    if (/[ыэъ]/iu.test(cleanText)) return "ru";
    return null;
  }

  const words = cleanText.toLocaleLowerCase().match(/[\p{L}]+/gu) ?? [];
  const vocabularies: Record<string, Set<string>> = {
    en: new Set(
      "the and you that this with have what your for are not was but all can our here there from they know want come when where why how".split(
        " ",
      ),
    ),
    es: new Set(
      "que los las del una por para como pero está estoy tienes nosotros aquí esto porque quiero dónde cuando".split(
        " ",
      ),
    ),
    fr: new Set(
      "les des une pas pour avec vous nous est suis dans cette mais pourquoi comment comme votre".split(
        " ",
      ),
    ),
    de: new Set(
      "und der die das nicht ist eine mit für ich du sie wir was wie warum haben hier".split(
        " ",
      ),
    ),
    it: new Set(
      "che non per una con sono come questo questa hai abbiamo perché dove quando cosa voglio andare notte dobbiamo restare qui scelta".split(
        " ",
      ),
    ),
    pt: new Set(
      "que não uma para com você vocês está estou isso como porquê onde quando".split(
        " ",
      ),
    ),
    nl: new Set(
      "het een niet voor met zijn maar wat waar waarom deze hier jullie".split(
        " ",
      ),
    ),
    tr: new Set(
      "ve bir değil için sen ben bu nasıl neden nerede şimdi çünkü".split(" "),
    ),
    pl: new Set(
      "nie jest że się dla tego jak bardzo tutaj dlaczego".split(" "),
    ),
  };
  const scores = Object.entries(vocabularies)
    .map(
      ([language, vocabulary]) =>
        [
          language,
          words.filter((word) => vocabulary.has(word)).length,
        ] as const,
    )
    .sort((a, b) => b[1] - a[1]);
  const [bestLanguage, bestScore] = scores[0];
  return bestScore >= 3 && bestScore - scores[1][1] >= 2 ? bestLanguage : null;
}

function noteTypeForLanguage(
  language: string | null,
  noteTypes: NoteTypeDef[],
): NoteTypeDef | null {
  if (!language) return null;
  const normalized = normalizeLanguageCode(language);
  if (!normalized) return null;
  return (
    noteTypes.find(
      (noteType) =>
        noteType.predefined &&
        normalizeLanguageCode(noteType.language) === normalized,
    ) ?? null
  );
}

/** Resolve a per-episode note type: explicit choice, filename language, detected
 * subtitle language, then the last manual/default selection if it still exists. */
export function resolveEpisodeNoteType(
  episode: Pick<
    EpisodeEntry,
    "targetSubsPath" | "noteTypeId" | "detectedLanguage"
  >,
  noteTypes: NoteTypeDef[],
  fallbackId: string,
): NoteTypeDef {
  const explicit =
    episode.noteTypeId && episode.noteTypeId !== "default" && episode.noteTypeId !== "auto" &&
    noteTypes.find((noteType) => noteType.id === episode.noteTypeId);
  if (explicit) return explicit;

  const fromPath = noteTypeForLanguage(
    inferLanguageFromPath(episode.targetSubsPath),
    noteTypes,
  );
  if (fromPath) return fromPath;

  const detected = noteTypeForLanguage(
    episode.detectedLanguage ?? null,
    noteTypes,
  );
  if (detected) return detected;

  return (
    noteTypes.find((noteType) => noteType.id === fallbackId) ??
    noteTypes.find((noteType) => noteType.id === "default") ??
    predefinedNoteTypeForLanguage("")
  );
}
