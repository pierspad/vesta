// @vitest-environment jsdom
import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import Sidebar from "./Sidebar.svelte";

const storage = vi.hoisted(() => new Map<string, string>());
vi.mock("$lib/config/vestaConfig", () => ({
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, value: string) => storage.set(key, value),
  removeItem: (key: string) => storage.delete(key),
}));

vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: vi.fn().mockResolvedValue({ version: "1.0.0", license: "GPL-3.0-only" }),
}));

vi.mock("$lib/stores/updateCheckerStore.svelte", () => ({
  updateCheckerStore: {
    updateStatus: "none",
    latestVersion: "",
    init: vi.fn(),
  },
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

it("unlocks experimental tab via 7 clicks on logo within 4 seconds", async () => {
  storage.set("vesta-experimental-unlocked", "false");

  const target = document.createElement("div");
  document.body.append(target);

  const onTabChange = vi.fn();
  components.push(
    mount(Sidebar, {
      target,
      props: {
        activeTab: "flashcards",
        onTabChange,
      },
    })
  );

  await tick();

  // Initially hidden
  expect(target.textContent).not.toContain("Experimental");

  const logoBtn = target.querySelector<HTMLButtonElement>('button[aria-label="vesta"]');
  expect(logoBtn).not.toBeNull();

  // Click 6 times: not yet unlocked
  for (let i = 0; i < 6; i++) {
    logoBtn?.click();
  }
  await tick();
  expect(target.textContent).not.toContain("Experimental");

  // 7th click: unlocks!
  logoBtn?.click();
  await tick();

  expect(target.textContent).toContain("Experimental");
  expect(storage.get("vesta-experimental-unlocked")).toBe("true");

  // Test toggling back off via event
  window.dispatchEvent(new CustomEvent("vesta-experimental-toggled", { detail: { unlocked: false } }));
  await tick();
  expect(target.textContent).not.toContain("Experimental");
});
