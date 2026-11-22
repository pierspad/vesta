import { languages, normalizeLanguageCode } from "$lib/config/languages";

// This file has been fully split apart; what remains are path/language
// utilities with no other natural home. See:
// - llmProviders.ts: LLM provider/model catalog
// - languages.ts: language list + matching
// - shortcuts.ts: keyboard shortcuts
// - noteTypes.ts: field names / note types / card templates
// - apiKeys.ts: ApiProviderId, ApiKeyConfig, loadAndValidateApiKeys
// - translationTiers.ts: translation tier/failover system
// - transcribeTiers.ts: transcription tier/failover system
// - transcribeProviders.ts: cloud transcription provider catalog + settings
// - vadSelection.ts: Silero VAD variant selection
//
// Dropped as dead code along the way: saveCustomModel/deleteCustomModel/
// getCustomModels (no import site anywhere) and formatContextWindow (no
// import site anywhere).

export function getFileName(path: string): string {
  const normalized = path.replace(/\\/g, "/");
  return normalized.split("/").pop() || path;
}

export function inferLanguageFromPath(filePath: string): string | null {
  const base = getFileName(filePath).toLowerCase().replace(/\.[^/.]+$/, "");
  const tokens = base.split(/[.\-_\s()[\]]+/).filter(Boolean);
  for (let i = tokens.length - 1; i >= 0; i--) {
    // Longest suffix first: zh-Hant-TW must never fall back to simplified zh.
    for (let length = Math.min(3, i + 1); length >= 1; length--) {
      const candidate = tokens.slice(i - length + 1, i + 1).join("-");
      // Only combine actual locale subtags; avoid treating a movie title as a locale.
      if (length > 1 && !/^(zh-(hant|hans)(-(tw|hk|cn|sg))?|zh-(tw|hk|cn|sg)|pt-br|en-(us|gb)|ar-sa|de-de|es-es|fr-fr|it-it|ja-jp|ko-kr|ru-ru|hi-in|nl-nl|pl-pl|tr-tr)$/.test(candidate)) continue;
      const code = normalizeLanguageCode(candidate);
      if (code) return code;
    }
  }
  return null;
}

export function getFlagForPath(path: string): string {
  const code = inferLanguageFromPath(path);
  if (!code) return "";
  const lang = languages.find((l) => l.code === code);
  return lang?.flag || "";
}
