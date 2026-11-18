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
  expect(fetch.mock.calls[0][0]).toBe("https://generativelanguage.googleapis.com/v1beta/models");
  expect(fetch.mock.calls[0][1].headers["x-goog-api-key"]).toBe("key");
  expect(fetch.mock.calls[1][0]).toBe("https://generativelanguage.googleapis.com/v1beta/models/allowed:generateContent");
  expect(fetch.mock.calls[1][1].headers["x-goog-api-key"]).toBe("key");
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

it("ignores malformed Google models and deduplicates normalized IDs before probing", async () => {
  const fetch = vi.fn().mockResolvedValueOnce(response({ models: [
    null, 42, "bad", {},
    { name: "models/no-methods", supportedGenerationMethods: "generateContent" },
    { name: "models/", supportedGenerationMethods: ["generateContent"] },
    { name: "models/valid", displayName: "Valid", supportedGenerationMethods: ["generateContent"] },
    { id: "valid", supportedGenerationMethods: ["generateContent"] },
  ] })).mockResolvedValue(response({}));
  expect(await discoverManagedProviderModels("google", "key", fetch as unknown as typeof tauriFetch))
    .toEqual([{ id: "valid", name: "Valid" }]);
  expect(fetch).toHaveBeenCalledTimes(2);
});

it("limits simultaneous probes, refills freed slots after errors and preserves catalog order", async () => {
  const resolvers = new Map<string, { resolve: (value: ReturnType<typeof response>) => void; reject: (error: Error) => void }>();
  const models = Array.from({ length: 8 }, (_, i) => ({
    name: `models/m${i}`, supportedGenerationMethods: ["generateContent"],
  }));
  let active = 0;
  let maximum = 0;
  const fetch = vi.fn(async (url: string) => {
    if (url.endsWith("/models")) return response({ models });
    active += 1;
    maximum = Math.max(maximum, active);
    const id = url.match(/models\/(m\d+):/)![1];
    try {
      return await new Promise<ReturnType<typeof response>>((resolve, reject) => {
        resolvers.set(id, { resolve, reject });
      });
    } finally { active -= 1; }
  });
  const pending = discoverManagedProviderModels("google", "key", fetch as unknown as typeof tauriFetch);
  await vi.waitFor(() => expect(resolvers.size).toBe(4));
  resolvers.get("m2")!.reject(new Error("offline"));
  await vi.waitFor(() => expect(resolvers.has("m4")).toBe(true));
  resolvers.get("m4")!.resolve(response({}, false));
  await vi.waitFor(() => expect(resolvers.has("m5")).toBe(true));
  resolvers.get("m3")!.resolve(response({}));
  await vi.waitFor(() => expect(resolvers.has("m6")).toBe(true));
  resolvers.get("m6")!.resolve(response({}));
  await vi.waitFor(() => expect(resolvers.has("m7")).toBe(true));
  for (const id of ["m7", "m5", "m1", "m0"]) resolvers.get(id)!.resolve(response({}));
  expect((await pending).map(model => model.id)).toEqual(["m0", "m1", "m3", "m5", "m6", "m7"]);
  expect(maximum).toBe(4);
  expect(fetch).toHaveBeenCalledTimes(9);
});
