// @vitest-environment jsdom
import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import NotificationInbox from "./NotificationInbox.svelte";
import Sidebar from "./Sidebar.svelte";

const updates = vi.hoisted(() => ({
  updateStatus: "available", latestVersion: "v9.0.0", managerHint: "", canInstall: true,
  installing: false, downloadPercent: 0, releaseNotes: "<img src=x> Release notes",
  init: vi.fn(), openRelease: vi.fn(), installUpdate: vi.fn(),
}));
vi.mock("$lib/stores/updateCheckerStore.svelte", () => ({ updateCheckerStore: updates }));
vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: vi.fn().mockResolvedValue({ version: "1.0.0", license: "GPL-3.0-only" }) }));
const components: ReturnType<typeof mount>[] = [];
const originalShowModal = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, "showModal");
const originalClose = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, "close");

beforeEach(() => {
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", { configurable: true, value: function (this: HTMLDialogElement) { this.setAttribute("open", ""); } });
  Object.defineProperty(HTMLDialogElement.prototype, "close", { configurable: true, value: function (this: HTMLDialogElement) { this.removeAttribute("open"); } });
  updates.managerHint = "";
  updates.releaseNotes = "## Changes\n\n* **Audio:** improved previews ([commit](https://github.com/pierspad/vesta/commit/123))\n\n<img src=x onerror=alert(1)>";
  vi.clearAllMocks();
});
afterEach(async () => {
  for (const component of components.splice(0)) await unmount(component);
  document.body.replaceChildren();
  vi.restoreAllMocks();
  for (const [name, descriptor] of [["showModal", originalShowModal], ["close", originalClose]] as const) {
    if (descriptor) Object.defineProperty(HTMLDialogElement.prototype, name, descriptor);
    else Reflect.deleteProperty(HTMLDialogElement.prototype, name);
  }
});

async function renderInbox(hash = "whisper-model-missing:v1|llm-default-unready:v1") {
  const target = document.createElement("div"); document.body.append(target);
  const onsettings = vi.fn(), onclose = vi.fn();
  components.push(mount(NotificationInbox, { target, props: { hash, releaseUrl: "https://github.com/pierspad/vesta/releases/tag/v1.0.0", onsettings, onclose } }));
  await tick();
  return { target, onsettings, onclose };
}

it("opens a modal inbox with actionable configuration and update notices", async () => {
  const { target, onsettings } = await renderInbox();
  expect(target.querySelector("dialog")?.hasAttribute("open")).toBe(true);
  const articles = target.querySelectorAll("article");
  expect(articles).toHaveLength(4);
  (articles[1].querySelector("button") as HTMLButtonElement).click();
  expect(onsettings).toHaveBeenCalledWith("whisper");
  (articles[0].querySelector("button") as HTMLButtonElement).click();
  expect(updates.installUpdate).toHaveBeenCalledOnce();
});

it("renders sanitized Markdown and closes through the close control", async () => {
  const { target, onclose } = await renderInbox("");
  await vi.waitFor(() => expect(target.querySelector(".markdown-content h2")?.textContent).toBe("Changes"));
  expect(target.querySelector(".markdown-content strong")?.textContent).toBe("Audio:");
  expect(target.querySelector(".markdown-content li a")?.getAttribute("target")).toBe("_blank");
  expect(target.querySelector("img")).toBeNull();
  (target.querySelector("button") as HTMLButtonElement).click();
  expect(onclose).toHaveBeenCalledOnce();
});

it("keeps package manager instructions instead of offering its own installer", async () => {
  updates.managerHint = "Update through your package manager";
  const { target } = await renderInbox("");
  const article = target.querySelector("article")!;
  expect(article.textContent).toContain(updates.managerHint);
  expect(article.querySelectorAll("button")).toHaveLength(0);
});

it("closes only when a pointer press starts and ends on the backdrop", async () => {
  const { target, onclose } = await renderInbox("");
  const dialog = target.querySelector("dialog")!;
  vi.spyOn(dialog, "getBoundingClientRect").mockReturnValue({ left: 100, top: 100, right: 500, bottom: 400 } as DOMRect);
  const pointer = (type: string, x: number, y: number) => dialog.dispatchEvent(new MouseEvent(type, { bubbles: true, clientX: x, clientY: y }));
  pointer("pointerdown", 110, 110); pointer("pointerup", 110, 110);
  pointer("pointerdown", 110, 110); pointer("pointerup", 50, 50);
  expect(onclose).not.toHaveBeenCalled();
  pointer("pointerdown", 50, 50); pointer("pointerup", 50, 50);
  expect(onclose).toHaveBeenCalledOnce();
});

it("places update notifications in the inbox rather than on Shortcuts", async () => {
  const target = document.createElement("div"); document.body.append(target);
  const onTabChange = vi.fn();
  components.push(mount(Sidebar, { target, props: { activeTab: "settings", settingsSection: "shortcuts", onTabChange } }));
  await tick();
  const shortcuts = [...target.querySelectorAll("button")].find(button => button.textContent?.trim() === "Shortcuts")!;
  expect(shortcuts).toBeDefined();
  expect(shortcuts.querySelector(".rounded-full")).toBeNull();
  const inbox = target.querySelector<HTMLButtonElement>('[aria-haspopup="dialog"]')!;
  expect(inbox).not.toBeNull();
  inbox.click(); await tick();
  expect(target.querySelector("dialog")).not.toBeNull();
  expect(onTabChange).not.toHaveBeenCalled();
});

it("preserves the opener scroll and restores focus without scrolling on close", async () => {
 const scroller = document.createElement("div"), opener = document.createElement("button");
 scroller.append(opener); document.body.append(scroller); opener.focus(); scroller.scrollTop = 160;
 Object.defineProperty(HTMLDialogElement.prototype, "showModal", {configurable:true,value:function(this:HTMLDialogElement){this.setAttribute("open",""); scroller.scrollTop = 0;}});
 const focus = vi.spyOn(opener,"focus");
 await renderInbox(""); expect(scroller.scrollTop).toBe(160);
 await unmount(components.pop()!);
 expect(focus).toHaveBeenCalledWith({preventScroll:true}); expect(scroller.scrollTop).toBe(160);
});

it("keeps release notes collapsed and exposes a single disclosure action", async () => {
  const { target } = await renderInbox();
  const details = target.querySelector<HTMLButtonElement>(".release-summary")!;
  expect(details.getAttribute("aria-expanded")).toBe("false");
  expect(target.querySelectorAll("article:first-child button")).toHaveLength(1);
  details.click();
  await vi.waitFor(() => expect(details.getAttribute("aria-expanded")).toBe("true"));
  expect(updates.openRelease).not.toHaveBeenCalled();
  details.click();
  await vi.waitFor(() => expect(details.getAttribute("aria-expanded")).toBe("false"));
});
