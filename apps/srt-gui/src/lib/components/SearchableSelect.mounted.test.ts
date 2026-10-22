// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import SearchableSelect from "./SearchableSelect.svelte";
const instances: ReturnType<typeof mount>[] = [];
afterEach(async () => { for (const instance of instances.splice(0)) await unmount(instance); document.body.replaceChildren(); vi.restoreAllMocks(); });
async function render() {
 const target = document.createElement("div"); document.body.append(target);
 const onchange = vi.fn();
 instances.push(mount(SearchableSelect, {target, props: {options:[{value:"llm",label:"LLM"},{value:"stt",label:"STT"}],value:"llm",onchange,portal:true}}));
 const anchor = target.firstElementChild!;
 vi.spyOn(anchor,"getBoundingClientRect").mockReturnValue({left:24,right:324,top:80,bottom:120,width:300,height:40} as DOMRect);
 const input = target.querySelector("input")!; input.focus(); await tick();
 return {target,input,onchange};
}
it("escapes scroll clipping, stays below its input, and selects a portalled option", async () => {
 const {target,onchange} = await render();
 const menu = document.body.querySelector<HTMLDivElement>(".searchable-select-dropdown")!;
 expect(menu.parentElement).toBe(document.body); expect(target.contains(menu)).toBe(false);
 expect(menu.style.position).toBe("fixed"); expect(menu.style.top).toBe("124px"); expect(menu.style.width).toBe("300px");
 (menu.querySelectorAll("button")[1] as HTMLButtonElement).click(); await tick();
 expect(onchange).toHaveBeenCalledWith("stt"); expect(menu.isConnected).toBe(false);
});
it("closes outside and removes a portalled menu when the select unmounts", async () => {
 const {input} = await render();
 document.body.click(); await tick(); expect(document.querySelector(".searchable-select-dropdown")).toBeNull();
 input.blur(); input.focus(); await tick(); expect(document.querySelector(".searchable-select-dropdown")).not.toBeNull();
 await unmount(instances.pop()!); expect(document.querySelector(".searchable-select-dropdown")).toBeNull();
});
it("closes on outside pointerdown even when bubbling is stopped", async () => {
 await render();
 const outside = document.createElement("button"); document.body.append(outside);
 outside.addEventListener("pointerdown", event => event.stopPropagation());
 outside.dispatchEvent(new Event("pointerdown", {bubbles:true})); await tick();
 expect(document.querySelector(".searchable-select-dropdown")).toBeNull();
});
it("closes with Escape and allows opening again", async () => {
 const {input} = await render();
 input.dispatchEvent(new KeyboardEvent("keydown", {key:"Escape",bubbles:true})); await tick();
 expect(document.querySelector(".searchable-select-dropdown")).toBeNull();
 input.focus(); await tick();
 expect(document.querySelector(".searchable-select-dropdown")).not.toBeNull();
});
it("toggles an open select closed with its arrow button", async () => {
 const {target} = await render();
 const toggle = target.querySelector<HTMLButtonElement>('button[aria-expanded]')!;
 expect(toggle.getAttribute("aria-expanded")).toBe("true");
 toggle.click(); await tick();
 expect(document.querySelector(".searchable-select-dropdown")).toBeNull();
 toggle.click(); await tick();
 expect(document.querySelector(".searchable-select-dropdown")).not.toBeNull();
});
