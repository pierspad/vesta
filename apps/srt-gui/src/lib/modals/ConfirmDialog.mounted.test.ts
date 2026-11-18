// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";

vi.mock("$lib/i18n", async () => {
  const { writable } = await import("svelte/store");
  return { locale: writable((key: string) => key) };
});

import ConfirmDialogHarness from "$lib/test/fixtures/ConfirmDialogHarness.svelte";

const mounted: ReturnType<typeof mount>[] = [];

afterEach(async () => {
  for (const instance of mounted.splice(0)) await unmount(instance);
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

function renderHarness(props: {
  variant?: "danger" | "warning" | "info" | "success";
  conditionalMount?: boolean;
  onconfirm?: () => void;
  oncancel?: () => void;
} = {}) {
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(ConfirmDialogHarness, { target, props });
  mounted.push(instance);
  return { target, instance };
}

it("moves focus inside on open, traps Tab/Shift+Tab, and restores focus on close", async () => {
  const onconfirm = vi.fn();
  const oncancel = vi.fn();
  const { target } = renderHarness({ variant: "danger", onconfirm, oncancel });

  const trigger = target.querySelector<HTMLButtonElement>('[data-testid="trigger"]')!;
  const outside = target.querySelector<HTMLButtonElement>('[data-testid="outside"]')!;
  trigger.focus();
  expect(document.activeElement).toBe(trigger);

  trigger.click();
  await tick();

  const dialog = target.querySelector<HTMLElement>('[role="dialog"]')!;
  const cancelBtn = target.querySelector<HTMLButtonElement>(".confirm-btn-cancel")!;
  const confirmBtn = target.querySelector<HTMLButtonElement>(".confirm-btn-action")!;
  expect(dialog.getAttribute("aria-describedby")).toBe("confirm-modal-message");
  expect(document.activeElement).toBe(cancelBtn);

  const tabForward = new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true });
  window.dispatchEvent(tabForward);
  expect(tabForward.defaultPrevented).toBe(true);
  expect(document.activeElement).toBe(confirmBtn);

  window.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true }));
  expect(document.activeElement).toBe(cancelBtn);

  window.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", shiftKey: true, bubbles: true, cancelable: true }));
  expect(document.activeElement).toBe(confirmBtn);

  // Focus attempting to escape outside the dialog is pulled back inside.
  outside.focus();
  expect(dialog.contains(document.activeElement)).toBe(true);

  // Confirm via Enter while confirmBtn is focused and verify focus returns to trigger.
  confirmBtn.focus();
  confirmBtn.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
  await tick();
  expect(onconfirm).toHaveBeenCalledOnce();
  expect(target.querySelector('[role="dialog"]')).toBeNull();
  expect(document.activeElement).toBe(trigger);

  // Reopen and verify Enter while Cancel is focused cancels and restores focus.
  trigger.click();
  await tick();
  const reopenedCancelBtn = target.querySelector<HTMLButtonElement>(".confirm-btn-cancel")!;
  expect(document.activeElement).toBe(reopenedCancelBtn);
  reopenedCancelBtn.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
  await tick();
  expect(oncancel).toHaveBeenCalledOnce();
  expect(target.querySelector('[role="dialog"]')).toBeNull();
  expect(document.activeElement).toBe(trigger);
});

it("restores focus when mounted conditionally and unmounted on cancel/Escape", async () => {
  const oncancel = vi.fn();
  const { target } = renderHarness({ variant: "warning", conditionalMount: true, oncancel });

  const trigger = target.querySelector<HTMLButtonElement>('[data-testid="trigger"]')!;
  trigger.focus();
  trigger.click();
  await tick();

  const cancelBtn = target.querySelector<HTMLButtonElement>(".confirm-btn-cancel")!;
  expect(document.activeElement).toBe(cancelBtn);

  window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  await tick();

  expect(oncancel).toHaveBeenCalledOnce();
  expect(target.querySelector('[role="dialog"]')).toBeNull();
  expect(document.activeElement).toBe(trigger);
});

it("focuses the confirm action initially for non-destructive info dialogs", async () => {
  const { target } = renderHarness({ variant: "info" });

  const trigger = target.querySelector<HTMLButtonElement>('[data-testid="trigger"]')!;
  trigger.focus();
  trigger.click();
  await tick();

  const confirmBtn = target.querySelector<HTMLButtonElement>(".confirm-btn-action")!;
  expect(document.activeElement).toBe(confirmBtn);
});
