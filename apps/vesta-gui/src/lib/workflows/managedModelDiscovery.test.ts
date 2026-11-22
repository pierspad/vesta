import { expect, it, vi } from "vitest";
import { ManagedModelDiscovery } from "./managedModelDiscovery.svelte";
const models = [{ id: "model", name: "Model" }];
const cache = () => ({ getItem: vi.fn().mockReturnValue(null), setItem: vi.fn() });
it("uses valid cached lists and refreshes explicitly", async () => {
  const storage = cache(); storage.getItem.mockReturnValue(JSON.stringify({ version: 2, ts: Date.now(), models }));
  const discover = vi.fn().mockResolvedValue(models);
  const state = new ManagedModelDiscovery(storage, discover);
  await state.load("groq", "key"); expect(discover).not.toHaveBeenCalled();
  await state.load("groq", "key", true); expect(discover).toHaveBeenCalledOnce();
});
it("ignores old provider responses and does not publish them to cache", async () => {
  let resolve!: (v: typeof models) => void;
  const discover = vi.fn().mockImplementationOnce(() => new Promise(done => { resolve = done; })).mockResolvedValueOnce([{ id: "new", name: "New" }]);
  const storage = cache(); const state = new ManagedModelDiscovery(storage, discover);
  const old = state.load("google", "old-key"); await vi.waitFor(() => expect(discover).toHaveBeenCalledTimes(1));
  await state.load("groq", "new-key");
  resolve(models); await old;
  expect(state.models[0].id).toBe("new"); expect(storage.setItem).toHaveBeenCalledOnce();
});
it("does not retry a failed identity automatically, permits explicit retry", async () => {
  const discover = vi.fn().mockRejectedValue(new Error("offline")); const error = vi.fn();
  const state = new ManagedModelDiscovery(cache(), discover, error);
  await state.load("groq", "key"); await state.load("groq", "key");
  expect(discover).toHaveBeenCalledOnce(); expect(state.loading).toBe(false);
  await state.load("groq", "key", true); expect(discover).toHaveBeenCalledTimes(2);
});
it("invalidates pending requests on provider clear and disposal", async () => {
  for (const dispose of [false, true]) {
    let resolve!: (v: typeof models) => void;
    const discover = vi.fn().mockImplementation(() => new Promise(done => { resolve = done; }));
    const storage = cache(); const state = new ManagedModelDiscovery(storage, discover);
    const pending = state.load("groq", "key");
    await vi.waitFor(() => expect(discover).toHaveBeenCalled());
    if (dispose) state.dispose(); else await state.load("local", "");
    resolve(models); await pending;
    expect(state.models).toEqual([]); expect(storage.setItem).not.toHaveBeenCalled();
  }
});
it("falls through malformed cache and reloads when credentials change", async () => {
  const storage = cache(); storage.getItem.mockReturnValue('[{"broken":true}]');
  const discover = vi.fn().mockResolvedValue(models); const state = new ManagedModelDiscovery(storage, discover);
  await state.load("groq", "key-one"); await state.load("groq", "key-two");
  expect(discover).toHaveBeenCalledTimes(2); expect(state.models).toEqual(models);
});
it("uses full credential identity and expires persisted lists", async () => {
  const storage = cache(); const discover = vi.fn().mockResolvedValue(models);
  const state = new ManagedModelDiscovery(storage, discover);
  await state.load("groq", "abcdefgh-first");
  await state.load("groq", "abcdefgh-second");
  expect(discover).toHaveBeenCalledTimes(2);
  const [firstKey, firstValue] = storage.setItem.mock.calls[0];
  expect(firstKey).not.toContain("abcdefgh");
  expect(JSON.parse(firstValue)).toMatchObject({ version: 2, models });

  const expired = cache();
  expired.getItem.mockReturnValue(JSON.stringify({ version: 2, ts: Date.now() - 25 * 60 * 60 * 1000, models }));
  const second = new ManagedModelDiscovery(expired, discover);
  await second.load("groq", "abcdefgh-first");
  expect(discover).toHaveBeenCalledTimes(3);
});
