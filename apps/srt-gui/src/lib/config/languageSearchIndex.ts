/**
 * Universal Multi-lingual Language Search Index & Metadata
 * Provides high-speed exhaustive keyword mapping and localized names
 * across all 15 supported UI locales in Vesta.
 */

export interface LanguageMeta {
  code: string;
  flag: string;
  en: string;
  it: string;
  zh: string;
  ja: string;
  ko: string;
  de: string;
  fr: string;
  es: string;
  ru: string;
  ar: string;
  pt: string;
  tr: string;
  pl: string;
  nl: string;
  hi: string;
  native: string;
  aliases: string[];
}

export const LANGUAGE_REGISTRY: Record<string, LanguageMeta> = {
  ja: {
    code: "ja",
    flag: "🇯🇵",
    en: "Japanese",
    it: "Giapponese",
    zh: "日语",
    ja: "日本語",
    ko: "일본어",
    de: "Japanisch",
    fr: "Japonais",
    es: "Japonés",
    ru: "Японский",
    ar: "اليابانية",
    pt: "Japonês",
    tr: "Japonca",
    pl: "Japoński",
    nl: "Japans",
    hi: "जापानी",
    native: "日本語",
    aliases: [
      "jpn", "japan", "japanese", "giapponese", "giappone", "nihongo", "nippon",
      "にほんご", "日本語", "日文", "日语", "日語", "일본어", "일본",
      "japanisch", "japonais", "japones", "japonés", "japonese", "yaponese", "japão", "japao", "japoński"
    ],
  },
  zh: {
    code: "zh",
    flag: "🇨🇳",
    en: "Chinese (Simplified)",
    it: "Cinese (Semplificato)",
    zh: "中文 (简体)",
    ja: "中国語 (簡体字)",
    ko: "중국어 (간체)",
    de: "Chinesisch (Vereinfacht)",
    fr: "Chinois (Simplifié)",
    es: "Chino (Simplificado)",
    ru: "Китайский (упрощенный)",
    ar: "الصينية (المبسطة)",
    pt: "Chinês (Simplificado)",
    tr: "Çince (Basitleştirilmiş)",
    pl: "Chiński (uproszczony)",
    nl: "Chinees (Vereenvoudigd)",
    hi: "चीनी (सरलीकृत)",
    native: "中文 (简体)",
    aliases: [
      "chi", "zho", "cmn", "zh-cn", "chs", "chinese", "mandarin", "simplified chinese",
      "cinese", "cina", "cinese semplificato", "mandarino",
      "中文", "汉语", "漢語", "普通话", "普通話", "华语", "華語", "国语", "國語", "zhongwen", "hanyu", "putonghua",
      "中国語", "ちゅうごくご", "중국어", "중어",
      "chinesisch", "chinois", "chino", "chinês", "chines", "kitayskiy", "китайский", "الصينية", "chiński"
    ],
  },
  "zh-tw": {
    code: "zh-tw",
    flag: "🇹🇼",
    en: "Chinese (Traditional)",
    it: "Cinese (Tradizionale)",
    zh: "中文 (繁體)",
    ja: "中国語 (繁体字)",
    ko: "중국어 (번체)",
    de: "Chinesisch (Traditionell)",
    fr: "Chinois (Traditionnel)",
    es: "Chino (Tradicional)",
    ru: "Китайский (традиционный)",
    ar: "الصينية (التقليدية)",
    pt: "Chinês (Tradicional)",
    tr: "Çince (Geleneksel)",
    pl: "Chiński (tradycyjny)",
    nl: "Chinees (Traditioneel)",
    hi: "चीनी (पारंपरिक)",
    native: "中文 (繁體)",
    aliases: [
      "chi", "zho", "cmn", "zh-hant", "zh-tw", "cht", "taiwan", "taiwanese", "traditional chinese",
      "cinese tradizionale", "taiwanese", "taiwan",
      "繁體中文", "繁体中文", "正體中文", "台灣華語", "國語", "臺灣", "台湾",
      "台湾語", "대만어", "번체",
      "chinois traditionnel", "chino tradicional", "chinesisch traditionell", "chinês tradicional"
    ],
  },
  ko: {
    code: "ko",
    flag: "🇰🇷",
    en: "Korean",
    it: "Coreano",
    zh: "韩语 / 朝鲜语",
    ja: "韓国語",
    ko: "한국어",
    de: "Koreanisch",
    fr: "Coréen",
    es: "Coreano",
    ru: "Корейский",
    ar: "الكورية",
    pt: "Coreano",
    tr: "Korece",
    pl: "Koreański",
    nl: "Koreaans",
    hi: "कोरियाई",
    native: "한국어",
    aliases: [
      "kor", "korea", "korean", "coreano", "corea", "hangul", "hangeul", "hangugeo", "joseonmal",
      "한국어", "조선말", "한글", "韓国語", "かんこくご", "朝鲜语", "韓國語", "韩国语",
      "koreanisch", "coréen", "coreen", "coreano", "koreyskiy", "корейский", "الكورية", "koreański"
    ],
  },
  en: {
    code: "en",
    flag: "🇬🇧",
    en: "English",
    it: "Inglese",
    zh: "英语",
    ja: "英語",
    ko: "영어",
    de: "Englisch",
    fr: "Anglais",
    es: "Inglés",
    ru: "Английский",
    ar: "الإنجليزية",
    pt: "Inglês",
    tr: "İngilizce",
    pl: "Angielski",
    nl: "Engels",
    hi: "अंग्रेज़ी",
    native: "English",
    aliases: [
      "eng", "english", "inglese", "inghilterra", "anglais", "inglés", "ingles", "englisch",
      "en-us", "en-gb", "american", "british",
      "英语", "英語", "英文", "えいご", "영어", "angielski", "engels", "английский", "الإنجليزية"
    ],
  },
  de: {
    code: "de",
    flag: "🇩🇪",
    en: "German",
    it: "Tedesco",
    zh: "德语",
    ja: "ドイツ語",
    ko: "독일어",
    de: "Deutsch",
    fr: "Allemand",
    es: "Alemán",
    ru: "Немецкий",
    ar: "الألمانية",
    pt: "Alemão",
    tr: "Almanca",
    pl: "Niemiecki",
    nl: "Duits",
    hi: "जर्मन",
    native: "Deutsch",
    aliases: [
      "ger", "deu", "german", "germany", "tedesco", "tedesca", "germania", "deutsch", "deutsche",
      "allemand", "alemán", "aleman", "alemão", "alemao", "duits", "niemiecki",
      "德语", "德國語", "ドイツ語", "독일어", "немецкий", "الألمانية"
    ],
  },
  fr: {
    code: "fr",
    flag: "🇫🇷",
    en: "French",
    it: "Francese",
    zh: "法语",
    ja: "フランス語",
    ko: "프랑스어",
    de: "Französisch",
    fr: "Français",
    es: "Francés",
    ru: "Французский",
    ar: "الفرنسية",
    pt: "Francês",
    tr: "Fransızca",
    pl: "Francuski",
    nl: "Frans",
    hi: "फ़्रेंच",
    native: "Français",
    aliases: [
      "fre", "fra", "french", "france", "francese", "francia", "français", "francais",
      "französisch", "franzosisch", "francés", "frances", "francês", "frans", "francuski",
      "法语", "法語", "フランス語", "프랑스어", "불어", "французский", "الفرنسية"
    ],
  },
  es: {
    code: "es",
    flag: "🇪🇸",
    en: "Spanish",
    it: "Spagnolo",
    zh: "西班牙语",
    ja: "スペイン語",
    ko: "스페인어",
    de: "Spanisch",
    fr: "Espagnol",
    es: "Español",
    ru: "Испанский",
    ar: "الإسبانية",
    pt: "Espanhol",
    tr: "İspanyolca",
    pl: "Hiszpański",
    nl: "Spaans",
    hi: "स्पैनिश",
    native: "Español",
    aliases: [
      "spa", "esp", "spanish", "spain", "spagnolo", "spagna", "español", "espanol", "castellano", "castigliano",
      "spanisch", "espagnol", "espanhol", "spaans", "hiszpański",
      "西班牙语", "西班牙語", "スペイン語", "스페인어", "испанский", "الإسبانية"
    ],
  },
  it: {
    code: "it",
    flag: "🇮🇹",
    en: "Italian",
    it: "Italiano",
    zh: "意大利语",
    ja: "イタリア語",
    ko: "이탈리아어",
    de: "Italienisch",
    fr: "Italien",
    es: "Italiano",
    ru: "Итальянский",
    ar: "الإيطالية",
    pt: "Italiano",
    tr: "İtalyanca",
    pl: "Włoski",
    nl: "Italiaans",
    hi: "इतालवी",
    native: "Italiano",
    aliases: [
      "ita", "italian", "italy", "italiano", "italiana", "italia", "italienisch", "italien", "italiano",
      "włoski", "wloski", "italiaans",
      "意大利语", "義大利語", "イタリア語", "이탈리아어", "итальянский", "الإيطالية"
    ],
  },
  ru: {
    code: "ru",
    flag: "🇷🇺",
    en: "Russian",
    it: "Russo",
    zh: "俄语",
    ja: "ロシア語",
    ko: "러시아어",
    de: "Russisch",
    fr: "Russe",
    es: "Ruso",
    ru: "Русский",
    ar: "الروسية",
    pt: "Russo",
    tr: "Rusça",
    pl: "Rosyjski",
    nl: "Russisch",
    hi: "रूसी",
    native: "Русский",
    aliases: [
      "rus", "russia", "russian", "russo", "russia", "русский", "russkiy", "russisch", "russe", "roos", "rosyjski",
      "俄语", "俄語", "ロシア語", "러시아어", "الروسية"
    ],
  },
  ar: {
    code: "ar",
    flag: "🇸🇦",
    en: "Arabic",
    it: "Arabo",
    zh: "阿拉伯语",
    ja: "アラビア語",
    ko: "아랍어",
    de: "Arabisch",
    fr: "Arabe",
    es: "Árabe",
    ru: "Арабский",
    ar: "العربية",
    pt: "Árabe",
    tr: "Arapça",
    pl: "Arabski",
    nl: "Arabisch",
    hi: "अरबी",
    native: "العربية",
    aliases: [
      "ara", "arb", "arabic", "arab", "arabo", "arabia", "العربية", "عربي", "arabisch", "arabe", "árabe", "arabisch", "arabski",
      "阿拉伯语", "アラビア語", "아랍어", "арабский"
    ],
  },
  pt: {
    code: "pt",
    flag: "🇵🇹",
    en: "Portuguese",
    it: "Portoghese",
    zh: "葡萄牙语",
    ja: "ポルトガル語",
    ko: "포르투갈어",
    de: "Portugiesisch",
    fr: "Portugais",
    es: "Portugués",
    ru: "Португальский",
    ar: "البرتغالية",
    pt: "Português",
    tr: "Portekizce",
    pl: "Portugalski",
    nl: "Portugees",
    hi: "पुर्तगाली",
    native: "Português",
    aliases: [
      "por", "portuguese", "portoghese", "portogallo", "português", "portugues", "portugiesisch", "portugais", "portugees", "portugalski",
      "葡萄牙语", "ポルトガル語", "포르투갈어", "португальский", "البرتغالية"
    ],
  },
  "pt-br": {
    code: "pt-br",
    flag: "🇧🇷",
    en: "Portuguese (Brazil)",
    it: "Portoghese (Brasile)",
    zh: "巴西葡萄牙语",
    ja: "ブラジルポルトガル語",
    ko: "브라질 포르투갈어",
    de: "Portugiesisch (Brasilien)",
    fr: "Portugais (Brésil)",
    es: "Portugués (Brasil)",
    ru: "Португальский (Бразилия)",
    ar: "البرتغالية (البرازيل)",
    pt: "Português (Brasil)",
    tr: "Portekizce (Brezilya)",
    pl: "Portugalski (Brazylia)",
    nl: "Portugees (Brazilië)",
    hi: "पुर्तगाली (ब्राजील)",
    native: "Português (Brasil)",
    aliases: [
      "por", "pt-br", "ptbr", "brazilian", "brazilian portuguese", "brasiliano", "portoghese brasiliano", "brasil", "brazil",
      "巴西葡萄牙语", "ブラジルポルトガル語", "브라질포르투갈어"
    ],
  },
  tr: {
    code: "tr",
    flag: "🇹🇷",
    en: "Turkish",
    it: "Turco",
    zh: "土耳其语",
    ja: "トルコ語",
    ko: "터키어",
    de: "Türkisch",
    fr: "Turc",
    es: "Turco",
    ru: "Турецкий",
    ar: "التركية",
    pt: "Turco",
    tr: "Türkçe",
    pl: "Turecki",
    nl: "Turks",
    hi: "तुर्की",
    native: "Türkçe",
    aliases: [
      "tur", "turkish", "turkey", "turco", "turchia", "türkçe", "turkce", "türkisch", "turc", "turco", "turecki",
      "土耳其语", "トルコ語", "터키어", "турецкий", "التركية"
    ],
  },
  pl: {
    code: "pl",
    flag: "🇵🇱",
    en: "Polish",
    it: "Polacco",
    zh: "波兰语",
    ja: "ポーランド語",
    ko: "폴란드어",
    de: "Polnisch",
    fr: "Polonais",
    es: "Polaco",
    ru: "Польский",
    ar: "البولندية",
    pt: "Polonês",
    tr: "Lehçe",
    pl: "Polski",
    nl: "Pools",
    hi: "पोलिश",
    native: "Polski",
    aliases: [
      "pol", "polish", "poland", "polacco", "polonia", "polski", "polnisch", "polonais", "polaco", "pools",
      "波兰语", "ポーランド語", "폴란드어", "польский", "البولندية"
    ],
  },
  nl: {
    code: "nl",
    flag: "🇳🇱",
    en: "Dutch",
    it: "Olandese",
    zh: "荷兰语",
    ja: "オランダ語",
    ko: "네덜란드어",
    de: "Niederländisch",
    fr: "Néerlandais",
    es: "Holandés",
    ru: "Нидерландский",
    ar: "الهولندية",
    pt: "Holandês",
    tr: "Felemenkçe",
    pl: "Holenderski",
    nl: "Nederlands",
    hi: "डच",
    native: "Nederlands",
    aliases: [
      "dut", "nld", "dutch", "netherlands", "holland", "olandese", "olanda", "nederlands", "niederländisch", "hollandais", "holandés", "holenderski",
      "荷兰语", "オランダ語", "네덜란드어", "нидерландский", "الهولندية"
    ],
  },
  hi: {
    code: "hi",
    flag: "🇮🇳",
    en: "Hindi",
    it: "Hindi",
    zh: "印地语",
    ja: "ヒンディー語",
    ko: "힌디어",
    de: "Hindi",
    fr: "Hindi",
    es: "Hindi",
    ru: "Хинди",
    ar: "الهندية",
    pt: "Hindi",
    tr: "Hintçe",
    pl: "Hindi",
    nl: "Hindi",
    hi: "हिन्दी",
    native: "हिन्दी",
    aliases: [
      "hin", "hindi", "india", "indiano", "हिन्दी", "हिंदी",
      "印地语", "ヒンディー語", "힌디어", "хинди", "الهندية"
    ],
  },
  la: {
    code: "la",
    flag: "🏛️",
    en: "Latin",
    it: "Latino",
    zh: "拉丁语",
    ja: "ラテン語",
    ko: "라틴어",
    de: "Latein",
    fr: "Latin",
    es: "Latín",
    ru: "Латынь",
    ar: "اللاتينية",
    pt: "Latim",
    tr: "Latince",
    pl: "Łacina",
    nl: "Latijn",
    hi: "लैटिन",
    native: "Lingua Latina",
    aliases: [
      "lat", "latin", "latino", "lingua latina", "latein", "latin", "latín", "lacina", "łacina",
      "拉丁语", "ラテン語", "라틴어", "латынь", "اللاتينية"
    ],
  },
  grc: {
    code: "grc",
    flag: "🏛️",
    en: "Ancient Greek",
    it: "Greco Antico",
    zh: "古希腊语",
    ja: "古代ギリシャ語",
    ko: "고대 그리스어",
    de: "Altgriechisch",
    fr: "Grec Ancien",
    es: "Griego Antiguo",
    ru: "Древнегреческий",
    ar: "اليونانية القديمة",
    pt: "Grego Antigo",
    tr: "Antik Yunanca",
    pl: "Starogrecki",
    nl: "Oudgrieks",
    hi: "प्राचीन ग्रीक",
    native: "Ἑλληνική",
    aliases: [
      "grc", "ell", "el", "greek", "ancient greek", "greco", "greco antico", "grecia", "altgriechisch", "grec ancien", "griego antiguo",
      "古希腊语", "古代ギリシャ語", "고대그리스어", "древнегреческий", "اليونانية القديمة"
    ],
  },
  "*": {
    code: "*",
    flag: "🌐",
    en: "All Languages",
    it: "Tutte le lingue",
    zh: "所有语言",
    ja: "全言語",
    ko: "모든 언어",
    de: "Alle Sprachen",
    fr: "Toutes les langues",
    es: "Todos los idiomas",
    ru: "Все языки",
    ar: "جميع اللغات",
    pt: "Todos os idiomas",
    tr: "Tüm Diller",
    pl: "Wszystkie języki",
    nl: "Alle talen",
    hi: "सभी भाषाएँ",
    native: "Universal (*)",
    aliases: ["all", "any", "universal", "tutte", "universale", "multilanguage", "global", "globale", "通用", "全言語", "모든언어"],
  },
};

