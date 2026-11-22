import { expect, it, vi } from "vitest";
import { AsyncOperation, LatestRequest } from "./asyncControl.svelte";
function deferred() { let resolve!: () => void; const promise = new Promise<void>(done => resolve = done); return { promise, resolve }; }
it("blocks double starts and keeps cancellation locked until both requests settle", async () => {
 for (const first of ["work", "cancel"]) {
  const operation = new AsyncOperation(); const run = operation.begin()!; const cancel = deferred();
  expect(operation.begin()).toBeNull(); const cancelling = operation.cancel(() => cancel.promise);
  expect(run.isCancelled()).toBe(true); expect(operation.cancelling).toBe(true);
  const secondCancel = vi.fn(); expect(await operation.cancel(secondCancel)).toBe(false); expect(secondCancel).not.toHaveBeenCalled();
  if (first === "work") run.finish(); else { cancel.resolve(); await cancelling; }
  expect(operation.busy).toBe(true); expect(operation.begin()).toBeNull();
  if (first === "work") { cancel.resolve(); await cancelling; } else run.finish();
  expect(operation.busy).toBe(false); expect(operation.begin()).not.toBeNull();
 }
});
it("a stale finally cannot release a newer run and a failed cancel is retryable", async () => {
 const operation = new AsyncOperation(); const first = operation.begin()!;
 await expect(operation.cancel(async () => { throw new Error("IPC failed"); })).rejects.toThrow("IPC failed");
 expect(operation.cancelling).toBe(false); expect(first.isCancelled()).toBe(false); expect(operation.busy).toBe(true);
 expect(await operation.cancel(async () => {})).toBe(true); first.finish();
 const next = operation.begin()!; first.finish(); expect(operation.busy).toBe(true); expect(first.isCurrent()).toBe(false);
 next.finish(); expect(operation.busy).toBe(false);
});
it("disposal suppresses late results and new work", () => {
 const operation = new AsyncOperation(); const run = operation.begin()!; operation.dispose();
 expect(run.isCurrent()).toBe(false); expect(run.isCancelled()).toBe(true); run.finish(); expect(operation.begin()).toBeNull();
});
it("invalidates overlapping same-file loads and close/reopen responses", () => {
 const requests = new LatestRequest(); const first = requests.begin(); const second = requests.begin();
 expect(first.isCurrent()).toBe(false); expect(second.isCurrent()).toBe(true);
 requests.invalidate(); expect(second.isCurrent()).toBe(false); expect(requests.begin().isCurrent()).toBe(true);
});
