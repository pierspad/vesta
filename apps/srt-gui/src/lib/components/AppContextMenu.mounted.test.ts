// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import AppContextMenu from "./AppContextMenu.svelte";
const clipboard = vi.hoisted(() => ({ read: vi.fn(), write: vi.fn() }));
vi.mock("$lib/utils/clipboard", () => ({ readClipboard: clipboard.read, writeClipboard: clipboard.write }));
let instance: ReturnType<typeof mount> | undefined;
afterEach(async () => { if(instance) await unmount(instance); instance=undefined; document.body.replaceChildren(); vi.clearAllMocks(); });
async function render(readOnly = false) {
 const target = document.createElement("div"); const input = document.createElement("textarea");
 input.value="original text"; input.readOnly=readOnly; document.body.append(target,input);
 instance=mount(AppContextMenu,{target}); await tick();
 input.focus(); input.setSelectionRange(0,8);
 input.dispatchEvent(new MouseEvent("contextmenu",{bubbles:true,cancelable:true,clientX:20,clientY:20})); await tick();
 return input;
}
const item = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].find(button=>button.textContent?.includes(label))!;
it("allows copying and selecting readonly text but disables cut and paste", async () => {
 await render(true);
 expect(item("Copy").disabled).toBe(false); expect(item("Select all").disabled).toBe(false);
 expect(item("Cut").disabled).toBe(true); expect(item("Paste").disabled).toBe(true);
 clipboard.write.mockResolvedValue(true); item("Copy").click(); await tick();
 expect(clipboard.write).toHaveBeenCalledWith("original");
});
it("does not overwrite text changed while awaiting clipboard permission", async () => {
 const input=await render();
 let resolve!: (text: string) => void;
 clipboard.read.mockReturnValue(new Promise<string>(r=>resolve=r));
 item("Paste").click(); input.value="new content"; resolve("clipboard text"); await tick(); await tick();
 expect(input.value).toBe("new content");
});
it("does not show a menu of disabled editing actions on empty space", async () => {
 const input=await render();
 document.querySelector<HTMLElement>('[role="menu"]')!.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true})); await tick();
 input.blur(); window.getSelection()?.removeAllRanges();
 document.body.dispatchEvent(new MouseEvent("contextmenu",{bubbles:true,cancelable:true})); await tick();
 expect(document.querySelector('[role="menu"]')).toBeNull();
});
