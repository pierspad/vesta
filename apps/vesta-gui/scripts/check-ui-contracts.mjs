#!/usr/bin/env node
/**
 * Programmatic UI contract checks for Vesta:
 * 1. Shortcut integrity: every defaultShortcut resolves its i18n key, maps to
 *    ShortcutsTab + ShortcutOverlay categories, avoids scope conflicts/reserved
 *    bindings, uses semantically valid descriptions, and has an active handler.
 * 2. Tooltip, aria-label, placeholder & button hygiene across Svelte components:
 *    no missing i18n keys, no hardcoded English prose in attributes, and no
 *    unlabelled icon-only buttons.
 */
import { readFileSync, readdirSync } from "node:fs";
import { join, dirname, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ALLOWED_LITERAL_ATTRS = [
  /^[+-]?\d+(?:\.\d+)?(?:s|ms|px|%)$/,
  /^[XYWH]\s*\(%\)$/,
  /^https?:\/\//,
  /^docs\/[A-Za-z0-9_.-]+$/,
  /^Pierpaolo Spadafora$/,
  /^Vesta_[A-Za-z0-9_]+$/,
  /^WebP$/,
  /^[^\p{L}\p{N}]+$/u,
  /^\d+$/,
];

export function isAllowedLiteralAttr(value) {
  const trimmed = value.trim();
  if (trimmed.length <= 2) return true;
  return ALLOWED_LITERAL_ATTRS.some((re) => re.test(trimmed));
}

export function extractShortcuts(shortcutsSource) {
  const defs = [];
  const re =
    /\{\s*id:\s*"([^"]+)",\s*action:\s*"([^"]+)",\s*description:\s*"([^"]+)",\s*defaultKey:\s*"([^"]+)",\s*category:\s*"([^"]+)"\s*\}/g;
  for (const m of shortcutsSource.matchAll(re)) {
    defs.push({
      id: m[1],
      action: m[2],
      description: m[3],
      defaultKey: m[4],
      category: m[5],
    });
  }
  return defs;
}

function collectFiles(dir, predicate, out = []) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "node_modules" || entry.name === "locales" || entry.name === "test") continue;
      collectFiles(full, predicate, out);
    } else if (predicate(full)) {
      out.push(full);
    }
  }
  return out;
}

