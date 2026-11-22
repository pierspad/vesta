import { parseJsonWithComments } from "$lib/utils/jsonComments";
import { locale } from "$lib/i18n";
import * as vestaConfig from "$lib/config/vestaConfig";

export interface SmartMatchingRules {
  episodeRegexes: string[];
  originalSubtitleHints: string[];
  referenceSubtitleHints: string[];
  removableNameTokens: string[];
}

export const DEFAULT_SMART_MATCHING_RULES: SmartMatchingRules = {
  episodeRegexes: [
    "[Ss]\\d{1,2}[\\s_\\-.]?[Ee](\\d{1,4})",
    "(?:^|[\\s._-])\\d{1,2}[xX](\\d{1,4})(?=$|[\\s._-])",
    "[Ss]\\d{1,2}[Ee](\\d{1,4})[\\-~_Ee]\\d{1,4}",
    "[Ss]\\d{1,2}[\\s_\\-.]?(?:[Ee][Pp]?|[Ee][Ss][Pp]?)[\\s_\\-.]*(\\d{1,4})",
    "第\\s*(\\d{1,4})\\s*[话話集期]",
    "(?:[Ee]pisode|[Éé]pisode|[Ee]pisodio|[Ee]pizod|[Ff]olge|[Cc]ap[ií]tulo|[Cc]hapter)[\\s_\\-.]*(\\d{1,4})",
    "(?:[Pp]art|[Pp]arte|[Pp]artie)[\\s_\\-.]*(\\d{1,4})",
    "\\b(?:[Ee][Pp]?|[Ee][Ss][Pp]?)\\.?\\s*(\\d{1,4})\\b",
    "(?:OVA|OAD|SP|NCED|NCOP|EX)[\\s_\\-.]*(\\d{1,4})",
    "(\\d{1,4})\\s*화",
    "(?:[Сс]ерия|[Вв]ыпуск)[\\s_\\-.]*(\\d{1,4})",
    "#[\\s_\\-.]?(\\d{1,4})\\b",
    "\\[(\\d{1,4})\\]",
    "[-_\\s](\\d{1,4})(?:v\\d)?[\\s_\\[\\].-]",
    "[\\s_\\-.](\\d{1,4})[\\s_\\-.]",
    "^(\\d{1,4})[\\s_\\-.]",
    "[\\s_\\-.](\\d{1,4})$",
  ],
  originalSubtitleHints: [
    "native",
    "original",
    "orig",
    "originale",
    "source",
    "sorgente",
    "raw",
    "raws",
    "dialogue",
    "target",
    "main",
    "primary",
    "audio",
    "nativo",
    "fuente",
    "quelle",
    "vo",
    "vost",
    "vostfr",
    "vostit",
    "omuu",
  ],
  referenceSubtitleHints: [
    "translated",
    "translation",
    "trans",
    "tradotto",
    "traduzione",
    "trad",
    "reference",
    "ref",
    "sub_ita",
    "sub-ita",
    "subita",
    "ita_sub",
    "sub_en",
    "sub-en",
    "sub_eng",
    "sub-eng",
    "subeng",
    "eng_sub",
    "sub_es",
    "sub-es",
    "sub_esp",
    "subesp",
    "traducido",
    "traduccion",
    "traducción",
    "traduit",
    "traduite",
    "traduction",
    "sub_fr",
    "sub-fr",
    "vf",
    "übersetzt",
    "uebersetzt",
    "übersetzung",
    "uebersetzung",
    "sub_de",
    "sub-de",
    "traduzido",
    "tradução",
    "traducao",
    "sub_pt",
    "sub-pt",
    "перевод",
    "переведено",
    "sub_ru",
    "sub-ru",
    "subbed",
    "subs",
    "subtitle",
    "subtitles",
    "secondary",
    "companion",
    "sub_target",
    "sub_ref",
    "aligned",
  ],
  removableNameTokens: [
    "2160p",
    "4k",
    "uhd",
    "1080p",
    "1080i",
    "720p",
    "576p",
    "480p",
    "360p",
    "h264",
    "h265",
    "h.264",
    "h.265",
    "x264",
    "x265",
    "x.264",
    "x.265",
    "hevc",
    "avc",
    "av1",
    "vp9",
    "xvid",
    "divx",
    "10bit",
    "10-bit",
    "hi10p",
    "hi10",
    "8bit",
    "hdr",
    "hdr10",
    "hdr10plus",
    "hdr10+",
    "dv",
    "dovi",
    "dolby-vision",
    "dolbyvision",
    "sdr",
    "hlg",
    "aac",
    "ac3",
    "eac3",
    "e-ac3",
    "ddp5.1",
    "dd5.1",
    "dd+",
    "dts",
    "dts-hd",
    "dts-ma",
    "truehd",
    "atmos",
    "dolby-atmos",
    "flac",
    "opus",
    "mp3",
    "vorbis",
    "lossless",
    "2ch",
    "5.1ch",
    "7.1ch",
    "5.1",
    "7.1",
    "dual-audio",
    "multi-audio",
    "dual",
    "multi",
    "dub",
    "dubbed",
    "bluray",
    "blu-ray",
    "bdrip",
    "brrip",
    "webrip",
    "web-dl",
    "webdl",
    "web",
    "dvdrip",
    "dvd",
    "hdtv",
    "pdtv",
    "tvrip",
    "remux",
    "vhsrip",
    "repack",
    "proper",
    "rerip",
    "v2",
    "v3",
    "v4",
    "uncut",
    "extended",
    "directors-cut",
    "theatrical",
    "remastered",
    "restored",
    "subbed",
    "softsub",
    "hardsub",
    "raw",
    "crc32",
    "subpack",
  ],
};

