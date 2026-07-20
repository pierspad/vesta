import * as vestaConfig from "$lib/config/vestaConfig";

/**
 * Single source of truth for the default flashcard-refinement prompt.
 *
 * Design notes:
 * - Deliberately concise: the goal is a short gloss of the genuinely hard
 *   words in a sentence, not an essay. Verbose notes make bad Anki cards.
 * - Few-shot: the examples (EN + DE) show the model that ordinary sentences
 *   get little or no annotation, and that etymology only appears for truly
 *   obscure terms. This calibrates "what counts as difficult" far better
 *   than abstract instructions.
 * - The learner's language is inferred from the card's Back field, so the
 *   same prompt works for every UI language.
 */

export const DEFAULT_REFINEMENT_PROMPT = `You are a concise language-learning annotator. You write the "Notes" field of Anki flashcards created from real movie/TV subtitles.

Task: explain ONLY the genuinely difficult parts of the sentence on the Front: idioms, slang, phrasal verbs, rare/archaic words, false friends, meanings a learner cannot guess from a dictionary. Everything obvious stays unexplained.

Rules:
- Be brief: one line per item, at most 3 items per card.
- Write explanations in the learner's native language (the language of the Back field). The examples below use Italian — use the learner's language instead.
- Add a short etymology ONLY when the term is truly obscure and the origin helps memory.
- If nothing in the sentence needs explaining, output exactly: —
- Output plain Anki-compatible HTML (<b>, <br>). No introductions, no full translation of the sentence.
- If the User Notes contain a question, answer it first, briefly.

Examples:

Front (EN): "I'll pick you up at eight."
Notes:
<b>pick up</b>: passare a prendere qualcuno. Phrasal verb molto comune.

Front (EN): "Are you taking the piss?"
Notes:
<b>take the piss</b>: prendere in giro qualcuno (slang britannico). Il significato non c'entra con quello letterale: nasce come scherno popolare ottocentesco.

Front (EN): "He walked into the room and sat down."
Notes:
—

Front (DE): "Das ist mir Wurst."
Notes:
<b>Das ist mir Wurst</b>: "non mi importa" (lett. "per me è salsiccia"). Idioma colloquiale: una salsiccia è uguale da entrambe le estremità, quindi indifferente.

Front (DE): "Er hat gestern den Zug verpasst."
Notes:
<b>verpassen</b>: perdere (un treno, un'occasione). Da non confondere con "verlieren" (perdere un oggetto).

Card:
Front: {{front}}
Back: {{back}}
User Notes/Context: {{notes}}`;

/** Fingerprints of previous default prompts, used to auto-migrate users who
 *  never customised the prompt. Matching is done on stable substrings instead
 *  of full-string equality so whitespace drift doesn't break migration. */
const OLD_PROMPT_FINGERPRINTS = [
  "Spiega le parole desuete e più astruse della frase",
  "Analizza la frase fornita e identifica le parole chiave",
  "[INSERT SENTENCE HERE]",
  "You are a language teacher specialized in vocabulary acquisition",
  "You are a language learning assistant specialized in extracting useful linguistic insights",
];

export const REFINEMENT_PROMPT_STORAGE_KEY = "vesta-default-refinement-prompt";

/** True if `prompt` is one of the historical default prompts (never customised). */
export function isLegacyDefaultPrompt(prompt: string): boolean {
  return OLD_PROMPT_FINGERPRINTS.some((f) => prompt.includes(f));
}

/**
 * Load the stored refinement prompt, transparently migrating legacy defaults
 * to the current one. Custom user prompts are always preserved.
 */
export function loadRefinementPrompt(): string {
  let stored: string | null = null;
  try {
    stored = vestaConfig.getItem(REFINEMENT_PROMPT_STORAGE_KEY);
  } catch {
    /* storage unavailable */
  }

  if (!stored || isLegacyDefaultPrompt(stored)) {
    try {
      vestaConfig.setItem(REFINEMENT_PROMPT_STORAGE_KEY, DEFAULT_REFINEMENT_PROMPT);
    } catch {
      /* storage unavailable */
    }
    return DEFAULT_REFINEMENT_PROMPT;
  }
  return stored;
}
