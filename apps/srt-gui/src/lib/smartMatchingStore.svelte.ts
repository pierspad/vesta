import { locale } from "./i18n";

export interface SmartMatchingRules {
  episodeRegexes: string[];
  originalSubtitleHints: string[];
  referenceSubtitleHints: string[];
  removableNameTokens: string[];
}

export const DEFAULT_SMART_MATCHING_RULES: SmartMatchingRules = {
  episodeRegexes: [
    "[Ss]\\d{1,2}[Ee](\\d{1,4})",
    "[Pp]art(?:e)?[\\s_\\-.]?(\\d{1,4})",
    "[Ee][Pp]?\\.?\\s*(\\d{1,4})",
    "[Ee]pisode\\.?\\s*(\\d{1,4})",
    "[Xx](\\d{1,4})",
    "[\\s_\\-.](\\d{1,4})[\\s_\\-.]",
    "^(\\d{1,4})[\\s_\\-.]",
    "[\\s_\\-.](\\d{1,4})$",
  ],
  originalSubtitleHints: ["native", "original", "orig", "source"],
  referenceSubtitleHints: [
    "translated",
    "translation",
    "tradotto",
    "traduzione",
    "reference",
    "ref",
  ],
  removableNameTokens: [
    "1080p",
    "720p",
    "480p",
    "bluray",
    "webrip",
    "web-dl",
    "h264",
    "h265",
    "x264",
    "x265",
    "hevc",
    "dvdrip",
    "brrip",
    "ac3",
    "aac",
    "dts",
    "dual-audio",
    "dual",
    "multi",
    "remux",
  ],
};

function stripJsonComments(jsonString: string): string {
  return jsonString.replace(/\\"|"(?:\\"|[^"])*"|(\/\/.*|\/\*[\s\S]*?\*\/)/g, (m, g) => g ? "" : m);
}

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
  enabled = $state(localStorage.getItem("vesta-flashcards-smart-file-matching-enabled") !== "false");
  rules = $state<SmartMatchingRules>(DEFAULT_SMART_MATCHING_RULES);

  constructor() {
    this.load();
  }

  load() {
    try {
      const saved = localStorage.getItem("vesta-flashcards-smart-matching-rules");
      if (saved) {
        this.rules = normalizeRules(JSON.parse(stripJsonComments(saved)));
      } else {
        this.rules = normalizeRules(DEFAULT_SMART_MATCHING_RULES);
      }
    } catch {
      this.rules = normalizeRules(DEFAULT_SMART_MATCHING_RULES);
    }
  }

  setEnabled(val: boolean) {
    this.enabled = val;
    localStorage.setItem("vesta-flashcards-smart-file-matching-enabled", String(val));
  }

  saveRules(rules: SmartMatchingRules) {
    this.rules = rules;
    localStorage.setItem("vesta-flashcards-smart-matching-rules", JSON.stringify(rules, null, 2));
  }

  resetRules() {
    this.rules = normalizeRules(DEFAULT_SMART_MATCHING_RULES);
    localStorage.setItem("vesta-flashcards-smart-matching-rules", JSON.stringify(this.rules, null, 2));
  }
}

export const smartMatchingStore = new SmartMatchingStore();
