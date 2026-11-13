// @vitest-environment happy-dom

import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";

const nativeWindow = vi.hoisted(() => ({
  scaleFactor: vi.fn().mockResolvedValue(1),
  setMinSize: vi.fn().mockResolvedValue(undefined),
  setSize: vi.fn().mockResolvedValue(undefined),
  onResized: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => nativeWindow }));
vi.mock("$lib/utils/preloadTabs", () => ({ preloadTabs: () => vi.fn() }));
vi.mock("$lib/i18n", async () => {
  const { writable } = await import("svelte/store");
  return { locale: writable((key: string) => key) };
});
vi.mock("$lib/config/vestaConfig", () => ({
  getItem: () => null,
  removeItem: vi.fn(),
  setItem: vi.fn(),
  flush: vi.fn(),
  isNewInstallation: () => false,
}));
vi.mock("$lib/stores/ankiStore.svelte", () => ({ ankiStore: { checkConnection: vi.fn() } }));
vi.mock("$lib/stores/aiStore.svelte", () => ({ aiStore: { killSwitchActive: false } }));
vi.mock("$lib/stores/snackbarStore.svelte", () => ({ snackbar: { message: null } }));
vi.mock("$lib/components/Sidebar.svelte", () => import("./fixtures/AppTestSidebar.svelte"));
vi.mock("$lib/components/ShortcutOverlay.svelte", () => import("./fixtures/AppTestStub.svelte"));
vi.mock("$lib/components/AppContextMenu.svelte", () => import("./fixtures/AppTestStub.svelte"));
vi.mock("$lib/components/Snackbar.svelte", () => import("./fixtures/AppTestStub.svelte"));
vi.mock("$lib/tabs/ExtractTab.svelte", () => import("./fixtures/AppTestStub.svelte"));
vi.mock("$lib/tabs/PluginsTab.svelte", () => import("./fixtures/AppTestStub.svelte"));
vi.mock("$lib/tabs/FlashcardsTab.svelte", () => import("./fixtures/AppTabProbe.svelte"));
vi.mock("$lib/tabs/TranslateTab.svelte", () => import("./fixtures/AppTabProbe.svelte"));
vi.mock("$lib/tabs/SettingsTab.svelte", () => import("./fixtures/AppTabProbe.svelte"));

import App from "../../App.svelte";
import { SupportLogStore } from "$lib/stores/supportLogStore.svelte";

const mounted: any[] = [];
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((yes) => { resolve = yes; });
  return { promise, resolve };
}

async function render() {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(App, { target, intro: false });
  mounted.push(component);
  await tick();
  return { target, component };
}

afterEach(async () => {
  while (mounted.length) await unmount(mounted.pop()!);
  document.body.replaceChildren();
  delete (window as any).__appTabLifecycle;
  delete (window as any).changeTab;
  nativeWindow.scaleFactor.mockReset().mockResolvedValue(1);
  nativeWindow.setMinSize.mockReset().mockResolvedValue(undefined);
  nativeWindow.setSize.mockReset().mockResolvedValue(undefined);
  nativeWindow.onResized.mockReset().mockResolvedValue(() => {});
});

it("keeps loaded tabs mounted across navigation and updates their active prop", async () => {
  const diagnostics = new SupportLogStore();
  const { target, component } = await render();
  expect(target.querySelectorAll('[data-testid="tab-probe"]')).toHaveLength(1);
  expect(target.textContent).not.toContain("common.loading");
  const active = () => target.querySelector<HTMLElement>('[data-testid="tab-probe"][data-active="true"]');
  active()!.querySelector<HTMLButtonElement>('[data-testid="run-operation"]')!.click();
  await tick();
  expect(active()!.querySelector('[data-testid="operation-count"]')!.textContent).toBe("1");
  target.querySelector<HTMLButtonElement>('[data-testid="switch-translate"]')!.click();
  await vi.waitFor(() => expect(target.querySelectorAll('[data-testid="tab-probe"]')).toHaveLength(2));
  expect(active()!.getAttribute("data-active")).toBe("true");
  const translateProbe = active();
  const flashcardsProbe = [...target.querySelectorAll<HTMLElement>('[data-testid="tab-probe"]')].find((probe) => probe !== translateProbe)!;
  expect((window as any).__appTabLifecycle).toEqual({ mounted: 2, destroyed: 0 });
  const consumed = new KeyboardEvent("keydown", { key: "1", altKey: true, bubbles: true, cancelable: true });
  consumed.preventDefault();
  window.dispatchEvent(consumed);
  await tick();
  expect(active()).toBe(translateProbe);
  const composing = new KeyboardEvent("keydown", { key: "1", altKey: true, isComposing: true, bubbles: true });
  window.dispatchEvent(composing);
  await tick();
  expect(active()).toBe(translateProbe);
  expect(flashcardsProbe.getAttribute("data-active")).toBe("false");
  target.querySelector<HTMLButtonElement>('[data-testid="switch-flashcards"]')!.click();
  await tick();

  expect(active()!.querySelector('[data-testid="operation-count"]')!.textContent).toBe("1");
  expect((window as any).__appTabLifecycle).toEqual({ mounted: 2, destroyed: 0 });
  await unmount(component);
  mounted.splice(mounted.indexOf(component), 1);
  expect((window as any).__appTabLifecycle).toEqual({ mounted: 2, destroyed: 2 });
  expect(diagnostics.preview.filter(entry => entry.kind === "test.tab").map(entry => entry.message).sort())
    .toEqual(["active", "active", "active", "inactive", "inactive"]);
});