export function checkUiContracts({
  srcDir,
  enCatalog,
}) {
  const errors = [];

  const shortcutsPath = join(srcDir, "lib/utils/shortcuts.ts");
  const overlayPath = join(srcDir, "lib/components/ShortcutOverlay.svelte");
  const shortcutsTabPath = join(srcDir, "lib/tabs/ShortcutsTab.svelte");

  const shortcutsSource = readFileSync(shortcutsPath, "utf8");
  const overlaySource = readFileSync(overlayPath, "utf8");
  const shortcutsTabSource = readFileSync(shortcutsTabPath, "utf8");

  const sourceFiles = collectFiles(
    srcDir,
    (p) => (p.endsWith(".svelte") || p.endsWith(".ts")) && !p.includes(".test."),
  );

  // Code outside shortcuts definition/UI tables where handlers must exist.
  const handlerCorpus = sourceFiles
    .filter((p) => p !== shortcutsPath && p !== overlayPath && p !== shortcutsTabPath)
    .map((p) => readFileSync(p, "utf8"))
    .join("\n");

  const defs = extractShortcuts(shortcutsSource);
  if (defs.length === 0) {
    errors.push("No shortcut definitions extracted from src/lib/utils/shortcuts.ts");
  }

  const globalKeys = new Set(defs.filter((d) => d.category === "global").map((d) => d.defaultKey));
  globalKeys.add("Shift+?");
  const perCategory = new Map();

  for (const def of defs) {
    if (!Object.hasOwn(enCatalog, def.description)) {
      errors.push(`Shortcut "${def.id}" uses missing i18n description key "${def.description}"`);
    }
    if (def.category === "transcribe" && def.description === "shortcuts.action.openSrt") {
      errors.push(`Shortcut "${def.id}" in transcribe category wrongly describes opening SRT instead of media`);
    }

    if (def.category !== "global") {
      if (!shortcutsTabSource.includes(`${def.category}:`)) {
        errors.push(`Shortcut category "${def.category}" missing from ShortcutsTab.svelte categoryLabels`);
      }
      if (!overlaySource.includes(`"${def.category}"`)) {
        errors.push(`Shortcut category "${def.category}" missing from ShortcutOverlay.svelte tabToCategory`);
      }
      if (globalKeys.has(def.defaultKey)) {
        errors.push(`Non-global shortcut "${def.id}" (${def.defaultKey}) collides with a reserved global shortcut`);
      }
    }

    const catMap = perCategory.get(def.category) ?? new Map();
    perCategory.set(def.category, catMap);
    const existing = catMap.get(def.defaultKey);
    if (existing) {
      errors.push(`Duplicate defaultKey "${def.defaultKey}" in category "${def.category}": ${existing} vs ${def.id}`);
    } else {
      catMap.set(def.defaultKey, def.id);
    }

    if (def.id === "show-help") {
      if (!overlaySource.includes(def.defaultKey.split("+").at(-1) || "")) {
        errors.push(`Shortcut "${def.id}" (${def.defaultKey}) is not handled in ShortcutOverlay.svelte`);
      }
      continue;
    }

    const isHistory = def.id.endsWith("-undo") || def.id.endsWith("-redo");
    if (isHistory) {
      const scope = def.id.replace(/-(undo|redo)$/, "");
      const hasHistoryCall = new RegExp(`historyShortcutAction\\([^,]+,\\s*"${scope}"`).test(handlerCorpus);
      if (!hasHistoryCall) {
        errors.push(`History shortcut "${def.id}" has no historyShortcutAction(..., "${scope}") handler`);
      }
    } else {
      const actionQuoted = `"${def.action}"`;
      const actionSingle = `'${def.action}'`;
      if (!handlerCorpus.includes(actionQuoted) && !handlerCorpus.includes(actionSingle)) {
        errors.push(`Shortcut "${def.id}" action "${def.action}" has no handler in App or tab listeners`);
      }
    }
  }

  // Tooltip, aria-label, placeholder, and icon-only button checks in .svelte components
  const svelteFiles = collectFiles(join(srcDir, "lib"), (p) => p.endsWith(".svelte") && !p.includes("/test/"));
  for (const filePath of svelteFiles) {
    const rel = relative(srcDir, filePath).replaceAll("\\", "/");
    const content = readFileSync(filePath, "utf8");
    const lines = content.split(/\r?\n/);

    lines.forEach((line, idx) => {
      const lineNo = idx + 1;
      for (const m of line.matchAll(/\b(title|aria-label|placeholder)=(?:"([^"]*)"|\{([^}]+)\})/g)) {
        const attr = m[1];
        const literal = m[2];
        const expr = m[3];
        if (literal !== undefined) {
          if (literal.trim() === "" && attr !== "placeholder") {
            errors.push(`${rel}:${lineNo}: empty literal ${attr}=""`);
          } else if (literal.trim() !== "" && !isAllowedLiteralAttr(literal)) {
            errors.push(`${rel}:${lineNo}: hardcoded literal ${attr}="${literal}" — move to i18n catalog`);
          }
        } else if (expr) {
          for (const tm of expr.matchAll(/\bt\(\s*"([^"]+)"/g)) {
            const key = tm[1];
            if (!Object.hasOwn(enCatalog, key)) {
              errors.push(`${rel}:${lineNo}: ${attr} uses missing i18n key "${key}"`);
            }
          }
        }
      }
    });

    for (const m of content.matchAll(/<button\b([^>]*)>([\s\S]*?)<\/button>/g)) {
      const attrs = m[1];
      const body = m[2];
      if (!body.includes("<svg")) continue;
      // Ignore child elements when checking whether the button carries visible text or label bindings.
      const withoutTags = body.replace(/<[^>]+>/g, " ").replace(/\{[/:#@][^}]*\}/g, " ").trim();
      if (withoutTags === "" && !/\btitle=/.test(attrs) && !/\baria-label=/.test(attrs)) {
        const lineNo = content.slice(0, m.index).split(/\r?\n/).length;
        errors.push(`${rel}:${lineNo}: icon-only <button> is missing both title and aria-label`);
      }
    }
  }

  return { errors, shortcutCount: defs.length, componentCount: svelteFiles.length };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const appRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
  const srcDir = join(appRoot, "src");
  const enCatalog = JSON.parse(readFileSync(join(srcDir, "lib/i18n/locales/en.json"), "utf8"));
  const result = checkUiContracts({ srcDir, enCatalog });
  if (result.errors.length > 0) {
    console.error("UI contract check failed:\n  - " + result.errors.join("\n  - "));
    process.exit(1);
  }
  console.log(
    `UI contract check: 0 error(s) (${result.shortcutCount} shortcuts, ${result.componentCount} components checked).`,
  );
}
