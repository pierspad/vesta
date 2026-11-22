import { expect, it } from "vitest";
import { supportLinks } from "./supportLinks";
it("ships unique support links to the intended HTTPS providers", () => {
  expect(new Set(supportLinks.map(item => item.id)).size).toBe(supportLinks.length);
  expect(supportLinks.map(item => new URL(item.url).origin)).toEqual([
    "https://github.com", "https://buymeacoffee.com", "https://ko-fi.com",
  ]);
});
