// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import TierListKeyboardHarness from "../test/fixtures/TierListKeyboardHarness.svelte";

let component: ReturnType<typeof mount> | undefined;
afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

it("toggles from header keyboard input but ignores keys dispatched by nested actions", async () => {
  const onAction = vi.fn();
  const host = document.createElement("div");
  document.body.append(host);
  component = mount(TierListKeyboardHarness, { target: host, intro: false, props: { onAction } });
  await tick();

  const headers = [...host.querySelectorAll<HTMLElement>('[role="button"]')];
  expect(headers[0].getAttribute("aria-expanded")).toBe("true");
  headers[0].dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true }));
  await tick();
  expect(headers[0].getAttribute("aria-expanded")).toBe("false");
  expect([...host.querySelectorAll('[data-testid="body"]')].map((body) => body.textContent)).toEqual(["second"]);

  headers[0].dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  await tick();
  expect(headers[0].getAttribute("aria-expanded")).toBe("true");

  headers[0].querySelector<HTMLButtonElement>('button[aria-label="Move down"]')
    ?.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  await tick();
  expect(headers[0].getAttribute("aria-expanded")).toBe("true");
  expect(onAction).not.toHaveBeenCalled();
  expect([...host.querySelectorAll('[data-testid="body"]')].map((body) => body.textContent)).toEqual(["first", "second"]);

  headers[0].querySelector<HTMLButtonElement>('button[aria-label="Remove"]')
    ?.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  await tick();
  expect(headers[0].getAttribute("aria-expanded")).toBe("true");
  expect(onAction).not.toHaveBeenCalled();
});
