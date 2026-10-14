// @vitest-environment happy-dom

import { mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import CopyPathChip from "$lib/components/CopyPathChip.svelte";
import Snackbar from "$lib/components/Snackbar.svelte";
import SnackbarStoreHarness from "$lib/test/fixtures/SnackbarStoreHarness.svelte";
import { SnackbarStore, snackbar } from "$lib/stores/snackbarStore.svelte";

const mounted: Record<string, any>[] = [];
const stores: SnackbarStore[] = [];
const originalHiddenDescriptor = Object.getOwnPropertyDescriptor(document, "hidden");

afterEach(async () => {
  while (mounted.length > 0) {
    await unmount(mounted.pop()!);
  }
  for (const store of stores.splice(0)) store.clear();
  snackbar.clear();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  document.body.replaceChildren();
  if (originalHiddenDescriptor) Object.defineProperty(document, "hidden", originalHiddenDescriptor);
  else Reflect.deleteProperty(document, "hidden");
});

async function render(props: Record<string, unknown> = {}) {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(Snackbar, {
    target,
    intro: false,
    props: {
      message: "Saved",
      onclose: vi.fn(),
      duration: 5000,
      ...props,
    },
  });
  mounted.push(component);
  await tick();
  return target;
}

describe("Snackbar mounted component", () => {
  it("pauses on hover and focus, and resumes only after focus leaves the snackbar", async () => {
    const onpause = vi.fn();
    const onresume = vi.fn();
    const onclose = vi.fn();
    const target = await render({ onpause, onresume, onclose });
    const snackbar = target.querySelector("[role=status]")!;
    const close = snackbar.querySelector("button")!;
    const outside = document.createElement("button");
    document.body.append(outside);

    snackbar.dispatchEvent(new MouseEvent("mouseenter"));
    snackbar.dispatchEvent(new MouseEvent("mouseleave"));
    snackbar.dispatchEvent(new FocusEvent("focusin", { bubbles: true, relatedTarget: null }));
    snackbar.dispatchEvent(new FocusEvent("focusout", { bubbles: true, relatedTarget: close }));
    snackbar.dispatchEvent(new FocusEvent("focusout", { bubbles: true, relatedTarget: outside }));
    close.click();

    expect(onpause.mock.calls).toEqual([["hover"], ["focus"]]);
    expect(onresume.mock.calls).toEqual([["visibility"], ["hover"], ["focus"]]);
    expect(onclose).toHaveBeenCalledOnce();
  });

  it("subscribes to visibility while mounted and removes the listener on unmount", async () => {
    const onpause = vi.fn();
    const onresume = vi.fn();
    Object.defineProperty(document, "hidden", { configurable: true, value: true });
    const target = await render({ onpause, onresume });

    expect(onpause).toHaveBeenCalledTimes(1);
    expect(onpause).toHaveBeenLastCalledWith("visibility");

    Object.defineProperty(document, "hidden", { configurable: true, value: false });
    document.dispatchEvent(new Event("visibilitychange"));
    expect(onresume).toHaveBeenCalledTimes(1);
    expect(onresume).toHaveBeenLastCalledWith("visibility");

    const component = mounted.pop()!;
    await unmount(component);
    expect(onresume).toHaveBeenCalledTimes(2);

    document.dispatchEvent(new Event("visibilitychange"));
    expect(onresume).toHaveBeenCalledTimes(2);
    target.remove();
  });

  it("keeps the real store timer paused until every interaction reason resumes", async () => {
    vi.useFakeTimers();
    const store = new SnackbarStore();
    stores.push(store);
    store.show("Saved", "success");
    const target = document.createElement("div");
    document.body.append(target);
    const component = mount(SnackbarStoreHarness, { target, props: { store }, intro: false });
    mounted.push(component);
    await tick();
    const status = target.querySelector("[role=status]")!;
    const outside = document.createElement("button");
    document.body.append(outside);

    await vi.advanceTimersByTimeAsync(500);
    status.dispatchEvent(new MouseEvent("mouseenter"));
    status.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    await vi.advanceTimersByTimeAsync(5000);
    expect(store.message).toBe("Saved");

    status.dispatchEvent(new MouseEvent("mouseleave"));
    await vi.advanceTimersByTimeAsync(5000);
    expect(store.message).toBe("Saved");

    status.dispatchEvent(new FocusEvent("focusout", { bubbles: true, relatedTarget: outside }));
    await vi.advanceTimersByTimeAsync(1199);
    expect(store.message).toBe("Saved");
    await vi.advanceTimersByTimeAsync(1);
    await tick();
    expect(store.message).toBeNull();
    expect(target.querySelector("[role=status]")).toBeNull();
  });

  it("pauses the real timer for a hidden document and detaches visibility on unmount", async () => {
    vi.useFakeTimers();
    const store = new SnackbarStore();
    stores.push(store);
    store.show("Still here", "info");
    Object.defineProperty(document, "hidden", { configurable: true, value: true });
    const target = document.createElement("div");
    document.body.append(target);
    const component = mount(SnackbarStoreHarness, { target, props: { store }, intro: false });
    mounted.push(component);
    await tick();

    expect(store.paused).toBe(true);
    await vi.advanceTimersByTimeAsync(5000);
    expect(store.message).toBe("Still here");

    mounted.splice(mounted.indexOf(component), 1);
    await unmount(component);
    document.dispatchEvent(new Event("visibilitychange"));
    expect(store.paused).toBe(false);
  });

  it("routes a denied mounted CopyPathChip write to an error, never a success toast", async () => {
    snackbar.clear();
    const writeText = vi.fn().mockRejectedValue(new Error("Permission denied"));
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(mount(CopyPathChip, {
      target,
      intro: false,
      props: { path: "/tmp/output.srt", copiedMessage: "Copied", tooltip: "Copy path" },
    }));

    target.querySelector<HTMLButtonElement>('button[aria-label="Copy path: /tmp/output.srt"]')!.click();
    await vi.waitFor(() => expect(snackbar.variant).toBe("error"));

    expect(writeText).toHaveBeenCalledWith("/tmp/output.srt");
    expect(snackbar.message).not.toBe("Copied");
    expect(snackbar.variant).toBe("error");
  });
});
