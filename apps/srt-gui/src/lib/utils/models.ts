import { languages } from "$lib/config/languages";

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

const knownLangCodes = new Set(languages.map((l) => l.code.toLowerCase()));

export function inferLanguageFromPath(filePath: string): string | null {
  const filename = getFileName(filePath).toLowerCase();
  const base = filename.replace(/\.[^/.]+$/, "");
  const tokens = base.split(/[.\-_]+/).filter(Boolean);
  for (let i = tokens.length - 1; i >= 0; i--) {
    if (knownLangCodes.has(tokens[i])) {
      const lang = languages.find((l) => l.code.toLowerCase() === tokens[i]);
      if (lang) return lang.code;
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
