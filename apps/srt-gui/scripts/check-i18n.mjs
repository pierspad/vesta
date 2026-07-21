#!/usr/bin/env node
/**
 * i18n consistency checker.
 *
 * 1. Every locale must contain exactly the keys of en.json (reference).
 * 2. Every `t("...")` key used in src/ must exist in en.json.
 * 3. `{{placeholder}}` sets must match the reference for each key.
 * 4. Heuristic scan for hardcoded UI text in Svelte templates.
 *
 * Exit code 1 on any error — wire it into CI or run `npm run check:i18n`.
 */
import { readFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const localesDir = join(root, "src", "lib", "i18n", "locales");
const srcDir = join(root, "src");

const locales = readdirSync(localesDir).filter((f) => f.endsWith(".json"));
const reference = JSON.parse(readFileSync(join(localesDir, "en.json"), "utf8"));
const refKeys = new Set(Object.keys(reference));

let errors = 0;
let warnings = 0;
const err = (msg) => { console.error(`  ✗ ${msg}`); errors++; };
const warn = (msg) => { console.warn(`  ⚠ ${msg}`); warnings++; };

// ── 1 + 3. Locale parity and placeholder consistency ─────────────────────────
const placeholders = (s) => [...String(s).matchAll(/\{\{(\w+)\}\}/g)].map((m) => m[1]).sort().join(",");

for (const file of locales) {
  const lang = file.replace(".json", "");
  const data = JSON.parse(readFileSync(join(localesDir, file), "utf8"));
  const keys = new Set(Object.keys(data));

  console.log(`Locale ${lang}: ${keys.size} keys`);
  for (const k of refKeys) if (!keys.has(k)) err(`${lang}: missing key "${k}"`);
  for (const k of keys) if (!refKeys.has(k)) err(`${lang}: extra key "${k}" (not in en.json)`);
  for (const k of keys) {
    if (refKeys.has(k) && placeholders(data[k]) !== placeholders(reference[k])) {
      err(`${lang}: placeholder mismatch in "${k}" (en: [${placeholders(reference[k])}] vs ${lang}: [${placeholders(data[k])}])`);
    }
  }
}

// ── 2. Keys referenced in source must exist ──────────────────────────────────
function* walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!["node_modules", "locales"].includes(entry.name)) yield* walk(p);
    } else if (/\.(svelte|ts|js)$/.test(entry.name)) {
      yield p;
    }
  }
}

console.log("\nScanning source for t(...) usages…");
const usedKeys = new Map(); // key -> first file
for (const file of walk(srcDir)) {
  const text = readFileSync(file, "utf8");
  for (const m of text.matchAll(/\bt\(\s*["'`]([\w.:-]+)["'`]/g)) {
    if (!usedKeys.has(m[1])) usedKeys.set(m[1], file);
  }
}
for (const [key, file] of usedKeys) {
  if (!refKeys.has(key)) err(`key "${key}" used in ${file.replace(root + "/", "")} but missing from en.json`);
}

// ── 3b. Locale hygiene: invisible chars, English leaks, whitespace ───────────
// Catches the bug classes found in the 2026-07 audit: zero-width spaces pasted
// from editors, English words leaking into machine-translated strings, and
// stray leading/trailing whitespace.
console.log("\nLocale hygiene checks…");
const INVISIBLE = /[​‌‍﻿­⁠]/;
for (const file of locales) {
  const lang = file.replace(".json", "");
  const data = JSON.parse(readFileSync(join(localesDir, file), "utf8"));
  for (const [k, v] of Object.entries(data)) {
    if (typeof v !== "string") continue;
    if (INVISIBLE.test(v)) err(`${lang}: invisible character (ZWSP/BOM/soft hyphen) in "${k}"`);
    if (v !== v.trim()) err(`${lang}: leading/trailing whitespace in "${k}"`);
    if (lang !== "en" && / and /.test(v)) warn(`${lang}: possible English leak (" and ") in "${k}": "${v.slice(0, 60)}"`);
  }
}

// ── 3c. Bilingual it/en ternary hacks in source ──────────────────────────────
// `$currentLanguage === 'it' ? ... : ...` shows English to the other 13
// locales — every string must go through t(). The language-selector grid in
// OverviewSettingsPanel legitimately compares codes, so equality against a
// string literal is what we flag.
console.log("\nScanning for bilingual ternary hacks…");
for (const file of walk(srcDir)) {
  const text = readFileSync(file, "utf8");
  for (const m of text.matchAll(/currentLanguage\s*===?\s*["'`](\w+)["'`]/g)) {
    err(`${file.replace(root + "/", "")}: hardcoded language check (currentLanguage === "${m[1]}") — use a t() key instead`);
  }
}

// ── 4. Heuristic: hardcoded text in Svelte templates ─────────────────────────
// Flags element text nodes with 3+ letters that aren't an expression. Curated
// noise filter — treat findings as review hints, not hard failures.
console.log("\nHeuristic scan for hardcoded template text…");
const IGNORE = /^(APKG|TSV|GitHub|Repo|vesta|OK|ms|px|kb\/s|sub|car\.|v\d|GPL.*|Tauri.*|H\.264.*|MPEG.*|Bitrate|Codec|Preset|RPM|URL|API|LLM|ID|SRT|FFmpeg|Whisper|Anki|Silero VAD|&\w+;|\d+.*)$/i;

function stripSvelteExpressions(text) {
  let result = "";
  let depth = 0;
  for (let i = 0; i < text.length; i++) {
    const char = text[i];
    if (char === "{") {
      depth++;
    } else if (char === "}") {
      if (depth > 0) depth--;
    } else if (depth === 0) {
      result += char;
    }
  }
  return result;
}

for (const file of walk(srcDir)) {
  if (!file.endsWith(".svelte")) continue;
  const text = readFileSync(file, "utf8");
  const template = text.replace(/<script[\s\S]*?<\/script>/g, "").replace(/<style[\s\S]*?<\/style>/g, "");
  const templateClean = stripSvelteExpressions(template);
  for (const m of templateClean.matchAll(/>([^<>{}]*[A-Za-zÀ-ÿ]{3,}[^<>{}]*)</g)) {
    const s = m[1].trim();
    if (!s || IGNORE.test(s)) continue;
    warn(`${file.replace(root + "/", "")}: possible hardcoded text "${s.slice(0, 60)}"`);
  }
}

console.log(`\n${errors} error(s), ${warnings} warning(s).`);
process.exit(errors > 0 ? 1 : 0);
