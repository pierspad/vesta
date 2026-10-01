import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
import { expect, it } from "vitest";
const directory = fileURLToPath(new URL("./locales", import.meta.url));
const read = (name) => JSON.parse(readFileSync(join(directory, name), "utf8"));
const english = read("en.json");
const placeholders = value => [...value.matchAll(/\{\{[^}]+\}\}|\{[A-Za-z0-9_.-]+\}/g)].map(m => m[0]).sort();
it("provides every UI key and preserves its parameters in all 15 languages", () => {
  const files = readdirSync(directory).filter(name => /^[a-z]{2}\.json$/.test(name));
  expect(files).toHaveLength(15);
  for (const name of files) {
    const catalog = read(name);
    expect(Object.keys(catalog).sort(), name).toEqual(Object.keys(english).sort());
    for (const [key, value] of Object.entries(catalog)) {
      expect(typeof value, `${name}:${key}`).toBe("string");
      expect(value.trim(), `${name}:${key}`).not.toBe("");
      expect(placeholders(value), `${name}:${key}`).toEqual(placeholders(english[key]));
    }
  }
});
it("translates the new extraction, setup and backup prose rather than falling back to English", () => {
  for (const name of readdirSync(directory).filter(name => /^[a-z]{2}\.json$/.test(name) && name !== "en.json")) {
    const catalog = read(name);
    for (const key of ["extract.search", "extract.working", "setup.setUpYourExperience", "settings.backup.desc", "common.loadFailed"]) {
      expect(catalog[key], `${name}:${key}`).not.toBe(english[key]);
    }
  }
});
