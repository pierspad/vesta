// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount,tick,unmount } from "svelte";
import PathPickerField from "./PathPickerField.svelte";
import { setLanguage, getLanguage } from "$lib/i18n";
let instance:ReturnType<typeof mount>|undefined;
afterEach(async()=>{if(instance)await unmount(instance);instance=undefined;setLanguage("en");await vi.waitFor(()=>expect(getLanguage()).toBe("en"));document.body.replaceChildren();});
it("updates the default browse label when language changes",async()=>{
 setLanguage("de");await vi.waitFor(()=>expect(getLanguage()).toBe("de"));
 const target=document.createElement("div");document.body.append(target);
 instance=mount(PathPickerField,{target,props:{value:"",placeholder:"Path",browseTitle:"Browse",onbrowse:vi.fn()}});await tick();
 expect(target.textContent).toContain("Durchsuchen");
 setLanguage("ru");await vi.waitFor(()=>expect(getLanguage()).toBe("ru"));await tick();
 expect(target.textContent).not.toContain("Durchsuchen");
 expect(target.textContent).toContain("Обзор");
});
