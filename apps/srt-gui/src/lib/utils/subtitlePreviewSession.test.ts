import { expect, it, vi } from "vitest";
import { SubtitlePreviewSession } from "./subtitlePreviewSession";

function deferred() {
  let resolve!: (text: string) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<string>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
async function flush() { for (let i = 0; i < 10; i++) await Promise.resolve(); }

it("preloads all language variants, then navigates from cache", async () => {
  const load = vi.fn(async (index: number) => `subtitle ${index}`);
  const session = new SubtitlePreviewSession([1, 2, 3, 4, 5], load);
  expect(await session.select(3)).toBe("subtitle 3");
  await flush();
  expect(load.mock.calls.map(([index]) => index)).toEqual([3, 2, 4, 1, 5]);
  expect(await session.select(4)).toBe("subtitle 4");
  await flush();
  expect(load.mock.calls.map(([index]) => index)).toEqual([3, 2, 4, 1, 5]);
  session.close();
});

it("shares an in-flight prefetch when the user navigates to it", async () => {
  const first = deferred(); const next = deferred();
  const load = vi.fn((index: number) => index === 1 ? first.promise : next.promise);
  const session = new SubtitlePreviewSession([1, 2], load);
  const initial = session.select(1);
  await flush();
  const selected = session.select(2);
  next.resolve("second");
  expect(await selected).toBe("second");
  first.resolve("first");
  await initial;
  expect(load).toHaveBeenCalledTimes(2);
  session.close();
});

it("bounds concurrent extractions and prioritizes the requested queued track", async () => {
  const jobs = new Map([1, 2, 3, 4, 5].map(index => [index, deferred()]));
  const load = vi.fn((index: number) => jobs.get(index)!.promise);
  const session = new SubtitlePreviewSession([1, 2, 3, 4, 5], load);
  const initial = session.select(2);
  await flush();
  expect(load.mock.calls.map(([index]) => index)).toEqual([2, 1]);
  const selected = session.select(4);
  jobs.get(2)!.resolve("second");
  await initial; await flush();
  expect(load.mock.calls.map(([index]) => index)).toEqual([2, 1, 4]);
  jobs.get(4)!.resolve("fourth");
  expect(await selected).toBe("fourth");
  session.close();
  jobs.get(1)!.resolve("first");
  await flush();
});

it("retains distant previews until close", async () => {
  const load = vi.fn(async (index: number) => String(index));
  const session = new SubtitlePreviewSession([1, 2, 3, 4, 5], load);
  await session.select(1); await flush();
  await session.select(4); await flush();
  await session.select(1); await flush();
  expect(load.mock.calls.filter(([index]) => index === 1)).toHaveLength(1);
  session.close();
});

it("does not retain failed prefetches and retries on selection", async () => {
  const load = vi.fn(async (index: number) => {
    if (index === 2 && load.mock.calls.filter(([value]) => value === 2).length === 1) throw new Error("probe failed");
    return String(index);
  });
  const session = new SubtitlePreviewSession([1, 2], load);
  expect(await session.select(1)).toBe("1"); await flush();
  expect(await session.select(2)).toBe("2");
  expect(load.mock.calls.filter(([index]) => index === 2)).toHaveLength(2);
  session.close();
});

it("propagates foreground errors and permits retry", async () => {
  const load = vi.fn().mockRejectedValueOnce(new Error("broken")).mockResolvedValue("retry");
  const session = new SubtitlePreviewSession([1], load);
  await expect(session.select(1)).rejects.toThrow("broken"); await flush();
  expect(await session.select(1)).toBe("retry");
  session.close();
});

it("rejects active requests on close and prevents queued work from starting", async () => {
  const jobs = [deferred(), deferred()];
  const load = vi.fn((index: number) => jobs[index - 1].promise);
  const session = new SubtitlePreviewSession([1, 2, 3], load);
  const selected = session.select(2);
  const rejected = expect(selected).rejects.toThrow("closed");
  await flush();
  session.close();
  await rejected;
  jobs[0].resolve("late first"); jobs[1].resolve("late second"); await flush();
  expect(load.mock.calls.map(([index]) => index)).toEqual([2, 1]);
  await expect(session.select(2)).rejects.toThrow("closed");
  const fresh = new SubtitlePreviewSession([2], load);
  expect(await fresh.select(2)).toBe("late second");
  expect(load).toHaveBeenCalledTimes(3);
  fresh.close();
});

it("does not dispatch IPC when closed immediately", async () => {
  const load = vi.fn(async () => "unused");
  const session = new SubtitlePreviewSession([1, 2], load);
  const selected = session.select(1);
  const rejected = expect(selected).rejects.toThrow("closed");
  session.close();
  await rejected; await flush();
  expect(load).not.toHaveBeenCalled();
});

it("handles boundaries and empty subtitle text without duplicate work", async () => {
  const load = vi.fn(async () => "");
  const session = new SubtitlePreviewSession([7], load);
  expect(await session.select(7)).toBe(""); await flush();
  expect(await session.select(7)).toBe("");
  expect(load).toHaveBeenCalledTimes(1);
  await expect(session.select(99)).rejects.toThrow("Unknown");
  session.close();
});

it("retains late results for navigation", async () => {
  const late = deferred();
  const load = vi.fn((index: number) => index === 1 ? late.promise : Promise.resolve(String(index)));
  const session = new SubtitlePreviewSession([1, 2, 3, 4, 5], load);
  const old = session.select(1);
  await flush();
  const current = session.select(4);
  await flush();
  expect(await current).toBe("4");
  late.resolve("late first");
  await old; await flush();
  expect(await session.select(1)).toBe("late first");
  expect(load.mock.calls.filter(([index]) => index === 1)).toHaveLength(1);
  session.close();
});

it("prioritizes navigation over queued background variants", async () => {
  const active = deferred();
  const load = vi.fn(async (index: number) => index <= 2 ? active.promise : String(index));
  const session = new SubtitlePreviewSession([1, 2, 3, 4, 5, 6], load);
  const first = session.select(1);
  await flush();
  const latest = session.select(6);
  active.resolve("old");
  await first;
  expect(await latest).toBe("6"); await flush();
  expect(load.mock.calls.map(([index]) => index).slice(0, 3)).toEqual([1, 2, 6]);
  session.close();
});
