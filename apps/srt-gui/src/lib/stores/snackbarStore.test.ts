import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { SnackbarStore, SNACKBAR_MAX_PENDING, createSnackbarNotifier, snackbar } from "./snackbarStore.svelte";
beforeEach(() => vi.useFakeTimers());
afterEach(() => { snackbar.clear(); vi.useRealTimers(); });
it("uses ordinary timing but never shortens error reading time", () => {
 snackbar.show("Saved", "success"); vi.advanceTimersByTime(1700); expect(snackbar.message).toBeNull();
 snackbar.show("Download failed", "error", 1300); expect(snackbar.duration).toBe(5000);
 vi.advanceTimersByTime(4999); expect(snackbar.message).toBe("Download failed");
 vi.advanceTimersByTime(1); expect(snackbar.message).toBeNull();
});
it("queues simultaneous messages and gives waiting errors priority", () => {
 const notify = createSnackbarNotifier(); notify("First"); notify("Saved", "success"); notify("Failed", "error");
 expect(snackbar.message).toBe("First"); vi.advanceTimersByTime(1700); expect(snackbar.message).toBe("Failed");
 notify("Another success", "success"); vi.advanceTimersByTime(4999); expect(snackbar.message).toBe("Failed");
 vi.advanceTimersByTime(1); expect(snackbar.message).toBe("Saved");
});
it("coalesces duplicates without repeatedly restarting their timers", () => {
 snackbar.show("Saved"); vi.advanceTimersByTime(1000); snackbar.show("Saved"); snackbar.show("Next"); snackbar.show("Next");
 vi.advanceTimersByTime(700); expect(snackbar.message).toBe("Next");
 vi.advanceTimersByTime(1700); expect(snackbar.message).toBeNull();
});
it("pauses for overlapping pointer, keyboard and background reading", () => {
 snackbar.show("Failed", "error"); vi.advanceTimersByTime(1000); snackbar.pause("pointer"); snackbar.pause("focus");
 vi.advanceTimersByTime(60_000); expect(snackbar.message).toBe("Failed");
 snackbar.resume("pointer"); vi.advanceTimersByTime(60_000); expect(snackbar.message).toBe("Failed");
 snackbar.resume("focus"); vi.advanceTimersByTime(3999); expect(snackbar.message).toBe("Failed");
 vi.advanceTimersByTime(1); expect(snackbar.message).toBeNull();
});
it("ignores stale dismissal handlers and preserves explicit longer durations", () => {
 const store = new SnackbarStore(); store.show("First"); const firstKey = store.key;
 store.show("Second", "warning", 7000); store.close(); store.close(firstKey);
 expect(store.message).toBe("Second"); expect(store.duration).toBe(7000); store.clear();
});
it("bounds notification floods without allowing info to displace queued errors", () => {
 const store = new SnackbarStore(); store.show("Active");
 for (let i = 0; i < SNACKBAR_MAX_PENDING; i++) store.show(`Error ${i}`, "error");
 for (let i = 0; i < 100; i++) store.show(`Info ${i}`);
 for (let i = 0; i < SNACKBAR_MAX_PENDING; i++) { store.close(); expect(store.message).toBe(`Error ${i}`); }
 store.close(); expect(store.message).toBeNull(); store.clear();
});
it("handles empty messages and invalid durations without immediate dismissal", () => {
 snackbar.show("   "); expect(snackbar.message).toBeNull();
 snackbar.show("Error", "error", NaN); expect(snackbar.duration).toBe(5000);
});

it("keeps explicitly persistent messages until closed", () => {
  snackbar.show("Needs attention", "error", 0);
  vi.advanceTimersByTime(60000);
  expect(snackbar.message).toBe("Needs attention");
  snackbar.close();
  expect(vi.getTimerCount()).toBe(0);
});

it.each([-1, NaN, Infinity])("falls back for invalid duration %s", duration => {
  snackbar.show("Saved", "success", duration);
  expect(snackbar.duration).toBe(1700);
  vi.advanceTimersByTime(1700);
  expect(snackbar.message).toBeNull();
});

it("allows time to read long messages while preserving the minimum reading time", () => {
  snackbar.show("word ".repeat(100), "warning");
  expect(snackbar.duration).toBeGreaterThan(3500);
  expect(snackbar.duration).toBeLessThanOrEqual(15000);
  snackbar.clear();
  snackbar.show("Short warning", "warning", 7000);
  expect(snackbar.duration).toBe(7000);
});

it("preserves remaining reading time across overlapping hover and focus", () => {
  snackbar.show("Saved", "success", 3000);
  vi.advanceTimersByTime(1000);
  snackbar.pause("hover");
  snackbar.pause("focus");
  vi.advanceTimersByTime(10000);
  snackbar.resume("hover");
  vi.advanceTimersByTime(10000);
  expect(snackbar.message).toBe("Saved");
  snackbar.resume("focus");
  vi.advanceTimersByTime(1999);
  expect(snackbar.message).toBe("Saved");
  vi.advanceTimersByTime(1);
  expect(snackbar.message).toBeNull();
});

it("starts a fresh timer when advancing from a paused message", () => {
  snackbar.show("First");
  snackbar.pause();
  snackbar.show("Second", "info", 1000);
  snackbar.close();
  expect(snackbar.paused).toBe(false);
  vi.advanceTimersByTime(1700);
  expect(snackbar.message).toBeNull();
  snackbar.resume();
  expect(vi.getTimerCount()).toBe(0);
});
