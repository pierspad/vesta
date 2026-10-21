import { expect, it, vi } from "vitest";
import { TranslationTierDiscovery } from "./translationTierDiscovery.svelte";

function cache() { return { getItem: vi.fn().mockReturnValue(null), setItem: vi.fn() }; }
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(done => resolve = done); return { promise, resolve }; }

it("deduplicates local polling and publishes without scheduling a second request", async () => {
  const result = [{ id: "local-model", name: "Local" }];
  const discover = vi.fn().mockResolvedValue(result); const state = new TranslationTierDiscovery(cache(), discover);
  await state.ensure("local", "local", "", "http://localhost");
  await state.ensure("local", "local", "", "http://localhost");
  expect(discover).toHaveBeenCalledOnce(); expect(state.models.size).toBe(1);
});

it("separates credential and URL identities while preserving newest visible results", async () => {
  const old = deferred<{ id: string; name: string }[]>();
  const discover = vi.fn().mockImplementationOnce(() => old.promise).mockResolvedValueOnce([{ id: "new", name: "New" }]);
  const storage = cache(); const state = new TranslationTierDiscovery(storage, discover);
  const first = state.ensure("openai", "account", "old-secret", "https://one");
  await vi.waitFor(() => expect(discover).toHaveBeenCalledOnce());
  const newIdentity = state.key("openai", "account", "new-secret", "https://one");
  await state.ensure("openai", "account", "new-secret", "https://one");
  state.retain(new Set([newIdentity]));
  old.resolve([{ id: "old", name: "Old" }]); await first;
  expect(state.models.get(newIdentity)?.[0].id).toBe("new");
  expect(storage.setItem).toHaveBeenCalledOnce();
  expect(state.key("openai", "account", "new-secret", "https://one")).not.toBe(state.key("openai", "account", "new-secret", "https://two"));
});

it("deduplicates requests while the credential fingerprint is being computed and invalidates removed credentials", async () => {
  const old = deferred<{ id: string; name: string }[]>();
  const discover = vi.fn().mockImplementation(() => old.promise); const storage = cache();
  const state = new TranslationTierDiscovery(storage, discover);
  const oldIdentity = state.key("groq", "account", "old-secret", "https://endpoint");
  const first = state.ensure("groq", "account", "old-secret", "https://endpoint");
  const duplicate = state.ensure("groq", "account", "old-secret", "https://endpoint");
  await vi.waitFor(() => expect(discover).toHaveBeenCalledOnce());
  const newIdentity = state.key("groq", "account", "rotated-secret", "https://endpoint");
  state.retain(new Set([newIdentity]));
  old.resolve([{ id: "old", name: "Old" }]); await Promise.all([first, duplicate]);
  expect(state.models.has(oldIdentity)).toBe(false); expect(storage.setItem).not.toHaveBeenCalled();
});

it("uses versioned expiring cache records and refreshes expired entries", async () => {
  const storage = cache(); storage.getItem.mockReturnValue(JSON.stringify({ version: 2, ts: Date.now() - 25 * 60 * 60 * 1000, models: [{ id: "stale", name: "Stale" }] }));
  const discover = vi.fn().mockResolvedValue([{ id: "fresh", name: "Fresh" }]);
  const state = new TranslationTierDiscovery(storage, discover);
  await state.ensure("groq", "id", "secret", "https://example");
  expect(discover).toHaveBeenCalledOnce();
  expect([...state.models.values()][0]?.[0].id).toBe("fresh");
});

it("reuses a valid persistent cache and separates full identities without storing secrets in keys", async () => {
  const records = new Map<string, string>();
  const storage = { getItem: (key: string) => records.get(key) ?? null, setItem: (key: string, value: string) => { records.set(key, value); } };
  const discover = vi.fn().mockResolvedValue([{ id: "model", name: "Model" }]);
  await new TranslationTierDiscovery(storage, discover).ensure("groq", "account", "abcdefgh-first", "https://one");
  await new TranslationTierDiscovery(storage, discover).ensure("groq", "account", "abcdefgh-first", "https://one");
  expect(discover).toHaveBeenCalledOnce();
  await new TranslationTierDiscovery(storage, discover).ensure("groq", "account", "abcdefgh-second", "https://one");
  await new TranslationTierDiscovery(storage, discover).ensure("groq", "account", "abcdefgh-first", "https://two");
  expect(discover).toHaveBeenCalledTimes(3);
  expect(records.size).toBe(3);
  expect([...records.keys()].every(key => !key.includes("abcdefgh"))).toBe(true);
});

it("continues live discovery if persistent cache fingerprinting is unavailable", async () => {
  const digest = vi.spyOn(crypto.subtle, "digest").mockRejectedValueOnce(new Error("unavailable"));
  try {
    const storage = cache(); const discover = vi.fn().mockResolvedValue([{ id: "live", name: "Live" }]);
    const state = new TranslationTierDiscovery(storage, discover);
    await state.ensure("groq", "account", "secret", "https://one");
    expect(discover).toHaveBeenCalledOnce();
    expect([...state.models.values()][0][0].id).toBe("live");
    expect(storage.setItem).not.toHaveBeenCalled();
  } finally { digest.mockRestore(); }
});
