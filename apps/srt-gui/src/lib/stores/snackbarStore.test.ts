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
