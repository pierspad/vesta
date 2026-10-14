import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { createSnackbarNotifier, snackbar } from "./snackbarStore.svelte";

beforeEach(() => vi.useFakeTimers());
afterEach(() => { snackbar.close(); vi.useRealTimers(); });

it("dismisses ordinary messages after 1700ms and gives errors more reading time", () => {
  snackbar.show("Saved", "success");
  vi.advanceTimersByTime(1699);
  expect(snackbar.message).toBe("Saved");
  vi.advanceTimersByTime(1);
  expect(snackbar.message).toBeNull();
  snackbar.show("Download failed", "error");
  expect(snackbar.duration).toBe(3500);
  vi.advanceTimersByTime(1700);
  expect(snackbar.message).toBe("Download failed");
  vi.advanceTimersByTime(1800);
  expect(snackbar.message).toBeNull();
});

it("replacing a message cancels its old timer and preserves explicit overrides", () => {
  const notify = createSnackbarNotifier();
  notify("First");
  vi.advanceTimersByTime(1000);
  notify("Second", "warning", 2000);
  vi.advanceTimersByTime(700);
  expect(snackbar.message).toBe("Second");
  expect(snackbar.duration).toBe(2000);
  vi.advanceTimersByTime(1300);
  expect(snackbar.message).toBeNull();
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

it("allows time to read long messages without extending explicit durations", () => {
  snackbar.show("word ".repeat(100), "warning");
  expect(snackbar.duration).toBeGreaterThan(3500);
  expect(snackbar.duration).toBeLessThanOrEqual(15000);
  snackbar.show("word ".repeat(100), "warning", 2000);
  expect(snackbar.duration).toBe(2000);
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

it("starts a fresh timer when replacing a paused message", () => {
  snackbar.show("First");
  snackbar.pause();
  snackbar.show("Second", "info", 1000);
  expect(snackbar.paused).toBe(false);
  vi.advanceTimersByTime(1000);
  expect(snackbar.message).toBeNull();
  snackbar.resume();
  expect(vi.getTimerCount()).toBe(0);
});
