// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import SearchInput from "./SearchInput.svelte";

let instance: ReturnType<typeof mount> | undefined;
afterEach(async () => {
  if (instance) await unmount(instance);
  instance = undefined;
  document.body.replaceChildren();
});

it("renders with placeholder and magnifying glass icon", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  instance = mount(SearchInput, {
    target,
    props: { value: "", placeholder: "Search something..." },
  });
  await tick();

  const input = target.querySelector("input") as HTMLInputElement;
  expect(input).not.toBeNull();
  expect(input.placeholder).toBe("Search something...");
  expect(target.querySelector("svg")).not.toBeNull();
  // Clear button is not visible when empty
  expect(target.querySelector("button")).toBeNull();
});

it("displays clear button when value is non-empty and clicking it clears value", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const onclear = vi.fn();
  const oninput = vi.fn();
  instance = mount(SearchInput, {
    target,
    props: { value: "Initial query", placeholder: "Filter...", onclear, oninput },
  });
  await tick();

  const input = target.querySelector("input") as HTMLInputElement;
  expect(input.value).toBe("Initial query");

  const clearBtn = target.querySelector("button") as HTMLButtonElement;
  expect(clearBtn).not.toBeNull();

  clearBtn.click();
  await tick();

  expect(input.value).toBe("");
  expect(onclear).toHaveBeenCalledOnce();
  expect(oninput).toHaveBeenCalledWith("");
});
