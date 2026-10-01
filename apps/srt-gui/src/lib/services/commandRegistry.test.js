import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
import { expect, it } from "vitest";

it("registers every literal frontend IPC command in the desktop handler", () => {
  const root = fileURLToPath(new URL("../../..", import.meta.url));
  const main = readFileSync(join(root, "src-tauri/src/main.rs"), "utf8");
  const block = main.split("generate_handler![")[1].split("]")[0];
  const registered = new Set(block.match(/\b[a-z][a-z_0-9]+\b/g));
  const missing = [];
  function walk(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) walk(path);
      else if (/\.(svelte|ts)$/.test(path) && !path.endsWith(".test.ts")) {
        const text = readFileSync(path, "utf8");
        for (const match of text.matchAll(/(?:invoke|invokeCommand)(?:<[^;\n]*?>)?\s*\(\s*["']([a-z_0-9]+)["']/g)) {
          if (!registered.has(match[1])) missing.push(`${match[1]} in ${path}`);
        }
      }
    }
  }
  walk(join(root, "src"));
  expect(missing).toEqual([]);
});

it("bundles the flags used by every language", () => {
  const root = fileURLToPath(new URL("../../..", import.meta.url));
  const source = readFileSync(join(root, "src/lib/config/languages.ts"), "utf8");
  for (const [, flag] of source.matchAll(/flag: "([^"]+)"/g)) {
    const country = Array.from(flag).map(char => String.fromCharCode(char.codePointAt(0) - 0x1f1e6 + 97)).join("");
    expect(readFileSync(join(root, `public/flags/${country}.svg`), "utf8")).toContain("<svg");
  }
});
