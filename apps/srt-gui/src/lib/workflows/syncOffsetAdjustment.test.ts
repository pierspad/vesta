import { expect, it, vi } from "vitest";
import { SyncOffsetAdjustment } from "./syncOffsetAdjustment.svelte";
function setup() {
  let next = 0;
  const frames = new Map<number, FrameRequestCallback>();
  const cancel = vi.fn((id: number) => { frames.delete(id); });
  const state = new SyncOffsetAdjustment(callback => { frames.set(next, callback); return next++; }, cancel);
  const flush = () => { const callbacks = [...frames.values()]; frames.clear(); callbacks.forEach(fn => fn(0)); };
  return { state, frames, cancel, flush };
}
it("coalesces adjustments including the first frame ID zero", () => {
  const s = setup();
  s.state.update(100); s.state.update(500); s.state.update(-100);
  expect(s.state.value).toBe(0);
  expect(s.frames.size).toBe(1);
  expect(s.cancel).toHaveBeenCalledWith(0);
  s.flush(); expect(s.state.value).toBe(500);
});
it("reset cancels pending work and starts from zero", () => {
  const s = setup(); s.state.update(3000); s.state.reset(); s.flush();
  expect(s.state.value).toBe(0);
  s.state.update(-100); s.flush(); expect(s.state.value).toBe(-100);
});
it("disposal cancels a frame and prevents subsequent scheduling", () => {
  const s = setup(); s.state.update(100); s.state.dispose(); s.state.update(100);
  expect(s.frames.size).toBe(0); s.flush(); expect(s.state.value).toBe(0);
});
