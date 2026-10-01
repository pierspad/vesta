import { beforeEach, describe, expect, it, vi } from "vitest";
import { buildModelsUrl, discoverModels, extractModelsFromPayload } from "./modelDiscovery";
import { fetch } from "./tauriHttp";
vi.mock("./tauriHttp", () => ({ fetch: vi.fn() }));
const mockedFetch = vi.mocked(fetch);

describe("model discovery", () => {
  beforeEach(() => vi.resetAllMocks());
  it.each([
    ["http://localhost:11434", "http://localhost:11434/v1/models"],
    ["http://localhost:1234/api/v1/", "http://localhost:1234/v1/models"],
    ["https://api.groq.com/openai/v1", "https://api.groq.com/openai/v1/models"],
    ["https://openrouter.ai/api/v1", "https://openrouter.ai/api/v1/models"],
  ])("normalizes %s", (base, expected) => expect(buildModelsUrl(base)).toBe(expected));
  it("deduplicates mixed payloads and rejects empty identifiers", () => {
    expect(extractModelsFromPayload({ data: [{ id: "x", displayName: "X" }, { id: "x" }, {}], models: [" y "] }))
      .toEqual([{ id: "x", name: "X" }, { id: "y", name: "y" }]);
  });
  it("filters Gemini models by generation capability", async () => {
    mockedFetch.mockResolvedValue({ ok: true, json: async () => ({ models: [
      { name: "models/gemini-test", displayName: "Gemini", supportedGenerationMethods: ["generateContent"] },
      { name: "models/embed-test", supportedGenerationMethods: ["embedContent"] },
    ] }) } as Awaited<ReturnType<typeof fetch>>);
    expect(await discoverModels("google", "key", "https://generativelanguage.googleapis.com/v1beta"))
      .toEqual([{ id: "gemini-test", name: "Gemini" }]);
  });
  it("reports authorization failures", async () => {
    mockedFetch.mockResolvedValue({ ok: false, status: 401 } as Awaited<ReturnType<typeof fetch>>);
    await expect(discoverModels("openai", "bad", "https://api.openai.com/v1")).rejects.toThrow("HTTP 401");
  });
});
