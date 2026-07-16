// Lista lingue disponibili per la traduzione
// nameEn è usato per la ricerca (es: digita "Italian" per trovare "Italiano")
// Ordinate alfabeticamente per nameEn
export const languages = [
  { code: "ar", name: "العربية", nameEn: "Arabic", flag: "🇸🇦" },
  { code: "ca", name: "Català", nameEn: "Catalan", flag: "🇪🇸" },
  { code: "zh", name: "中文 (简体)", nameEn: "Chinese Simplified", flag: "🇨🇳" },
  { code: "zh-tw", name: "中文 (繁體)", nameEn: "Chinese Traditional", flag: "🇹🇼" },
  { code: "cs", name: "Čeština", nameEn: "Czech", flag: "🇨🇿" },
  { code: "da", name: "Dansk", nameEn: "Danish", flag: "🇩🇰" },
  { code: "nl", name: "Nederlands", nameEn: "Dutch", flag: "🇳🇱" },
  { code: "en", name: "English", nameEn: "English", flag: "🇬🇧" },
  { code: "fi", name: "Suomi", nameEn: "Finnish", flag: "🇫🇮" },
  { code: "fr", name: "Français", nameEn: "French", flag: "🇫🇷" },
  { code: "de", name: "Deutsch", nameEn: "German", flag: "🇩🇪" },
  { code: "el", name: "Ελληνικά", nameEn: "Greek", flag: "🇬🇷" },
  { code: "he", name: "עברית", nameEn: "Hebrew", flag: "🇮🇱" },
  { code: "hi", name: "हिंदी", nameEn: "Hindi", flag: "🇮🇳" },
  { code: "hu", name: "Magyar", nameEn: "Hungarian", flag: "🇭🇺" },
  { code: "is", name: "Íslenska", nameEn: "Icelandic", flag: "🇮🇸" },
  { code: "id", name: "Bahasa Indonesia", nameEn: "Indonesian", flag: "🇮🇩" },
  { code: "it", name: "Italiano", nameEn: "Italian", flag: "🇮🇹" },
  { code: "ja", name: "日本語", nameEn: "Japanese", flag: "🇯🇵" },
  { code: "ko", name: "한국어", nameEn: "Korean", flag: "🇰🇷" },
  { code: "ms", name: "Bahasa Melayu", nameEn: "Malay", flag: "🇲🇾" },
  { code: "no", name: "Norsk", nameEn: "Norwegian", flag: "🇳🇴" },
  { code: "pl", name: "Polski", nameEn: "Polish", flag: "🇵🇱" },
  { code: "pt", name: "Português", nameEn: "Portuguese", flag: "🇵🇹" },
  { code: "pt-br", name: "Português (Brasil)", nameEn: "Portuguese Brazil", flag: "🇧🇷" },
  { code: "ro", name: "Română", nameEn: "Romanian", flag: "🇷🇴" },
  { code: "ru", name: "Русский", nameEn: "Russian", flag: "🇷🇺" },
  { code: "es", name: "Español", nameEn: "Spanish", flag: "🇪🇸" },
  { code: "sv", name: "Svenska", nameEn: "Swedish", flag: "🇸🇪" },
  { code: "th", name: "ไทย", nameEn: "Thai", flag: "🇹🇭" },
  { code: "tr", name: "Türkçe", nameEn: "Turkish", flag: "🇹🇷" },
  { code: "uk", name: "Українська", nameEn: "Ukrainian", flag: "🇺🇦" },
  { code: "vi", name: "Tiếng Việt", nameEn: "Vietnamese", flag: "🇻🇳" },
];

