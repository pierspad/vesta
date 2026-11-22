import { expect, it, vi } from "vitest";
import { fetchModelCatalog } from "./modelCatalog";
import type { fetch as TauriFetch } from "./tauriHttp";

const reply = (data: unknown) => ({ ok: true, text: async () => JSON.stringify(data) });

it.each([
  ["google", { models: [{ name: "first" }], nextPageToken: "a/b ?&" }, { models: [{ name: "last" }] }, "pageToken", "models"],
  ["anthropic", { data: [{ id: "first" }], has_more: true, last_id: "a/b ?&" }, { data: [{ id: "last" }], has_more: false }, "after_id", "data"],
] as const)("collects %s pages on the same endpoint with the same authentication", async (provider, first, last, parameter, field) => {
  const fetch = vi.fn().mockResolvedValueOnce(reply(first)).mockResolvedValueOnce(reply(last));
  const options = { headers: { "x-api-key": "secret" }, timeoutMs: 8000, allowCustomHost: true };
  const result = await fetchModelCatalog("https://proxy.test/v1/models?limit=1", options, provider, fetch as unknown as typeof TauriFetch);
  expect((result as Record<string, unknown[]>)[field]).toHaveLength(2);
  const next = new URL(fetch.mock.calls[1][0]);
  expect(next.origin).toBe("https://proxy.test");
  expect(next.pathname).toBe("/v1/models");
  expect(next.searchParams.get(parameter)).toBe("a/b ?&");
  expect(next.searchParams.get("limit")).toBe("1");
  expect(fetch.mock.calls[1][1]).toMatchObject({ headers: options.headers, allowCustomHost: true, method: "GET" });
});

it("rejects repeated cursors, missing Anthropic cursors and errors on later pages", async () => {
  for (const [provider, responses] of [
    ["google", [reply({ models: [], nextPageToken: "same" }), reply({ models: [], nextPageToken: "same" })]],
    ["anthropic", [reply({ data: [], has_more: true })]],
    ["google", [reply({ models: [], nextPageToken: "next" }), { ok: false, status: 401 }]],
  ] as const) {
    const fetch = vi.fn();
    for (const response of responses) fetch.mockResolvedValueOnce(response);
    await expect(fetchModelCatalog("https://proxy.test/models", {}, provider, fetch as unknown as typeof TauriFetch)).rejects.toThrow();
    expect(fetch.mock.calls.length).toBeLessThanOrEqual(2);
  }
});

it("bounds pages, total entries and the overall discovery deadline", async () => {
  let page = 0;
  const pages = vi.fn(async () => reply({ models: [], nextPageToken: String(++page) }));
  await expect(fetchModelCatalog("https://proxy.test/models", {}, "google", pages as unknown as typeof TauriFetch)).rejects.toThrow("page limit");
  expect(pages).toHaveBeenCalledTimes(50);
  const large = vi.fn().mockResolvedValue(reply({ models: Array.from({ length: 10001 }, () => ({})) }));
  await expect(fetchModelCatalog("https://proxy.test/models", {}, "google", large as unknown as typeof TauriFetch)).rejects.toThrow("entry limit");
  const now = vi.spyOn(Date, "now");
  now.mockReturnValueOnce(0).mockReturnValueOnce(0).mockReturnValueOnce(9000);
  const slow = vi.fn().mockResolvedValue(reply({ models: [], nextPageToken: "next" }));
  try {
    await expect(fetchModelCatalog("https://proxy.test/models", { timeoutMs: 8000 }, "google", slow as unknown as typeof TauriFetch)).rejects.toThrow("timed out");
    expect(slow).toHaveBeenCalledOnce();
  } finally { now.mockRestore(); }
});
