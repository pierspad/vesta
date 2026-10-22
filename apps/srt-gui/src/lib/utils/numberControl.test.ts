// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { numberControl } from "./numberControl";
let destroy = () => {};
afterEach(() => { destroy(); document.body.replaceChildren(); });
function render(value = "20", step = "1") {
  const input = document.createElement("input"); input.type = "number";
  input.value = value; input.step = step; input.min = "0"; input.max = "500";
  document.body.append(input); destroy = numberControl(input).destroy;
  const change = vi.fn(); input.addEventListener("input", change);
  const key = (key: string, modifiers = {}) => input.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...modifiers }));
  return { input, key, change };
}
it("supports all arrow directions and accelerated steps without exceeding bounds", () => {
  const {input,key,change} = render();
  key("ArrowRight"); expect(input.value).toBe("21");
  key("ArrowUp", {shiftKey:true}); expect(input.value).toBe("31");
  key("ArrowLeft", {ctrlKey:true}); expect(input.value).toBe("0");
  key("ArrowDown"); expect(input.value).toBe("0");
  expect(change).toHaveBeenCalledTimes(4);
});
it("preserves fractional steps and updates existing input bindings from buttons", () => {
  const {input,key,change} = render("1", "0.5");
  key("ArrowDown"); expect(input.value).toBe("0.5");
  document.querySelector<HTMLButtonElement>("button")!.click();
  expect(input.value).toBe("1"); expect(change).toHaveBeenCalledTimes(2);
});
it("does not change readonly controls and restores the DOM on destruction", () => {
  const {input,key} = render(); input.readOnly = true;
  key("ArrowRight"); expect(input.value).toBe("20");
  destroy(); destroy = () => {};
  expect(document.body.firstElementChild).toBe(input);
  expect(document.querySelector("button")).toBeNull();
});
