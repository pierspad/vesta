// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import { SvelteSet } from "svelte/reactivity";

vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("$lib/stores/snackbarStore.svelte", () => ({
  snackbar: { show: vi.fn() },
}));

import ShortcutsTab from "./ShortcutsTab.svelte";

const mounted: ReturnType<typeof mount>[] = [];

afterEach(async () => {
  for (const component of mounted.splice(0)) {
    await unmount(component);
  }
  document.body.replaceChildren();
});

it("stops recording and key-search global window listeners when inactive prop flips to false", async () => {
  const activeFlags = new SvelteSet(["active"]);
  const target = document.createElement("div");
  document.body.append(target);
  mounted.push(
    mount(ShortcutsTab, {
      target,
      props: {
        get active() {
          return activeFlags.has("active");
        },
      },
    }),
  );
  await tick();

  // Switch search bar to "KEYS" mode and click its capture field to register the window listener.
  const keysModeBtn = [...target.querySelectorAll("button")].find(
    (b) => b.getAttribute("aria-pressed") !== null && b.textContent?.includes("KEYS"),
  );
  expect(keysModeBtn).toBeDefined();
  keysModeBtn!.click();
  await tick();

  const capturePrompt = [...target.querySelectorAll("button")].find((b) =>
    b.textContent?.includes("Search by keys"),
  );
  expect(capturePrompt).toBeDefined();
  capturePrompt!.click();
  await tick();

  const whileActive = new KeyboardEvent("keydown", {
    key: "k",
    ctrlKey: true,
    cancelable: true,
  });
  window.dispatchEvent(whileActive);
  expect(whileActive.defaultPrevented).toBe(true);

  activeFlags.delete("active");
  await tick();
  await tick();

  const afterDeactivate = new KeyboardEvent("keydown", {
    key: "k",
    ctrlKey: true,
    cancelable: true,
  });
  window.dispatchEvent(afterDeactivate);
  expect(afterDeactivate.defaultPrevented).toBe(false);
});
