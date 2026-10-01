import { expect, it, vi } from "vitest";
import { preloadTabs } from "./preloadTabs";
it("loads one tab at a time and continues after an error", async () => {
  const scheduled: Array<() => void> = [];
  const load = vi.fn().mockResolvedValueOnce(undefined).mockRejectedValueOnce(new Error("chunk failed")).mockResolvedValue(undefined);
  preloadTabs(["settings", "translate", "sync"], load, callback => { scheduled.push(callback); return () => {}; });
  expect(load).not.toHaveBeenCalled();
  for (const tab of ["settings", "translate", "sync"]) {
    scheduled.shift()!();
    await vi.waitFor(() => expect(load).toHaveBeenLastCalledWith(tab));
    await Promise.resolve(); await Promise.resolve();
  }
  expect(load).toHaveBeenCalledTimes(3);
});
it("cancels remaining preloads after teardown", async () => {
  const queue: Array<() => void> = [];
  const load = vi.fn().mockResolvedValue(undefined);
  const cancel = vi.fn();
  const stop = preloadTabs([1, 2], load, callback => { queue.push(callback); return cancel; });
  stop(); queue[0]();
  expect(load).not.toHaveBeenCalled(); expect(cancel).toHaveBeenCalledOnce();
});