/**
 * Returns the localized name of a language code according to the current UI locale.
 */
export function getLocalizedLanguageName(langCode: string | null | undefined, localeCode: string = "en"): string {
  if (!langCode || langCode === "*") {
    const meta = LANGUAGE_REGISTRY["*"];
    return (meta as any)[localeCode] || meta.en;
  }
  const code = langCode.toLowerCase().trim();
  const meta = LANGUAGE_REGISTRY[code] || LANGUAGE_REGISTRY[code.split("-")[0]];
  if (!meta) return langCode.toUpperCase();
  return (meta as any)[localeCode] || meta.en || meta.native;
}

/**
 * Returns a comprehensive, space-separated string of search keywords for a language code.
 */
export function getMultilingualSearchTermsForLang(langCode: string | null | undefined): string {
  if (!langCode) return "";
  const code = langCode.toLowerCase().trim();
  const meta = LANGUAGE_REGISTRY[code] || LANGUAGE_REGISTRY[code.split("-")[0]];
  if (!meta) {
    return `${code} ${code.split("-")[0]}`;
  }

  const terms = new Set<string>([
    meta.code,
    meta.code.split("-")[0],
    meta.en.toLowerCase(),
    meta.it.toLowerCase(),
    meta.zh.toLowerCase(),
    meta.ja.toLowerCase(),
    meta.ko.toLowerCase(),
    meta.de.toLowerCase(),
    meta.fr.toLowerCase(),
    meta.es.toLowerCase(),
    meta.ru.toLowerCase(),
    meta.ar.toLowerCase(),
    meta.pt.toLowerCase(),
    meta.tr.toLowerCase(),
    meta.pl.toLowerCase(),
    meta.nl.toLowerCase(),
    meta.hi.toLowerCase(),
    meta.native.toLowerCase(),
    ...meta.aliases.map((a) => a.toLowerCase()),
  ]);

  return Array.from(terms).join(" ");
}

/**
 * Get language flag emoji
 */
export function getLanguageFlag(langCode: string | null | undefined): string {
  if (!langCode || langCode === "*") return "🌐";
  const code = langCode.toLowerCase().trim();
  const meta = LANGUAGE_REGISTRY[code] || LANGUAGE_REGISTRY[code.split("-")[0]];
  return meta ? meta.flag : "🌐";
}
