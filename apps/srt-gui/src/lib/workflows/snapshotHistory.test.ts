import { afterEach, expect, it, vi } from "vitest";
import { SnapshotHistory } from "./snapshotHistory";
afterEach(() => vi.useRealTimers());
it("groups typing bursts and captures a fresh snapshot after undo within the debounce window", () => {
  vi.useFakeTimers(); let value = "initial";
  const history = new SnapshotHistory(() => value, snapshot => { value = snapshot; });
  history.schedule(); value = "first";
  history.schedule(); value = "second";
  history.undo(); expect(value).toBe("initial");
  history.schedule(); value = "third";
  history.undo(); expect(value).toBe("initial");
});
it("keeps independent bursts and copies mutable data at capture time", () => {
  vi.useFakeTimers(); let value = [{ text: "initial" }];
  const history = new SnapshotHistory(() => value, snapshot => { value = snapshot; });
  history.schedule(); value[0].text = "first";
  vi.advanceTimersByTime(500); history.schedule(); value[0].text = "second";
  history.undo(); expect(value[0].text).toBe("first");
  history.undo(); expect(value[0].text).toBe("initial");
});
it("deduplicates consecutive snapshots and bounds history", () => {
  let value = 0; const history = new SnapshotHistory(() => value, snapshot => { value = snapshot; }, 2);
  history.push(); history.push(); value = 1; history.push(); value = 2; history.push();
  history.undo(); expect(value).toBe(1); history.undo(); expect(value).toBe(1);
  history.undo(); expect(value).toBe(1);
});
it("new-file reset clears snapshots and pending grouping; disposal releases timers", () => {
  vi.useFakeTimers(); let value = "old";
  const history = new SnapshotHistory(() => value, snapshot => { value = snapshot; });
  history.schedule(); history.reset(); value = "new";
  history.schedule(); value = "edited"; history.undo(); expect(value).toBe("new");
  history.schedule(); history.dispose(); expect(vi.getTimerCount()).toBe(0);
});

it("redo restores grouped edits and a fresh edit discards the redo branch", () => {
  vi.useFakeTimers(); let value = "initial";
  const history = new SnapshotHistory(() => value, snapshot => { value = snapshot; });
  history.schedule(); value = "first";
  history.schedule(); value = "second";
  expect(history.undo()).toBe(true); expect(value).toBe("initial");
  expect(history.redo()).toBe(true); expect(value).toBe("second");
  history.undo(); history.schedule(); value = "branch";
  expect(history.redo()).toBe(false);
  history.undo(); expect(value).toBe("initial");
});
it("an explicit operation ends a typing burst so following edits have their own history", () => {
  vi.useFakeTimers(); let value = "initial";
  const history = new SnapshotHistory(() => value, snapshot => { value = snapshot; });
  history.schedule(); value = "typed";
  history.push(); value = "generated";
  history.schedule(); value = "after";
  history.undo(); expect(value).toBe("generated");
  history.undo(); expect(value).toBe("typed");
  history.undo(); expect(value).toBe("initial");
});