it("unlistens when App unmounts while native resize registration is pending", async () => {
  const registration = deferred<() => void>();
  const unlisten = vi.fn();
  nativeWindow.onResized.mockReturnValue(registration.promise);
  const { component } = await render();
  await vi.waitFor(() => expect(nativeWindow.onResized).toHaveBeenCalledOnce());

  await unmount(component);
  mounted.splice(mounted.indexOf(component), 1);
  registration.resolve(unlisten);
  await vi.waitFor(() => expect(unlisten).toHaveBeenCalledOnce());
});


it("opens Diagnostics with the global Alt+9 shortcut", async () => {
  const { target } = await render();
  const event = new KeyboardEvent("keydown", { key: "9", altKey: true, cancelable: true });
  window.dispatchEvent(event);
  expect(event.defaultPrevented).toBe(true);
  await vi.waitFor(() => expect(target.querySelector('[data-active="true"][data-section="diagnostics"]')).not.toBeNull());
});

it("switches to Flashcards when vesta-plugin-open-flashcards is dispatched", async () => {
  const { target } = await render();
  target.querySelector<HTMLButtonElement>('[data-testid="switch-translate"]')!.click();
  await vi.waitFor(() => expect(target.querySelectorAll('[data-testid="tab-probe"]')).toHaveLength(2));

  const probes = [...target.querySelectorAll<HTMLElement>('[data-testid="tab-probe"]')];
  const flashcardsProbe = probes.find((probe) => probe.getAttribute("data-active") === "false")!;
  expect(flashcardsProbe).toBeDefined();

  window.dispatchEvent(
    new CustomEvent("vesta-plugin-open-flashcards", {
      detail: { videoPath: "/tmp/movie.mp4", subtitlePath: "/tmp/movie.srt" },
    })
  );
  await tick();
  expect(flashcardsProbe.getAttribute("data-active")).toBe("true");
});

it("switches to Extract and Plugins with Alt+Shift+E and Alt+Shift+P", async () => {
  const { target } = await render();
  const tabStubWrapper = (propCount: string) =>
    [...target.querySelectorAll<HTMLElement>(".flex-1 > div")].find(
      (wrapper) => wrapper.querySelector('[data-testid="app-stub"]')?.textContent === propCount
    );
  const flashcardsProbe = target.querySelector<HTMLElement>('[data-testid="tab-probe"]')!;
  expect(flashcardsProbe.getAttribute("data-active")).toBe("true");

  const extractEvent = new KeyboardEvent("keydown", { key: "E", altKey: true, shiftKey: true, cancelable: true });
  window.dispatchEvent(extractEvent);
  expect(extractEvent.defaultPrevented).toBe(true);
  await tick();

  const extractWrapper = tabStubWrapper("1")!;
  expect(extractWrapper).toBeDefined();
  expect(extractWrapper.classList.contains("hidden")).toBe(false);
  expect(flashcardsProbe.getAttribute("data-active")).toBe("false");

  const pluginsEvent = new KeyboardEvent("keydown", { key: "p", altKey: true, shiftKey: true, cancelable: true });
  window.dispatchEvent(pluginsEvent);
  expect(pluginsEvent.defaultPrevented).toBe(true);
  await vi.waitFor(() => expect(tabStubWrapper("3")).toBeDefined());

  const pluginsWrapper = tabStubWrapper("3")!;
  expect(pluginsWrapper.classList.contains("hidden")).toBe(false);
  expect(extractWrapper.classList.contains("hidden")).toBe(true);

  const extractLowerEvent = new KeyboardEvent("keydown", { key: "e", altKey: true, shiftKey: true, cancelable: true });
  window.dispatchEvent(extractLowerEvent);
  expect(extractLowerEvent.defaultPrevented).toBe(true);
  await tick();

  expect(extractWrapper.classList.contains("hidden")).toBe(false);
  expect(pluginsWrapper.classList.contains("hidden")).toBe(true);
});
