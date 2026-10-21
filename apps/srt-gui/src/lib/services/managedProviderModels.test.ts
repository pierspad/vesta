import { expect, it, vi } from "vitest";
import { discoverManagedProviderModels } from "./managedProviderModels";
import type { fetch as tauriFetch } from "./tauriHttp";
function response(data: unknown, ok = true) {
  return { ok, status: ok ? 200 : 403, text: async () => JSON.stringify(data) };
}
it("lists Groq models with bearer authorization without generation probes", async () => {
  const fetch = vi.fn().mockResolvedValue(response({ data: [{ id: "model" }] }));
  expect(await discoverManagedProviderModels("groq", "key", fetch as unknown as typeof tauriFetch)).toEqual([{ id: "model", name: "model" }]);
  expect(fetch).toHaveBeenCalledOnce();
  expect(fetch.mock.calls[0][1].headers.Authorization).toBe("Bearer key");
});
it("filters Google generation methods and keeps only accessible models", async () => {
  const fetch = vi.fn().mockResolvedValueOnce(response({ models: [
    { name: "models/allowed", displayName: "Allowed", supportedGenerationMethods: ["generateContent"] },
    { name: "models/denied", supportedGenerationMethods: ["generateContent"] },
    { name: "models/embedding", supportedGenerationMethods: ["embedContent"] },
  ] })).mockResolvedValueOnce(response({}, true)).mockResolvedValueOnce(response({}, false));
  expect(await discoverManagedProviderModels("google", "key", fetch as unknown as typeof tauriFetch)).toEqual([{ id: "allowed", name: "Allowed" }]);
  expect(fetch).toHaveBeenCalledTimes(3);
});
it("preserves the discovery list when all Google probes fail", async () => {
  const fetch = vi.fn().mockResolvedValueOnce(response({ models: [
    { name: "models/fallback", supportedGenerationMethods: ["generateContent"] },
  ] })).mockRejectedValue(new Error("unreachable"));
  expect(await discoverManagedProviderModels("google", "key", fetch as unknown as typeof tauriFetch)).toEqual([{ id: "fallback", name: "fallback" }]);
});
it("rejects HTTP errors and empty lists so the caller can retain its static fallback", async () => {
  for (const data of [response({}, false), response({ data: [] })]) {
    const fetch = vi.fn().mockResolvedValue(data);
    await expect(discoverManagedProviderModels("groq", "key", fetch as unknown as typeof tauriFetch)).rejects.toThrow();
  }
});
