// @vitest-environment jsdom
import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import ExperimentalTab from "./ExperimentalTab.svelte";

const storage = vi.hoisted(() => new Map<string, string>());
vi.mock("$lib/config/vestaConfig", () => ({
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, value: string) => storage.set(key, value),
  removeItem: (key: string) => storage.delete(key),
  flush: vi.fn(),
  resetInstallation: vi.fn(),
}));

const components: ReturnType<typeof mount>[] = [];

beforeEach(() => {
  storage.clear();
  vi.clearAllMocks();
});

afterEach(async () => {
  for (const component of components.splice(0)) await unmount(component);
  document.body.replaceChildren();
});

it("hides experimental tab and returns to flashcards", async () => {
  storage.set("vesta-experimental-unlocked", "true");

  const target = document.createElement("div");
  document.body.append(target);

  const onGoToTab = vi.fn();
  const toggledListener = vi.fn();
  window.addEventListener("vesta-experimental-toggled", toggledListener);

  components.push(
    mount(ExperimentalTab, {
      target,
      props: {
        onGoToTab,
      },
    })
  );

  await tick();

  expect(target.textContent).toContain("Hide Experimental Tab");

  const buttons = Array.from(target.querySelectorAll("button"));
  const hideBtn = buttons.find((b) => b.textContent?.includes("Hide Tab"));
  expect(hideBtn).toBeDefined();

  hideBtn?.click();
  await tick();

  expect(storage.get("vesta-experimental-unlocked")).toBe("false");
  expect(onGoToTab).toHaveBeenCalledWith("flashcards");
  expect(toggledListener).toHaveBeenCalledWith(
    expect.objectContaining({
      detail: { unlocked: false },
    })
  );

  window.removeEventListener("vesta-experimental-toggled", toggledListener);
});