export const languageAliases: Record<string, string[]> = {
  ar: ["ara", "arb", "arabic", "arabo", "arab", "arabia"],
  ca: ["cat", "catalan", "català", "catala", "catalano"],
  zh: ["chi", "zho", "cmn", "zh-cn", "chs", "chinese", "mandarin", "simplified chinese", "cinese", "cinese semplificato"],
  "zh-tw": ["chi", "zho", "cmn", "zh-hant", "zh-tw", "cht", "traditional chinese", "cinese tradizionale", "taiwanese mandarin"],
  cs: ["cze", "ces", "czech", "čeština", "cestina", "ceco"],
  da: ["dan", "danish", "dansk", "danese"],
  nl: ["dut", "nld", "dutch", "nederlands", "olandese", "neerlandese"],
  en: ["eng", "english", "inglese", "ingles", "anglais", "inglés", "en-us", "en-gb"],
  fi: ["fin", "finnish", "suomi", "finlandese"],
  fr: ["fre", "fra", "french", "français", "francais", "francese"],
  de: ["ger", "deu", "german", "deutsch", "tedesco"],
  el: ["gre", "ell", "greek", "ελληνικά", "ellinika", "greco"],
  he: ["heb", "hebrew", "עברית", "ivrit", "ebraico"],
  hi: ["hin", "hindi", "हिंदी"],
  hu: ["hun", "hungarian", "magyar", "ungherese"],
  is: ["ice", "isl", "icelandic", "íslenska", "islenska", "islandese"],
  id: ["ind", "indonesian", "bahasa indonesia", "indonesiano"],
  it: ["ita", "italian", "italiano", "italiana"],
  ja: ["jpn", "japanese", "日本語", "nihongo", "giapponese"],
  ko: ["kor", "korean", "한국어", "hangul", "coreano"],
  ms: ["may", "msa", "malay", "bahasa melayu", "malese"],
  no: ["nor", "norwegian", "norsk", "norvegese", "nb", "nob", "nn", "nno"],
  pl: ["pol", "polish", "polski", "polacco"],
  pt: ["por", "portuguese", "português", "portugues", "portoghese"],
  "pt-br": ["por", "pt-br", "ptbr", "brazilian portuguese", "português brasil", "portugues brasil", "portoghese brasiliano", "brasiliano"],
  ro: ["rum", "ron", "romanian", "română", "romana", "rumeno"],
  ru: ["rus", "russian", "русский", "russkiy", "russo"],
  es: ["spa", "esp", "spanish", "español", "espanol", "espaniol", "castellano", "spagnolo"],
  sv: ["swe", "sved", "swedish", "svenska", "svedese"],
  th: ["tha", "thai", "ไทย", "tailandese"],
  tr: ["tur", "turkish", "türkçe", "turkce", "turco"],
  uk: ["ukr", "ukrainian", "українська", "ukrayinska", "ucraino"],
  vi: ["vie", "vietnamese", "tiếng việt", "tieng viet", "vietnamita"],
};

export function normalizeLanguageText(value: string | null | undefined): string {
  return (value || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[’']/g, "")
    .replace(/[^\p{L}\p{N}]+/gu, " ")
    .trim();
}

export function getLanguageSearchTerms(code: string): string {
  const lang = languages.find((item) => item.code === code);
  return [
    code,
    code.split("-")[0],
    lang?.nameEn,
    lang?.name,
    ...(languageAliases[code] || []),
    ...(languageAliases[code.split("-")[0]] || []),
  ]
    .filter(Boolean)
    .map((term) => String(term))
    .join(" ");
}

function uniqueLanguageTerms(code: string): string[] {
  return [
    code,
    code.split("-")[0],
    getLanguageSearchTerms(code),
  ]
    .join(" ")
    .split(/\s+/)
    .concat(
      (languageAliases[code] || []),
      (languageAliases[code.split("-")[0]] || []),
      languages.find((item) => item.code === code)?.nameEn || "",
      languages.find((item) => item.code === code)?.name || "",
    )
    .map(normalizeLanguageText)
    .filter(Boolean)
    .filter((term, index, arr) => arr.indexOf(term) === index);
}

function levenshteinDistance(a: string, b: string): number {
  if (a === b) return 0;
  if (!a.length) return b.length;
  if (!b.length) return a.length;

  const row = Array.from({ length: b.length + 1 }, (_, index) => index);
  for (let i = 1; i <= a.length; i += 1) {
    let previous = row[0];
    row[0] = i;
    for (let j = 1; j <= b.length; j += 1) {
      const current = row[j];
      row[j] =
        a[i - 1] === b[j - 1]
          ? previous
          : Math.min(row[j - 1] + 1, previous + 1, row[j] + 1);
      previous = current;
    }
  }
  return row[b.length];
}

function similarity(a: string, b: string): number {
  const maxLength = Math.max(a.length, b.length);
  if (maxLength === 0) return 1;
  return 1 - levenshteinDistance(a, b) / maxLength;
}

export function scoreLanguageMatch(value: string, code: string): number {
  const normalized = normalizeLanguageText(value);
  if (!normalized) return 0;

  const tokens = normalized.split(/\s+/).filter(Boolean);
  const tokenSet = new Set(tokens);
  let score = 0;

  for (const term of uniqueLanguageTerms(code)) {
    if (!term) continue;
    if (normalized === term) score = Math.max(score, 100);

    if (term.length <= 3) {
      if (tokenSet.has(term)) score = Math.max(score, 96);
      continue;
    }

    if (` ${normalized} `.includes(` ${term} `)) score = Math.max(score, 92);

    for (const token of tokens) {
      if (token.length < 4) continue;
      if (token === term) score = Math.max(score, 90);
      if (term.startsWith(token) || token.startsWith(term)) score = Math.max(score, 82);
      if (similarity(token, term) >= 0.86) score = Math.max(score, 76);
    }
  }

  return score;
}

export function detectLanguageCode(value: string): string | null {
  let bestCode: string | null = null;
  let bestScore = 0;

  for (const lang of languages) {
    const score = scoreLanguageMatch(value, lang.code);
    if (score > bestScore) {
      bestScore = score;
      bestCode = lang.code;
    }
  }

  return bestScore >= 76 ? bestCode : null;
}
