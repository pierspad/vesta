// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import AppTooltip from "./AppTooltip.svelte";

const mounted: ReturnType<typeof mount>[] = [];
afterEach(async () => {
  for (const component of mounted.splice(0)) await unmount(component);
  document.body.replaceChildren();
  vi.useRealTimers();
});

it("connects focused tooltip text to its owner and restores native attributes on blur", async () => {
  const button = document.createElement("button");
  button.title = "Helpful description";
  button.setAttribute("aria-describedby", "existing-description");
  document.body.append(button);
  const host = document.createElement("div");
  document.body.append(host);
  mounted.push(mount(AppTooltip, { target: host, intro: false }));
  await tick();

  button.focus();
  await tick();
  expect(button.hasAttribute("title")).toBe(false);
  expect(button.getAttribute("aria-describedby")).toBe("existing-description app-tooltip");
  expect(host.querySelector('[role="tooltip"]')?.textContent).toBe("Helpful description");

  button.blur();
  await tick();
  expect(button.title).toBe("Helpful description");
  expect(button.getAttribute("aria-describedby")).toBe("existing-description");
});

it("shows immediately when focus arrives during the hover delay", async () => {
  vi.useFakeTimers();
  const button = document.createElement("button");
  button.title = "Keyboard description";
  document.body.append(button);
  const host = document.createElement("div");
  document.body.append(host);
  mounted.push(mount(AppTooltip, { target: host, intro: false }));
  await tick();

  button.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
  expect(host.querySelector('[role="tooltip"]')?.classList.contains("visible")).toBe(false);
  button.focus();
  await tick();
  expect(host.querySelector('[role="tooltip"]')?.textContent).toBe("Keyboard description");
  expect(host.querySelector('[role="tooltip"]')?.classList.contains("visible")).toBe(true);
  await vi.advanceTimersByTimeAsync(500);
  expect(host.querySelector('[role="tooltip"]')?.classList.contains("visible")).toBe(true);
});

it("restores tooltip state and removes global handlers on component teardown", async () => {
  vi.useFakeTimers();
  const button = document.createElement("button");
  button.title = "Delayed description";
  document.body.append(button);
  const host = document.createElement("div");
  document.body.append(host);
  const component = mount(AppTooltip, { target: host, intro: false });
  mounted.push(component);
  await tick();
  button.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
  expect(button.hasAttribute("title")).toBe(false);
  await unmount(component);
  mounted.splice(mounted.indexOf(component), 1);
  expect(button.title).toBe("Delayed description");
  expect(button.hasAttribute("aria-describedby")).toBe(false);
  button.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
  await vi.advanceTimersByTimeAsync(500);
  expect(host.querySelector('[role="tooltip"]')).toBeNull();
});
