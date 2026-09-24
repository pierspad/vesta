import { describe, expect, it } from "vitest";
import { isTrustedSupportUrl, supportLinks } from "./supportLinks";

describe("support links", () => {
  it("contains three unique trusted HTTPS providers", () => {
    expect(new Set(supportLinks.map((item) => item.id)).size).toBe(3);
    expect(supportLinks.every((item) => isTrustedSupportUrl(item.url))).toBe(true);
  });
  it("rejects lookalike and non-HTTPS links", () => {
    expect(isTrustedSupportUrl("http://github.com/sponsors/pierspad")).toBe(false);
    expect(isTrustedSupportUrl("https://github.com.evil.test/pierspad")).toBe(false);
  });
});