function normalizeRules(value: unknown): SmartMatchingRules {
  const obj = value && typeof value === "object" ? (value as Partial<SmartMatchingRules>) : {};
  return {
    episodeRegexes: Array.isArray(obj.episodeRegexes)
      ? obj.episodeRegexes.filter((x): x is string => typeof x === "string")
      : [...DEFAULT_SMART_MATCHING_RULES.episodeRegexes],
    originalSubtitleHints: Array.isArray(obj.originalSubtitleHints)
      ? obj.originalSubtitleHints.filter((x): x is string => typeof x === "string")
      : [...DEFAULT_SMART_MATCHING_RULES.originalSubtitleHints],
    referenceSubtitleHints: Array.isArray(obj.referenceSubtitleHints)
      ? obj.referenceSubtitleHints.filter((x): x is string => typeof x === "string")
      : [...DEFAULT_SMART_MATCHING_RULES.referenceSubtitleHints],
    removableNameTokens: Array.isArray(obj.removableNameTokens)
      ? obj.removableNameTokens.filter((x): x is string => typeof x === "string")
      : [...DEFAULT_SMART_MATCHING_RULES.removableNameTokens],
  };
}

class SmartMatchingStore {
  enabled = $state(vestaConfig.getItem("vesta-flashcards-smart-file-matching-enabled") !== "false");
  rules = $state<SmartMatchingRules>(DEFAULT_SMART_MATCHING_RULES);

  constructor() {
    this.load();
  }

  load() {
    try {
      const saved = vestaConfig.getItem("vesta-flashcards-smart-matching-rules");
      if (saved) {
        this.rules = normalizeRules(parseJsonWithComments(saved));
      } else {
        this.rules = normalizeRules(DEFAULT_SMART_MATCHING_RULES);
      }
    } catch {
      this.rules = normalizeRules(DEFAULT_SMART_MATCHING_RULES);
    }
  }

  setEnabled(val: boolean) {
    this.enabled = val;
    vestaConfig.setItem("vesta-flashcards-smart-file-matching-enabled", String(val));
  }

  saveRules(rules: SmartMatchingRules) {
    this.rules = normalizeRules(rules);
    vestaConfig.setItem("vesta-flashcards-smart-matching-rules", JSON.stringify(this.rules, null, 2));
  }

  resetRules() {
    this.rules = normalizeRules(DEFAULT_SMART_MATCHING_RULES);
    vestaConfig.setItem("vesta-flashcards-smart-matching-rules", JSON.stringify(this.rules, null, 2));
  }
}

export const smartMatchingStore = new SmartMatchingStore();
