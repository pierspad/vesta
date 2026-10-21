/** Helpers shared by the i18n consistency checker and its regression tests. */

/** @param {string} text @param {string} [filePath] @returns {Set<string>} */
export function extractReferencedTranslationKeys(text, filePath = "") {
  const keys = new Set();
  for (const match of text.matchAll(/\b(?:t|setupT)\(\s*["'`]([\w.:-]+)["'`]/g)) {
    keys.add(match[1]);
  }
  if (filePath.endsWith("/tabs/SettingsTab.svelte")) {
    for (const match of text.matchAll(/\bs\(\s*["']([\w]+)["']/g)) {
      keys.add(`settings.copy.${match[1]}`);
    }
  }
  return keys;
}

/** @param {string} text @returns {string} */
export function stripSvelteExpressions(text) {
  let result = "";
  let depth = 0;
  for (const char of text) {
    if (char === "{") depth++;
    else if (char === "}") {
      if (depth > 0) depth--;
    } else if (depth === 0) result += char;
  }
  return result;
}

// These exact, visible literals are standards, product names, or measured
// technical values. Keep this list narrow: nearby explanatory copy must still
// be reviewed as UI text.
const ALLOWED_TECHNICAL_LITERALS = new Set([
  "Opus ·",
  "AnkiConnect",
  "Silero Voice Activity Detection 6.2.0",
  "-14 LUFS",
  "GStreamer",
]);

const LEGACY_TECHNICAL_LITERALS = new Set([
  "APKG", "TSV", "GitHub", "Repo", "vesta", "OK", "ms", "px", "kb/s",
  "sub", "car.", "Bitrate", "Codec", "Preset", "RPM", "URL", "API", "LLM",
  "ID", "SRT", "FFmpeg", "Whisper",
]);

/** @param {string} value @returns {boolean} */
function isKnownTechnicalLiteral(value) {
  return ALLOWED_TECHNICAL_LITERALS.has(value)
    || LEGACY_TECHNICAL_LITERALS.has(value)
    || value.toLowerCase() === "vesta"
    || /^v\d/i.test(value)
    || /^(GPL|Tauri|H\.264|MPEG).*/i.test(value)
    || /^&\w+;$/.test(value)
    || /^\d+.*/.test(value);
}

/** @param {string} filePath @param {string} text @returns {string[]} */
export function findHardcodedTemplateText(filePath, text) {
  // Test fixtures contain deliberate example copy, not shipped UI. This check
  // is intentionally limited to fixtures; translation-key scanning still
  // traverses these files via extractReferencedTranslationKeys.
  if (/(?:^|[\\/])src[\\/]lib[\\/]test[\\/]fixtures[\\/]/.test(filePath)) return [];

  const template = text.replace(/<script[\s\S]*?<\/script>/g, "").replace(/<style[\s\S]*?<\/style>/g, "");
  const clean = stripSvelteExpressions(template);
  const findings = [];
  for (const match of clean.matchAll(/>([^<>{}]*[A-Za-zÀ-ÿ]{3,}[^<>{}]*)</g)) {
    const value = match[1].trim();
    if (value && !isKnownTechnicalLiteral(value)) findings.push(value);
  }
  return findings;
}
