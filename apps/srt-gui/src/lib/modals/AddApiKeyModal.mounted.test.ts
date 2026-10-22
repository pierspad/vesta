// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import AddApiKeyModal from "./AddApiKeyModal.svelte";
import { apiKeyEditorStore as store } from "$lib/stores/apiKeyEditorStore.svelte";
const instances: ReturnType<typeof mount>[]=[];
afterEach(async()=>{for(const instance of instances.splice(0)) await unmount(instance);store.reset();document.body.replaceChildren();});
async function render(){const target=document.createElement("div");document.body.append(target);instances.push(mount(AddApiKeyModal,{target,props:{onSave:vi.fn()}}));await tick();return target;}
it("shows only static provider identity when editing and preserves provider data",async()=>{
 store.openEdit({id:"local",apiType:"local",name:"Local server",apiKey:"",apiUrl:"http://localhost:11434/v1"},"llm");
 const target=await render();expect(target.querySelector('[data-testid="editing-provider"]')?.textContent).toContain("Local");
 expect(target.querySelectorAll('button:disabled')).toHaveLength(0);
 store.selectProvider("openai");expect(store.newKeyType).toBe("local");expect(store.newKeyUrl).toContain("11434");
});
it("keeps the provider catalog selectable for a new configuration",async()=>{
 store.openAdd("llm");const target=await render();expect(target.querySelector('[data-testid="editing-provider"]')).toBeNull();
 const button=[...target.querySelectorAll("button")].find(button=>button.querySelector("span.text-sm.font-bold")?.textContent?.trim().startsWith("Mistral"))!;
 button.click();await tick();expect(store.newKeyType).toBe("mistral");
});
it("offers deletion only for removable configurations and delegates to confirmation",async()=>{
 store.openEdit({id:"cloud-1",apiType:"openai",name:"My API",apiKey:"example"},"llm");
 const onDelete=vi.fn(),target=document.createElement("div");document.body.append(target);
 instances.push(mount(AddApiKeyModal,{target,props:{onSave:vi.fn(),onDelete}}));await tick();
 target.querySelector<HTMLButtonElement>('[data-provider-action="delete"]')!.click();
 expect(onDelete).toHaveBeenCalledWith("cloud-1");expect(store.showAddKey).toBe(true);
 store.openEdit({id:"local",apiType:"local",name:"Local server",apiKey:"",apiUrl:"http://localhost:11434/v1"},"llm");await tick();
 expect(target.querySelector('[data-provider-action="delete"]')).toBeNull();
});
