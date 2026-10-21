import { expect, it, vi } from "vitest";
import { SpeechModelDiscovery } from "./speechModelDiscovery.svelte";

function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(done => resolve = done); return { promise, resolve }; }

it("deduplicates identical checks but lets a forced refresh supersede the old response", async () => {
  const old = deferred<{ id: string; name: string }[]>();
  const discover = vi.fn().mockImplementationOnce(() => old.promise).mockResolvedValueOnce([{ id: "new", name: "New" }]);
  const state = new SpeechModelDiscovery(discover, () => "fallback");
  const identity = state.identity("id", "old-secret", "https://speech");
  const first = state.load(identity, "old-secret", "https://speech");
  const duplicate = state.load(identity, "old-secret", "https://speech");
  await vi.waitFor(() => expect(discover).toHaveBeenCalledOnce());
  await state.load(identity, "old-secret", "https://speech", true);
  old.resolve([{ id: "old", name: "Old" }]); await Promise.all([first, duplicate]);
  expect(state.models.get(identity)?.[0].id).toBe("new");
});

it("separates credential revisions and suppresses removed-identity responses", async () => {
  const old = deferred<{ id: string; name: string }[]>();
  const discover = vi.fn().mockImplementation(() => old.promise);
  const state = new SpeechModelDiscovery(discover, () => "fallback");
  const identity = state.identity("id", "secret", "https://speech");
  const pending = state.load(identity, "secret", "https://speech");
  state.invalidate(identity);
  old.resolve([{ id: "late", name: "Late" }]); await pending;
  expect(state.models.has(identity)).toBe(false); expect(state.errors.has(identity)).toBe(false);
  expect(state.identity("id", "rotated", "https://speech")).not.toBe(identity);
});

it("deduplicates before discovery starts and invalidates removed entries", async () => {
  let resolve!: (value: { id: string; name: string }[]) => void;
  const discover = vi.fn().mockImplementation(() => new Promise(done => resolve = done));
  const state = new SpeechModelDiscovery(discover, () => "fallback");
  const identity = state.identity("id", "secret", "https://speech");
  const first = state.load(identity, "secret", "https://speech");
  const duplicate = state.load(identity, "secret", "https://speech");
  await vi.waitFor(() => expect(discover).toHaveBeenCalledOnce());
  state.retain(new Set()); resolve([{ id: "late", name: "Late" }]); await Promise.all([first, duplicate]);
  expect(state.models.has(identity)).toBe(false); expect(state.errors.has(identity)).toBe(false);
});

it("ignores stale errors after a newer successful result", async () => {
  let reject!: (error: Error) => void;
  const discover = vi.fn().mockImplementationOnce(() => new Promise((_, fail) => reject = fail)).mockResolvedValueOnce([{ id: "ok", name: "OK" }]);
  const state = new SpeechModelDiscovery(discover, () => "fallback"); const identity = state.identity("id", "key", "url");
  const first = state.load(identity, "key", "url"); await state.load(identity, "key", "url", true);
  reject(new Error("old failure")); await first;
  expect(state.errors.has(identity)).toBe(false); expect(state.models.get(identity)?.[0].id).toBe("ok");
});
