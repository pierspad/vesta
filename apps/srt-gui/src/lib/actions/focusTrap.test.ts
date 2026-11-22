// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { focusTrap } from "./focusTrap";

const cleanups: Array<() => void> = [];

afterEach(() => {
  while (cleanups.length > 0) {
    cleanups.pop()?.();
  }
  document.body.innerHTML = "";
});

function keyTab(shiftKey = false) {
  return new KeyboardEvent("keydown", {
    key: "Tab",
    shiftKey,
    bubbles: true,
    cancelable: true,
  });
}

describe("focusTrap action", () => {
  it("focuses first control on activate, cycles Tab and Shift+Tab, handles Escape, and restores focus", () => {
    const trigger = document.createElement("button");
    trigger.textContent = "Open Modal";
    document.body.appendChild(trigger);
    trigger.focus();
    expect(document.activeElement).toBe(trigger);

    const dialog = document.createElement("div");
    const first = document.createElement("button");
    first.textContent = "First";
    const middle = document.createElement("input");
    const disabled = document.createElement("button");
    disabled.disabled = true;
    const last = document.createElement("button");
    last.textContent = "Last";
    dialog.append(first, middle, disabled, last);
    document.body.appendChild(dialog);

    const onEscape = vi.fn();
    const action = focusTrap(dialog, { enabled: true, onEscape });
    cleanups.push(() => action.destroy());

    // Initial focus lands on the first focusable control inside dialog.
    expect(document.activeElement).toBe(first);

    // Shift+Tab from first wraps to last (skipping disabled button).
    const wrapBack = keyTab(true);
    document.dispatchEvent(wrapBack);
    expect(wrapBack.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(last);

    // Tab from last wraps to first.
    const wrapForward = keyTab(false);
    document.dispatchEvent(wrapForward);
    expect(wrapForward.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(first);

    // Tab from middle does not wrap prematurely.
    middle.focus();
    const normalTab = keyTab(false);
    document.dispatchEvent(normalTab);
    expect(normalTab.defaultPrevented).toBe(false);

    // Escape invokes callback.
    const esc = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
    document.dispatchEvent(esc);
    expect(esc.defaultPrevented).toBe(true);
    expect(onEscape).toHaveBeenCalledOnce();

    // Destroying restores focus back to the trigger.
    action.destroy();
    expect(document.activeElement).toBe(trigger);
  });

  it("supports toggling enabled off and back on", () => {
    const opener = document.createElement("button");
    document.body.appendChild(opener);
    opener.focus();

    const dialog = document.createElement("div");
    const btn = document.createElement("button");
    dialog.appendChild(btn);
    document.body.appendChild(dialog);

    const action = focusTrap(dialog, { enabled: false });
    cleanups.push(() => action.destroy());
    expect(document.activeElement).toBe(opener);

    action.update({ enabled: true });
    expect(document.activeElement).toBe(btn);

    action.update({ enabled: false });
    expect(document.activeElement).toBe(opener);
  });
});
