import type { DictionaryProvider, CustomDictionary } from "$lib/types/dictionaryTypes";
import {
  getMultilingualSearchTermsForLang,
  getLanguageFlag,
  getLocalizedLanguageName,
  formatDictionaryFlags,
} from "$lib/config/languageSearchIndex";

export const builtInDictionaries: DictionaryProvider[] = [
  // ─── Japanese ─────────────────────────────────────────────────────────────
  {
    id: "ja_jisho",
    name: "Jisho",
    targetLanguage: "ja",
    nativeLanguage: "en",
    wordUrlTemplate: "https://jisho.org/search/{word}",
    sentenceUrlTemplate: "https://jisho.org/search/{sentence}",
    isPredefined: true,
    category: "specialized",
    description: "Kanji, radicals, furigana breakdown and sentence dictionary.",
  },
  {
    id: "ja_weblio",
    name: "Weblio",
    targetLanguage: "ja",
    nativeLanguage: "ja",
    wordUrlTemplate: "https://www.weblio.jp/content/{word}",
    sentenceUrlTemplate: "https://jisho.org/search/{sentence}",
    isPredefined: true,
    category: "monolingual",
    description: "Monolingual Japanese dictionary with usage examples and thesaurus.",
  },
  {
    id: "ja_wadoku",
    name: "Wadoku",
    targetLanguage: "ja",
    nativeLanguage: "de",
    wordUrlTemplate: "https://wadoku.de/search/?q={word}",
    sentenceUrlTemplate: "https://jisho.org/search/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "Authoritative Japanese-German dictionary.",
  },

  // ─── Chinese ──────────────────────────────────────────────────────────────
  {
    id: "zh_mdbg",
    name: "MDBG",
    targetLanguage: "zh",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.mdbg.net/chinese/dictionary?page=worddict&wdrst=0&wdqb={word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/chinese-english/{sentence}",
    isPredefined: true,
    category: "specialized",
    description: "Standard Chinese-English dictionary with pinyin, hanzi character breakdown and audio.",
  },
  {
    id: "zh_yellowbridge",
    name: "YellowBridge",
    targetLanguage: "zh",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.yellowbridge.com/chinese/dictionary.php?word={word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/chinese-english/{sentence}",
    isPredefined: true,
    category: "specialized",
    description: "Chinese dictionary with animated stroke order, radicals and etymology.",
  },
  {
    id: "zh_linedict",
    name: "LINE Dict",
    targetLanguage: "zh",
    nativeLanguage: "en",
    wordUrlTemplate: "https://dict.naver.com/linedict/zhendict/dict.html#/cnen/search?query={word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/chinese-english/{sentence}",
    isPredefined: true,
    category: "specialized",
    description: "Comprehensive Chinese-English learning dictionary with sentence examples.",
  },

  // ─── Korean ───────────────────────────────────────────────────────────────
  {
    id: "ko_naver",
    name: "Naver Dictionary + Papago",
    targetLanguage: "ko",
    nativeLanguage: "en",
    wordUrlTemplate: "https://korean.dict.naver.com/koendict/#/search?query={word}",
    sentenceUrlTemplate: "https://papago.naver.com/?sk=ko&tk=auto&st={sentence}",
    isPredefined: true,
    category: "specialized",
    description: "Leading Korean dictionary portal with Papago neural sentence context.",
  },
  {
    id: "ko_krdict",
    name: "Krdict (National Institute of Korean Language)",
    targetLanguage: "ko",
    nativeLanguage: "ko",
    wordUrlTemplate: "https://krdict.korean.go.kr/eng/dicSearch/search?nation=eng&sort=W&nationCode=6&searchFlag=Y&word={word}",
    sentenceUrlTemplate: "https://papago.naver.com/?sk=ko&tk=auto&st={sentence}",
    isPredefined: true,
    category: "monolingual",
    description: "Official learner dictionary of the National Institute of Korean Language.",
  },

  // ─── Arabic ───────────────────────────────────────────────────────────────
  {
    id: "ar_almaany",
    name: "Almaany (Arabic Roots & Context)",
    targetLanguage: "ar",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.almaany.com/en/dict/ar-en/{word}/",
    sentenceUrlTemplate: "https://context.reverso.net/translation/arabic-english/{sentence}",
    isPredefined: true,
    category: "specialized",
    description: "Arabic triconsonantal roots, grammar, synonyms and English definitions.",
  },
  {
    id: "ar_wordref_en",
    name: "WordReference (Arabic - English)",
    targetLanguage: "ar",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.wordreference.com/aren/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/arabic-english/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "Arabic-English WordReference dictionary with community context forum.",
  },

  // ─── Russian ──────────────────────────────────────────────────────────────
  {
    id: "ru_openrussian",
    name: "OpenRussian (Accents & Declensions)",
    targetLanguage: "ru",
    nativeLanguage: "en",
    wordUrlTemplate: "https://en.openrussian.org/ru/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/russian-english/{sentence}",
    isPredefined: true,
    category: "specialized",
    description: "Russian dictionary with full declension and conjugation tables, tonic accents and audio.",
  },
  {
    id: "ru_wordref_en",
    name: "WordReference (Russian - English)",
    targetLanguage: "ru",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.wordreference.com/ruen/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/russian-english/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "Russian-English dictionary with idiomatic examples.",
  },
  {
    id: "ru_multitran",
    name: "Multitran",
    targetLanguage: "ru",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.multitran.com/m.exe?s={word}&l1=2&l2=1",
    sentenceUrlTemplate: "https://context.reverso.net/translation/russian-english/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "Specialized multi-disciplinary Russian contextual dictionary.",
  },

  // ─── English ──────────────────────────────────────────────────────────────
  {
    id: "en_wordref_it",
    name: "WordReference + Reverso Context",
    targetLanguage: "en",
    nativeLanguage: "it",
    wordUrlTemplate: "https://www.wordreference.com/enit/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/english-italian/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "English-Italian WordReference dictionary and Reverso Context sentence lookup.",
  },
  {
    id: "en_wordref_es",
    name: "WordReference + Reverso Context",
    targetLanguage: "en",
    nativeLanguage: "es",
    wordUrlTemplate: "https://www.wordreference.com/enes/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/english-spanish/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "English-Spanish WordReference dictionary and Reverso Context sentence lookup.",
  },
  {
    id: "en_wordref_fr",
    name: "WordReference + Reverso Context",
    targetLanguage: "en",
    nativeLanguage: "fr",
    wordUrlTemplate: "https://www.wordreference.com/enfr/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/english-french/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "English-French WordReference dictionary and Reverso Context sentence lookup.",
  },
  {
    id: "en_wordref_de",
    name: "WordReference + Reverso Context",
    targetLanguage: "en",
    nativeLanguage: "de",
    wordUrlTemplate: "https://www.wordreference.com/ende/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/english-german/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "English-German WordReference dictionary and Reverso Context sentence lookup.",
  },
  {
    id: "en_cambridge",
    name: "Cambridge Dictionary",
    targetLanguage: "en",
    nativeLanguage: "en",
    wordUrlTemplate: "https://dictionary.cambridge.org/dictionary/english/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/english-italian/{sentence}",
    isPredefined: true,
    category: "monolingual",
    description: "Authoritative English learner's dictionary with IPA phonetics and CEFR levels.",
  },
  {
    id: "en_merriam",
    name: "Merriam-Webster",
    targetLanguage: "en",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.merriam-webster.com/dictionary/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/english-italian/{sentence}",
    isPredefined: true,
    category: "monolingual",
    description: "Leading American English dictionary with etymology and usage notes.",
  },

  // ─── German ───────────────────────────────────────────────────────────────
  {
    id: "de_wordref_it",
    name: "WordReference + Reverso Context",
    targetLanguage: "de",
    nativeLanguage: "it",
    wordUrlTemplate: "https://www.wordreference.com/deit/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/german-italian/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "German-Italian WordReference dictionary and Reverso Context sentence lookup.",
  },
  {
    id: "de_wordref_en",
    name: "WordReference + Reverso Context",
    targetLanguage: "de",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.wordreference.com/deen/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/german-english/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "German-English WordReference dictionary and Reverso Context sentence lookup.",
  },
  {
    id: "de_duden",
    name: "Duden (Monolingual)",
    targetLanguage: "de",
    nativeLanguage: "de",
    wordUrlTemplate: "https://www.duden.de/suchen/dudenonline/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/german-italian/{sentence}",
    isPredefined: true,
    category: "monolingual",
    description: "Authoritative standard German monolingual reference dictionary.",
  },
  {
    id: "de_dwds",
    name: "DWDS (Digitales Wörterbuch)",
    targetLanguage: "de",
    nativeLanguage: "de",
    wordUrlTemplate: "https://www.dwds.de/wb/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/german-italian/{sentence}",
    isPredefined: true,
    category: "monolingual",
    description: "Digital dictionary of the German language by the Berlin-Brandenburg Academy of Sciences.",
  },

  // ─── Spanish ──────────────────────────────────────────────────────────────
  {
    id: "es_wordref_it",
    name: "WordReference + Reverso Context",
    targetLanguage: "es",
    nativeLanguage: "it",
    wordUrlTemplate: "https://www.wordreference.com/esit/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/spanish-italian/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "Spanish-Italian WordReference dictionary and Reverso Context sentence lookup.",
  },
  {
    id: "es_wordref_en",
    name: "WordReference + Reverso Context",
    targetLanguage: "es",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.wordreference.com/esen/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/spanish-english/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "Spanish-English WordReference dictionary and Reverso Context sentence lookup.",
  },
  {
    id: "es_rae",
    name: "RAE (Diccionario de la Lengua Española)",
    targetLanguage: "es",
    nativeLanguage: "es",
    wordUrlTemplate: "https://dle.rae.es/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/spanish-italian/{sentence}",
    isPredefined: true,
    category: "monolingual",
    description: "Official Royal Spanish Academy dictionary.",
  },

  // ─── French ───────────────────────────────────────────────────────────────
  {
    id: "fr_wordref_it",
    name: "WordReference + Reverso Context",
    targetLanguage: "fr",
    nativeLanguage: "it",
    wordUrlTemplate: "https://www.wordreference.com/frit/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/french-italian/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "French-Italian WordReference dictionary and Reverso Context sentence lookup.",
  },
  {
    id: "fr_wordref_en",
    name: "WordReference + Reverso Context",
    targetLanguage: "fr",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.wordreference.com/fren/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/french-english/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "French-English WordReference dictionary and Reverso Context sentence lookup.",
  },
  {
    id: "fr_larousse",
    name: "Larousse (Monolingual)",
    targetLanguage: "fr",
    nativeLanguage: "fr",
    wordUrlTemplate: "https://www.larousse.fr/dictionnaires/francais/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/french-italian/{sentence}",
    isPredefined: true,
    category: "monolingual",
    description: "Authoritative French monolingual dictionary and encyclopedia.",
  },

  // ─── Italian ──────────────────────────────────────────────────────────────
  {
    id: "it_wordref_en",
    name: "WordReference (Italian - English)",
    targetLanguage: "it",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.wordreference.com/iten/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/italian-english/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "Italian-English WordReference dictionary and Reverso Context sentence lookup.",
  },
  {
    id: "it_treccani",
    name: "Treccani (Vocabolario Italiano)",
    targetLanguage: "it",
    nativeLanguage: "it",
    wordUrlTemplate: "https://www.treccani.it/vocabolario/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/italian-english/{sentence}",
    isPredefined: true,
    category: "monolingual",
    description: "Most prestigious and comprehensive Italian monolingual dictionary.",
  },

  // ─── Portuguese ───────────────────────────────────────────────────────────
  {
    id: "pt_wordref_en",
    name: "WordReference (Portuguese - English)",
    targetLanguage: "pt",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.wordreference.com/pten/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/portuguese-english/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "Portuguese-English WordReference dictionary and Reverso Context lookup.",
  },
  {
    id: "pt_priberam",
    name: "Priberam (Dicionário da Língua Portuguesa)",
    targetLanguage: "pt",
    nativeLanguage: "pt",
    wordUrlTemplate: "https://dicionario.priberam.org/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/portuguese-english/{sentence}",
    isPredefined: true,
    category: "monolingual",
    description: "Leading contemporary Portuguese dictionary covering European and Brazilian Portuguese.",
  },

  // ─── Turkish ──────────────────────────────────────────────────────────────
  {
    id: "tr_tureng",
    name: "Tureng (Turkish - English)",
    targetLanguage: "tr",
    nativeLanguage: "en",
    wordUrlTemplate: "https://tureng.com/tr/turkce-ingilizce/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/turkish-english/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "Standard Turkish-English dictionary with idioms and contextual examples.",
  },

  // ─── Polish ───────────────────────────────────────────────────────────────
  {
    id: "pl_diki",
    name: "Diki + Reverso Context",
    targetLanguage: "pl",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.diki.pl/slownik-angielskiego?q={word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/polish-english/{sentence}",
    isPredefined: true,
    category: "bilingual",
    description: "Polish-English multimedia dictionary with Reverso sentence context.",
  },

  // ─── Dutch ────────────────────────────────────────────────────────────────
  {
    id: "nl_vandale",
    name: "Van Dale",
    targetLanguage: "nl",
    nativeLanguage: "nl",
    wordUrlTemplate: "https://www.vandale.nl/gratis-woordenboek/nederlands/betekenis/{word}",
    sentenceUrlTemplate: "https://context.reverso.net/translation/dutch-english/{sentence}",
    isPredefined: true,
    category: "monolingual",
    description: "Standard Dutch dictionary by Van Dale.",
  },

  // ─── Hindi ────────────────────────────────────────────────────────────────
  {
    id: "hi_shabdkosh",
    name: "Shabdkosh (Hindi - English)",
    targetLanguage: "hi",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.shabdkosh.com/search-dictionary?e={word}&l=hi",
    sentenceUrlTemplate: "https://www.shabdkosh.com/search-dictionary?e={sentence}&l=hi",
    isPredefined: true,
    category: "bilingual",
    description: "Leading Hindi-English dictionary with pronunciation and example sentences.",
  },

  // ─── Latin & Classical Greek ──────────────────────────────────────────────
  {
    id: "la_perseus",
    name: "Perseus Tufts (Latin - English)",
    targetLanguage: "la",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.perseus.tufts.edu/hopper/morph?l={word}&la=la",
    isPredefined: true,
    category: "specialized",
    description: "Perseus Digital Library Latin morphological analyzer and Lewis & Short dictionary.",
  },
  {
    id: "grc_perseus",
    name: "Perseus Tufts (Ancient Greek - English)",
    targetLanguage: "grc",
    nativeLanguage: "en",
    wordUrlTemplate: "https://www.perseus.tufts.edu/hopper/morph?l={word}&la=greek",
    isPredefined: true,
    category: "specialized",
    description: "Perseus Digital Library Ancient Greek morphological analyzer and Liddell-Scott dictionary.",
  },

  // ─── Universal / Fallback ─────────────────────────────────────────────────
  {
    id: "generic_deepl",
    name: "DeepL Translator",
    targetLanguage: "*",
    wordUrlTemplate: "https://www.deepl.com/translator#auto/auto/{word}",
    sentenceUrlTemplate: "https://www.deepl.com/translator#auto/auto/{sentence}",
    isPredefined: true,
    category: "specialized",
    description: "Universal neural translation for single words and full sentences across all languages.",
  },
];

/**
 * Format a display label for dictionary option including language flags:
 * e.g. "🇯🇵 ➔ 🇬🇧 Jisho (Japanese ➔ English)"
 */
export function formatDictionaryOptionLabel(d: DictionaryProvider, locale: string = "en"): string {
  const flags = formatDictionaryFlags(d.targetLanguage, d.nativeLanguage);
  const targetName = getLocalizedLanguageName(d.targetLanguage, locale);
  if (d.nativeLanguage && d.nativeLanguage !== d.targetLanguage) {
    const nativeName = getLocalizedLanguageName(d.nativeLanguage, locale);
    return `${flags} ${d.name} (${targetName} ➔ ${nativeName})`;
  }
  if (d.targetLanguage === "*") {
    return `${flags} ${d.name} (${targetName})`;
  }
  return `${flags} ${d.name} (${targetName})`;
}

/**
 * Build rich search terms for any dictionary provider (target & native languages in 15+ languages, aliases, keywords).
 */
export function getDictionarySearchTerms(d: DictionaryProvider): string {
  const targetTerms = getMultilingualSearchTermsForLang(d.targetLanguage);
  const nativeTerms = d.nativeLanguage ? getMultilingualSearchTermsForLang(d.nativeLanguage) : "";
  const desc = d.description || "";
  return `${d.name} ${d.id} ${d.targetLanguage} ${d.nativeLanguage || ""} ${targetTerms} ${nativeTerms} ${desc}`.toLowerCase();
}

/**
 * Returns all custom dictionaries converted to DictionaryProvider format.
 */
export function mapCustomDictionaries(customDicts: CustomDictionary[] = []): DictionaryProvider[] {
  return customDicts.map((c) => ({
    id: `custom_${c.id}`,
    name: `⭐ ${c.name}`,
    targetLanguage: c.targetLanguage,
    nativeLanguage: c.nativeLanguage,
    wordUrlTemplate: c.wordUrlTemplate,
    sentenceUrlTemplate: c.sentenceUrlTemplate,
    isPredefined: false,
    category: "custom",
    description: c.description,
  }));
}

/**
 * Returns all dictionaries (custom + all built-in for all languages).
 */
export function getAllDictionaries(customDicts: CustomDictionary[] = []): DictionaryProvider[] {
  const custom = mapCustomDictionaries(customDicts);
  return [...custom, ...builtInDictionaries];
}

/**
 * Infer the most suitable dictionary for a given language pair.
 */
export function inferDictionaryForLanguages(
  targetLang?: string,
  nativeLang?: string
): DictionaryProvider {
  const normTarget = (targetLang || "").toLowerCase().trim();
  const normNative = (nativeLang || "").toLowerCase().trim();

  // 1. Exact match target + native
  const exact = builtInDictionaries.find(
    (d) =>
      d.targetLanguage === normTarget &&
      d.nativeLanguage === normNative &&
      d.isPredefined
  );
  if (exact) return exact;

  // 2. Exact match target with native = "en" or first available
  const targetMatches = builtInDictionaries.filter(
    (d) => d.targetLanguage === normTarget && d.isPredefined
  );
  if (targetMatches.length > 0) {
    return targetMatches[0];
  }

  // 3. Fallback to generic translator
  return builtInDictionaries.find((d) => d.id === "generic_deepl") || builtInDictionaries[0];
}

/**
 * Get all available dictionary providers, prioritizing the user's studied language and native language,
 * followed by all other dictionaries sorted alphabetically.
 */
export function getDictionariesForLanguage(
  targetLang: string,
  nativeLangOrCustom?: string | CustomDictionary[],
  customDictsArg: CustomDictionary[] = []
): DictionaryProvider[] {
  let nativeLang: string | undefined;
  let customDicts: CustomDictionary[] = customDictsArg;

  if (Array.isArray(nativeLangOrCustom)) {
    customDicts = nativeLangOrCustom;
    nativeLang = undefined;
  } else {
    nativeLang = nativeLangOrCustom;
  }

  const normTarget = (targetLang || "").toLowerCase().trim();
  const normNative = (nativeLang || "").toLowerCase().trim();
  const all = getAllDictionaries(customDicts);

  if (!normTarget || normTarget === "*") {
    return all.sort((a, b) => a.name.localeCompare(b.name));
  }

  const priorityExact: DictionaryProvider[] = [];
  const priorityTarget: DictionaryProvider[] = [];
  const universal: DictionaryProvider[] = [];
  const others: DictionaryProvider[] = [];

  for (const d of all) {
    if (d.targetLanguage === normTarget) {
      if (normNative && d.nativeLanguage === normNative) {
        priorityExact.push(d);
      } else {
        priorityTarget.push(d);
      }
    } else if (d.targetLanguage === "*") {
      universal.push(d);
    } else {
      others.push(d);
    }
  }

  // Sort other dictionaries alphabetically by target language then name
  others.sort((a, b) => {
    const langCmp = a.targetLanguage.localeCompare(b.targetLanguage);
    if (langCmp !== 0) return langCmp;
    return a.name.localeCompare(b.name);
  });

  return [...priorityExact, ...priorityTarget, ...universal, ...others];
}
